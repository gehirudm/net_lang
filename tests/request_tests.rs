use netlang::{
    ast::{BinaryOp, Expr, HttpMethod, ObjectField, Stmt},
    lexer::Lexer,
    parser::Parser,
};
fn expression(source: &str) -> Expr {
    Parser::new(Lexer::new(source).unwrap().tokenize().unwrap())
        .unwrap()
        .parse_expression_complete()
        .unwrap()
}
#[test]
fn all_methods_without_config() {
    for (word, method) in [
        ("GET", HttpMethod::Get),
        ("POST", HttpMethod::Post),
        ("PUT", HttpMethod::Put),
        ("PATCH", HttpMethod::Patch),
        ("DELETE", HttpMethod::Delete),
        ("HEAD", HttpMethod::Head),
    ] {
        assert_eq!(
            expression(&format!("{word} \"https://example.com\"")),
            Expr::Request {
                method,
                url: Box::new(Expr::String("https://example.com".into())),
                config: None
            }
        );
    }
}
#[test]
fn request_url_expression_and_nested_configuration() {
    let Expr::Request {
        method,
        url,
        config,
    } = expression(
        "POST api + \"/users\" { headers: { \"Authorization\": \"Bearer abc\" }, query: { page: 1 }, json: { name: \"Alice\", age: 24, active: true }, timeout: 5s, retry: 3, }",
    )
    else {
        panic!()
    };
    assert_eq!(method, HttpMethod::Post);
    assert_eq!(
        *url,
        Expr::Binary {
            left: Box::new(Expr::Identifier("api".into())),
            operator: BinaryOp::Add,
            right: Box::new(Expr::String("/users".into()))
        }
    );
    let Expr::Object(fields) = *config.unwrap() else {
        panic!()
    };
    assert_eq!(fields.len(), 5);
    assert_eq!(
        fields[0],
        ObjectField {
            key: "headers".into(),
            value: Expr::Object(vec![ObjectField {
                key: "Authorization".into(),
                value: Expr::String("Bearer abc".into())
            }])
        }
    );
    assert_eq!(fields[3].value, Expr::Duration(5000));
    assert_eq!(fields[4].value, Expr::Integer(3));
    assert!(
        matches!(expression("GET url {}"), Expr::Request { config: Some(c), .. } if *c == Expr::Object(vec![]))
    );
}
#[test]
fn parentheses_separate_result_operations_and_control_flow_blocks() {
    assert!(
        matches!(expression("(GET url).status"), Expr::Property { object, .. } if matches!(*object, Expr::Request { config: None, .. }))
    );
    let source = "if (GET url { timeout: 5s }).status == 200 {}";
    let program = Parser::new(Lexer::new(source).unwrap().tokenize().unwrap())
        .unwrap()
        .parse_program()
        .unwrap();
    assert!(matches!(
        &program.statements[0],
        Stmt::If {
            condition: Expr::Binary {
                operator: BinaryOp::Equal,
                ..
            },
            ..
        }
    ));
}
#[test]
fn request_errors() {
    for source in ["GET", "GET url { timeout 5s }", "GET url { timeout: }"] {
        assert!(
            Parser::new(Lexer::new(source).unwrap().tokenize().unwrap())
                .unwrap()
                .parse_expression_complete()
                .is_err()
        );
    }
}
