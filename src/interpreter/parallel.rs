use super::{Binding, Environment, Flow, Function, Interpreter, Limits, Result, RuntimeError};
use crate::{
    ast::{Expr, HttpMethod, Stmt, Transport},
    runtime::{Runtime, Value},
};
use std::{
    collections::HashMap,
    sync::{Arc, atomic::AtomicUsize},
    thread,
};

struct Snapshot {
    types: crate::types::TypeRegistry,
    span: Option<crate::source::Span>,
    bindings: Vec<Binding>,
    functions: Vec<Function>,
    env: Environment,
    call_stack: Vec<String>,
}

/// The host belongs to one worker. Only its network operations run on that
/// thread; print output is buffered and later delivered to the parent host.
struct WorkerRuntime {
    host: Box<dyn Runtime + Send>,
    output: Vec<String>,
    output_bytes: usize,
    output_limit: usize,
}

impl Runtime for WorkerRuntime {
    fn listen_tcp(&mut self, address: &str) -> std::result::Result<Value, String> {
        self.host.listen_tcp(address)
    }
    fn accept(&mut self, listener: &Value) -> std::result::Result<Value, String> {
        self.host.accept(listener)
    }
    fn local_address(&mut self, connection: &Value) -> std::result::Result<String, String> {
        self.host.local_address(connection)
    }
    fn set_timeout(
        &mut self,
        connection: &Value,
        milliseconds: u64,
    ) -> std::result::Result<(), String> {
        self.host.set_timeout(connection, milliseconds)
    }
    fn bind_udp(&mut self, address: &str) -> std::result::Result<Value, String> {
        self.host.bind_udp(address)
    }
    fn close(&mut self, connection: &Value) -> std::result::Result<(), String> {
        self.host.close(connection)
    }
    fn print(&mut self, text: &str) -> std::result::Result<(), String> {
        let bytes = text
            .len()
            .checked_add(1)
            .and_then(|n| self.output_bytes.checked_add(n))
            .filter(|n| *n <= self.output_limit)
            .ok_or_else(|| "parallel output buffer limit exceeded".to_string())?;
        self.output.push(text.into());
        self.output_bytes = bytes;
        Ok(())
    }

    fn request(
        &mut self,
        method: HttpMethod,
        url: &str,
        config: &Value,
    ) -> std::result::Result<Value, String> {
        self.host.request(method, url, config)
    }

    fn fork(&mut self) -> std::result::Result<Box<dyn Runtime + Send>, String> {
        self.host.fork()
    }

    fn connect(
        &mut self,
        transport: Transport,
        address: &str,
        protocol: Option<&str>,
    ) -> std::result::Result<Value, String> {
        self.host.connect(transport, address, protocol)
    }

    fn send(
        &mut self,
        connection: &Value,
        data: &Value,
        destination: Option<&str>,
    ) -> std::result::Result<Value, String> {
        self.host.send(connection, data, destination)
    }

    fn receive(&mut self, connection: &Value) -> std::result::Result<Value, String> {
        self.host.receive(connection)
    }
}

struct Job {
    index: usize,
    value: Value,
    host: Box<dyn Runtime + Send>,
}

struct Outcome {
    output: Vec<String>,
    result: Result<()>,
}

impl<R: Runtime> Interpreter<'_, R> {
    pub(super) fn parallel(
        &mut self,
        variable: &str,
        iterable: &Expr,
        body: &Stmt,
        env: &Environment,
    ) -> Result<()> {
        let values = self.expression(iterable, env)?;
        let Value::Array(values) = values else {
            return Err(self.error("parallel requires an array"));
        };
        if values.is_empty() {
            return Ok(());
        }
        let snapshot = Arc::new(Snapshot {
            types: self.types.clone(),
            span: self.current_span,
            bindings: self.bindings.clone(),
            functions: self.functions.clone(),
            env: env.clone(),
            call_stack: self.call_stack.clone(),
        });
        let width = if self.in_parallel_worker {
            1
        } else {
            self.limits.parallel_workers
        };
        for (batch, values) in values.chunks(width).enumerate() {
            let mut jobs = Vec::with_capacity(values.len());
            for (offset, value) in values.iter().enumerate() {
                self.tick()?;
                let host = self.runtime.fork().map_err(|message| self.error(message))?;
                jobs.push(Job {
                    index: batch * width + offset,
                    value: value.clone(),
                    host,
                });
            }
            let outcomes = if self.in_parallel_worker {
                jobs.into_iter()
                    .map(|job| {
                        execute_worker(
                            job,
                            variable,
                            body,
                            &snapshot,
                            self.limits,
                            self.remaining_steps.clone(),
                        )
                    })
                    .collect()
            } else {
                thread::scope(|scope| {
                    let mut handles = Vec::new();
                    let mut spawn_error = None;
                    for job in jobs {
                        let index = job.index;
                        let snapshot = snapshot.clone();
                        let budget = self.remaining_steps.clone();
                        let limits = self.limits;
                        let handle = thread::Builder::new()
                            .name(format!("netlang-parallel-{}", index + 1))
                            .stack_size(8 * 1024 * 1024)
                            .spawn_scoped(scope, move || {
                                execute_worker(job, variable, body, &snapshot, limits, budget)
                            });
                        match handle {
                            Ok(handle) => handles.push((index, handle)),
                            Err(error) => {
                                spawn_error = Some(Outcome {
                                    output: Vec::new(),
                                    result: Err(self.error(format!(
                                        "cannot start parallel iteration {}: {error}",
                                        index + 1
                                    ))),
                                });
                                break;
                            }
                        }
                    }
                    // Join every started worker, even when another one fails.
                    let mut outcomes = Vec::new();
                    for (index, handle) in handles {
                        outcomes.push(handle.join().unwrap_or_else(|_| Outcome {
                            output: Vec::new(),
                            result: Err(
                                self.error(format!("parallel iteration {} panicked", index + 1)),
                            ),
                        }));
                    }
                    if let Some(error) = spawn_error {
                        outcomes.push(error);
                    }
                    outcomes
                })
            };
            let mut first_error = None;
            for outcome in outcomes {
                for line in outcome.output {
                    if let Err(message) = self.runtime.print(&line) {
                        if first_error.is_none() {
                            first_error = Some(self.error(message));
                        }
                        break;
                    }
                }
                if first_error.is_none() {
                    first_error = outcome.result.err();
                }
            }
            if let Some(error) = first_error {
                return Err(error);
            }
        }
        Ok(())
    }
}

fn execute_worker(
    job: Job,
    variable: &str,
    body: &Stmt,
    snapshot: &Snapshot,
    limits: Limits,
    remaining_steps: Arc<AtomicUsize>,
) -> Outcome {
    let mut runtime = WorkerRuntime {
        host: job.host,
        output: Vec::new(),
        output_bytes: 0,
        output_limit: limits.parallel_output_bytes,
    };
    let mut worker = Interpreter {
        types: snapshot.types.clone(),
        current_span: snapshot.span,
        runtime: &mut runtime,
        bindings: snapshot.bindings.clone(),
        functions: snapshot.functions.clone(),
        limits,
        remaining_steps,
        expression_depth: 0,
        call_stack: snapshot.call_stack.clone(),
        captured_bindings: snapshot.bindings.len(),
        in_parallel_worker: true,
        return_type: None,
    };
    let id = worker.allocate(Some(job.value), true);
    let mut env = snapshot.env.clone();
    env.push(HashMap::from([(variable.into(), id)]));
    let result = worker
        .body(body, &mut env)
        .and_then(|flow| match flow {
            Flow::Normal | Flow::Continue => Ok(()),
            Flow::Break => Err(worker.error("break cannot exit a parallel iteration")),
            Flow::Return(_) => Err(worker.error("return cannot exit a parallel iteration")),
        })
        .map_err(|mut error: RuntimeError| {
            error.message = format!("parallel iteration {}: {}", job.index + 1, error.message);
            error
        });
    Outcome {
        output: runtime.output,
        result,
    }
}
