use netlang::{
    ast::Transport,
    interpreter::execute,
    lexer::Lexer,
    parser::Parser,
    runtime::{Runtime, StandardRuntime, Value},
};
use std::{
    io::{Read, Write},
    net::{TcpListener, UdpSocket},
    thread,
    time::Duration,
};

#[test]
fn tcp_preserves_binary_bytes_and_reports_eof_without_assuming_message_boundaries() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let peer = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut input = [0; 5];
        stream.read_exact(&mut input).unwrap();
        assert_eq!(&input, b"hi\0\xff!");
        stream.write_all(&[0xff, 0, 1]).unwrap();
    });
    let mut runtime = StandardRuntime::new(Vec::new());
    let conn = runtime.connect(Transport::Tcp, &address, None).unwrap();
    assert_eq!(
        runtime
            .send(&conn, &Value::String("hi".into()), None)
            .unwrap(),
        Value::Null
    );
    runtime
        .send(&conn, &Value::Bytes(vec![0, 255, b'!']), None)
        .unwrap();
    let mut received = Vec::new();
    loop {
        match runtime.receive(&conn).unwrap() {
            Value::Bytes(bytes) => received.extend(bytes),
            Value::Null => break,
            value => panic!("unexpected {value:?}"),
        }
    }
    assert_eq!(received, [255, 0, 1]);
    assert_eq!(runtime.receive(&conn).unwrap(), Value::Null);
    peer.join().unwrap();
}

#[test]
fn udp_preserves_empty_and_binary_datagrams_and_rejects_invalid_sends() {
    let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
    let mut runtime = StandardRuntime::new(Vec::new());
    let conn = runtime
        .connect(
            Transport::Udp,
            &peer.local_addr().unwrap().to_string(),
            None,
        )
        .unwrap();
    assert!(
        runtime
            .send(&conn, &Value::Integer(1), None)
            .unwrap_err()
            .contains("string or bytes")
    );
    assert!(
        runtime
            .send(&conn, &Value::String("x".into()), Some("127.0.0.1:1"))
            .unwrap_err()
            .contains("unconnected")
    );
    runtime
        .send(&conn, &Value::String("hé".into()), None)
        .unwrap();
    let mut buffer = [0; 10];
    let (count, address) = peer.recv_from(&mut buffer).unwrap();
    assert_eq!(&buffer[..count], "hé".as_bytes());
    peer.send_to(&[], address).unwrap();
    peer.send_to(&[255, 0], address).unwrap();
    assert_eq!(runtime.receive(&conn).unwrap(), Value::Bytes(vec![]));
    let packet = runtime.receive(&conn).unwrap();
    assert_eq!(packet, Value::Bytes(vec![255, 0]));
    runtime.send(&conn, &packet, None).unwrap();
    let (count, _) = peer.recv_from(&mut buffer).unwrap();
    assert_eq!(&buffer[..count], &[255, 0]);
    assert!(
        runtime
            .send(&conn, &Value::Bytes(vec![0; 65536]), None)
            .is_err()
    );
}

#[test]
fn handles_are_runtime_local_and_worker_connections_are_independent() {
    let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
    let address = peer.local_addr().unwrap().to_string();
    let mut runtime = StandardRuntime::new(Vec::new());
    let conn = runtime.connect(Transport::Udp, &address, None).unwrap();
    let mut worker = runtime.fork().unwrap();
    assert!(
        worker
            .receive(&conn)
            .unwrap_err()
            .contains("another runtime")
    );
    let other = worker.connect(Transport::Udp, &address, None).unwrap();
    assert_ne!(conn, other);
    assert!(
        runtime
            .receive(&other)
            .unwrap_err()
            .contains("another runtime")
    );
    assert_eq!(conn.clone(), conn);
    assert_eq!(conn.to_string(), "<connection>");
    assert!(
        runtime
            .connect(Transport::Tcp, &address, Some("Login"))
            .unwrap_err()
            .contains("not implemented")
    );
    assert!(
        runtime
            .connect(Transport::Tcp, "not an address", None)
            .is_err()
    );
}

#[test]
fn udp_large_datagrams_are_not_truncated_and_receive_has_a_timeout() {
    let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
    let mut runtime = StandardRuntime::new(Vec::new());
    let conn = runtime
        .connect(
            Transport::Udp,
            &peer.local_addr().unwrap().to_string(),
            None,
        )
        .unwrap();
    runtime.send(&conn, &Value::Bytes(vec![]), None).unwrap();
    let (_, address) = peer.recv_from(&mut [0; 1]).unwrap();
    // Stay below platform-specific send limits (some systems default to 9 KB).
    let payload = vec![0xa5; 8192];
    peer.send_to(&payload, address).unwrap();
    assert_eq!(runtime.receive(&conn).unwrap(), Value::Bytes(payload));
    let started = std::time::Instant::now();
    assert!(
        runtime
            .receive(&conn)
            .unwrap_err()
            .contains("UDP RECEIVE failed")
    );
    assert!(started.elapsed() >= Duration::from_secs(4));
    assert!(started.elapsed() < Duration::from_secs(15));
}

#[test]
fn parallel_interpreter_workers_can_open_their_own_sockets() {
    let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
    let address = peer.local_addr().unwrap();
    let source = format!(
        "parallel message in [\"a\", \"b\"] {{ let socket = UDP \"{address}\"; socket SEND message; }}"
    );
    let program = Parser::new(Lexer::new(&source).unwrap().tokenize().unwrap())
        .unwrap()
        .parse_program()
        .unwrap();
    execute(&program, &mut StandardRuntime::new(Vec::new())).unwrap();
    let mut received = Vec::new();
    for _ in 0..2 {
        let mut buffer = [0; 1];
        assert_eq!(peer.recv_from(&mut buffer).unwrap().0, 1);
        received.push(buffer[0]);
    }
    received.sort();
    assert_eq!(received, b"ab");
}

#[test]
fn interpreter_executes_tcp_and_runtime_drop_closes_connections() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let peer = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut buffer = [0; 5];
        stream.read_exact(&mut buffer).unwrap();
        assert_eq!(&buffer, b"hello");
        stream.write_all(&[255]).unwrap();
        let mut echoed = [0; 1];
        stream.read_exact(&mut echoed).unwrap();
        assert_eq!(echoed, [255]);
        assert_eq!(stream.read(&mut echoed).unwrap(), 0);
    });
    let source = format!(
        "fn main() {{ let conn = TCP \"{address}\"; conn SEND \"hello\"; let bytes = conn RECEIVE; print(bytes); conn SEND bytes; }}"
    );
    let program = Parser::new(Lexer::new(&source).unwrap().tokenize().unwrap())
        .unwrap()
        .parse_program()
        .unwrap();
    let mut output = Vec::new();
    {
        let mut runtime = StandardRuntime::new(&mut output);
        execute(&program, &mut runtime).unwrap();
    }
    assert_eq!(String::from_utf8(output).unwrap(), "bytes[255]\n");
    peer.join().unwrap();
}
