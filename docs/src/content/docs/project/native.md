---
title: "Future compiler architecture"
description: "Future compiler architecture in the Net-lang language and toolchain."
---

Planned architecture; the frontend and interpreter portions already work.

Net-lang will be built in stages. The frontend, initial semantic analysis, and
core interpreter with HTTP and parallel execution are implemented. Further
transports, additional concurrency primitives, IR, the native backend, and the
cross-platform toolchain remain under development or future work.

### Phase 1 — Compiler frontend

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

### Phase 2 — Interpreter

```text
AST
  ↓
Semantic Analysis
  ↓
Tree-Walking Interpreter
  ↓
Net-lang Runtime
```

The interpreter is being built before native code generation so that language
semantics, the networking model, error handling, concurrency behavior, and
runtime APIs can be tested before the compiler commits to a native backend. It
already executes variables and expressions, functions, control flow, HTTP
requests, and isolated parallel iteration. TCP, UDP, and `SEND` / `RECEIVE`
execute through the standard runtime for TCP clients/listeners and connected/unconnected UDP sockets.
Protocol-aware transport execution and additional concurrency primitives remain planned.

### Phase 3 — Intermediate Representation

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

### Phase 4 — Native compilation

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

### Phase 5 — Cross-platform compilation

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
