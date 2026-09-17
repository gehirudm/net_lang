use netlang::{
    ast::{Expr, Program, Stmt},
    lexer::Lexer,
    parser::Parser,
    semantic::{SemanticError, SemanticErrorKind as K, analyze},
};

fn check(source: &str) -> Result<(), Vec<SemanticError>> {
    let program = Parser::new(Lexer::new(source).unwrap().tokenize().unwrap())
        .unwrap()
        .parse_program()
        .unwrap();
    analyze(&program)
}

fn errors(source: &str) -> Vec<SemanticError> {
    check(source).unwrap_err()
}

#[test]
fn complete_examples_pass() {
    for source in [
        "",
        include_str!("../examples/hello.net"),
        include_str!("../examples/request.net"),
        include_str!("../examples/parallel.net"),
        include_str!("../examples/complete.net"),
    ] {
        assert_eq!(check(source), Ok(()));
    }
}

#[test]
fn lexical_scope_shadowing_and_mutable_bindings() {
    assert_eq!(
        check(
            r#"
        let x = 1;
        { let x = x + 1; x = 3; print(x); }
        x = 4;
        fn f(value) { value = 2; { let value = 3; } return value; }
        for x in [x] { x = 5; }
        parallel x in [x] { x = 6; }
        let print = f;
        print(1);
    "#
        ),
        Ok(())
    );
}

#[test]
fn variables_require_declaration_before_use() {
    for source in [
        "print(x); let x = 1;",
        "let x = x;",
        "x = 1;",
        "{ let hidden = 1; } print(hidden);",
        "fn f() { print(later); } let later = 1;",
    ] {
        let diagnostics = errors(source);
        assert_eq!(diagnostics.len(), 1, "{source}: {diagnostics:?}");
        assert_eq!(diagnostics[0].kind, K::UndefinedName);
    }
}

#[test]
fn duplicates_in_shared_scopes_are_rejected() {
    for source in [
        "let x = 1; let x = 2;",
        "fn f() {} fn f() {}",
        "fn f(x, x) {}",
        "fn f(x) { let x = 1; }",
        "let f = 1; fn f() {}",
        "fn f() {} let f = 1;",
        "for x in [] { let x = 1; }",
        "parallel x in [] { fn x() {} }",
    ] {
        let diagnostics = errors(source);
        assert_eq!(diagnostics.len(), 1, "{source}: {diagnostics:?}");
        assert_eq!(diagnostics[0].kind, K::DuplicateDeclaration);
    }
}

#[test]
fn functions_support_forward_calls_and_mutual_recursion() {
    assert_eq!(
        check(
            r#"
        f(1);
        fn f(x) { return g(x); }
        fn g(x) { return f(x); }
        fn outer() {
            inner();
            fn inner() { return inner(); }
        }
    "#
        ),
        Ok(())
    );
    assert_eq!(
        errors("{ fn local() {} } local();")[0].kind,
        K::UndefinedName
    );
}

#[test]
fn direct_calls_check_arity_using_the_nearest_binding() {
    let diagnostics = errors("fn f(x) {} f(); f(1, 2);");
    assert_eq!(diagnostics.len(), 2);
    assert!(
        diagnostics
            .iter()
            .all(|error| error.kind == K::ArgumentCount)
    );
    assert_eq!(
        diagnostics[0].message,
        "function 'f' expects 1 argument(s), got 0"
    );
    assert_eq!(check("fn f(x) {} { fn f() {} f(); } f(1);"), Ok(()));
    // Dynamic callees and the built-in print have no inferred signature yet.
    assert_eq!(
        check("fn f(x) {} { let f = null; f(); } print(); print(1, 2);"),
        Ok(())
    );
}

#[test]
fn function_bindings_cannot_be_reassigned() {
    let diagnostics = errors("fn f() {} f = 1; print = 2;");
    assert_eq!(diagnostics.len(), 2);
    assert!(
        diagnostics
            .iter()
            .all(|error| error.kind == K::ImmutableAssignment)
    );
    assert_eq!(check("fn f() {} { let f = 1; f = 2; }"), Ok(()));
}

#[test]
fn return_context_is_restored_after_functions() {
    let diagnostics =
        errors("fn f() { fn g() { return; } return; } return 1; while true { return; }");
    assert_eq!(diagnostics.len(), 2);
    assert!(
        diagnostics
            .iter()
            .all(|error| error.kind == K::ReturnOutsideFunction)
    );
}

#[test]
fn loops_branches_and_match_arms_do_not_leak_bindings() {
    let diagnostics = errors(
        r#"
        for x in [x] { print(x); } print(x);
        parallel y in [y] { print(y); } print(y);
        if true { let a = 1; } else { print(a); }
        while true { let b = 1; } print(b);
        match 1 { 1 => { let c = 1; } _ => { print(c); } }
    "#,
    );
    assert_eq!(diagnostics.len(), 7, "{diagnostics:?}");
    assert!(
        diagnostics
            .iter()
            .all(|error| error.kind == K::UndefinedName)
    );
}

#[test]
fn traversal_checks_every_expression_position_but_not_property_or_object_keys() {
    let diagnostics = errors(
        r#"
        let a = [missing_array];
        let b = { key: missing_field };
        let c = -missing_unary + missing_binary;
        missing_callee(missing_argument);
        missing_object.property;
        missing_base[missing_index] = missing_value;
        GET missing_url { timeout: missing_timeout };
        if missing_condition {} else if missing_else_condition {}
        while missing_while {}
        match missing_match { _ => {} }
        fn f() { return missing_return; }
    "#,
    );
    assert_eq!(diagnostics.len(), 17, "{diagnostics:?}");
    assert!(
        diagnostics
            .iter()
            .all(|error| error.kind == K::UndefinedName)
    );
    assert_eq!(check("let x = { key: 1 }; x.key = 2; x[0] = 3;"), Ok(()));
}

#[test]
fn errors_keep_context_and_analysis_has_no_cross_program_state() {
    let diagnostics = errors("fn f() { print(missing); }");
    assert_eq!(
        diagnostics[0].context,
        vec!["program", "statement 1", "function 'f'", "statement 1"]
    );
    assert!(
        diagnostics[0]
            .to_string()
            .contains("undefined name 'missing'")
    );
    assert_eq!(check("let x = 1;"), Ok(()));
    assert_eq!(errors("x;")[0].kind, K::UndefinedName);
    assert_eq!(check("print(1);"), Ok(()));
}

#[test]
fn independently_built_asts_are_checked_without_a_lexer() {
    let program = Program {
        statements: vec![Stmt::Expression(Expr::Assignment {
            target: Box::new(Expr::Integer(1)),
            value: Box::new(Expr::Identifier("missing".into())),
        })],
    };
    let diagnostics = analyze(&program).unwrap_err();
    assert_eq!(
        diagnostics.iter().map(|e| e.kind).collect::<Vec<_>>(),
        vec![K::InvalidAssignmentTarget, K::UndefinedName]
    );
}
