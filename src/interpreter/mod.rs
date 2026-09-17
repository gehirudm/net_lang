//! A checked tree-walking interpreter. Bindings live in per-run arenas so
//! captured lexical environments do not create reference-counting cycles.
mod error;
mod expression;
mod operations;
use crate::{
    ast::{Pattern, Program, Stmt},
    runtime::{FunctionId, Runtime, Value},
    semantic,
};
pub use error::{ExecutionError, RuntimeError};
use std::{collections::HashMap, sync::Arc};

type Environment = Vec<HashMap<String, usize>>;
type Result<T> = std::result::Result<T, RuntimeError>;

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub steps: usize,
    pub call_depth: usize,
    pub expression_depth: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            steps: 1_000_000,
            call_depth: 128,
            expression_depth: 256,
        }
    }
}

/// Execute top-level statements, then call a top-level `fn main()` if present.
/// Semantic errors are returned before the runtime is invoked.
pub fn execute(
    program: &Program,
    runtime: &mut impl Runtime,
) -> std::result::Result<Value, ExecutionError> {
    execute_with_limits(program, runtime, Limits::default())
}

pub fn execute_with_limits(
    program: &Program,
    runtime: &mut impl Runtime,
    limits: Limits,
) -> std::result::Result<Value, ExecutionError> {
    semantic::analyze(program).map_err(ExecutionError::Semantic)?;
    let mut interpreter = Interpreter {
        runtime,
        bindings: vec![Binding {
            value: Some(Value::Print),
            mutable: false,
        }],
        functions: Vec::new(),
        limits,
        steps: 0,
        expression_depth: 0,
        call_stack: Vec::new(),
    };
    let mut env = vec![HashMap::from([("print".into(), 0)]), HashMap::new()];
    let result = (|| {
        // Reject an invalid entry point before top-level side effects.
        if program.statements.iter().any(|stmt| matches!(stmt, Stmt::Function { name, parameters, .. } if name == "main" && !parameters.is_empty())) {
            return Err(interpreter.error("entry point 'main' must have no parameters"));
        }
        interpreter.statements(&program.statements, &mut env)?;
        if program
            .statements
            .iter()
            .any(|stmt| matches!(stmt, Stmt::Function { name, .. } if name == "main"))
            && let Some(id) = env.last().unwrap().get("main")
            && let Some(Value::Function(function)) = interpreter.bindings[*id].value.clone()
        {
            interpreter.call(Value::Function(function), Vec::new())
        } else {
            Ok(Value::Null)
        }
    })();
    result.map_err(ExecutionError::Runtime)
}

struct Binding {
    value: Option<Value>,
    mutable: bool,
}
#[derive(Clone)]
struct Function {
    name: String,
    parameters: Vec<String>,
    body: Arc<Stmt>,
    closure: Environment,
}
enum Flow {
    Continue,
    Return(Value),
}

struct Interpreter<'a, R: Runtime> {
    runtime: &'a mut R,
    bindings: Vec<Binding>,
    functions: Vec<Function>,
    limits: Limits,
    steps: usize,
    expression_depth: usize,
    call_stack: Vec<String>,
}

impl<R: Runtime> Interpreter<'_, R> {
    fn error(&self, message: impl Into<String>) -> RuntimeError {
        RuntimeError {
            message: message.into(),
            call_stack: self.call_stack.clone(),
        }
    }
    fn tick(&mut self) -> Result<()> {
        if self.steps >= self.limits.steps {
            return Err(self.error("execution step limit exceeded"));
        }
        self.steps += 1;
        Ok(())
    }
    fn allocate(&mut self, value: Option<Value>, mutable: bool) -> usize {
        let id = self.bindings.len();
        self.bindings.push(Binding { value, mutable });
        id
    }
    fn resolve(&self, env: &Environment, name: &str) -> Result<usize> {
        env.iter()
            .rev()
            .find_map(|scope| scope.get(name))
            .copied()
            .ok_or_else(|| self.error(format!("undefined name '{name}'")))
    }
    fn read(&self, id: usize) -> Result<Value> {
        self.bindings[id]
            .value
            .clone()
            .ok_or_else(|| self.error("binding was used before its initializer executed"))
    }

    fn statements(&mut self, statements: &[Stmt], env: &mut Environment) -> Result<Flow> {
        // Reserve binding slots without exposing future variables. All functions
        // are visible immediately, but each captures the names at its declaration.
        let mut slots = Vec::with_capacity(statements.len());
        for stmt in statements {
            let slot = match stmt {
                Stmt::Let { .. } => Some(self.allocate(None, true)),
                Stmt::Function { name, .. } => {
                    let id = self.allocate(None, false);
                    env.last_mut().unwrap().insert(name.clone(), id);
                    Some(id)
                }
                _ => None,
            };
            slots.push(slot);
        }
        let mut declaration_env = env.clone();
        for (stmt, slot) in statements.iter().zip(&slots) {
            match stmt {
                Stmt::Let { name, .. } => {
                    declaration_env
                        .last_mut()
                        .unwrap()
                        .insert(name.clone(), slot.unwrap());
                }
                Stmt::Function {
                    name,
                    parameters,
                    body,
                } => {
                    let function = FunctionId(self.functions.len());
                    self.functions.push(Function {
                        name: name.clone(),
                        parameters: parameters.clone(),
                        body: Arc::new(*body.clone()),
                        closure: declaration_env.clone(),
                    });
                    self.bindings[slot.unwrap()].value = Some(Value::Function(function));
                }
                _ => {}
            }
        }
        for (stmt, slot) in statements.iter().zip(slots) {
            self.tick()?;
            let flow = if let Stmt::Let { name, value } = stmt {
                let value = self.expression(value, env)?;
                let id = slot.unwrap();
                self.bindings[id].value = Some(value);
                env.last_mut().unwrap().insert(name.clone(), id);
                Flow::Continue
            } else {
                self.statement(stmt, env)?
            };
            if let Flow::Return(_) = flow {
                return Ok(flow);
            }
        }
        Ok(Flow::Continue)
    }

    fn body(&mut self, body: &Stmt, env: &mut Environment) -> Result<Flow> {
        match body {
            Stmt::Block(statements) => self.statements(statements, env),
            stmt => self.statements(std::slice::from_ref(stmt), env),
        }
    }
    fn scoped_body(&mut self, body: &Stmt, env: &Environment) -> Result<Flow> {
        let mut child = env.clone();
        child.push(HashMap::new());
        self.body(body, &mut child)
    }
    fn boolean(&self, value: Value) -> Result<bool> {
        if let Value::Boolean(value) = value {
            Ok(value)
        } else {
            Err(self.error(format!("expected boolean, got {}", value.type_name())))
        }
    }

    fn statement(&mut self, stmt: &Stmt, env: &mut Environment) -> Result<Flow> {
        match stmt {
            Stmt::Let { .. } => unreachable!("let statements use their reserved slot"),
            Stmt::Function { .. } => {}
            Stmt::Expression(expr) => {
                self.expression(expr, env)?;
            }
            Stmt::Block(_) => return self.scoped_body(stmt, env),
            Stmt::Return { value } => {
                return Ok(Flow::Return(match value {
                    Some(expr) => self.expression(expr, env)?,
                    None => Value::Null,
                }));
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let value = self.expression(condition, env)?;
                if self.boolean(value)? {
                    return self.scoped_body(then_branch, env);
                }
                if let Some(branch) = else_branch {
                    return self.scoped_body(branch, env);
                }
            }
            Stmt::While { condition, body } => loop {
                let value = self.expression(condition, env)?;
                if !self.boolean(value)? {
                    break;
                }
                if let flow @ Flow::Return(_) = self.scoped_body(body, env)? {
                    return Ok(flow);
                }
            },
            Stmt::For {
                variable,
                iterable,
                body,
            } => {
                let values = self.expression(iterable, env)?;
                let Value::Array(values) = values else {
                    return Err(self.error("for requires an array"));
                };
                for value in values {
                    self.tick()?;
                    let id = self.allocate(Some(value), true);
                    let mut child = env.clone();
                    child.push(HashMap::from([(variable.clone(), id)]));
                    if let flow @ Flow::Return(_) = self.body(body, &mut child)? {
                        return Ok(flow);
                    }
                }
            }
            Stmt::Match { expression, arms } => {
                let value = self.expression(expression, env)?;
                for arm in arms {
                    let matches = match (&arm.pattern, &value) {
                        (Pattern::Wildcard, _) | (Pattern::Null, Value::Null) => true,
                        (Pattern::Integer(a), Value::Integer(b)) => a == b,
                        (Pattern::String(a), Value::String(b)) => a == b,
                        (Pattern::Boolean(a), Value::Boolean(b)) => a == b,
                        _ => false,
                    };
                    if matches {
                        return self.scoped_body(&arm.body, env);
                    }
                }
            }
            Stmt::Parallel { .. } => {
                return Err(self.error("parallel execution is not implemented yet"));
            }
        }
        Ok(Flow::Continue)
    }

    fn call(&mut self, callee: Value, arguments: Vec<Value>) -> Result<Value> {
        match callee {
            Value::Print => {
                let text = arguments
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" ");
                self.runtime
                    .print(&text)
                    .map_err(|message| self.error(message))?;
                Ok(Value::Null)
            }
            Value::Function(id) => {
                if self.call_stack.len() >= self.limits.call_depth {
                    return Err(self.error("function call depth limit exceeded"));
                }
                let function = self
                    .functions
                    .get(id.0)
                    .cloned()
                    .ok_or_else(|| self.error("invalid function handle"))?;
                if arguments.len() != function.parameters.len() {
                    return Err(self.error(format!(
                        "function '{}' expects {} argument(s), got {}",
                        function.name,
                        function.parameters.len(),
                        arguments.len()
                    )));
                }
                let mut env = function.closure;
                let mut parameters = HashMap::new();
                for (name, value) in function.parameters.into_iter().zip(arguments) {
                    parameters.insert(name, self.allocate(Some(value), true));
                }
                env.push(parameters);
                self.call_stack.push(function.name);
                // Expression nesting inside a callee is separate from its caller.
                let depth = std::mem::replace(&mut self.expression_depth, 0);
                let result = self.body(&function.body, &mut env);
                self.expression_depth = depth;
                self.call_stack.pop();
                match result? {
                    Flow::Continue => Ok(Value::Null),
                    Flow::Return(value) => Ok(value),
                }
            }
            value => Err(self.error(format!("cannot call {}", value.type_name()))),
        }
    }
}
