---
title: "Compiler architecture"
description: "Compiler architecture in the Net-lang language and toolchain."
---

Implemented behavior, with limitations and future work called out below.

```text
source → Flex scanner (C) → C bridge → safe Rust Lexer → Rust Parser → Rust AST
                                                                        ↓
                                                               Semantic analysis
```

- `lexer/netlang.l`: token recognition and per-instance position tracking.
- `lexer/lexer_bridge.{c,h}`: scanner ownership and a stable C token interface.
- `src/lexer/ffi.rs`: all unsafe Rust, scanner destruction, copied token text,
  and terminal EOF/error handling.
- `src/lexer/token.rs`: token kinds, lexemes, and source positions.
- `src/source.rs`: shared half-open spans and one-based byte positions.
- `src/parser/`: expression precedence, statements, and located errors.
- `src/ast/`: lexer-independent AST types and tree formatting.
- `src/semantic/`: lexical symbol tables, name resolution, and semantic errors.
- `src/interpreter/`: checked AST execution, lexical environments, and call stacks.
- `src/runtime/`: runtime values and host-effect interfaces.
- `src/diagnostic.rs`: shared source-line and caret rendering.
- `src/main.rs`: file loading and command dispatch.

C token constants and Rust token discriminants form the bridge ABI and must stay
aligned. Lexer tests check every token kind across that boundary. No Rust module
outside `src/lexer/ffi.rs` calls C or contains unsafe code.

Each stage can be used independently:

```rust
use netlang::{lexer::Lexer, parser::Parser, semantic};

let tokens = Lexer::new("let timeout = 5s;")?.tokenize()?;
let program = Parser::new(tokens)?.parse_program()?;
if let Err(errors) = semantic::analyze(&program) {
    for error in errors {
        eprintln!("{error}");
    }
}
println!("{program}");
Ok::<(), Box<dyn std::error::Error>>(())
```

`Parser::new` validates that the token stream ends with exactly one EOF.
`parse_program` parses a complete program; `parse_expression_complete` is useful
for isolated expression tests. Parsing returns the first error without exiting
the host process. The CLI reports that error and exits unsuccessfully.
`semantic::analyze(&program)` accepts an AST independently of the lexer/parser
and returns `Result<(), Vec<SemanticError>>`, starting with fresh scopes for each
program.

Locations are one-based lines and UTF-8 **byte** columns. `Token::span()` gives a
half-open range over raw source text; `ParseError::span` preserves that range
alongside its existing start line/column fields. Parser diagnostics underline
the whole unexpected token; EOF errors retain an insertion caret. Multiline
ranges show the first source line and an end-position note. Diagnostics convert
byte positions to character columns and expand tabs to four spaces.
The CLI preserves statement, expression, and pattern spans with `Parser::with_spans()` and highlights
relevant expressions for semantic/runtime failures, falling back to owning statements, including function and worker
errors. Library callers can opt into the same behavior; plain `Parser::new`
retains its bare-AST default. `Stmt::Located`, `Expr::Located`, and `Pattern::Located`
carry the metadata; their `unspanned()` methods expose the underlying syntax.
Location wrappers do not consume execution budget or change pretty-printed AST output.
Source file IDs and display-width handling for wide/combining characters remain
future improvements.

### Lexer, parser, and AST details

Cargo's [build script](https://github.com/gehirudm/net_lang/blob/main/build.rs) invokes Flex, the C compiler, and the archiver
directly; it does not currently use the Rust `cc` crate. Flex generates C inside
Cargo's output directory. The scanner uses per-instance `NetPosition` state for
byte columns and lines, including ignored comments. The bridge copies source
bytes and exposes token text until the next lexer call. The safe Rust wrapper
copies that text and owns scanner destruction. EOF and lexer errors are terminal
and stable on repeated calls.

Expression parsing descends through assignment, SEND, logical OR, logical AND,
equality, comparison, additive, multiplicative, unary, postfix, and primary levels.
Postfix calls, indexing, properties, and RECEIVE compose repeatedly. Parsing is
fail-fast; recovery for incomplete editor input remains future work.

`Program` stores statements. `Stmt` represents declarations, blocks, control flow,
functions, returns, parallel iteration, match, and loop control. `Expr` represents
values, collections, named construction, operators, calls, member access, requests,
and transport operations. `Pattern` covers literals and a wildcard. Annotations
and type fields are Rust AST data independent of Flex. The shared `src/types.rs`
registry resolves primitive and top-level nominal types; runtime record identity
is specific to an execution. The `ast` CLI renders a text tree, not a graphical editor.
