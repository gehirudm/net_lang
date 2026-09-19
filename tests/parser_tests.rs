use netlang::{
    ast::{Expr, Program, Stmt},
    lexer::Lexer,
    parser::{ParseError, Parser},
};
fn parse(s: &str) -> Result<Program, ParseError> {
    Parser::new(Lexer::new(s).unwrap().tokenize().unwrap())
        .unwrap()
        .parse_program()
}
#[test]
fn functions_variables_and_return() {
    assert_eq!(
        parse("fn greet(name) { let x = 42; print(name); return x; }").unwrap(),
        Program {
            statements: vec![Stmt::Function {
                name: "greet".into(),
                parameters: vec!["name".into()],
                body: Box::new(Stmt::Block(vec![
                    Stmt::Let {
                        annotation: None,
                        name: "x".into(),
                        value: Expr::Integer(42)
                    },
                    Stmt::Expression(Expr::Call {
                        callee: Box::new(Expr::Identifier("print".into())),
                        arguments: vec![Expr::Identifier("name".into())]
                    }),
                    Stmt::Return {
                        value: Some(Expr::Identifier("x".into()))
                    },
                ]))
            }]
        }
    );
    assert_eq!(
        parse("return;").unwrap().statements,
        vec![Stmt::Return { value: None }]
    );
}
#[test]
fn conditionals_and_loops() {
    let program =
        parse("if true {} else if false {} else {} while true {} for item in [1, 2] { item = 3; }")
            .unwrap();
    let Stmt::If {
        condition,
        then_branch,
        else_branch,
    } = &program.statements[0]
    else {
        panic!()
    };
    assert_eq!(condition, &Expr::Boolean(true));
    assert_eq!(**then_branch, Stmt::Block(vec![]));
    let Stmt::If {
        condition,
        else_branch,
        ..
    } = else_branch.as_deref().unwrap()
    else {
        panic!()
    };
    assert_eq!(condition, &Expr::Boolean(false));
    assert_eq!(else_branch.as_deref(), Some(&Stmt::Block(vec![])));
    assert_eq!(
        program.statements[1],
        Stmt::While {
            condition: Expr::Boolean(true),
            body: Box::new(Stmt::Block(vec![]))
        }
    );
    let Stmt::For {
        variable,
        iterable,
        body,
    } = &program.statements[2]
    else {
        panic!()
    };
    assert_eq!(variable, "item");
    assert_eq!(
        iterable,
        &Expr::Array(vec![Expr::Integer(1), Expr::Integer(2)])
    );
    assert!(
        matches!(body.as_ref(), Stmt::Block(s) if matches!(&s[0], Stmt::Expression(Expr::Assignment { .. })))
    );
}
#[test]
fn empty_program_and_nested_blocks() {
    assert_eq!(parse("").unwrap().statements, vec![]);
    assert_eq!(
        parse("{{}}").unwrap().statements,
        vec![Stmt::Block(vec![Stmt::Block(vec![])])]
    );
    let program = parse("fn f(a, b) { fn g() {} }").unwrap();
    assert!(
        matches!(&program.statements[0], Stmt::Function { parameters, .. } if parameters == &["a", "b"])
    );
}
#[test]
fn malformed_statements_are_located() {
    let error = parse("let x = 1\nlet y = 2;").unwrap_err();
    assert_eq!((error.line, error.column), (2, 1));
    assert_eq!(error.message, "expected ';' after variable declaration");
    for source in [
        "let = 1;",
        "let x 1;",
        "fn () {}",
        "fn f(a {}",
        "{",
        "if true print(1);",
        "for x xs {}",
        "return 1",
        "foo()",
        "else {}",
        "while true",
    ] {
        assert!(parse(source).is_err(), "{source}");
    }
}
