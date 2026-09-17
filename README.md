# Project Net-lang

Net-lang v0.1 is a compiler frontend for a small language built around network
communication. It tokenizes source with a reentrant Flex scanner compiled as C,
then uses a handwritten Rust recursive-descent parser to construct a Rust AST.

HTTP requests, parallel iteration, and match statements are syntax and AST nodes.
This version does not execute programs or make network requests.

## Build and use

Prerequisites:

- Rust with Cargo, supporting the Rust 2024 edition.
- Flex available as `flex`.
- A C99 compiler available as `cc`, and an archiver available as `ar`.

The native build uses these system tools directly and has no Cargo dependencies.
It has been verified on macOS with Apple's Flex and Clang. Cross-compilation and
Windows toolchains are not configured.

```sh
cargo build
cargo run -- tokens examples/complete.net
cargo run -- ast examples/complete.net
```

The binary is also available directly:

```sh
./target/debug/netlang tokens examples/request.net
./target/debug/netlang ast examples/complete.net
```

To install the binary on your Cargo executable path:

```sh
cargo install --path .
netlang ast examples/complete.net
```

`FLEX`, `CC`, and `AR` can override the tool executable paths. Build output,
including generated `lex.yy.c`, stays in Cargo's target directory. Bison and
`libfl` are not required.

## Example

```text
fn main() {
    let response = GET "https://api.example.com/users" {
        headers: { "Accept": "application/json" },
        timeout: 5s,
        retry: 3
    };

    match response.status {
        200 => { print(response.body); }
        _ => { print("Request failed"); }
    }
}
```

A request fragment prints as:

```text
Program
└── Let response
    └── Request GET
        ├── URL
        │   └── Identifier url
        └── Config
            └── Object
                └── Field "timeout"
                    └── Duration 5000ms
```

Examples cover [functions](examples/hello.net), [requests](examples/request.net),
[parallel iteration](examples/parallel.net), [matching](examples/match.net), and
the full [v0.1 success-criteria program](examples/complete.net).

## Syntax implemented

- Untyped functions, calls, `let`, assignment, and optional return values.
- Blocks, `if` / `else if` / `else`, `while`, and `for item in items`.
- Integers, floats, strings, booleans, null, arrays, and objects.
- Property access, indexing, unary `!` and `-`, arithmetic, comparisons,
  equality, logical operators, and right-associative assignment.
- `GET`, `POST`, `PUT`, `PATCH`, `DELETE`, and `HEAD` request expressions.
- Duration literals in `ms`, `s`, and `m`, normalized to checked `u64`
  milliseconds in the AST.
- `parallel item in collection` statements.
- Match patterns: integer, string, boolean, null, and `_`.
- Line comments and non-nested block comments.

Simple statements (`let`, assignment, calls, `return`, and other expressions)
require semicolons. Block-based constructs do not require a trailing semicolon,
as in the specification's examples. Lists of array elements, object fields,
arguments, and parameters accept an optional trailing comma.

Identifiers follow `[a-zA-Z_][a-zA-Z0-9_]*`. Object keys are identifiers or
strings; quote a keyword when using it as a key. `timeout`, `retry`, `headers`,
`query`, and `json` remain ordinary identifiers. Request options are not
semantically validated.

Strings support `\n`, `\t`, `\r`, `\"`, and `\\`. Raw newlines are
not allowed inside strings. Strings may contain UTF-8 text. Interpolation is not
performed; use `"/users/" + id`.

Precedence from tightest to loosest is grouping, postfix operations, unary
operators, multiplication/division/remainder, addition/subtraction, comparisons,
equality, `&&`, `||`, assignment. Binary operators associate left; assignment
associates right. Assignment targets must be identifiers, properties, or indexes.

### Request expression boundaries

The grammar deliberately follows `HTTP_METHOD expression object?`:

```text
GET api + "/users" { timeout: 5s }
```

The full expression `api + "/users"` is the URL, and the following object is its
configuration. To access the **request result**, put the request in parentheses:

```text
(GET url).status
if (GET url { timeout: 5s }).status == 200 {
    print("OK");
}
```

Because an immediately following brace belongs to the request, use parentheses
when a request without configuration appears directly before a control-flow
block: `if (GET url) { ... }`. Similarly, `(GET url) == other` compares the
request result; `GET url == other` places the comparison inside its URL AST.
A leading brace in statement position is a block; an object expression statement
can be parenthesized.

## Architecture

```text
source → Flex scanner (C) → C bridge → safe Rust Lexer → Rust Parser → Rust AST
```

- `lexer/netlang.l`: token recognition and per-instance position tracking.
- `lexer/lexer_bridge.{c,h}`: scanner ownership and a stable C token interface.
- `src/lexer/ffi.rs`: all unsafe Rust, scanner destruction, copied token text,
  and terminal EOF/error handling.
- `src/lexer/token.rs`: token kinds, lexemes, and source positions.
- `src/parser/`: expression precedence, statements, and located errors.
- `src/ast/`: lexer-independent AST types and tree formatting.
- `src/diagnostic.rs`: shared source-line and caret rendering.
- `src/main.rs`: file loading and command dispatch.

C token constants and Rust token discriminants form the bridge ABI and must stay
aligned. Lexer tests check every token kind across that boundary. No Rust module
outside `src/lexer/ffi.rs` calls C or contains unsafe code.

Each stage can be used independently:

```rust
use netlang::{lexer::Lexer, parser::Parser};

let tokens = Lexer::new("let timeout = 5s;")?.tokenize()?;
let program = Parser::new(tokens)?.parse_program()?;
println!("{program}");
# Ok::<(), Box<dyn std::error::Error>>(())
```

`Parser::new` validates that the token stream ends with exactly one EOF.
`parse_program` parses a complete program; `parse_expression_complete` is useful
for isolated expression tests. Parsing returns the first error without exiting
the host process. The CLI reports that error and exits unsuccessfully.

Locations are one-based lines and UTF-8 **byte** columns. Diagnostics convert the
source prefix to characters and expand tabs to four spaces for a basic caret.
Full source spans and display-width handling for wide/combining characters are
future improvements.

## Development

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
```

Tests assert token kinds, source positions, scanner independence, operator
precedence, AST structure, malformed syntax, numeric overflow, request
configuration, match patterns, the success-criteria program, tree output, and CLI
diagnostics. Implementation milestones are recorded as separate local commits.

The v0.1 scope excludes runtime execution, actual HTTP/concurrency, type checking,
imports, interpolation, and compiled backends. An interpreter can be the next
stage once the frontend design is settled.
