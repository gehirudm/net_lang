//! Host effects used by the interpreter and, later, compiled programs.
mod builtin;
mod http;
pub use builtin::Builtin;
mod transport;
mod value;
use crate::ast::{HttpMethod, Transport};
use std::io::Write;
pub use value::{ConnectionId, FunctionId, Value};

/// Embedders can supply deterministic effects for tests or another runtime.
pub trait Runtime {
    fn listen_tcp(&mut self, _address: &str) -> Result<Value, String> {
        Err("TCP listeners are not supported by this runtime".into())
    }
    fn accept(&mut self, _listener: &Value) -> Result<Value, String> {
        Err("TCP accept is not supported by this runtime".into())
    }
    fn local_address(&mut self, _connection: &Value) -> Result<String, String> {
        Err("local address queries are not supported by this runtime".into())
    }
    fn set_timeout(&mut self, _connection: &Value, _milliseconds: u64) -> Result<(), String> {
        Err("socket timeout configuration is not supported by this runtime".into())
    }
    fn bind_udp(&mut self, _address: &str) -> Result<Value, String> {
        Err("unconnected UDP is not supported by this runtime".into())
    }
    fn print(&mut self, text: &str) -> Result<(), String>;

    fn close(&mut self, _connection: &Value) -> Result<(), String> {
        Err("connection close is not supported by this runtime".into())
    }

    /// Make an independent host for a parallel iteration. Worker print output
    /// is buffered by the interpreter and replayed through the parent's print.
    fn fork(&mut self) -> Result<Box<dyn Runtime + Send>, String> {
        Err("parallel execution requires a runtime that supports worker forks".into())
    }

    fn connect(
        &mut self,
        _transport: Transport,
        _address: &str,
        _protocol: Option<&str>,
    ) -> Result<Value, String> {
        Err("TCP/UDP connections are not supported by this runtime".into())
    }

    fn send(
        &mut self,
        _connection: &Value,
        _data: &Value,
        _destination: Option<&str>,
    ) -> Result<Value, String> {
        Err("SEND is not supported by this runtime".into())
    }

    fn receive(&mut self, _connection: &Value) -> Result<Value, String> {
        Err("RECEIVE is not supported by this runtime".into())
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
    transport: transport::TransportRuntime,
}

impl<W: Write> StandardRuntime<W> {
    pub fn new(output: W) -> Self {
        Self {
            output,
            http: http::HttpRuntime::default(),
            transport: transport::TransportRuntime::default(),
        }
    }
}

impl<W: Write> Runtime for StandardRuntime<W> {
    fn listen_tcp(&mut self, address: &str) -> Result<Value, String> {
        self.transport.listen_tcp(address)
    }
    fn accept(&mut self, listener: &Value) -> Result<Value, String> {
        self.transport.accept(listener)
    }
    fn local_address(&mut self, connection: &Value) -> Result<String, String> {
        self.transport.local_address(connection)
    }
    fn set_timeout(&mut self, connection: &Value, milliseconds: u64) -> Result<(), String> {
        self.transport.set_timeout(connection, milliseconds)
    }
    fn bind_udp(&mut self, address: &str) -> Result<Value, String> {
        self.transport.bind_udp(address)
    }
    fn close(&mut self, connection: &Value) -> Result<(), String> {
        self.transport.close(connection)
    }
    fn connect(
        &mut self,
        transport: Transport,
        address: &str,
        protocol: Option<&str>,
    ) -> Result<Value, String> {
        self.transport.connect(transport, address, protocol)
    }

    fn send(
        &mut self,
        connection: &Value,
        data: &Value,
        destination: Option<&str>,
    ) -> Result<Value, String> {
        self.transport.send(connection, data, destination)
    }

    fn receive(&mut self, connection: &Value) -> Result<Value, String> {
        self.transport.receive(connection)
    }
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
            transport: transport::TransportRuntime::default(),
        }))
    }
}
