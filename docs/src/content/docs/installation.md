---
title: "Installation"
description: "Installation in the Net-lang language and toolchain."
---

Implemented behavior, with limitations and future work called out below.

Run shell commands from the repository root; inline source paths below are also
relative to that root.

Prerequisites:

- Rust with Cargo, supporting the Rust 2024 edition.
- Flex available as `flex`.
- A C99 compiler available as `cc`, and an archiver available as `ar`.

The lexer build uses these system tools directly. Cargo also builds the HTTP
runtime's `reqwest`/Rustls and `serde_json` dependencies; the TLS dependency may
require CMake. The build has been verified on macOS with Apple's Flex and Clang.
Cross-compilation and Windows toolchains are not configured.

```sh
cargo build
cargo run -- tokens examples/complete.net
cargo run -- ast examples/complete.net
cargo run -- check examples/complete.net
cargo run -- run examples/hello.net
cargo run -- run examples/parallel_compute.net
```

The binary is also available directly:

```sh
./target/debug/netlang tokens examples/request.net
./target/debug/netlang ast examples/complete.net
./target/debug/netlang check examples/complete.net
./target/debug/netlang run examples/hello.net
```

To install the binary on your Cargo executable path:

```sh
cargo install --path .
netlang ast examples/complete.net
```

`FLEX`, `CC`, and `AR` can override the tool executable paths. Build output,
including generated `lex.yy.c`, stays in Cargo's target directory. Bison and
`libfl` are not required.
