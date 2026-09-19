use netlang::{
    ast::{Expr, Pattern, Stmt},
    interpreter::{ExecutionError, Limits, execute, execute_with_limits},
    lexer::Lexer,
    parser::Parser,
    runtime::{StandardRuntime, Value},
    semantic,
    source::Span,
};

fn parser(source: &str) -> Parser {
    Parser::new(Lexer::new(source).unwrap().tokenize().unwrap()).unwrap()
}
fn text(source: &str, span: Span) -> &str {
    assert_eq!(span.start.line, 1);
    assert_eq!(span.end.line, 1);
    &source[span.start.column - 1..span.end.column - 1]
}

fn check_tree(expr: &Expr) {
    assert!(expr.span().is_some(), "missing span: {expr:?}");
    match expr.unspanned() {
        Expr::Array(values) => values.iter().for_each(check_tree),
        Expr::Object(fields) | Expr::Construct { fields, .. } => {
            fields.iter().for_each(|f| check_tree(&f.value))
        }
        Expr::Unary { expression, .. } => check_tree(expression),
        Expr::Binary { left, right, .. } => {
            check_tree(left);
            check_tree(right);
        }
        Expr::Assignment { target, value } => {
            check_tree(target);
            check_tree(value);
        }
        Expr::Call { callee, arguments } => {
            check_tree(callee);
            arguments.iter().for_each(check_tree);
        }
        Expr::Property { object, .. } => check_tree(object),
        Expr::Index { object, index } => {
            check_tree(object);
            check_tree(index);
        }
        Expr::Request { url, config, .. } => {
            check_tree(url);
            if let Some(c) = config {
                check_tree(c);
            }
        }
        Expr::Connection { address, .. } => check_tree(address),
        Expr::Send {
            connection,
            data,
            destination,
        } => {
            check_tree(connection);
            check_tree(data);
            if let Some(d) = destination {
                check_tree(d);
            }
        }
        Expr::Receive { connection } => check_tree(connection),
        _ => {}
    }
}

#[test]
fn every_expression_node_has_a_range_including_request_configuration() {
    for source in [
        "a = b = -1 + 2 * 3",
        "foo(1, 2)[0].name",
        "!false || true && a != b",
        "[1.0, null, \"é\\n\", 5s]",
        "GET url + \"/x\" { headers: { Accept: \"json\" }, timeout: 5s }",
        "TCP { listen: address }",
        "conn SEND { name: \"é\" } TO address",
        "conn RECEIVE.status",
        "(array[0]).value = (2 + 3)",
        "Group { user: User { id: 1 } }",
    ] {
        let expr = parser(source)
            .with_spans()
            .parse_expression_complete()
            .unwrap();
        assert_eq!(text(source, expr.span().unwrap()), source);
        check_tree(&expr);
    }
    let source = "1 + 2 * 3 + 4";
    let expr = parser(source)
        .with_spans()
        .parse_expression_complete()
        .unwrap();
    let Expr::Binary { left, .. } = expr.unspanned() else {
        panic!()
    };
    assert_eq!(text(source, left.span().unwrap()), "1 + 2 * 3");
    let Expr::Binary { right, .. } = left.unspanned() else {
        panic!()
    };
    assert_eq!(text(source, right.span().unwrap()), "2 * 3");
}

#[test]
fn literal_patterns_keep_source_spelling_and_match_normally() {
    let source =
        "match \"é\" { 2 => {} \"é\" => { print(42); } true => {} false => {} null => {} _ => {} }";
    let program = parser(source).with_spans().parse_program().unwrap();
    let Stmt::Match { arms, .. } = program.statements[0].unspanned() else {
        panic!()
    };
    let actual: Vec<_> = arms
        .iter()
        .map(|arm| text(source, arm.pattern.span().unwrap()))
        .collect();
    assert_eq!(actual, ["2", "\"é\"", "true", "false", "null", "_"]);
    assert!(matches!(arms[5].pattern.unspanned(), Pattern::Wildcard));
    assert_eq!(
        program.pretty(),
        parser(source).parse_program().unwrap().pretty()
    );
    let mut output = Vec::new();
    execute(&program, &mut StandardRuntime::new(&mut output)).unwrap();
    assert_eq!(output, b"42\n");
}

#[test]
fn errors_select_nested_expressions_and_keep_assignment_and_call_validation() {
    for (source, expected) in [
        ("print(1 + 4 / 0);", "4 / 0"),
        ("let a = []; print(a[0]);", "a[0]"),
        ("if 42 {}", "42"),
        ("print(encode_utf8(1));", "encode_utf8(1)"),
        ("let x = 1; x = 2 / 0;", "2 / 0"),
    ] {
        let error = execute(
            &parser(source).with_spans().parse_program().unwrap(),
            &mut StandardRuntime::new(Vec::new()),
        )
        .unwrap_err();
        let ExecutionError::Runtime(error) = error else {
            panic!()
        };
        assert_eq!(text(source, error.span.unwrap()), expected);
    }
    for (source, expected) in [
        ("print(unknown + 1);", "unknown"),
        ("(unknown) = 2;", "(unknown)"),
        ("fn f(a) {} (f)();", "(f)()"),
        (
            "let a = [{x: 0}]; parallel n in [1] { (a[0]).x = n; }",
            "(a[0]).x",
        ),
    ] {
        let errors =
            semantic::analyze(&parser(source).with_spans().parse_program().unwrap()).unwrap_err();
        assert_eq!(text(source, errors[0].span.unwrap()), expected);
    }
}

#[test]
fn locations_preserve_assignments_short_circuit_and_budget_limits() {
    let source =
        "let a = [{x: 0}]; let n = 0; (a[n]).x = n = 4; false && (n = 99); print(a[0].x, n);";
    for steps in 0..60 {
        let bare = parser(source).parse_program().unwrap();
        let located = parser(source).with_spans().parse_program().unwrap();
        let limits = Limits {
            steps,
            ..Limits::default()
        };
        let mut a = Vec::new();
        let mut b = Vec::new();
        let ra = execute_with_limits(&bare, &mut StandardRuntime::new(&mut a), limits);
        let rb = execute_with_limits(&located, &mut StandardRuntime::new(&mut b), limits);
        assert_eq!(ra.is_ok(), rb.is_ok());
        assert_eq!(a, b);
        if steps == 59 {
            assert_eq!(ra.unwrap(), Value::Null);
            assert_eq!(a, b"4 4\n");
        }
    }
}
