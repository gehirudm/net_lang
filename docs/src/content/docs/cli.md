---
title: CLI reference
description: Tokens, ASTs, semantic checks, and interpreter execution.
---

After `cargo build --locked`, use `./target/debug/netlang`, or install with
`cargo install --path .`. All commands take one UTF-8 source file:

| Command | Behavior |
| --- | --- |
| `netlang tokens file.net` | Lex and print tokens, including EOF. |
| `netlang ast file.net` | Parse and print a readable AST tree; no semantic check or execution. |
| `netlang check file.net` | Lex, parse, and collect semantic errors without effects. |
| `netlang run file.net` | Check, then execute through the interpreter. |
| `netlang --help` or `netlang -h` | Display usage. |

```sh
cargo run --locked -- tokens examples/complete.net
cargo run --locked -- ast examples/complete.net
cargo run --locked -- check examples/complete.net
cargo run --locked -- run examples/bytes.net
```

Errors use a nonzero exit status. Function return values do not set the process
exit code. Lexer/parser errors stop the frontend at its first failure; semantic
analysis can report multiple errors. Diagnostics include file/source locations
when available. There is no REPL, `build` command, native backend, or target flag yet.

Syntax-only fragments such as `examples/match.net` parse but fail semantic checking
because they use undeclared bindings. Use `examples/complete.net` for a complete
program. Running that program makes network requests; `check` does not.
