use netlang::{
    ast::{Expr, Stmt},
    interpreter::{ExecutionError, execute},
    lexer::Lexer,
    parser::Parser,
    runtime::StandardRuntime,
    semantic::{SemanticErrorKind, analyze},
};

fn parse(source: &str, spans: bool) -> netlang::ast::Program {
    let parser = Parser::new(Lexer::new(source).unwrap().tokenize().unwrap()).unwrap();
    let mut parser = if spans { parser.with_spans() } else { parser };
    parser.parse_program().unwrap()
}

fn run(source: &str) -> Result<String, ExecutionError> {
    let mut bytes = Vec::new();
    execute(&parse(source, true), &mut StandardRuntime::new(&mut bytes))?;
    Ok(String::from_utf8(bytes).unwrap())
}

#[test]
fn annotations_are_names_with_optional_spans_and_do_not_reserve_identifiers() {
    let source = "let port: int = 8080; let int = 3;";
    let program = parse(source, true);
    let Stmt::Let {
        name,
        annotation,
        value,
    } = program.statements[0].unspanned()
    else {
        panic!()
    };
    assert_eq!(name, "port");
    let annotation = annotation.as_ref().unwrap();
    assert_eq!(annotation.name, "int");
    assert_eq!(annotation.span.unwrap().start.column, 11);
    assert_eq!(annotation.span.unwrap().end.column, 14);
    assert_eq!(value.unspanned(), &Expr::Integer(8080));
    assert_eq!(program.pretty(), parse(source, false).pretty());
    assert!(program.pretty().contains("Let port: int"));
    for source in ["let x: = 1;", "let x: int;", "let x: int string = 1;"] {
        assert!(
            Parser::new(Lexer::new(source).unwrap().tokenize().unwrap())
                .unwrap()
                .parse_program()
                .is_err()
        );
    }
}

#[test]
fn literal_and_annotated_assignment_mismatches_are_semantic_errors() {
    for source in [
        "let x: int = true;",
        "let x: float = 1;",
        "let x: string = null;",
        "let x: bytes = [];",
        "let x: int = {};",
        "let x: bool = 3 + 2;",
        "let x: int = 1; x = false;",
        "let x: int = 1; let y: string = x;",
        "let x: int = 1; fn f() { x = false; }",
        "let x: int = 1; let y: bool = (x = 2);",
    ] {
        let errors = analyze(&parse(source, true)).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.kind == SemanticErrorKind::TypeMismatch),
            "{source}: {errors:?}"
        );
        assert!(errors.iter().all(|e| e.span.is_some()));
    }
    let errors = analyze(&parse("let x: Unknown = 1;", true)).unwrap_err();
    assert_eq!(errors[0].kind, SemanticErrorKind::UnknownType);
    assert_eq!(errors[0].span.unwrap().start.column, 8);
    let mut bytes = Vec::new();
    assert!(
        execute(
            &parse("print(1); let x: int = false;", true),
            &mut StandardRuntime::new(&mut bytes)
        )
        .is_err()
    );
    assert!(bytes.is_empty());
}

#[test]
fn dynamic_boundaries_are_checked_at_runtime_including_captured_writes() {
    for source in [
        "let value = false; let x: int = value;",
        "let x: int = 1; let value = false; x = value;",
        "let x: int = 1; fn set(v) { x = v; } set(false);",
        "fn value() { return false; } let x: int = value();",
        "parallel item in [1, false] { let x: int = item; }",
        "let x: bytes = encode_utf8(1);",
    ] {
        analyze(&parse(source, true)).unwrap();
        let ExecutionError::Runtime(error) = run(source).unwrap_err() else {
            panic!("{source}")
        };
        assert!(error.span.is_some());
    }
}

#[test]
fn unannotated_code_stays_dynamic_and_annotations_survive_shadowing_and_workers() {
    let source = r#"
        let dynamic = 1; dynamic = "changed"; dynamic = [true, 2]; print(dynamic);
        let count: int = 1;
        fn add(v) { count = count + v; }
        add(2);
        { let count = "shadow"; count = false; print(count); }
        print(count);
        let number: float = 1.5; let ok: bool = true;
        let text: string = "hello"; let wait: duration = 1s;
        let data: bytes = encode_utf8(text);
        print(number, ok, wait, decode_utf8(data));
        parallel n in [1, 2] { let local: int = n; local = local + count; print(local); }
    "#;
    assert_eq!(
        run(source).unwrap(),
        "[true, 2]\nfalse\n3\n1.5 true 1000ms hello\n4\n5\n"
    );
    let mut bytes = Vec::new();
    execute(&parse(source, false), &mut StandardRuntime::new(&mut bytes)).unwrap();
    assert_eq!(String::from_utf8(bytes).unwrap(), run(source).unwrap());
}

#[test]
fn function_signatures_parse_mix_dynamic_parameters_and_preserve_pretty_output() {
    let source = "fn f(x: int, y, z: string,) -> bool { return true; }";
    let program = parse(source, true);
    let Stmt::Function {
        parameters,
        return_annotation,
        ..
    } = program.statements[0].unspanned()
    else {
        panic!()
    };
    assert_eq!(parameters[0].name, "x");
    assert_eq!(parameters[0].annotation.as_ref().unwrap().name, "int");
    assert!(parameters[1].annotation.is_none());
    assert_eq!(return_annotation.as_ref().unwrap().name, "bool");
    assert!(return_annotation.as_ref().unwrap().span.is_some());
    assert_eq!(program.pretty(), parse(source, false).pretty());
    for source in [
        "fn f(x:) {}",
        "fn f() -> {}",
        "fn f(x int) {}",
        "fn f() => int {}",
    ] {
        assert!(
            Parser::new(Lexer::new(source).unwrap().tokenize().unwrap())
                .unwrap()
                .parse_program()
                .is_err()
        );
    }
}

#[test]
fn annotated_signatures_check_forward_calls_returns_and_parameter_reassignments() {
    for source in [
        "f(false); fn f(x: int) {}",
        "fn f() -> int { return false; }",
        "fn f() -> int { return; }",
        "fn f(x: int) { x = false; }",
        "fn f(x: int) -> string { return x; }",
        "fn f() -> int { return 1; } let x: bool = f();",
        "fn f() -> int { fn g() -> bool { return true; } return false; }",
    ] {
        let errors = analyze(&parse(source, true)).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.kind == SemanticErrorKind::TypeMismatch),
            "{source}: {errors:?}"
        );
    }
    for source in ["fn f(x: Missing) {}", "fn f() -> Missing {}"] {
        let errors = analyze(&parse(source, true)).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].kind, SemanticErrorKind::UnknownType);
    }
    // Parameters shadow outer functions; a call through one has a dynamic signature.
    analyze(&parse(
        "fn f() -> bool { return true; } fn g(f) -> int { return f(); }",
        true,
    ))
    .unwrap();
}

#[test]
fn runtime_contracts_follow_function_values_and_check_dynamic_returns_and_fallthrough() {
    for source in [
        "fn f(x: int) { print(99); } let alias = f; alias(false);",
        "fn f(x) -> int { return x; } f(false);",
        "fn f() -> int {} f();",
        "fn f(flag) -> int { if flag { return 1; } } f(false);",
        "fn f(x: int, y) { x = y; } f(1, false);",
        "fn main() -> int {}",
        "fn f(x: int) {} parallel item in [false] { f(item); }",
    ] {
        analyze(&parse(source, true)).unwrap();
        let ExecutionError::Runtime(error) = run(source).unwrap_err() else {
            panic!("{source}")
        };
        assert!(
            error.message.contains("expected int"),
            "{source}: {error:?}"
        );
        assert!(error.span.is_some(), "{source}");
    }
    let source = "fn f(x) -> int { return x; } f(false);";
    let ExecutionError::Runtime(error) = run(source).unwrap_err() else {
        panic!()
    };
    assert_eq!(error.call_stack, vec!["f"]);
    assert_eq!(error.span.unwrap().start.column, 25);
    let mut output = Vec::new();
    let program = parse(
        "fn f(x: int) { print(99); } let alias = f; alias(false);",
        true,
    );
    assert!(execute(&program, &mut StandardRuntime::new(&mut output)).is_err());
    assert!(output.is_empty());
}

#[test]
fn annotated_functions_preserve_recursion_nested_contracts_and_dynamic_parameters() {
    let source = r#"
        fn factorial(n: int) -> int {
            if n < 2 { return 1; }
            return n * factorial(n - 1);
        }
        fn outer(n: int) -> int {
            fn label() -> string { return "ok"; }
            print(label());
            n = n + 1;
            return n;
        }
        fn dynamic(x) { x = false; return x; }
        print(factorial(5), outer(2), dynamic(1));
        let alias = factorial;
        parallel n in [2, 3] { print(alias(n)); }
    "#;
    assert_eq!(run(source).unwrap(), "ok\n120 3 false\n2\n6\n");
    let mut bytes = Vec::new();
    execute(&parse(source, false), &mut StandardRuntime::new(&mut bytes)).unwrap();
    assert_eq!(String::from_utf8(bytes).unwrap(), run(source).unwrap());
}
