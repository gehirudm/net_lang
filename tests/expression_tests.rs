use netlang::{
    ast::{BinaryOp as B, Expr as E, ObjectField, UnaryOp},
    lexer::Lexer,
    parser::Parser,
};
fn parse(s: &str) -> E {
    Parser::new(Lexer::new(s).unwrap().tokenize().unwrap())
        .unwrap()
        .parse_expression_complete()
        .unwrap()
}
fn id(s: &str) -> E {
    E::Identifier(s.into())
}
fn binary(left: E, operator: B, right: E) -> E {
    E::Binary {
        left: Box::new(left),
        operator,
        right: Box::new(right),
    }
}
#[test]
fn precedence_and_associativity() {
    assert_eq!(
        parse("10 + 20 * 3"),
        binary(
            E::Integer(10),
            B::Add,
            binary(E::Integer(20), B::Multiply, E::Integer(3))
        )
    );
    assert_eq!(
        parse("10 - 2 - 3"),
        binary(
            binary(E::Integer(10), B::Subtract, E::Integer(2)),
            B::Subtract,
            E::Integer(3)
        )
    );
    assert_eq!(
        parse("(10 + 20) * 3"),
        binary(
            binary(E::Integer(10), B::Add, E::Integer(20)),
            B::Multiply,
            E::Integer(3)
        )
    );
    assert_eq!(
        parse("a = b = 3"),
        E::Assignment {
            target: Box::new(id("a")),
            value: Box::new(E::Assignment {
                target: Box::new(id("b")),
                value: Box::new(E::Integer(3))
            })
        }
    );
    assert_eq!(
        parse("a || b && c == d < e + f * -g"),
        binary(
            id("a"),
            B::Or,
            binary(
                id("b"),
                B::And,
                binary(
                    id("c"),
                    B::Equal,
                    binary(
                        id("d"),
                        B::Less,
                        binary(
                            id("e"),
                            B::Add,
                            binary(
                                id("f"),
                                B::Multiply,
                                E::Unary {
                                    operator: UnaryOp::Negate,
                                    expression: Box::new(id("g"))
                                }
                            )
                        )
                    )
                )
            )
        )
    );
}
#[test]
fn every_binary_operator() {
    for (text, op) in [
        ("+", B::Add),
        ("-", B::Subtract),
        ("*", B::Multiply),
        ("/", B::Divide),
        ("%", B::Remainder),
        ("<", B::Less),
        ("<=", B::LessEqual),
        (">", B::Greater),
        (">=", B::GreaterEqual),
        ("==", B::Equal),
        ("!=", B::NotEqual),
        ("&&", B::And),
        ("||", B::Or),
    ] {
        assert_eq!(parse(&format!("a {text} b")), binary(id("a"), op, id("b")));
    }
    assert_eq!(
        parse("!a"),
        E::Unary {
            operator: UnaryOp::Not,
            expression: Box::new(id("a"))
        }
    );
}
#[test]
fn literals_and_objects() {
    assert_eq!(
        parse("[42, 3.14, true, false, null, 100ms, 5s, 2m,]"),
        E::Array(vec![
            E::Integer(42),
            E::Float(314.0 / 100.0),
            E::Boolean(true),
            E::Boolean(false),
            E::Null,
            E::Duration(100),
            E::Duration(5000),
            E::Duration(120000)
        ])
    );
    assert_eq!(
        parse("{name: \"Alice\", \"x-y\": [1]}"),
        E::Object(vec![
            ObjectField {
                key: "name".into(),
                value: E::String("Alice".into())
            },
            ObjectField {
                key: "x-y".into(),
                value: E::Array(vec![E::Integer(1)])
            }
        ])
    );
    assert_eq!(
        parse("\"héllo\\n\\t\\r\\\"\\\\\""),
        E::String("héllo\n\t\r\"\\".into())
    );
    assert_eq!(parse("{}"), E::Object(vec![]));
}
#[test]
fn postfix_chains() {
    assert_eq!(
        parse("foo(1)[0].name"),
        E::Property {
            object: Box::new(E::Index {
                object: Box::new(E::Call {
                    callee: Box::new(id("foo")),
                    arguments: vec![E::Integer(1)]
                }),
                index: Box::new(E::Integer(0))
            }),
            name: "name".into()
        }
    );
    for text in ["x.y = 1", "x[0] = 1"] {
        assert!(matches!(parse(text), E::Assignment { .. }));
    }
}
#[test]
fn malformed_expressions_return_errors() {
    for source in [
        "",
        "1 = 2",
        "1 +",
        "f(1",
        "x[0",
        "x.",
        "{x 1}",
        "[1 2]",
        "9223372036854775808",
        "18446744073709551615m",
        "1 2",
    ] {
        assert!(
            Parser::new(Lexer::new(source).unwrap().tokenize().unwrap())
                .unwrap()
                .parse_expression_complete()
                .is_err(),
            "{source}"
        );
    }
    assert!(Parser::new(vec![]).is_err());
}
