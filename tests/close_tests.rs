use netlang::{
    ast::Transport,
    interpreter::execute,
    lexer::Lexer,
    parser::Parser,
    runtime::{Runtime, StandardRuntime, Value},
};
use std::{
    io::Read,
    net::{TcpListener, UdpSocket},
    thread,
    time::Duration,
};

#[test]
fn close_releases_tcp_while_runtime_remains_alive_and_invalidates_aliases() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let peer = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        assert_eq!(stream.read(&mut [0; 1]).unwrap(), 0);
    });
    let mut runtime = StandardRuntime::new(Vec::new());
    let conn = runtime.connect(Transport::Tcp, &address, None).unwrap();
    let alias = conn.clone();
    runtime.close(&conn).unwrap();
    peer.join().unwrap();
    assert!(runtime.receive(&alias).unwrap_err().contains("closed"));
    assert!(
        runtime
            .send(&alias, &Value::String("x".into()), None)
            .unwrap_err()
            .contains("closed")
    );
    assert!(runtime.close(&alias).unwrap_err().contains("closed"));
    assert!(
        runtime
            .close(&Value::Null)
            .unwrap_err()
            .contains("requires a connection")
    );
}

#[test]
fn repeated_open_close_reuses_capacity_and_foreign_close_cannot_affect_owner() {
    let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
    let address = peer.local_addr().unwrap().to_string();
    let mut runtime = StandardRuntime::new(Vec::new());
    for _ in 0..1030 {
        let conn = runtime.connect(Transport::Udp, &address, None).unwrap();
        runtime.close(&conn).unwrap();
    }
    let conn = runtime.connect(Transport::Udp, &address, None).unwrap();
    assert!(
        runtime
            .fork()
            .unwrap()
            .close(&conn)
            .unwrap_err()
            .contains("another runtime")
    );
    runtime
        .send(&conn, &Value::String("x".into()), None)
        .unwrap();
    assert_eq!(peer.recv_from(&mut [0; 1]).unwrap().0, 1);
    runtime.close(&conn).unwrap();
}

#[test]
fn interpreter_close_supports_aliases_workers_and_arity_checks() {
    let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
    let address = peer.local_addr().unwrap();
    let source = format!(
        "let release = close; parallel n in [1, 2] {{ let socket = UDP \"{address}\"; print(release(socket)); }}"
    );
    let parse = |source: &str| {
        Parser::new(Lexer::new(source).unwrap().tokenize().unwrap())
            .unwrap()
            .parse_program()
            .unwrap()
    };
    let mut output = Vec::new();
    execute(&parse(&source), &mut StandardRuntime::new(&mut output)).unwrap();
    assert_eq!(String::from_utf8(output).unwrap(), "null\nnull\n");
    for source in [
        "close();",
        "let release = close; release();",
        "close(42);",
        "close = 42;",
    ] {
        assert!(execute(&parse(source), &mut StandardRuntime::new(Vec::new())).is_err());
    }
}
