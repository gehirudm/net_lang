use super::{ConnectionId, Value};
use crate::ast::Transport;
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    net::{TcpStream, ToSocketAddrs, UdpSocket},
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
const TIMEOUT: Duration = Duration::from_secs(5);

enum Socket {
    Tcp(TcpStream),
    Udp(UdpSocket),
}

#[derive(Default)]
pub(super) struct TransportRuntime {
    sockets: BTreeMap<u64, Socket>,
}

impl TransportRuntime {
    pub fn connect(
        &mut self,
        transport: Transport,
        address: &str,
        protocol: Option<&str>,
    ) -> Result<Value, String> {
        if protocol.is_some() {
            return Err("protocol-aware connections are not implemented".into());
        }
        if self.sockets.len() >= 1024 {
            return Err("runtime connection limit (1024) reached".into());
        }
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
                    let id = NEXT_ID
                        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
                        .map_err(|_| "connection IDs exhausted")?;
                    self.sockets.insert(id, socket);
                    return Ok(Value::Connection(ConnectionId(id)));
                }
                Err(error) => last_error = format!("cannot connect: {error}"),
            }
        }
        Err(last_error)
    }

    fn socket(&mut self, connection: &Value) -> Result<&mut Socket, String> {
        let Value::Connection(id) = connection else {
            return Err("SEND/RECEIVE requires a connection".into());
        };
        self.sockets.get_mut(&id.0).ok_or_else(|| {
            "connection belongs to another runtime; create connections inside parallel workers"
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
        if destination.is_some() {
            return Err("TO requires an unconnected UDP socket; unconnected socket construction is not implemented".into());
        }
        let bytes = match data {
            Value::String(text) => text.as_bytes(),
            Value::Bytes(bytes) => bytes,
            _ => return Err("SEND data must be a string or bytes".into()),
        };
        match socket {
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
