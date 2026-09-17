//! Host effects used by the interpreter and, later, compiled programs.
mod http;
mod value;
use crate::ast::HttpMethod;
use std::io::Write;
pub use value::{FunctionId, Value};

/// Embedders can supply deterministic effects for tests or another runtime.
pub trait Runtime {
    fn print(&mut self, text: &str) -> Result<(), String>;

    /// Make an independent host for a parallel iteration. Worker print output
    /// is buffered by the interpreter and replayed through the parent's print.
    fn fork(&mut self) -> Result<Box<dyn Runtime + Send>, String> {
        Err("parallel execution requires a runtime that supports worker forks".into())
    }

    fn request(
        &mut self,
        _method: HttpMethod,
        _url: &str,
        _config: &Value,
    ) -> Result<Value, String> {
        Err("HTTP execution is not supported by this runtime".into())
    }
}

pub struct StandardRuntime<W: Write> {
    output: W,
    http: http::HttpRuntime,
}

impl<W: Write> StandardRuntime<W> {
    pub fn new(output: W) -> Self {
        Self {
            output,
            http: http::HttpRuntime::default(),
        }
    }
}

impl<W: Write> Runtime for StandardRuntime<W> {
    fn print(&mut self, text: &str) -> Result<(), String> {
        writeln!(self.output, "{text}").map_err(|error| format!("cannot write output: {error}"))
    }

    fn request(&mut self, method: HttpMethod, url: &str, config: &Value) -> Result<Value, String> {
        self.http.request(method, url, config)
    }

    fn fork(&mut self) -> Result<Box<dyn Runtime + Send>, String> {
        Ok(Box::new(StandardRuntime {
            output: Vec::<u8>::new(),
            http: self.http.clone(),
        }))
    }
}
