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
