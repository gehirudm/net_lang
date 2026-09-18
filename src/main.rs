use netlang::{
    diagnostic, interpreter, lexer::Lexer, parser::Parser, runtime::StandardRuntime, semantic,
};
use std::{
    env, fs,
    io::{self, Write},
    process::ExitCode,
};

const USAGE: &str = "Usage: netlang <tokens|ast|check|run> <file.net>";
fn run(args: Vec<std::ffi::OsString>, output: &mut impl Write) -> Result<String, String> {
    if args.len() == 1 && (args[0] == "--help" || args[0] == "-h") {
        return Ok(format!("{USAGE}\n"));
    }
    if args.len() != 2
        || (args[0] != "tokens" && args[0] != "ast" && args[0] != "check" && args[0] != "run")
    {
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
            .and_then(|parser| parser.with_spans().parse_program())
            .map_err(|e| diagnostic::render_span(&filename, &source, e.span, &e.message))?;
        if args[0] == "check" {
            semantic::analyze(&program).map_err(|errors| {
                errors
                    .iter()
                    .map(|error| located_error(&filename, &source, error.span, &error.to_string()))
                    .collect::<String>()
            })?;
            Ok(format!("Semantic checks passed: {filename}\n"))
        } else if args[0] == "run" {
            interpreter::execute(&program, &mut StandardRuntime::new(output)).map_err(|error| {
                match error {
                    interpreter::ExecutionError::Runtime(error) => {
                        located_error(&filename, &source, error.span, &error.to_string())
                    }
                    interpreter::ExecutionError::Semantic(errors) => errors
                        .iter()
                        .map(|error| {
                            located_error(&filename, &source, error.span, &error.to_string())
                        })
                        .collect::<String>(),
                }
            })?;
            Ok(String::new())
        } else {
            Ok(program.pretty())
        }
    }
}
fn located_error(
    filename: &str,
    source: &str,
    span: Option<netlang::source::Span>,
    message: &str,
) -> String {
    match span {
        Some(span) => diagnostic::render_span(filename, source, span, message),
        None => format!("error: {message}\n --> {filename}\n"),
    }
}
fn main() -> ExitCode {
    let mut stdout = io::stdout().lock();
    match run(env::args_os().skip(1).collect(), &mut stdout) {
        Ok(output) => match stdout.write_all(output.as_bytes()) {
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
