use netlang::{
    ast::{Expr, Stmt},
    interpreter::{ExecutionError, execute},
    lexer::Lexer,
    parser::Parser,
    runtime::{Runtime, StandardRuntime, Value},
    semantic::{SemanticErrorKind as K, analyze},
};

fn parse(source: &str, spans: bool) -> netlang::ast::Program {
    let parser = Parser::new(Lexer::new(source).unwrap().tokenize().unwrap()).unwrap();
    let mut parser = if spans { parser.with_spans() } else { parser };
    parser.parse_program().unwrap()
}
fn run(source: &str) -> Result<(Value, String), ExecutionError> {
    let mut bytes = Vec::new();
    let value = execute(&parse(source, true), &mut StandardRuntime::new(&mut bytes))?;
    Ok((value, String::from_utf8(bytes).unwrap()))
}

#[test]
fn declarations_constructors_and_fields_have_independent_ast_nodes() {
    let source = "type User { id: int, name: string, } let u = User { name: \"A\", id: 1, };";
    let program = parse(source, true);
    let Stmt::Type { name, fields } = program.statements[0].unspanned() else {
        panic!()
    };
    assert_eq!(name, "User");
    assert_eq!(fields[0].name, "id");
    assert_eq!(fields[0].annotation.name, "int");
    assert!(fields[0].annotation.span.is_some());
    let Stmt::Let { value, .. } = program.statements[1].unspanned() else {
        panic!()
    };
    let Expr::Construct { name, fields } = value.unspanned() else {
        panic!()
    };
    assert_eq!(name, "User");
    assert_eq!(fields[0].key, "name");
    assert!(fields.iter().all(|f| f.value.span().is_some()));
    assert_eq!(program.pretty(), parse(source, false).pretty());
    for source in [
        "type U { id }",
        "type U { id: }",
        "type U { id: int name: string }",
        "let u = U { id 1 };",
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
fn declarations_and_constructors_validate_names_fields_and_nominal_identity() {
    for (source, kind) in [
        ("type U {} type U {}", K::DuplicateDeclaration),
        ("type int {}", K::InvalidTypeDeclaration),
        ("type Node { next: Node }", K::InvalidTypeDeclaration),
        ("type A { b: B } type B { a: A }", K::InvalidTypeDeclaration),
        ("fn f() { type U {} }", K::InvalidTypeDeclaration),
        ("type U { id: Missing }", K::UnknownType),
        ("type U { id: int, id: int }", K::DuplicateDeclaration),
        ("Missing {};", K::InvalidConstructor),
        ("type U { id: int } U {};", K::InvalidConstructor),
        ("type U {} U { id: 1 };", K::InvalidConstructor),
        (
            "type U { id: int } U { id: 1, id: 2 };",
            K::InvalidConstructor,
        ),
        ("type U { id: int } U { id: false };", K::TypeMismatch),
        ("type U { id: int } let u: U = { id: 1 };", K::TypeMismatch),
        ("type U {} type V {} let u: U = V {};", K::TypeMismatch),
        (
            "type U { id: int } let u: U = U { id: 1 }; u.id = false;",
            K::TypeMismatch,
        ),
        (
            "type U { id: int } let u: U = U { id: 1 }; print(u.other);",
            K::TypeMismatch,
        ),
    ] {
        let errors = analyze(&parse(source, true)).unwrap_err();
        assert!(
            errors.iter().any(|e| e.kind == kind),
            "{source}: {errors:?}"
        );
    }
}

#[test]
fn dynamic_construction_and_mutation_preserve_record_contracts() {
    for source in [
        "let value = false; U { id: value };",
        "let u = U { id: 1 }; let value = false; u.id = value;",
        "let u = U { id: 1 }; u.extra = 1;",
        "let u = U { id: 1 }; u[\"id\"] = false;",
        "let plain = { id: 1 }; let u: U = plain;",
        "fn take(u: U) {} let alias = take; alias({ id: 1 });",
        "fn make(v) -> U { return v; } make({ id: 1 });",
    ] {
        let source = format!("type U {{ id: int }} {source}");
        analyze(&parse(&source, true)).unwrap();
        let ExecutionError::Runtime(error) = run(&source).unwrap_err() else {
            panic!()
        };
        assert!(error.span.is_some());
    }
}

#[test]
fn nested_records_copy_by_value_and_work_in_functions_arrays_and_workers() {
    let source = r#"
        type Group { user: User }
        type User { id: int, name: string }
        type Other { id: int, name: string }
        fn make(id: int) -> User { return User { id: id, name: "Alice" }; }
        fn rename(user: User) -> User { user.name = "Bob"; return user; }
        let group: Group = Group { user: make(1) };
        let copy = group;
        copy.user.id = 2;
        print(group.user.id, copy.user.id, rename(group.user).name);
        print(group.user == User { id: 1, name: "Alice" });
        print(group.user == Other { id: 1, name: "Alice" });
        print(group.user == { id: 1, name: "Alice" });
        let users = [make(3)]; users[0]["id"] = 4; print(users[0].id);
        parallel n in [1, 2] { let local: User = make(n); local.id = local.id + 10; print(local.id); }
    "#;
    assert_eq!(
        run(source).unwrap().1,
        "1 2 Bob\ntrue\nfalse\nfalse\n4\n11\n12\n"
    );
    let mut bytes = Vec::new();
    execute(&parse(source, false), &mut StandardRuntime::new(&mut bytes)).unwrap();
    assert_eq!(String::from_utf8(bytes).unwrap(), run(source).unwrap().1);
    let bad = "type U { id: int } type G { u: U } let g = G { u: U { id: 1 } }; g.u = { id: 2 };";
    assert!(matches!(run(bad), Err(ExecutionError::Runtime(_))));
}

#[test]
fn control_flow_and_http_configuration_keep_their_braces() {
    let source = r#"
        type Flag { active: bool }
        let flag = true;
        if flag {} while false {} for flag in [] {} parallel flag in [] {}
        match flag { true => {} _ => {} }
        if (Flag { active: true }).active { print("ok"); }
        fn active(flag: Flag) -> bool { return flag.active; }
        if active(Flag { active: true }) { print("yes"); }
    "#;
    assert_eq!(run(source).unwrap().1, "ok\nyes\n");
    let program = parse("GET url { timeout: 5s };", false);
    assert!(
        matches!(&program.statements[0], Stmt::Expression(Expr::Request { url, config: Some(_) , .. }) if matches!(url.as_ref(), Expr::Identifier(_)))
    );
    analyze(&parse("type Flag { active: bool } let conn = TCP \"example:90\"; conn SEND Flag { active: true };", true)).unwrap();
}

#[test]
fn nominal_identity_cannot_be_imported_from_a_different_execution() {
    let (record, _) = run("type U { id: int } fn main() -> U { return U { id: 1 }; }").unwrap();
    let Value::Record(value) = &record else {
        panic!()
    };
    assert_eq!(value.name(), "U");
    assert_eq!(value.fields()["id"], Value::Integer(1));
    struct Host(Value);
    impl Runtime for Host {
        fn print(&mut self, _: &str) -> Result<(), String> {
            Ok(())
        }
        fn request(
            &mut self,
            _: netlang::ast::HttpMethod,
            _: &str,
            _: &Value,
        ) -> Result<Value, String> {
            Ok(self.0.clone())
        }
    }
    let program = parse("type U { id: int } let u: U = GET \"host\";", true);
    let error = execute(&program, &mut Host(record)).unwrap_err();
    assert!(error.to_string().contains("expected U"));
}
