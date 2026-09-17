use netlang::{
    ast::{BinaryOp, Expr, Stmt, Transport},
    interpreter::execute,
    lexer::Lexer,
    parser::Parser,
    runtime::{Runtime, StandardRuntime, Value},
    semantic,
};

fn parse(source: &str) -> netlang::ast::Program {
    Parser::new(Lexer::new(source).unwrap().tokenize().unwrap())
        .unwrap()
        .parse_program()
        .unwrap()
}
fn expr(source: &str) -> Expr {
    Parser::new(Lexer::new(source).unwrap().tokenize().unwrap())
        .unwrap()
        .parse_expression_complete()
        .unwrap()
}
fn id(name: &str) -> Box<Expr> {
    Box::new(Expr::Identifier(name.into()))
}

#[test]
fn connection_expressions_keep_transport_address_and_protocol() {
    assert_eq!(
        expr("TCP host + port using Login"),
        Expr::Connection {
            transport: Transport::Tcp,
            address: Box::new(Expr::Binary {
                left: id("host"),
                operator: BinaryOp::Add,
                right: id("port")
            }),
            protocol: Some("Login".into()),
        }
    );
    assert_eq!(
        expr("UDP address"),
        Expr::Connection {
            transport: Transport::Udp,
            address: id("address"),
            protocol: None
        }
    );
}

#[test]
fn send_receive_and_destinations_are_first_class_expressions() {
    assert_eq!(
        expr("conn SEND data TO address"),
        Expr::Send {
            connection: id("conn"),
            data: id("data"),
            destination: Some(id("address"))
        }
    );
    assert_eq!(
        expr("conn SEND data"),
        Expr::Send {
            connection: id("conn"),
            data: id("data"),
            destination: None
        }
    );
    assert_eq!(
        expr("conn RECEIVE"),
        Expr::Receive {
            connection: id("conn")
        }
    );
    assert_eq!(
        expr("x = conn RECEIVE"),
        Expr::Assignment {
            target: id("x"),
            value: Box::new(Expr::Receive {
                connection: id("conn")
            })
        }
    );
    assert_eq!(
        expr("conn SEND 1 + 2 * 3"),
        Expr::Send {
            connection: id("conn"),
            data: Box::new(Expr::Binary {
                left: Box::new(Expr::Integer(1)),
                operator: BinaryOp::Add,
                right: Box::new(Expr::Binary {
                    left: Box::new(Expr::Integer(2)),
                    operator: BinaryOp::Multiply,
                    right: Box::new(Expr::Integer(3))
                })
            }),
            destination: None
        }
    );
}

#[test]
fn receive_composes_with_postfix_and_control_flow() {
    assert_eq!(
        expr("connections[0] RECEIVE.status"),
        Expr::Property {
            object: Box::new(Expr::Receive {
                connection: Box::new(Expr::Index {
                    object: id("connections"),
                    index: Box::new(Expr::Integer(0))
                })
            }),
            name: "status".into()
        }
    );
    assert!(
        matches!(expr("(TCP address) RECEIVE"), Expr::Receive { connection } if matches!(*connection, Expr::Connection { .. }))
    );
    let program = parse(
        "let conn = TCP address; if conn RECEIVE == 1 {} while conn RECEIVE {} match conn RECEIVE { _ => {} }",
    );
    assert_eq!(program.statements.len(), 4);
    assert!(matches!(program.statements[1], Stmt::If { .. }));
}

#[test]
fn malformed_transport_syntax_is_rejected() {
    for source in [
        "TCP",
        "UDP",
        "TCP address using",
        "conn SEND",
        "conn SEND data TO",
        "conn TO address",
        "conn SEND data SEND other",
        "conn RECEIVE = 1",
        "TCP address using 1",
    ] {
        assert!(
            Parser::new(Lexer::new(source).unwrap().tokenize().unwrap())
                .unwrap()
                .parse_expression_complete()
                .is_err(),
            "{source}"
        );
    }
}

#[test]
fn semantic_analysis_visits_transport_operands_and_pretty_output_is_readable() {
    let errors = semantic::analyze(&parse(
        "TCP missing using Login; conn SEND data TO address; other RECEIVE;",
    ))
    .unwrap_err();
    assert_eq!(errors.len(), 5);
    assert!(
        errors
            .iter()
            .all(|error| error.kind == semantic::SemanticErrorKind::UndefinedName)
    );
    let program = parse(
        "let conn = TCP \"example.com:9000\" using Login; conn SEND \"hi\" TO \"peer:9000\"; conn RECEIVE;",
    );
    semantic::analyze(&program).unwrap();
    let pretty = program.pretty();
    for label in [
        "Connection TCP",
        "Address",
        "Protocol Login",
        "Send",
        "Data",
        "Destination",
        "Receive",
    ] {
        assert!(pretty.contains(label));
    }
}

#[derive(Default)]
struct MockTransport {
    events: Vec<String>,
    printed: Vec<String>,
}
impl Runtime for MockTransport {
    fn print(&mut self, text: &str) -> Result<(), String> {
        self.printed.push(text.into());
        Ok(())
    }
    fn connect(
        &mut self,
        transport: Transport,
        address: &str,
        protocol: Option<&str>,
    ) -> Result<Value, String> {
        self.events
            .push(format!("connect {transport:?} {address} {protocol:?}"));
        Ok(Value::String("test-handle".into()))
    }
    fn send(
        &mut self,
        connection: &Value,
        data: &Value,
        destination: Option<&str>,
    ) -> Result<Value, String> {
        assert_eq!(connection, &Value::String("test-handle".into()));
        self.events.push(format!("send {data} {destination:?}"));
        Ok(Value::Null)
    }
    fn receive(&mut self, connection: &Value) -> Result<Value, String> {
        assert_eq!(connection, &Value::String("test-handle".into()));
        self.events.push("receive".into());
        Ok(Value::String("reply".into()))
    }
    fn fork(&mut self) -> Result<Box<dyn Runtime + Send>, String> {
        Ok(Box::new(Self::default()))
    }
}

#[test]
fn interpreter_routes_connection_and_operators_through_the_runtime() {
    let mut host = MockTransport::default();
    execute(
        &parse(
            r#"fn main() {
        let conn = TCP "example.com:9000" using Login;
        conn SEND "hello";
        print(conn RECEIVE);
        let socket = UDP "peer:5000";
        socket SEND "packet" TO "other:5001";
    }"#,
        ),
        &mut host,
    )
    .unwrap();
    assert_eq!(
        host.events,
        [
            "connect Tcp example.com:9000 Some(\"Login\")",
            "send hello None",
            "receive",
            "connect Udp peer:5000 None",
            "send packet Some(\"other:5001\")"
        ]
    );
    assert_eq!(host.printed, ["reply"]);
    let mut host = MockTransport::default();
    execute(
        &parse(
            r#"parallel address in ["a:1", "b:2"] {
        let conn = TCP address; conn SEND "hi"; print(conn RECEIVE);
    }"#,
        ),
        &mut host,
    )
    .unwrap();
    assert_eq!(host.printed, ["reply", "reply"]);
}

#[test]
fn standard_runtime_rejects_invalid_transport_operations_explicitly() {
    for (source, expected) in [
        ("TCP \"localhost:9000\" using Login;", "not implemented"),
        ("1 SEND \"hello\";", "requires a connection"),
        ("1 RECEIVE;", "requires a connection"),
        ("TCP 42;", "address must be a string"),
        ("1 SEND \"hi\" TO 42;", "destination must be a string"),
    ] {
        let error = execute(&parse(source), &mut StandardRuntime::new(Vec::new())).unwrap_err();
        assert!(error.to_string().contains(expected), "{error}");
    }
}
