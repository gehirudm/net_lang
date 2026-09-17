use netlang::{
    interpreter::{ExecutionError, Limits, execute, execute_with_limits},
    lexer::Lexer,
    parser::Parser,
    runtime::{StandardRuntime, Value},
};

fn parse(source: &str) -> netlang::ast::Program {
    Parser::new(Lexer::new(source).unwrap().tokenize().unwrap())
        .unwrap()
        .parse_program()
        .unwrap()
}
fn run(source: &str) -> (Value, String) {
    let mut output = Vec::new();
    let value = execute(&parse(source), &mut StandardRuntime::new(&mut output)).unwrap();
    (value, String::from_utf8(output).unwrap())
}
fn failure(source: &str) -> String {
    execute(&parse(source), &mut StandardRuntime::new(Vec::new()))
        .unwrap_err()
        .to_string()
}

#[test]
fn expressions_short_circuit_and_checked_numbers() {
    let (_, output) = run(r#"
        print(2 + 3 * 4, 7 / 2, 7 % 2, 1 + 0.5, 2m + 5s);
        let x = 0; false && ((x = 1) == 1); true || ((x = 2) == 2); print(x);
        print("/users/" + 42, !false, 1 == 1.0, 2 < 2.5);
        print(9007199254740993 == 9007199254740992.0);
        print(9223372036854775807 < 9223372036854775808.0);
    "#);
    assert_eq!(
        output,
        "14 3 1 1.5 125000ms\n0\n/users/42 true true true\nfalse\ntrue\n"
    );
    for (source, message) in [
        ("1 / 0;", "division by zero"),
        ("1.0 % 0.0;", "division by zero"),
        ("9223372036854775807 + 1;", "overflow"),
        ("9007199254740993 + 0.5;", "exactly"),
        ("if 1 {}", "expected boolean"),
        ("true + 1;", "invalid binary"),
        ("!1;", "invalid unary"),
    ] {
        assert!(failure(source).contains(message), "{source}");
    }
}

#[test]
fn functions_entry_point_recursion_and_returns() {
    let (value, output) = run(r#"
        fn factorial(n) { if n <= 1 { return 1; } return n * factorial(n - 1); }
        fn main() { print(factorial(6)); return 42; }
    "#);
    assert_eq!(value, Value::Integer(42));
    assert_eq!(output, "720\n");
    assert_eq!(run(include_str!("../examples/hello.net")).1, "Hello, Bob\n");
    assert_eq!(run("fn f() { return; } print(f());").1, "null\n");
    assert!(failure("fn main(x) {}").contains("no parameters"));
    assert_eq!(run("fn f() { print(1); } let main = f;").1, "");
}

#[test]
fn lexical_captures_survive_calls_and_later_shadowing() {
    let (_, output) = run(r#"
        let x = "outer";
        { fn show() { print(x); } show(); let x = "inner"; show(); }
        fn counter() {
            let count = 0;
            fn next() { count = count + 1; return count; }
            return next;
        }
        let a = counter(); let b = counter(); print(a(), a(), b(), a());
        fn even(n) { if n == 0 { return true; } return odd(n - 1); }
        fn odd(n) { if n == 0 { return false; } return even(n - 1); }
        print(even(8));
    "#);
    assert_eq!(output, "outer\nouter\n1 2 1 3\ntrue\n");
    assert!(failure("f(); let x = 1; fn f() { print(x); }").contains("before its initializer"));
}

#[test]
fn loops_match_and_return_propagation() {
    let (_, output) = run(r#"
        let sum = 0;
        for n in [1, 2, 3] { sum = sum + n; }
        while sum < 8 { sum = sum + 1; }
        match sum { 8 => { print("eight"); } _ => { print("wrong"); } }
        fn find() { for n in [1, 2] { if n == 2 { return n; } } return 0; }
        print(find(), sum);
        match null { null => { print("null"); } }
        match true { false => { print("wrong"); } true => { print("true"); } }
    "#);
    assert_eq!(output, "eight\n2 8\nnull\ntrue\n");
}

#[test]
fn arrays_objects_and_parameters_use_value_semantics() {
    let (_, output) = run(r#"
        let a = { users: [{name: "Alice"}] }; let b = a;
        a.users[0].name = "Bob"; a["count"] = 1;
        fn change(value) { value.users[0].name = "Carol"; return value; }
        print(a.users[0].name, b.users[0].name, change(a).users[0].name);
        let i = 0; let values = [0, 0]; values[i = 1] = 7; print(values, a.count);
    "#);
    assert_eq!(output, "Bob Alice Carol\n[0, 7] 1\n");
    for source in [
        "let a = []; a[0];",
        "let a = [1]; a[-1] = 2;",
        "let a = {}; a.missing;",
        "fn f() { return {}; } f().x = 1;",
    ] {
        assert!(!failure(source).is_empty());
    }
}

#[test]
fn errors_have_call_stacks_and_effects_are_checked_first() {
    let error = failure("fn inner() { return 1 / 0; } fn main() { inner(); }");
    assert!(error.contains("in function 'inner'"));
    assert!(error.contains("in function 'main'"));
    let mut output = Vec::new();
    let error = execute(
        &parse("print(1); missing();"),
        &mut StandardRuntime::new(&mut output),
    )
    .unwrap_err();
    assert!(matches!(error, ExecutionError::Semantic(_)));
    assert!(output.is_empty());
    assert!(failure("let f = 1; f();").contains("cannot call integer"));
    assert!(failure("fn f(x) {} let g = f; g();").contains("expects 1 argument"));
}

#[test]
fn execution_limits_produce_errors() {
    for (source, expected) in [
        ("while true {}", "step limit"),
        ("fn f() { f(); } f();", "call depth"),
        ("1 + (2 + (3 + 4));", "expression depth"),
    ] {
        let limits = Limits {
            steps: 1000,
            call_depth: 8,
            expression_depth: 3,
        };
        let error = execute_with_limits(
            &parse(source),
            &mut StandardRuntime::new(Vec::new()),
            limits,
        )
        .unwrap_err();
        assert!(error.to_string().contains(expected), "{error}");
    }
}

#[test]
fn requests_use_the_runtime_boundary() {
    struct Mock {
        calls: usize,
    }
    impl netlang::runtime::Runtime for Mock {
        fn print(&mut self, _: &str) -> Result<(), String> {
            Ok(())
        }
        fn request(
            &mut self,
            method: netlang::ast::HttpMethod,
            url: &str,
            config: &Value,
        ) -> Result<Value, String> {
            assert_eq!(method, netlang::ast::HttpMethod::Get);
            assert_eq!(url, "https://example.com/users/42");
            assert!(
                matches!(config, Value::Object(fields) if fields.get("timeout") == Some(&Value::Duration(5000)))
            );
            self.calls += 1;
            Ok(Value::Integer(200))
        }
    }
    let mut runtime = Mock { calls: 0 };
    let result = execute(
        &parse(r#"fn main() { return GET "https://example.com/users/" + 42 { timeout: 5s }; }"#),
        &mut runtime,
    )
    .unwrap();
    assert_eq!(result, Value::Integer(200));
    assert_eq!(runtime.calls, 1);
}
