use netlang::{diagnostic, lexer::Lexer, parser::Parser, semantic};
use std::{
    env, fs,
    io::{self, Write},
    process::ExitCode,
};

const USAGE: &str = "Usage: netlang <tokens|ast|check> <file.net>";
fn run(args: Vec<std::ffi::OsString>) -> Result<String, String> {
    if args.len() == 1 && (args[0] == "--help" || args[0] == "-h") {
        return Ok(format!("{USAGE}\n"));
    }
    if args.len() != 2 || (args[0] != "tokens" && args[0] != "ast" && args[0] != "check") {
        return Err(format!("{USAGE}\n"));
    }
    let filename = args[1].to_string_lossy();
    let source = fs::read_to_string(&args[1])
        .map_err(|e| format!("error: cannot read {filename}: {e}\n"))?;
    let tokens = Lexer::new(&source)
        .and_then(Lexer::tokenize)
        .map_err(|e| diagnostic::render(&filename, &source, e.line, e.column, &e.message))?;
    if args[0] == "tokens" {
        use std::fmt::Write;
        let mut output = String::new();
        for token in tokens {
            // Escaping keeps a token on one output line.
            writeln!(
                output,
                "{:<16} {}",
                token.kind.name(),
                token.lexeme.escape_debug()
            )
            .unwrap();
        }
        Ok(output)
    } else {
        let program = Parser::new(tokens)
            .and_then(|mut parser| parser.parse_program())
            .map_err(|e| diagnostic::render(&filename, &source, e.line, e.column, &e.message))?;
        if args[0] == "check" {
            semantic::analyze(&program).map_err(|errors| {
                errors
                    .iter()
                    .map(|error| format!("error: {error}\n --> {filename}\n"))
                    .collect::<String>()
            })?;
            Ok(format!("Semantic checks passed: {filename}\n"))
        } else {
            Ok(program.pretty())
        }
    }
}
fn main() -> ExitCode {
    match run(env::args_os().skip(1).collect()) {
        Ok(output) => match io::stdout().lock().write_all(output.as_bytes()) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) if e.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("error: cannot write output: {e}");
                ExitCode::FAILURE
            }
        },
        Err(message) => {
            eprint!("{message}");
            ExitCode::FAILURE
        }
    }
}
