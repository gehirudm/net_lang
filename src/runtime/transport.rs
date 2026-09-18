use super::{ConnectionId, Value};
use crate::ast::Transport;
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    net::{TcpListener, TcpStream, ToSocketAddrs, UdpSocket},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
const TIMEOUT: Duration = Duration::from_secs(5);

enum Socket {
    Tcp(TcpStream),
    Udp(UdpSocket),
    UnconnectedUdp(UdpSocket),
    Listener {
        socket: TcpListener,
        timeout: Duration,
    },
}

#[derive(Default)]
pub(super) struct TransportRuntime {
    sockets: BTreeMap<u64, Socket>,
}

impl TransportRuntime {
    pub fn listen_tcp(&mut self, address: &str) -> Result<Value, String> {
        self.ensure_capacity()?;
        let socket = TcpListener::bind(address).map_err(|e| format!("TCP listen failed: {e}"))?;
        socket
            .set_nonblocking(true)
            .map_err(|e| format!("cannot configure TCP listener: {e}"))?;
        self.insert(Socket::Listener {
            socket,
            timeout: TIMEOUT,
        })
    }

    pub fn local_address(&mut self, connection: &Value) -> Result<String, String> {
        let address = match self.socket(connection)? {
            Socket::Tcp(socket) => socket.local_addr(),
            Socket::Udp(socket) | Socket::UnconnectedUdp(socket) => socket.local_addr(),
            Socket::Listener { socket, .. } => socket.local_addr(),
        };
        address
            .map(|address| address.to_string())
            .map_err(|e| format!("cannot read local address: {e}"))
    }

    pub fn accept(&mut self, listener: &Value) -> Result<Value, String> {
        self.ensure_capacity()?;
        let Socket::Listener { socket, timeout } = self.socket(listener)? else {
            return Err("accept requires a TCP listener".into());
        };
        let started = Instant::now();
        let (stream, address) = loop {
            if started.elapsed() >= *timeout {
                return Err("TCP accept timed out".into());
            }
            match socket.accept() {
                Ok(accepted) => break accepted,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    // Portable bounded wait without requiring an async runtime.
                    std::thread::sleep(
                        Duration::from_millis(5).min(timeout.saturating_sub(started.elapsed())),
                    );
                }
                Err(error) => return Err(format!("TCP accept failed: {error}")),
            }
        };
        // Explicitly establish stream mode rather than relying on OS inheritance.
        stream
            .set_nonblocking(false)
            .map_err(|e| format!("cannot configure accepted TCP connection: {e}"))?;
        stream
            .set_read_timeout(Some(*timeout))
            .map_err(|e| e.to_string())?;
        stream
            .set_write_timeout(Some(*timeout))
            .map_err(|e| e.to_string())?;
        let connection = self.insert(Socket::Tcp(stream))?;
        Ok(Value::Object(BTreeMap::from([
            ("connection".into(), connection),
            ("address".into(), Value::String(address.to_string())),
        ])))
    }

    fn ensure_capacity(&self) -> Result<(), String> {
        if self.sockets.len() >= 1024 {
            Err("runtime connection limit (1024) reached".into())
        } else {
            Ok(())
        }
    }

    pub fn set_timeout(&mut self, connection: &Value, milliseconds: u64) -> Result<(), String> {
        if !(1..=86_400_000).contains(&milliseconds) {
            return Err("socket timeout must be from 1ms through 24h".into());
        }
        let timeout = Some(Duration::from_millis(milliseconds));
        let result = match self.socket(connection)? {
            Socket::Listener {
                timeout: current, ..
            } => {
                *current = Duration::from_millis(milliseconds);
                Ok(())
            }
            Socket::Tcp(socket) => socket
                .set_read_timeout(timeout)
                .and_then(|()| socket.set_write_timeout(timeout)),
            Socket::Udp(socket) | Socket::UnconnectedUdp(socket) => socket
                .set_read_timeout(timeout)
                .and_then(|()| socket.set_write_timeout(timeout)),
        };
        result.map_err(|e| {
            format!("cannot set socket timeout (read timeout may already be updated): {e}")
        })
    }
    fn insert(&mut self, socket: Socket) -> Result<Value, String> {
        let id = NEXT_ID
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .map_err(|_| "connection IDs exhausted")?;
        self.sockets.insert(id, socket);
        Ok(Value::Connection(ConnectionId(id)))
    }

    pub fn bind_udp(&mut self, address: &str) -> Result<Value, String> {
        self.ensure_capacity()?;
        let socket = UdpSocket::bind(address).map_err(|e| format!("UDP bind failed: {e}"))?;
        socket
            .set_read_timeout(Some(TIMEOUT))
            .map_err(|e| e.to_string())?;
        socket
            .set_write_timeout(Some(TIMEOUT))
            .map_err(|e| e.to_string())?;
        self.insert(Socket::UnconnectedUdp(socket))
    }
    pub fn close(&mut self, connection: &Value) -> Result<(), String> {
        let Value::Connection(id) = connection else {
            return Err("close requires a connection".into());
        };
        self.sockets
            .remove(&id.0)
            .ok_or_else(|| "connection is closed or belongs to another runtime".to_string())?;
        Ok(())
    }
    pub fn connect(
        &mut self,
        transport: Transport,
        address: &str,
        protocol: Option<&str>,
    ) -> Result<Value, String> {
        if protocol.is_some() {
            return Err("protocol-aware connections are not implemented".into());
        }
        self.ensure_capacity()?;
        let addresses = address
            .to_socket_addrs()
            .map_err(|e| format!("cannot resolve connection address: {e}"))?;
        let mut last_error = "connection address resolved to no endpoints".to_string();
        for address in addresses {
            let result = (|| -> std::io::Result<Socket> {
                match transport {
                    Transport::Tcp => {
                        let socket = TcpStream::connect_timeout(&address, TIMEOUT)?;
                        socket.set_read_timeout(Some(TIMEOUT))?;
                        socket.set_write_timeout(Some(TIMEOUT))?;
                        Ok(Socket::Tcp(socket))
                    }
                    Transport::Udp => {
                        let socket = UdpSocket::bind(if address.is_ipv4() {
                            "0.0.0.0:0"
                        } else {
                            "[::]:0"
                        })?;
                        socket.connect(address)?;
                        socket.set_read_timeout(Some(TIMEOUT))?;
                        socket.set_write_timeout(Some(TIMEOUT))?;
                        Ok(Socket::Udp(socket))
                    }
                }
            })();
            match result {
                Ok(socket) => {
                    return self.insert(socket);
                }
                Err(error) => last_error = format!("cannot connect: {error}"),
            }
        }
        Err(last_error)
    }

    fn socket(&mut self, connection: &Value) -> Result<&mut Socket, String> {
        let Value::Connection(id) = connection else {
            return Err("socket operation requires a connection or listener handle".into());
        };
        self.sockets.get_mut(&id.0).ok_or_else(|| {
            "connection is closed or belongs to another runtime; create connections inside parallel workers"
                .into()
        })
    }

    pub fn send(
        &mut self,
        connection: &Value,
        data: &Value,
        destination: Option<&str>,
    ) -> Result<Value, String> {
        let socket = self.socket(connection)?;
        if destination.is_some() && !matches!(socket, Socket::UnconnectedUdp(_)) {
            return Err("TO requires an unconnected UDP socket".into());
        }
        let bytes = match data {
            Value::String(text) => text.as_bytes(),
            Value::Bytes(bytes) => bytes,
            _ => return Err("SEND data must be a string or bytes".into()),
        };
        match socket {
            Socket::Listener { .. } => {
                return Err("SEND requires a connection, not a listener".into());
            }
            Socket::UnconnectedUdp(socket) => {
                let destination =
                    destination.ok_or("unconnected UDP SEND requires TO destination")?;
                let count = socket
                    .send_to(bytes, destination)
                    .map_err(|e| format!("UDP SEND TO failed: {e}"))?;
                if count != bytes.len() {
                    return Err("UDP SEND did not send the complete datagram".into());
                }
            }
            Socket::Tcp(socket) => socket
                .write_all(bytes)
                .map_err(|e| format!("TCP SEND failed (data may have been partially sent): {e}"))?,
            Socket::Udp(socket) => {
                let count = socket
                    .send(bytes)
                    .map_err(|e| format!("UDP SEND failed: {e}"))?;
                if count != bytes.len() {
                    return Err("UDP SEND did not send the complete datagram".into());
                }
            }
        }
        Ok(Value::Null)
    }

    pub fn receive(&mut self, connection: &Value) -> Result<Value, String> {
        let socket = self.socket(connection)?;
        // Large enough for a complete UDP datagram; TCP still returns arbitrary chunks.
        let mut bytes = vec![0; 65535];
        let count = match socket {
            Socket::Listener { .. } => {
                return Err("RECEIVE requires a connection; use accept(listener) first".into());
            }
            Socket::UnconnectedUdp(socket) => {
                let (count, address) = socket
                    .recv_from(&mut bytes)
                    .map_err(|e| format!("UDP RECEIVE failed: {e}"))?;
                bytes.truncate(count);
                return Ok(Value::Object(BTreeMap::from([
                    ("data".into(), Value::Bytes(bytes)),
                    ("address".into(), Value::String(address.to_string())),
                ])));
            }
            Socket::Tcp(socket) => {
                let count = loop {
                    match socket.read(&mut bytes) {
                        Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                        result => break result.map_err(|e| format!("TCP RECEIVE failed: {e}"))?,
                    }
                };
                if count == 0 {
                    return Ok(Value::Null);
                }
                count
            }
            Socket::Udp(socket) => socket
                .recv(&mut bytes)
                .map_err(|e| format!("UDP RECEIVE failed: {e}"))?,
        };
        bytes.truncate(count);
        Ok(Value::Bytes(bytes))
    }
}
