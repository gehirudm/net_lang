//! Host effects used by the interpreter and, later, compiled programs.
mod value;
use crate::ast::HttpMethod;
use std::io::Write;
pub use value::{FunctionId, Value};

/// Embedders can supply deterministic effects for tests or another runtime.
pub trait Runtime {
    fn print(&mut self, text: &str) -> Result<(), String>;

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
}

impl<W: Write> StandardRuntime<W> {
    pub fn new(output: W) -> Self {
        Self { output }
    }
}

impl<W: Write> Runtime for StandardRuntime<W> {
    fn print(&mut self, text: &str) -> Result<(), String> {
        writeln!(self.output, "{text}").map_err(|error| format!("cannot write output: {error}"))
    }
}
