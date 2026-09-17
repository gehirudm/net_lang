use netlang::{
    ast::{Expr, HttpMethod, Pattern, Program, Stmt},
    lexer::Lexer,
    parser::Parser,
};
fn parse(s: &str) -> Program {
    Parser::new(Lexer::new(s).unwrap().tokenize().unwrap())
        .unwrap()
        .parse_program()
        .unwrap()
}
#[test]
fn parallel_and_all_match_patterns() {
    let program = parse(
        "parallel url in urls { GET url; } match status { 200 => {} \"ok\" => {} true => {} false => {} null => {} _ => {} }",
    );
    assert_eq!(
        program.statements[0],
        Stmt::Parallel {
            variable: "url".into(),
            iterable: Expr::Identifier("urls".into()),
            body: Box::new(Stmt::Block(vec![Stmt::Expression(Expr::Request {
                method: HttpMethod::Get,
                url: Box::new(Expr::Identifier("url".into())),
                config: None
            })]))
        }
    );
    let Stmt::Match { expression, arms } = &program.statements[1] else {
        panic!()
    };
    assert_eq!(expression, &Expr::Identifier("status".into()));
    assert_eq!(
        arms.iter().map(|a| a.pattern.clone()).collect::<Vec<_>>(),
        vec![
            Pattern::Integer(200),
            Pattern::String("ok".into()),
            Pattern::Boolean(true),
            Pattern::Boolean(false),
            Pattern::Null,
            Pattern::Wildcard
        ]
    );
    assert!(arms.iter().all(|a| a.body == Stmt::Block(vec![])));
}
#[test]
fn success_criterion_ast() {
    let program = parse(include_str!("../examples/complete.net"));
    let Stmt::Function {
        name,
        parameters,
        body,
    } = &program.statements[0]
    else {
        panic!()
    };
    assert_eq!(name, "main");
    assert!(parameters.is_empty());
    let Stmt::Block(statements) = body.as_ref() else {
        panic!()
    };
    assert!(
        matches!(&statements[0], Stmt::Let { name, value: Expr::Array(urls) } if name == "urls" && urls.len() == 2)
    );
    let Stmt::Parallel {
        variable,
        iterable,
        body,
    } = &statements[1]
    else {
        panic!()
    };
    assert_eq!(variable, "url");
    assert_eq!(iterable, &Expr::Identifier("urls".into()));
    let Stmt::Block(statements) = body.as_ref() else {
        panic!()
    };
    let Stmt::Let {
        name,
        value: Expr::Request {
            method,
            url,
            config,
        },
    } = &statements[0]
    else {
        panic!()
    };
    assert_eq!(name, "response");
    assert_eq!(method, &HttpMethod::Get);
    assert_eq!(url.as_ref(), &Expr::Identifier("url".into()));
    let Expr::Object(fields) = config.as_deref().unwrap() else {
        panic!()
    };
    assert_eq!(fields[1].value, Expr::Duration(5000));
    assert_eq!(fields[2].value, Expr::Integer(3));
    let Stmt::Match { expression, arms } = &statements[1] else {
        panic!()
    };
    assert_eq!(
        expression,
        &Expr::Property {
            object: Box::new(Expr::Identifier("response".into())),
            name: "status".into()
        }
    );
    assert_eq!(
        arms.iter().map(|a| a.pattern.clone()).collect::<Vec<_>>(),
        vec![
            Pattern::Integer(200),
            Pattern::Integer(404),
            Pattern::Wildcard
        ]
    );
    assert_eq!(
        arms[0].body,
        Stmt::Block(vec![Stmt::Expression(Expr::Call {
            callee: Box::new(Expr::Identifier("print".into())),
            arguments: vec![Expr::Property {
                object: Box::new(Expr::Identifier("response".into())),
                name: "body".into()
            }]
        })])
    );
}
#[test]
fn all_examples_parse() {
    for source in [
        include_str!("../examples/hello.net"),
        include_str!("../examples/request.net"),
        include_str!("../examples/parallel.net"),
        include_str!("../examples/match.net"),
    ] {
        assert!(!parse(source).statements.is_empty());
    }
}
#[test]
fn invalid_patterns_and_parallel_statements() {
    for source in [
        "match x { name => {} }",
        "match x { 1.5 => {} }",
        "match x { 1 {} }",
        "match x { _ => print(x); }",
        "match x {",
        "parallel in urls {}",
        "parallel x urls {}",
    ] {
        assert!(
            Parser::new(Lexer::new(source).unwrap().tokenize().unwrap())
                .unwrap()
                .parse_program()
                .is_err(),
            "{source}"
        );
    }
}
