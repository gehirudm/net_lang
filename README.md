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

### Long-term goal: Go-style cross-compilation

One of Net-lang's endgame goals is a single compiler and toolchain that can build
Net-lang programs for supported operating systems and architectures without
requiring the build to run on the target operating system. The intended user
experience is similar to Go-style cross-compilation:

```sh
netlang build app.net --target windows-x64
netlang build app.net --target linux-x64
netlang build app.net --target linux-arm64
netlang build app.net --target macos-arm64
```

These friendly Net-lang target names may internally map to platform target
triples used by the selected backend, linker, and runtime build.

The intended compilation model is:

```text
Net-lang source
    ↓
Frontend
    ↓
Semantic analysis
    ↓
Net-lang IR
    ↓
Native backend
    ↓
Target object code
    +
Net-lang runtime compiled for the target
    ↓
Linker
    ↓
Standalone native executable
```

The runtime should be portable and target-aware. OS-specific networking and
system behavior should be hidden behind the runtime instead of being emitted
throughout generated code. Networking primitives such as HTTP, TCP, UDP, `SEND`,
and `RECEIVE` should eventually lower to runtime operations conceptually like:

```text
net_http_get
net_tcp_connect
net_udp_bind
net_send
net_receive
```

Runtime implementations may be written in Rust and cross-compiled for each
supported target. The toolchain may ship prebuilt target-specific runtimes so
users do not need to build the runtime themselves. Produced binaries should aim
to be standalone and easy to distribute where practical.

Cross-compilation should begin with a deliberately small set of supported
platforms rather than attempting every operating system and architecture at
once. The proposed target tiers are:

- **Tier 1:** `windows-x64`, `linux-x64`, `macos-arm64`
- **Possible Tier 2:** `windows-arm64`, `linux-arm64`, `macos-x64`

The exact native backend is not finalized. LLVM, Cranelift, or another backend
may be chosen after the interpreter and Net-lang IR exist.

### Compiler and runtime work

These are not syntax features, but are required for Net-lang to become executable:

- [ ] Lexer tokens for `TCP`, `UDP`, `SEND`, `RECEIVE`, `TO`, and `USING`
- [ ] Parser support for TCP and UDP connection expressions
- [ ] Parser support for `SEND` and `RECEIVE` expressions
- [ ] AST nodes for TCP and UDP connections and `SEND` / `RECEIVE`
- [ ] Semantic analysis
- [ ] Symbol tables and scope checking
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
