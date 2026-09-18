use std::{path::PathBuf, process::Command};
fn cli(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_netlang"))
        .args(args)
        .output()
        .unwrap()
}
#[test]
fn tokens_and_ast_commands() {
    let tokens = cli(&["tokens", "examples/complete.net"]);
    assert!(tokens.status.success());
    let text = String::from_utf8(tokens.stdout).unwrap();
    assert!(text.starts_with("FN               fn\n"));
    assert!(text.contains("DURATION         5s"));
    assert!(text.contains("FAT_ARROW        =>"));
    assert!(text.contains("EOF"));
    let ast = cli(&["ast", "examples/complete.net"]);
    assert!(ast.status.success());
    let text = String::from_utf8(ast.stdout).unwrap();
    for label in ["Function", "Parallel", "Request", "Match", "Duration"] {
        assert!(text.contains(label), "{label}");
    }
}
#[test]
fn usage_and_missing_files() {
    assert!(cli(&["--help"]).status.success());
    for args in [
        &[][..],
        &["build", "examples/hello.net"],
        &["ast"],
        &["ast", "does-not-exist.net"],
        &["ast", "examples/hello.net", "extra"],
    ] {
        let output = cli(args);
        assert!(!output.status.success());
        assert!(!output.stderr.is_empty());
    }
}
#[test]
fn lexer_and_parser_errors_have_source_context() {
    let path: PathBuf =
        std::env::temp_dir().join(format!("netlang-diagnostics-{}.net", std::process::id()));
    for (source, location, message) in [
        ("let x = 1", ":1:10", "expected ';'"),
        ("let name unexpected;", ":1:10", "expected '='"),
        ("\n  @", ":2:3", "unexpected character"),
        ("/* never closed", ":1:1", "unterminated block comment"),
    ] {
        std::fs::write(&path, source).unwrap();
        let output = cli(&["ast", path.to_str().unwrap()]);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(error.contains(location), "{error}");
        assert!(error.contains(message), "{error}");
        assert!(error.contains('^'));
        if source == "let name unexpected;" {
            assert!(error.contains("^~~~~~~~~~"), "{error}");
        }
        assert!(error.contains(source.lines().last().unwrap()));
    }
    std::fs::remove_file(path).unwrap();
}
#[test]
fn caret_handles_tabs_unicode_and_empty_eof_line() {
    let output = netlang::diagnostic::render("test.net", "\t\"é\" @", 1, 7, "unexpected character");
    assert!(output.contains("1 |     \"é\" @"));
    assert!(output.ends_with("|         ^\n"), "{output}");
    let output = netlang::diagnostic::render("test.net", "let x =\n", 2, 1, "expected expression");
    assert!(output.contains("2 | \n"));
    assert!(output.ends_with("| ^\n"));
}

#[test]
fn check_command_accepts_the_complete_program_without_execution() {
    let output = cli(&["check", "examples/complete.net"]);
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "Semantic checks passed: examples/complete.net\n"
    );
    let help = cli(&["--help"]);
    assert!(
        String::from_utf8(help.stdout)
            .unwrap()
            .contains("tokens|ast|check")
    );
}

#[test]
fn check_reports_multiple_errors_and_ast_remains_syntax_only() {
    let path = std::env::temp_dir().join(format!("netlang-semantics-{}.net", std::process::id()));
    std::fs::write(
        &path,
        "fn main() { print(missing); } return; let x = 1; let x = 2;",
    )
    .unwrap();
    let output = cli(&["check", path.to_str().unwrap()]);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let error = String::from_utf8(output.stderr).unwrap();
    assert_eq!(error.matches("error:").count(), 3);
    for fragment in [
        "undefined name 'missing'",
        "function 'main'",
        "return is only allowed",
        "duplicate declaration of 'x'",
        path.to_str().unwrap(),
    ] {
        assert!(error.contains(fragment), "{error}");
    }
    assert!(cli(&["ast", path.to_str().unwrap()]).status.success());
    assert!(cli(&["tokens", path.to_str().unwrap()]).status.success());
    std::fs::remove_file(path).unwrap();
}

#[test]
fn check_preserves_lexer_parser_and_file_errors() {
    let path =
        std::env::temp_dir().join(format!("netlang-check-syntax-{}.net", std::process::id()));
    for (source, expected) in [("@", "unexpected character"), ("let x = 1", "expected ';'")] {
        std::fs::write(&path, source).unwrap();
        let output = cli(&["check", path.to_str().unwrap()]);
        assert!(!output.status.success());
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(error.contains(expected));
        assert!(error.contains('^'));
    }
    std::fs::remove_file(&path).unwrap();
    assert!(!cli(&["check", path.to_str().unwrap()]).status.success());
    assert!(!cli(&["check"]).status.success());
}

#[test]
fn run_executes_main_and_preserves_output_before_runtime_errors() {
    let output = cli(&["run", "examples/hello.net"]);
    assert!(output.status.success());
    assert_eq!(output.stdout, b"Hello, Bob\n");
    let path = std::env::temp_dir().join(format!("netlang-run-{}.net", std::process::id()));
    std::fs::write(&path, "fn main() { print(42); 1 / 0; }").unwrap();
    let output = cli(&["run", path.to_str().unwrap()]);
    assert!(!output.status.success());
    assert_eq!(output.stdout, b"42\n");
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(error.contains("division by zero"));
    assert!(error.contains("in function 'main'"));
    std::fs::remove_file(path).unwrap();
}

#[test]
fn run_executes_parallel_iterations_in_input_output_order() {
    let output = cli(&["run", "examples/parallel_compute.net"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"1 1\n2 4\n3 9\n4 16\ndone\n");
}
