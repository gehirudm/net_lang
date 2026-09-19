//! Lexical name resolution, structural checks, and optional type contracts.
//! This pass does not execute code or infer constraints on unannotated bindings.

mod error;
mod scope;
mod types;

use crate::ast::{Expr, Program, Stmt};
pub use error::{SemanticError, SemanticErrorKind};
use scope::{Scopes, Symbol};

/// Check an independently constructed or parsed AST, collecting all errors.
/// Every call starts with fresh scopes and the standard built-in bindings.
pub fn analyze(program: &Program) -> Result<(), Vec<SemanticError>> {
    let mut analyzer = Analyzer {
        scopes: Scopes::new(),
        function_depth: 0,
        context: vec!["program".into()],
        errors: Vec::new(),
        parallel_boundaries: Vec::new(),
        current_span: None,
        loops: Vec::new(),
    };
    analyzer.scopes.enter();
    analyzer.statements(&program.statements);
    analyzer.scopes.leave();
    if analyzer.errors.is_empty() {
        Ok(())
    } else {
        Err(analyzer.errors)
    }
}

struct Analyzer {
    loops: Vec<LoopKind>,
    current_span: Option<crate::source::Span>,
    scopes: Scopes,
    function_depth: usize,
    context: Vec<String>,
    errors: Vec<SemanticError>,
    // Scope depth and enclosing function depth at each worker boundary.
    parallel_boundaries: Vec<(usize, usize)>,
}

#[derive(Clone, Copy)]
enum LoopKind {
    Serial,
    Parallel,
}

impl Analyzer {
    fn error(&mut self, kind: SemanticErrorKind, message: impl Into<String>) {
        self.errors.push(SemanticError {
            kind,
            message: message.into(),
            context: self.context.clone(),
            span: self.current_span,
        });
    }

    fn declare(&mut self, name: &str, symbol: Symbol) {
        if !self.scopes.declare(name, symbol) {
            self.error(
                SemanticErrorKind::DuplicateDeclaration,
                format!("duplicate declaration of '{name}' in the same scope"),
            );
        }
    }

    fn resolve(&mut self, name: &str) -> Option<Symbol> {
        let symbol = self.scopes.resolve(name);
        if symbol.is_none() {
            self.error(
                SemanticErrorKind::UndefinedName,
                format!("undefined name '{name}'"),
            );
        }
        symbol
    }

    fn statements(&mut self, statements: &[Stmt]) {
        // Hoist only function declarations in this scope, enabling recursion and
        // mutually recursive functions. Variables are introduced in source order.
        for (i, stmt) in statements.iter().enumerate() {
            if let Stmt::Function {
                name, parameters, ..
            } = stmt.unspanned()
            {
                let previous_span = self.current_span;
                self.current_span = stmt.span().or(previous_span);
                self.context.push(format!("statement {}", i + 1));
                self.declare(
                    name,
                    Symbol::Function {
                        arity: parameters.len(),
                    },
                );
                self.context.pop();
                self.current_span = previous_span;
            }
        }
        for (i, stmt) in statements.iter().enumerate() {
            self.context.push(format!("statement {}", i + 1));
            self.statement(stmt);
            self.context.pop();
        }
    }

    /// Function parameters and loop variables share the body's outermost scope.
    /// Nested explicit blocks still introduce their own scope.
    fn body_in_current_scope(&mut self, body: &Stmt) {
        if let Stmt::Located { span, statement } = body {
            let previous_span = self.current_span.replace(*span);
            self.body_in_current_scope(statement);
            self.current_span = previous_span;
            return;
        }
        if let Stmt::Block(statements) = body.unspanned() {
            self.statements(statements);
        } else {
            // Also support ASTs assembled without the parser.
            self.statements(std::slice::from_ref(body));
        }
    }

    fn scoped_body(&mut self, label: impl Into<String>, body: &Stmt) {
        self.context.push(label.into());
        self.scopes.enter();
        self.body_in_current_scope(body);
        self.scopes.leave();
        self.context.pop();
    }

    fn statement(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Break => match self.loops.last() {
                Some(LoopKind::Serial) => {}
                Some(LoopKind::Parallel) => self.error(
                    SemanticErrorKind::BreakAcrossParallel,
                    "break cannot exit a parallel iteration; use continue to finish this iteration",
                ),
                None => self.error(
                    SemanticErrorKind::BreakOutsideLoop,
                    "break is only allowed inside a serial loop",
                ),
            },
            Stmt::Continue => {
                if self.loops.is_empty() {
                    self.error(
                        SemanticErrorKind::ContinueOutsideLoop,
                        "continue is only allowed inside a loop",
                    );
                }
            }
            Stmt::Located { span, statement } => {
                let previous_span = self.current_span.replace(*span);
                self.statement(statement);
                self.current_span = previous_span;
            }
            Stmt::Let {
                name,
                annotation,
                value,
            } => {
                // An initializer may refer to an outer binding of the same name.
                self.expression(value);
                let annotation = annotation
                    .as_ref()
                    .and_then(|annotation| self.resolve_type(annotation));
                if let Some(expected) = annotation {
                    self.check_type(value, expected);
                }
                self.declare(name, Symbol::Variable { annotation });
            }
            Stmt::Expression(expr) => self.expression(expr),
            Stmt::Block(_) => self.scoped_body("block", stmt),
            Stmt::Function {
                name,
                parameters,
                body,
            } => {
                self.context.push(format!("function '{name}'"));
                self.scopes.enter();
                self.function_depth += 1;
                // A function cannot transfer control to a caller's loop.
                let enclosing_loops = std::mem::take(&mut self.loops);
                for parameter in parameters {
                    self.declare(parameter, Symbol::Parameter);
                }
                self.body_in_current_scope(body);
                self.loops = enclosing_loops;
                self.function_depth -= 1;
                self.scopes.leave();
                self.context.pop();
            }
            Stmt::Return { value } => {
                if self
                    .parallel_boundaries
                    .last()
                    .is_some_and(|(_, depth)| *depth == self.function_depth)
                {
                    self.error(SemanticErrorKind::ReturnAcrossParallel,
                        "return cannot exit a parallel iteration; return from a called function instead");
                } else if self.function_depth == 0 {
                    self.error(
                        SemanticErrorKind::ReturnOutsideFunction,
                        "return is only allowed inside a function",
                    );
                }
                if let Some(value) = value {
                    self.expression(value);
                }
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.expression(condition);
                self.scoped_body("if branch", then_branch);
                if let Some(branch) = else_branch {
                    self.scoped_body("else branch", branch);
                }
            }
            Stmt::While { condition, body } => {
                self.expression(condition);
                self.loops.push(LoopKind::Serial);
                self.scoped_body("while body", body);
                self.loops.pop();
            }
            Stmt::For {
                variable,
                iterable,
                body,
            }
            | Stmt::Parallel {
                variable,
                iterable,
                body,
            } => {
                // The binding is not in scope while evaluating the iterable.
                self.expression(iterable);
                let keyword = if matches!(stmt, Stmt::For { .. }) {
                    "for"
                } else {
                    "parallel"
                };
                self.context.push(format!("{keyword} '{variable}'"));
                let parallel = matches!(stmt, Stmt::Parallel { .. });
                self.loops.push(if parallel {
                    LoopKind::Parallel
                } else {
                    LoopKind::Serial
                });
                if parallel {
                    self.parallel_boundaries
                        .push((self.scopes.depth(), self.function_depth));
                }
                self.scopes.enter();
                self.declare(variable, Symbol::LoopVariable);
                self.body_in_current_scope(body);
                self.scopes.leave();
                self.loops.pop();
                if parallel {
                    self.parallel_boundaries.pop();
                }
                self.context.pop();
            }
            Stmt::Match { expression, arms } => {
                self.expression(expression);
                for (i, arm) in arms.iter().enumerate() {
                    self.scoped_body(format!("match arm {}", i + 1), &arm.body);
                }
            }
        }
    }

    fn expression(&mut self, expr: &Expr) {
        match expr {
            Expr::Located { span, expression } => {
                let previous = self.current_span.replace(*span);
                self.expression(expression);
                self.current_span = previous;
            }
            Expr::Connection { address, .. } => self.expression(address),
            Expr::Send {
                connection,
                data,
                destination,
            } => {
                self.expression(connection);
                self.expression(data);
                if let Some(destination) = destination {
                    self.expression(destination);
                }
            }
            Expr::Receive { connection } => self.expression(connection),
            Expr::Integer(_)
            | Expr::Float(_)
            | Expr::String(_)
            | Expr::Boolean(_)
            | Expr::Null
            | Expr::Duration(_) => {}
            Expr::Identifier(name) => {
                self.resolve(name);
            }
            Expr::Array(values) => {
                for value in values {
                    self.expression(value);
                }
            }
            Expr::Object(fields) => {
                for field in fields {
                    self.expression(&field.value);
                }
            }
            Expr::Unary { expression, .. } => self.expression(expression),
            Expr::Binary { left, right, .. } => {
                self.expression(left);
                self.expression(right);
            }
            Expr::Assignment { target, value } => {
                let previous = self.current_span;
                self.current_span = target.span().or(previous);
                match target.unspanned() {
                    Expr::Identifier(name) => {
                        if self
                            .resolve(name)
                            .is_some_and(|symbol| !symbol.is_mutable())
                        {
                            self.error(
                                SemanticErrorKind::ImmutableAssignment,
                                format!("cannot assign to function '{name}'"),
                            );
                        }
                    }
                    Expr::Property { .. } | Expr::Index { .. } => self.expression(target),
                    _ => {
                        self.error(
                            SemanticErrorKind::InvalidAssignmentTarget,
                            "assignment target must be a variable, property, or index",
                        );
                        self.expression(target);
                    }
                }
                self.check_parallel_assignment(target);
                self.current_span = previous;
                self.expression(value);
                if let Expr::Identifier(name) = target.unspanned()
                    && let Some(Symbol::Variable {
                        annotation: Some(expected),
                    }) = self.scopes.resolve(name)
                {
                    self.check_type(value, expected);
                }
            }
            Expr::Call { callee, arguments } => {
                self.expression(callee);
                if let Expr::Identifier(name) = callee.unspanned()
                    && let Some(Symbol::Function { arity }) = self.scopes.resolve(name)
                    && arity != arguments.len()
                {
                    self.error(
                        SemanticErrorKind::ArgumentCount,
                        format!(
                            "function '{name}' expects {arity} argument(s), got {}",
                            arguments.len()
                        ),
                    );
                }
                for argument in arguments {
                    self.expression(argument);
                }
            }
            Expr::Property { object, .. } => self.expression(object),
            Expr::Index { object, index } => {
                self.expression(object);
                self.expression(index);
            }
            Expr::Request { url, config, .. } => {
                self.expression(url);
                if let Some(config) = config {
                    self.expression(config);
                }
            }
        }
    }

    fn check_parallel_assignment(&mut self, target: &Expr) {
        let mut root = target.unspanned();
        while let Expr::Property { object, .. } | Expr::Index { object, .. } = root {
            root = object.unspanned();
        }
        if let Expr::Identifier(name) = root
            && let Some((boundary, _)) = self.parallel_boundaries.last()
            && let Some((depth, symbol)) = self.scopes.resolve_with_depth(name)
            && depth < *boundary
            && symbol.is_mutable()
        {
            self.error(
                SemanticErrorKind::CapturedAssignment,
                format!("cannot assign to read-only parallel capture '{name}'"),
            );
        }
    }
}
