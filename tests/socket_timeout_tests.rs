use netlang::{
    ast::Transport,
    interpreter::execute,
    lexer::Lexer,
    parser::Parser,
    runtime::{Runtime, StandardRuntime, Value},
};
use std::{
    net::{TcpListener, UdpSocket},
    time::{Duration, Instant},
};

#[test]
fn configured_timeout_applies_to_tcp_and_both_udp_modes() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
    let mut runtime = StandardRuntime::new(Vec::new());
    let tcp = runtime
        .connect(
            Transport::Tcp,
            &listener.local_addr().unwrap().to_string(),
            None,
        )
        .unwrap();
    let (_accepted, _) = listener.accept().unwrap();
    let udp = runtime
        .connect(
            Transport::Udp,
            &peer.local_addr().unwrap().to_string(),
            None,
        )
        .unwrap();
    let unconnected = runtime.bind_udp("127.0.0.1:0").unwrap();
    for connection in [tcp, udp, unconnected] {
        runtime.set_timeout(&connection, 30).unwrap();
        let started = Instant::now();
        assert!(runtime.receive(&connection).is_err());
        assert!(started.elapsed() >= Duration::from_millis(10));
        assert!(started.elapsed() < Duration::from_secs(3));
        for duration in [0, 86_400_001, u64::MAX] {
            assert!(runtime.set_timeout(&connection, duration).is_err());
        }
        assert!(
            runtime
                .fork()
                .unwrap()
                .set_timeout(&connection, 30)
                .is_err()
        );
        runtime.close(&connection).unwrap();
        assert!(runtime.set_timeout(&connection, 30).is_err());
    }
    assert!(runtime.set_timeout(&Value::Null, 30).is_err());
}

fn run(source: &str) -> Result<(), String> {
    let program = Parser::new(Lexer::new(source).unwrap().tokenize().unwrap())
        .unwrap()
        .parse_program()
        .unwrap();
    execute(&program, &mut StandardRuntime::new(Vec::new()))
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[test]
fn builtin_validates_arity_and_duration_and_forwards_worker_effects() {
    run("parallel n in [1, 2] { let socket = UDP { bind: \"127.0.0.1:0\" }; let configure = set_timeout; configure(socket, 25ms); close(socket); }").unwrap();
    for source in [
        "set_timeout();",
        "let f = set_timeout; f(null);",
        "set_timeout(null, 0ms);",
        "set_timeout(null, 1441m);",
        "set_timeout(null, 10);",
        "set_timeout(null, 10ms);",
    ] {
        assert!(run(source).is_err(), "{source}");
    }
    assert!(
        run(
            "let socket = UDP { bind: \"127.0.0.1:0\" }; set_timeout(socket, 25ms); socket RECEIVE;"
        )
        .unwrap_err()
        .contains("UDP RECEIVE failed")
    );
}
