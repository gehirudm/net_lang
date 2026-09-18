use netlang::{
    interpreter::execute,
    lexer::Lexer,
    parser::Parser,
    runtime::{Runtime, StandardRuntime, Value},
};
use std::{net::UdpSocket, thread, time::Duration};

#[test]
fn interpreter_binds_receives_sender_and_replies_with_binary_data() {
    let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
    let address = peer.local_addr().unwrap();
    let server = thread::spawn(move || {
        let mut data = [0; 32];
        let (n, sender) = peer.recv_from(&mut data).unwrap();
        assert_eq!(&data[..n], b"hello");
        peer.send_to(&[255, 0, 42], sender).unwrap();
        let (n, reply_sender) = peer.recv_from(&mut data).unwrap();
        assert_eq!(sender, reply_sender);
        assert_eq!(&data[..n], &[255, 0, 42]);
    });
    let source = format!(
        r#"
        let socket = UDP {{ bind: "127.0.0.1:0" }};
        socket SEND "hello" TO "{address}";
        let packet = socket RECEIVE;
        print(packet.data);
        socket SEND packet.data TO packet.address;
        close(socket);
    "#
    );
    let mut output = Vec::new();
    run(&source, &mut StandardRuntime::new(&mut output)).unwrap();
    assert_eq!(output, b"bytes[255, 0, 42]\n");
    server.join().unwrap();
}

fn run(source: &str, runtime: &mut impl Runtime) -> Result<(), String> {
    let program = Parser::new(Lexer::new(source).unwrap().tokenize().unwrap())
        .unwrap()
        .parse_program()
        .unwrap();
    execute(&program, runtime)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[test]
fn datagrams_from_multiple_peers_keep_their_addresses_including_empty_data() {
    let mut runtime = StandardRuntime::new(Vec::new());
    let socket = runtime.bind_udp("127.0.0.1:0").unwrap();
    for payload in [vec![], vec![255, 1]] {
        let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
        peer.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        runtime
            .send(
                &socket,
                &Value::Bytes(vec![]),
                Some(&peer.local_addr().unwrap().to_string()),
            )
            .unwrap();
        let (_, sender) = peer.recv_from(&mut [0; 1]).unwrap();
        peer.send_to(&payload, sender).unwrap();
        let Value::Object(packet) = runtime.receive(&socket).unwrap() else {
            panic!("expected packet")
        };
        assert_eq!(packet["data"], Value::Bytes(payload));
        assert_eq!(
            packet["address"],
            Value::String(peer.local_addr().unwrap().to_string())
        );
    }
    assert!(
        runtime
            .send(&socket, &Value::Bytes(vec![]), None)
            .unwrap_err()
            .contains("requires TO")
    );
    assert!(
        runtime
            .send(&socket, &Value::Bytes(vec![]), Some("invalid"))
            .is_err()
    );
    runtime.close(&socket).unwrap();
}

#[test]
fn configuration_errors_and_parallel_runtime_forwarding() {
    for source in [
        "UDP {};",
        "UDP { bind: 42 };",
        "UDP { bind: \"127.0.0.1:0\", typo: 1 };",
        "TCP { bind: \"127.0.0.1:0\" };",
        "UDP { bind: \"127.0.0.1:0\" } using Login;",
    ] {
        assert!(
            run(source, &mut StandardRuntime::new(Vec::new())).is_err(),
            "{source}"
        );
    }
    run(
        "parallel n in [1, 2] { let s = UDP { bind: \"127.0.0.1:0\" }; close(s); }",
        &mut StandardRuntime::new(Vec::new()),
    )
    .unwrap();
    let occupied = UdpSocket::bind("127.0.0.1:0").unwrap();
    assert!(
        StandardRuntime::new(Vec::new())
            .bind_udp(&occupied.local_addr().unwrap().to_string())
            .is_err()
    );
}
