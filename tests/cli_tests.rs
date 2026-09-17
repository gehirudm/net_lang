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
        &["run", "examples/hello.net"],
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
