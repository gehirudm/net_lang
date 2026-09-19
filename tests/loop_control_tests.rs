use netlang::{
    ast::{Program, Stmt},
    interpreter::{Limits, execute, execute_with_limits},
    lexer::Lexer,
    parser::Parser,
    runtime::StandardRuntime,
    semantic::{SemanticErrorKind as K, analyze},
};

fn parse(source: &str, spans: bool) -> Program {
    let parser = Parser::new(Lexer::new(source).unwrap().tokenize().unwrap()).unwrap();
    let mut parser = if spans { parser.with_spans() } else { parser };
    parser.parse_program().unwrap()
}

fn output(source: &str) -> String {
    let mut output = Vec::new();
    execute(&parse(source, true), &mut StandardRuntime::new(&mut output)).unwrap();
    String::from_utf8(output).unwrap()
}

#[test]
fn syntax_ast_and_locations_require_semicolons() {
    let program = parse("break; continue;", false);
    assert_eq!(program.statements, vec![Stmt::Break, Stmt::Continue]);
    let located = parse("break; continue;", true);
    assert_eq!(program.pretty(), located.pretty());
    assert!(program.pretty().contains("Break"));
    assert!(program.pretty().contains("Continue"));
    assert_eq!(located.statements[1].span().unwrap().start.column, 8);
    for source in [
        "break",
        "continue",
        "while true { break }",
        "continue 1;",
        "let x = break;",
    ] {
        let error = Parser::new(Lexer::new(source).unwrap().tokenize().unwrap())
            .unwrap()
            .parse_program()
            .unwrap_err();
        assert!(!error.message.is_empty());
    }
}

#[test]
fn semantic_checks_respect_function_and_worker_boundaries_before_effects() {
    for (source, kind) in [
        ("break;", K::BreakOutsideLoop),
        ("continue;", K::ContinueOutsideLoop),
        ("while false { fn f() { break; } }", K::BreakOutsideLoop),
        (
            "for x in [] { fn f() { continue; } }",
            K::ContinueOutsideLoop,
        ),
        (
            "while false { parallel x in [] { break; } }",
            K::BreakAcrossParallel,
        ),
        (
            "parallel x in [] { fn f() { continue; } }",
            K::ContinueOutsideLoop,
        ),
    ] {
        let program = parse(source, true);
        let errors = analyze(&program).unwrap_err();
        assert_eq!(errors.len(), 1, "{source}: {errors:?}");
        assert_eq!(errors[0].kind, kind);
        assert!(errors[0].span.is_some());
    }
    analyze(&parse("while false { fn f() {} continue; break; } parallel x in [] { while false { break; } continue; }", true)).unwrap();
    let mut output = Vec::new();
    assert!(
        execute(
            &parse("print(1); break;", true),
            &mut StandardRuntime::new(&mut output)
        )
        .is_err()
    );
    assert!(output.is_empty());
    let errors = analyze(&parse("while false {} break; continue;", true)).unwrap_err();
    assert_eq!(
        errors.iter().map(|error| error.kind).collect::<Vec<_>>(),
        vec![K::BreakOutsideLoop, K::ContinueOutsideLoop]
    );
}

#[test]
fn nested_loops_blocks_matches_and_returns_propagate_correctly() {
    let source = r#"
        let n = 0;
        while n < 5 {
            n = n + 1;
            if n == 2 { continue; }
            match n { 4 => { break; } _ => {} }
            for item in [10, 20, 30] {
                if item == 20 { { break; } }
                print(n, item);
            }
        }
        print(n);
        for item in [1, 2, 3, 4] {
            match item { 2 => { continue; } 4 => { break; } _ => {} }
            print(item);
        }
        fn f() { for item in [1, 2] { while true { return item; } } }
        print(f());
    "#;
    assert_eq!(output(source), "1 10\n3 10\n4\n1\n3\n1\n");
    for spans in [false, true] {
        let mut bytes = Vec::new();
        execute(&parse(source, spans), &mut StandardRuntime::new(&mut bytes)).unwrap();
        assert_eq!(String::from_utf8(bytes).unwrap(), output(source));
    }
}

#[test]
fn parallel_continue_finishes_only_its_iteration() {
    assert_eq!(
        output(
            r#"
        parallel x in [1, 2, 3] {
            if x == 2 { continue; }
            for y in [1, 2] { if y == 2 { break; } print(x, y); }
            parallel z in [1, 2] { if z == 1 { continue; } print(x, z); }
            print(x);
        }
        print("done");
    "#
        ),
        "1 1\n1 2\n1\n3 1\n3 2\n3\ndone\n"
    );
}

#[test]
fn continue_consumes_budget_and_locations_do_not_change_limits() {
    let source = "let n = 0; while n < 3 { n = n + 1; continue; print(99); } print(n);";
    for steps in 0..80 {
        let mut outcomes = Vec::new();
        for spans in [false, true] {
            let mut bytes = Vec::new();
            let result = execute_with_limits(
                &parse(source, spans),
                &mut StandardRuntime::new(&mut bytes),
                Limits {
                    steps,
                    ..Limits::default()
                },
            );
            outcomes.push((result.is_ok(), bytes));
        }
        assert_eq!(outcomes[0], outcomes[1], "budget {steps}");
    }
    assert_eq!(output(source), "3\n");
    let error = execute_with_limits(
        &parse("while true { continue; }", true),
        &mut StandardRuntime::new(Vec::new()),
        Limits {
            steps: 20,
            ..Limits::default()
        },
    )
    .unwrap_err();
    assert!(error.to_string().contains("step"));
}
