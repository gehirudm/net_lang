# Project Net-lang

Net-lang v0.1 is a compiler frontend for a small language built around network
communication. It tokenizes source with a reentrant Flex scanner compiled as C,
then uses a handwritten Rust recursive-descent parser to construct a Rust AST.
An initial semantic-analysis pass checks lexical scopes and function usage.

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
cargo run -- check examples/complete.net
```

The binary is also available directly:

```sh
./target/debug/netlang tokens examples/request.net
./target/debug/netlang ast examples/complete.net
./target/debug/netlang check examples/complete.net
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

## Semantic analysis

`netlang check file.net` lexes, parses, and checks the program without executing
it. It reports all errors found by the semantic pass and exits unsuccessfully
when any check fails. The `tokens` and `ast` commands remain independently usable
for inspecting syntax, including programs with semantic errors.

The first pass implements these rules:

- Names resolve from the innermost lexical scope outward. A `let` binding is
  visible after its initializer; the initializer can use an outer binding with
  the same name. Other uses before declaration are errors.
- Functions are visible throughout their enclosing scope, supporting forward
  calls and mutual recursion. A function body can reference enclosing variables
  already declared when its declaration is analyzed. Capturing variables declared
  later is not supported by this pass.
- Duplicate declarations in the same scope are errors. Inner scopes may shadow
  outer names. Function parameters share a scope with the function body's direct
  declarations; loop variables share a scope with the loop body's declarations.
- Blocks, branches, loop bodies, and match arms have local scopes. A loop variable
  is available only in its body, not its iterable expression or after the loop.
- Variables, parameters, and loop variables are mutable. Named function and
  built-in bindings cannot be reassigned, though they may be shadowed.
- `return` requires an enclosing function.
- Calls to directly named user functions must supply the declared argument count.
  The built-in `print` is recognized, with no argument-count restriction in this
  initial pass. Signatures of dynamic callees, including variables holding
  functions, are not inferred yet.

Request URLs, request option values, object values, and all other expression
positions are traversed. Property names and object keys are not variable uses.
Request option validity, static types, callee types, match exhaustiveness, and
runtime initialization order are not checked yet. A successful check therefore
does not guarantee that a program will run successfully once execution exists.

Semantic errors currently identify the filename and AST scope/statement context.
Unlike lexer and parser errors, they cannot highlight an exact source location
until AST nodes carry source spans. `examples/match.net` is a syntax fragment
using an undeclared `response`; it parses but intentionally fails semantic
checking. The complete example declares that binding and passes.

## Architecture

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
- `src/parser/`: expression precedence, statements, and located errors.
- `src/ast/`: lexer-independent AST types and tree formatting.
- `src/semantic/`: lexical symbol tables, name resolution, and semantic errors.
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
# Ok::<(), Box<dyn std::error::Error>>(())
```

`Parser::new` validates that the token stream ends with exactly one EOF.
`parse_program` parses a complete program; `parse_expression_complete` is useful
for isolated expression tests. Parsing returns the first error without exiting
the host process. The CLI reports that error and exits unsuccessfully.
`semantic::analyze(&program)` accepts an AST independently of the lexer/parser
and returns `Result<(), Vec<SemanticError>>`, starting with fresh scopes for each
program.

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
configuration, match patterns, the success-criteria program, tree output, CLI
diagnostics, lexical scopes, recursion, argument counts, and invalid returns.
Implementation milestones are recorded as separate local commits.

## Roadmap and progress tracking

The checked items below are implemented in the current frontend. Unchecked items
are planned or under consideration and are not part of v0.1 unless moved into an
active milestone. This section should be updated whenever a feature is completed,
its syntax changes, or its priority is revised.

### Current frontend

- [x] Reentrant Flex lexer with a stable C bridge and safe Rust wrapper
- [x] Handwritten recursive-descent parser and Rust AST
- [x] Variables, assignment, functions, calls, returns, and control flow
- [x] Arrays, objects, property access, and indexing
- [x] HTTP request expressions for `GET`, `POST`, `PUT`, `PATCH`, `DELETE`, and `HEAD`
- [x] Duration literals normalized to milliseconds
- [x] `parallel` statements represented in the AST
- [x] Basic `match` statements with literal and wildcard patterns
- [x] Token and AST CLI commands
- [x] Source-line diagnostics and readable AST output
- [x] Initial semantic analysis with lexical symbol tables and scope checking
- [x] `check` CLI command with collected semantic diagnostics

### Language syntax

- [ ] **Static type annotations**

  ```netlang
  let port: int = 8080;

  fn fetch_user(id: int) -> Response {
      ...
  }
  ```

- [ ] **User-defined types and structs**

  ```netlang
  type User {
      id: int,
      name: string,
      active: bool
  }
  ```

- [ ] **Typed request responses**

  ```netlang
  let user: Response<User> = GET "/users/1";
  ```

- [ ] **String interpolation**

  ```netlang
  let response = GET "/users/${id}";
  ```

  String concatenation is currently required:

  ```netlang
  GET "/users/" + id;
  ```

- [ ] **Imports and modules**

  ```netlang
  import net.http;
  import "./utils.net";
  ```

- [ ] **Constants**

  ```netlang
  const API_URL = "https://api.example.com";
  ```

- [ ] **`break` and `continue`**

  ```netlang
  for item in items {
      if item == target {
          break;
      }
  }
  ```

- [ ] **Richer match patterns**, including ranges and eventually destructuring

  ```netlang
  match response.status {
      200..299 => { ... }
      404      => { ... }
      _        => { ... }
  }
  ```

- [ ] **First-class error handling**

  The syntax is not finalized. One possible form is:

  ```netlang
  try {
      ...
  } catch error {
      ...
  }
  ```

  A result-oriented model is also under consideration.

- [ ] **Additional concurrency syntax**, potentially including `spawn`, `await`,
  or a Net-lang-specific synchronization model

  ```netlang
  spawn {
      ...
  }
  ```

### Network-specific syntax

- [ ] **First-class `SEND` and `RECEIVE` operators**

  `SEND` and `RECEIVE` are Net-lang language constructs rather than ordinary
  library methods. They are intended to work across suitable transports,
  including TCP connections, WebSockets, connected UDP sockets, and custom
  protocol connections, depending on the target's type. Conventional method
  forms such as `conn.send(data)` and `conn.receive()` are not planned.

  ```netlang
  conn SEND data;
  let data = conn RECEIVE;
  ```

  An unconnected transport can include a destination:

  ```netlang
  socket SEND packet TO address;
  ```

  Structured and typed data should use the same operators:

  ```netlang
  conn SEND LoginPacket {
      username: "alice",
      token: token
  };

  let response = conn RECEIVE LoginResponse;
  ```

  Semantic analysis will verify whether the target transport or type supports
  `SEND`, `RECEIVE`, or both.

- [ ] **TCP connections**

  ```netlang
  let conn = TCP "example.com:9000";

  conn SEND "hello";

  let response = conn RECEIVE;
  ```

  A future protocol-aware form is:

  ```netlang
  let conn = TCP "example.com:9000" using MyProtocol;

  conn SEND Credentials {
      username: "alice",
      password: password
  };

  let result = conn RECEIVE;
  ```

  TCP server and listening syntax is **TBD**. No final syntax has been selected.

- [ ] **UDP communication**

  A connected UDP socket can use the standard `SEND` and `RECEIVE` operators:

  ```netlang
  let socket = UDP "192.168.1.20:5000";

  socket SEND data;

  let packet = socket RECEIVE;
  ```

  For an unconnected UDP socket, the currently proposed syntax supplies a
  destination with `TO`:

  ```netlang
  socket SEND data TO "192.168.1.20:5000";

  let packet = socket RECEIVE;
  ```

  `TO` is proposed syntax and may change.

- [ ] **WebSocket connections**

  ```netlang
  let socket = WS "wss://example.com/events";
  ```

- [ ] **Streaming syntax**

  ```netlang
  for message in WS "wss://example.com/events" {
      print(message);
  }
  ```

- [ ] **Reusable network policies** for timeout, retry, rate-limit, and
  connection behavior

  ```netlang
  policy external_api {
      timeout: 5s,
      retry: 3
  }
  ```

- [ ] **Native server and route declarations**

  ```netlang
  server 8080 {
      GET "/users/:id" {
          ...
      }

      POST "/users" {
          ...
      }
  }
  ```

- [ ] **Protocol definitions and protocol state machines**

  Protocol declarations may describe state transitions with the same `SEND` and
  `RECEIVE` language operators:

  ```netlang
  protocol Login {
      state Connected {
          SEND Credentials -> Waiting
      }

      state Waiting {
          RECEIVE Success -> Authenticated
          RECEIVE Failure -> Connected
      }
  }
  ```

  Normal Net-lang code using that protocol should conceptually look like:

  ```netlang
  let conn = TCP "server.example.com:9000" using Login;

  conn SEND Credentials {
      username: "alice",
      password: password
  };

  let result = conn RECEIVE;
  ```

- [ ] **Binary packet and protocol structures**

  ```netlang
  packet Header {
      version: u8,
      length: u16be
  }
  ```

- [ ] **Request pipelines and network data pipelines**

  ```netlang
  GET "/events"
      |> decode json
      |> process;
  ```

### Possible general language features

These are lower priority and should be added only when they serve networking
programs:

- [ ] Enums
- [ ] Generics
- [ ] Tuples
- [ ] Closures and anonymous functions
- [ ] Optional values
- [ ] Standard collection types such as maps and sets
- [ ] Visibility with `pub` and private declarations
- [ ] Package and module namespaces
- [ ] Macros

Classes and inheritance are intentionally not a current priority. Net-lang does
not aim to reproduce every feature of a general-purpose object-oriented language.

### Planned compiler architecture and implementation stages

Net-lang will be built in stages. The frontend and initial semantic analysis are
implemented today; the interpreter, IR, native backend, and cross-platform
toolchain described below remain future work.

#### Phase 1 — Compiler frontend

```text
Source
  ↓
Flex Lexer
  ↓
Tokens
  ↓
Rust Recursive-Descent Parser
  ↓
AST
  ↓
Semantic Analysis
```

This phase establishes the language syntax, AST, lexical scopes, and early
semantic rules. It is the implemented foundation for later execution stages.

#### Phase 2 — Interpreter

```text
AST
  ↓
Semantic Analysis
  ↓
Tree-Walking Interpreter
  ↓
Net-lang Runtime
```

The interpreter will be built before native code generation so that language
semantics, the networking model, error handling, concurrency behavior, and
runtime APIs can be tested before the compiler commits to a native backend. It
should eventually execute variables and expressions, functions, control flow,
HTTP requests, TCP, UDP, `SEND` / `RECEIVE`, and concurrency primitives.

#### Phase 3 — Intermediate Representation

After the interpreter and language semantics are stable, Net-lang will introduce
a language-specific IR:

```text
Source
  ↓
Lexer
  ↓
Parser
  ↓
AST
  ↓
Semantic Analysis
  ↓
Net-lang IR
```

The IR should remain independent of the eventual machine-code backend so the
frontend is not tied to LLVM.

#### Phase 4 — Native compilation

The current intended native compilation direction is:

```text
Net-lang IR
    ↓
LLVM IR
    ↓
LLVM native code generation
    ↓
Target object code
    +
Net-lang target runtime
    ↓
Linker
    ↓
Native executable
```

LLVM is the current preferred long-term backend direction, but this decision is
not locked in and will be reevaluated after the interpreter and Net-lang IR
exist. Cranelift or another native backend remain alternatives rather than the
current primary plan.

The runtime should be portable and target-aware. OS-specific networking and
system behavior should stay behind the runtime rather than being emitted
throughout generated code. HTTP, TCP, UDP, `SEND`, and `RECEIVE` should
eventually lower to runtime operations conceptually such as `net_http_get`,
`net_tcp_connect`, `net_udp_bind`, `net_send`, and `net_receive`. Runtime
implementations may be written in Rust and built for each supported target.

#### Phase 5 — Cross-platform compilation

The endgame toolchain should support Go-style cross-compilation for supported
targets, subject to platform and toolchain constraints:

```sh
netlang build app.net --target windows-x64
netlang build app.net --target linux-x64
netlang build app.net --target linux-arm64
netlang build app.net --target macos-arm64
```

Friendly Net-lang target names may map internally to platform target triples.
The compiler should eventually lower Net-lang IR to target-specific LLVM IR and
object code, select or build the Net-lang runtime for the requested target, link
the program and runtime, and produce a standalone native executable where
practical. The toolchain may ship prebuilt target-specific runtimes so users do
not need to build them themselves. The build itself should not need to run on
the target OS.

Cross-compilation should begin with a deliberately small set of platforms:

- **Tier 1:** `windows-x64`, `linux-x64`, `macos-arm64`
- **Possible Tier 2:** `windows-arm64`, `linux-arm64`, `macos-x64`

The implementation path is:

```text
Frontend
    ↓
Semantic Analyzer
    ↓
Interpreter
    ↓
Stabilize Language Semantics
    ↓
Net-lang IR
    ↓
LLVM Backend
    ↓
Cross-Platform Native Compilation
```

### Compiler and runtime work

These are not syntax features, but are required for Net-lang to become executable:

- [ ] Lexer tokens for `TCP`, `UDP`, `SEND`, `RECEIVE`, `TO`, and `USING`
- [ ] Parser support for TCP and UDP connection expressions
- [ ] Parser support for `SEND` and `RECEIVE` expressions
- [ ] AST nodes for TCP and UDP connections and `SEND` / `RECEIVE`
- [x] Initial semantic analysis: names, declarations, assignments, returns, and direct-call arity
- [x] Symbol tables and lexical scope checking
- [ ] Static type checking
- [ ] Semantic validation that a target supports `SEND` and `RECEIVE`
- [ ] Protocol-state checking as an advanced semantic-analysis feature
- [ ] Interpreter
- [ ] Actual HTTP execution
- [ ] Retry and timeout runtime behavior
- [ ] Real parallel execution
- [ ] Transport-specific runtime implementation
- [ ] TCP runtime
- [ ] UDP runtime
- [ ] WebSocket runtime
- [ ] Standard library
- [ ] Module loader
- [ ] Improved source spans and diagnostics
- [ ] Net-lang intermediate representation (IR)
- [ ] Backend abstraction
- [ ] Native code generation backend
- [ ] Target triple and target configuration support
- [ ] Target-specific Net-lang runtime builds
- [ ] Cross-platform linking
- [ ] Cross-compilation CLI support
- [ ] Standalone executable packaging
- [ ] Tier 1 target support
- [ ] Compiled backend
- [ ] Package manager

The v0.1 scope excludes runtime execution, actual HTTP and concurrency, type
checking, imports, interpolation, and compiled backends. An interpreter is the
intended first execution backend once the frontend design is stable.
