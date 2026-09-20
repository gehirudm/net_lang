# Project Net-lang

An experimental programming language built around network communication.

## Overview

Net-lang makes operations such as `GET`, `SEND`, and `RECEIVE` part of the
language. It is implemented primarily in Rust, using a reentrant Flex lexer
compiled as C, a handwritten parser, and a tree-walking interpreter.

## Features

- Variables, functions, control flow, arrays, objects, and literal pattern matching.
- Optional checked annotations and distinct named types with explicit construction.
- HTTP(S) requests with headers, query parameters, JSON bodies, timeouts, and retries.
- TCP clients/listeners, connected and unconnected UDP, and byte-oriented SEND/RECEIVE.
- Bounded parallel iteration with isolated workers.
- Token inspection, readable AST trees, semantic checks, and source diagnostics.

## Quick Start

Install stable Rust/Cargo, Flex, a C99 compiler, an archiver, and CMake for the TLS
dependency. Current CI is configured for Ubuntu and macOS host builds.

From the repository root:

```sh
cargo build --locked
cargo run --locked -- check examples/hello.net
cargo run --locked -- run examples/hello.net
```

The included offline example prints `Hello, Bob`.
Use `cargo run --locked -- tokens examples/complete.net` to inspect tokens or
`cargo run --locked -- ast examples/complete.net` to print an AST.

## Example

```netlang
fn main() {
    let response = GET "https://example.com" {
        headers: { "Accept": "text/html" },
        timeout: 5s,
        retry: 1
    };

    print(response.status);
}
```

This uses implemented syntax; running it makes a network request. `check` validates
a saved `.net` file without executing network operations. More runnable programs
are in [examples/](examples/).

## Documentation

- [Roadmap](ROADMAP.md) — verified milestones and upcoming work.
- [Language and compiler design](docs-internal/LANGUAGE_DESIGN.md) — syntax,
  runtime behavior, architecture, and clearly labeled proposals.
- Documentation website — planned; not published yet.

## Development Status

The frontend, initial semantic analyzer, and interpreter are working. Net-lang is
an experimental personal project; its language and runtime APIs are still evolving.
The next milestones are the documentation website, VS Code extension, and language
server. IR, native compilation, and cross-compilation remain future work.

See [ROADMAP.md](ROADMAP.md) for detailed scope, limitations, and acceptance criteria.
