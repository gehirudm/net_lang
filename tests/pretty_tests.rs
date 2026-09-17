use netlang::{lexer::Lexer, parser::Parser};
fn pretty(source: &str) -> String {
    Parser::new(Lexer::new(source).unwrap().tokenize().unwrap())
        .unwrap()
        .parse_program()
        .unwrap()
        .to_string()
}
#[test]
fn request_tree_has_readable_structure() {
    assert_eq!(
        pretty("let response = GET url { timeout: 5s };"),
        concat!(
            "Program\n",
            "└── Let response\n",
            "    └── Request GET\n",
            "        ├── URL\n",
            "        │   └── Identifier url\n",
            "        └── Config\n",
            "            └── Object\n",
            "                └── Field \"timeout\"\n",
            "                    └── Duration 5000ms\n",
        )
    );
}
#[test]
fn printer_covers_remaining_ast_nodes() {
    let output = pretty(
        r#"
        fn f(a) {
            let x = [1, 1.5, "x", true, null];
            x[0] = -1 + 2;
            if !a { return; } else { return f().value; }
            while false {}
            for item in x {}
            parallel item in x {}
            match a { 1 => {} "x" => {} true => {} false => {} null => {} _ => {} }
        }
    "#,
    );
    for label in [
        "Function f",
        "Parameters",
        "Array",
        "Float 1.5",
        "Boolean true",
        "Null",
        "Assignment",
        "Index",
        "Unary -",
        "Binary +",
        "If",
        "Unary !",
        "Else",
        "Return",
        "Call",
        "Property value",
        "While",
        "For item",
        "Parallel item",
        "Match",
        "Arm 1",
        "Arm \"x\"",
        "Arm true",
        "Arm false",
        "Arm null",
        "Arm _",
    ] {
        assert!(output.contains(label), "{label}");
    }
}
