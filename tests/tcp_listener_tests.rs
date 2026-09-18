use netlang::{
    ast::Transport,
    interpreter::execute,
    lexer::Lexer,
    parser::Parser,
    runtime::{Runtime, StandardRuntime, Value},
};
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    time::{Duration, Instant},
};

fn connection(accepted: &Value) -> &Value {
    let Value::Object(fields) = accepted else {
        panic!("expected accepted connection object")
    };
    &fields["connection"]
}

#[test]
fn accepts_binary_traffic_and_listener_close_preserves_accepted_stream() {
    let mut runtime = StandardRuntime::new(Vec::new());
    let listener = runtime.listen_tcp("127.0.0.1:0").unwrap();
    let address = runtime.local_address(&listener).unwrap();
    assert_ne!(address.parse::<SocketAddr>().unwrap().port(), 0);
    let mut client = TcpStream::connect(&address).unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    client
        .set_write_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let accepted = runtime.accept(&listener).unwrap();
    let Value::Object(fields) = &accepted else {
        unreachable!()
    };
    assert_eq!(
        fields["address"],
        Value::String(client.local_addr().unwrap().to_string())
    );
    let conn = connection(&accepted);
    assert_eq!(runtime.local_address(conn).unwrap(), address);
    runtime.close(&listener).unwrap();
    client.write_all(&[255, 0, 42]).unwrap();
    let mut received = Vec::new();
    while received.len() < 3 {
        let Value::Bytes(bytes) = runtime.receive(conn).unwrap() else {
            panic!("unexpected EOF")
        };
        received.extend(bytes);
    }
    assert_eq!(received, [255, 0, 42]);
    runtime.send(conn, &Value::Bytes(received), None).unwrap();
    let mut reply = [0; 3];
    client.read_exact(&mut reply).unwrap();
    assert_eq!(reply, [255, 0, 42]);
    runtime.close(conn).unwrap();
    assert_eq!(client.read(&mut reply).unwrap(), 0);
    assert!(runtime.accept(&listener).is_err());
}

#[test]
fn accept_timeout_is_bounded_and_inherited_by_new_streams() {
    let mut runtime = StandardRuntime::new(Vec::new());
    let listener = runtime.listen_tcp("127.0.0.1:0").unwrap();
    runtime.set_timeout(&listener, 30).unwrap();
    let started = Instant::now();
    assert!(runtime.accept(&listener).unwrap_err().contains("timed out"));
    assert!(started.elapsed() >= Duration::from_millis(20));
    assert!(started.elapsed() < Duration::from_secs(3));
    let _client = TcpStream::connect(runtime.local_address(&listener).unwrap()).unwrap();
    let accepted = runtime.accept(&listener).unwrap();
    let started = Instant::now();
    assert!(runtime.receive(connection(&accepted)).is_err());
    assert!(started.elapsed() >= Duration::from_millis(10));
    assert!(started.elapsed() < Duration::from_secs(3));
}

#[test]
fn listener_capabilities_ownership_and_local_addresses_are_checked() {
    let mut runtime = StandardRuntime::new(Vec::new());
    let listener = runtime.listen_tcp("127.0.0.1:0").unwrap();
    let address = runtime.local_address(&listener).unwrap();
    assert!(runtime.listen_tcp(&address).is_err());
    assert!(
        runtime
            .send(&listener, &Value::String("x".into()), None)
            .is_err()
    );
    assert!(runtime.receive(&listener).is_err());
    let mut worker = runtime.fork().unwrap();
    assert!(
        worker
            .accept(&listener)
            .unwrap_err()
            .contains("another runtime")
    );
    assert!(worker.local_address(&listener).is_err());
    for socket in [
        runtime.bind_udp("127.0.0.1:0").unwrap(),
        runtime
            .connect(Transport::Udp, "127.0.0.1:9", None)
            .unwrap(),
    ] {
        assert_ne!(
            runtime
                .local_address(&socket)
                .unwrap()
                .parse::<SocketAddr>()
                .unwrap()
                .port(),
            0
        );
        assert!(
            runtime
                .accept(&socket)
                .unwrap_err()
                .contains("requires a TCP listener")
        );
        runtime.close(&socket).unwrap();
        assert!(runtime.local_address(&socket).is_err());
    }
    assert!(runtime.local_address(&Value::Null).is_err());
    assert!(runtime.accept(&Value::Null).is_err());
}

fn run(source: &str) -> Result<String, String> {
    let program = Parser::new(Lexer::new(source).unwrap().tokenize().unwrap())
        .unwrap()
        .parse_program()
        .unwrap();
    let mut output = Vec::new();
    execute(&program, &mut StandardRuntime::new(&mut output)).map_err(|e| e.to_string())?;
    Ok(String::from_utf8(output).unwrap())
}

#[test]
fn interpreter_and_workers_execute_a_complete_local_server_exchange() {
    assert_eq!(
        run(include_str!("../examples/tcp_loopback.net")).unwrap(),
        "A\nB\n"
    );
    let program = r#"
        parallel n in [1, 2] {
            let listener = TCP { listen: "127.0.0.1:0" };
            set_timeout(listener, 1s);
            let client = TCP local_address(listener);
            let next = accept;
            let peer = next(listener);
            client SEND "A";
            print(decode_utf8(peer.connection RECEIVE));
            peer.connection SEND "B";
            print(decode_utf8(client RECEIVE));
            close(peer.connection);
            close(client);
            close(listener);
        }
    "#;
    assert_eq!(run(program).unwrap(), "A\nB\nA\nB\n");
}

#[test]
fn listener_configuration_and_builtin_arity_errors_are_explicit() {
    for source in [
        "TCP {};",
        "TCP { listen: 42 };",
        "TCP { listen: \"127.0.0.1:0\", bind: \"x\" };",
        "TCP { listen: \"127.0.0.1:0\" } using Login;",
        "UDP { listen: \"127.0.0.1:0\" };",
        "accept();",
        "local_address();",
        "let f = accept; f();",
        "let f = local_address; f();",
    ] {
        assert!(run(source).is_err(), "{source}");
    }
}
