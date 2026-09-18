use netlang::{
    ast::Program,
    interpreter::{ExecutionError, Limits, execute, execute_with_limits},
    lexer::Lexer,
    parser::Parser,
    runtime::StandardRuntime,
    semantic,
};

fn parse(source: &str, located: bool) -> Program {
    let parser = Parser::new(Lexer::new(source).unwrap().tokenize().unwrap()).unwrap();
    if located {
        parser.with_spans().parse_program().unwrap()
    } else {
        let mut parser = parser;
        parser.parse_program().unwrap()
    }
}

#[test]
fn optional_spans_preserve_pretty_output_and_execution_semantics() {
    for source in [
        include_str!("../examples/bytes.net"),
        include_str!("../examples/tcp_loopback.net"),
        include_str!("../examples/parallel_compute.net"),
        r#"
        fn main() { let x = 2; fn f(n) { if n == 0 { return x; } return f(n - 1); }
            let a = [{value: 0}]; (a[0]).value = f(2); print(a[0].value);
            while x > 0 { x = x - 1; } for item in [1, 2] { print(item); }
        }
    "#,
    ] {
        let bare = parse(source, false);
        let located = parse(source, true);
        assert_eq!(bare.pretty(), located.pretty());
        assert!(bare.statements[0].span().is_none());
        assert!(located.statements[0].span().is_some());
        let mut expected = Vec::new();
        let mut actual = Vec::new();
        let expected_value = execute(&bare, &mut StandardRuntime::new(&mut expected)).unwrap();
        let actual_value = execute(&located, &mut StandardRuntime::new(&mut actual)).unwrap();
        assert_eq!(expected, actual);
        assert_eq!(expected_value, actual_value);
    }
}

#[test]
fn semantic_errors_have_statement_locations_including_hoisted_duplicates() {
    let source =
        "fn one() {}\nfn one() {}\nfn main() {\n let x = missing;\n print(unknown);\n}\nreturn;";
    let errors = semantic::analyze(&parse(source, true)).unwrap_err();
    let lines: Vec<_> = errors
        .iter()
        .map(|error| error.span.unwrap().start.line)
        .collect();
    assert_eq!(lines, [2, 4, 5, 7]);
    assert!(
        semantic::analyze(&parse(source, false))
            .unwrap_err()
            .iter()
            .all(|error| error.span.is_none())
    );
}

#[test]
fn runtime_errors_preserve_callee_locations_call_stacks_and_worker_locations() {
    for (source, line, message) in [
        (
            "fn fail() {\n return 1 / 0;\n}\nfn main() { fail(); }",
            2,
            "division by zero",
        ),
        (
            "fn main() {\n parallel n in [1] {\n print(1 / 0);\n }\n}",
            3,
            "parallel iteration 1",
        ),
        ("fn main(x) {}", 1, "entry point"),
        ("let x = 1;\nprint(x[0]);", 2, "invalid property/index"),
    ] {
        let error =
            execute(&parse(source, true), &mut StandardRuntime::new(Vec::new())).unwrap_err();
        let ExecutionError::Runtime(error) = error else {
            panic!("expected runtime error")
        };
        assert_eq!(error.span.unwrap().start.line, line);
        assert!(error.message.contains(message), "{error}");
        if source.starts_with("fn fail") {
            assert_eq!(error.call_stack, ["main", "fail"]);
        }
    }
}

#[test]
fn location_wrappers_do_not_consume_execution_budget() {
    let source = "let x = 1; print(x + 2);";
    for steps in 0..15 {
        let mut a = Vec::new();
        let mut b = Vec::new();
        let limits = Limits {
            steps,
            ..Limits::default()
        };
        let bare = execute_with_limits(
            &parse(source, false),
            &mut StandardRuntime::new(&mut a),
            limits,
        );
        let located = execute_with_limits(
            &parse(source, true),
            &mut StandardRuntime::new(&mut b),
            limits,
        );
        assert_eq!(bare.is_ok(), located.is_ok());
        assert_eq!(a, b);
    }
}
