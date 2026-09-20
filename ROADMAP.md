# Project Net-lang Roadmap

## Project Vision

Net-lang makes network operations native language constructs. Build understandable
compiler stages, establish semantics with the interpreter, then introduce a
backend-independent IR and native compilation. Unannotated code stays dynamic;
optional annotations provide checked contracts.

This file owns progress tracking. [README.md](README.md) is the landing page;
[LANGUAGE_DESIGN.md](docs-internal/LANGUAGE_DESIGN.md) preserves technical details
and proposal examples for the future documentation website.

`[x]` means the stated scope is implemented and verified against source and tests.
`[ ]` means unfinished or partial. Completion requires the stated acceptance
criteria, relevant tests, and updated documentation. Update status as behavior
changes; proposals and percentages are not evidence of completion.

## Completed Milestones

### Milestone 1 — Compiler Frontend

- [x] Reentrant Flex lexer compiled as C, stable C bridge, and safe Rust FFI wrapper.
- [x] Tokens, literals, durations, keywords, operators, comments, and source positions.
- [x] Handwritten recursive-descent Rust parser with precedence and postfix chaining.
- [x] Lexer-independent Rust AST, including network expressions and nominal types.
- [x] `tokens` CLI and readable text-tree AST visualization through `ast`.
- [x] Source-line diagnostics, token spans, optional statement/expression/pattern spans,
  and semantic/runtime error locations.

**Acceptance met:** supported syntax produces tested token/AST structures; invalid
input returns located errors without terminating library callers. Visualization
is a text tree. Recovery, multiple source files, and full Unicode display-width
handling remain unfinished.

Evidence: [lexer](lexer/netlang.l), [FFI](src/lexer/ffi.rs), [parser](src/parser/),
[AST](src/ast/), [lexer tests](tests/lexer_tests.rs), [parser tests](tests/parser_tests.rs),
[expressions](tests/expression_tests.rs), [spans](tests/span_tests.rs),
[located ASTs](tests/located_ast_tests.rs), [expression spans](tests/expression_span_tests.rs),
[tree output](tests/pretty_tests.rs).

### Milestone 2 — Semantic Analysis

- [x] Symbol tables, lexical scopes, identifier resolution, shadowing, and duplicate checks.
- [x] Assignment target/mutability validation and read-only parallel capture checks.
- [x] Function hoisting, direct-call arity, returns, and loop-control boundary validation.
- [x] Optional primitive binding/parameter/return annotations and conservative static checks.
- [x] Top-level nominal types, explicit construction, required fields, named annotations,
  and duplicate/unknown field and required-field-cycle validation.
- [x] Independent semantic-analysis API and `check` CLI with collected diagnostics.

**Acceptance met:** invalid names, declarations, assignments, and supported contract
mismatches are diagnosed before execution. Dynamic values/calls still need runtime
checks. General inference, return-path analysis, static transport capability checks,
and protocol-state analysis remain unfinished.

Evidence: [analyzer](src/semantic/), [type registry](src/types.rs),
[semantic tests](tests/semantic_tests.rs), [annotations](tests/type_annotation_tests.rs),
[named types](tests/named_type_tests.rs), [loop control](tests/loop_control_tests.rs).

### Milestone 3 — Interpreter

- [x] Tree-walking execution, mutable variables, expressions, and runtime values.
- [x] Functions, lexical captures, recursion, conditionals, loops, match, and returns.
- [x] `break`/`continue`, including per-iteration parallel `continue` and boundary checks.
- [x] Arrays, objects, nominal records, value-copy semantics, indexing, and checked mutation.
- [x] Built-ins: `print`, `bytes`, `encode_utf8`, `decode_utf8`, `byte_len`,
  `close`, `set_timeout`, `accept`, and `local_address`.
- [x] Runtime annotation contracts, function aliases, call stacks, source errors, and limits.
- [x] `run` CLI and injectable runtime interface for host effects.

**Acceptance met:** offline programs execute with tested values, output, control
flow, and failures; semantic errors precede effects. This is an initial interpreter,
not a complete standard library or a stabilized runtime API.

Evidence: [interpreter](src/interpreter/), [built-ins](src/runtime/builtin.rs),
[values](src/runtime/value.rs), [interpreter tests](tests/interpreter_tests.rs),
[bytes](tests/byte_tests.rs), [CLI tests](tests/cli_tests.rs).

### Milestone 4 — Networking Runtime: working baseline

- [x] GET, POST, PUT, PATCH, DELETE, and HEAD; HTTPS support through Rustls.
- [x] Headers, query parameters, JSON-compatible request bodies, bounded UTF-8 responses,
  per-attempt HTTP timeouts, and explicit retries.
- [x] TCP clients/byte streams/EOF and initial listeners with bounded `accept` waits.
- [x] Connected and unconnected UDP, sender addresses, and destination-aware `TO` sends.
- [x] First-class SEND/RECEIVE for strings/bytes on supported TCP/UDP handles.
- [x] Byte construction, indexing, concatenation, length, and explicit UTF-8 conversion.
- [x] Socket close/alias invalidation, ownership, local addresses, and read/write timeouts.
- [x] Bounded real parallel workers, read-only snapshots, joins, ordered printing, and shared budgets.

**Acceptance met:** loopback/injected-runtime tests verify basic HTTP, socket, byte,
timeout/retry, and concurrency behavior. HTTPS is supported by the configured TLS
client; a dedicated certificate-based integration fixture is still missing.
DNS-inclusive deadlines, backoff, binary HTTP bodies, framing, typed network responses,
record serialization, protocol execution, cancellation, and WebSockets are unfinished.
`using Protocol` parses but is rejected by the standard runtime. `TO` executes on
unconnected UDP while remaining provisional syntax.

Evidence: [HTTP](src/runtime/http.rs), [transports](src/runtime/transport.rs),
[parallel execution](src/interpreter/parallel.rs), [HTTP tests](tests/http_runtime_tests.rs),
[transport syntax](tests/transport_tests.rs), [sockets](tests/socket_runtime_tests.rs),
[listeners](tests/tcp_listener_tests.rs), [unconnected UDP](tests/unconnected_udp_tests.rs),
[timeouts](tests/socket_timeout_tests.rs), [close](tests/close_tests.rs),
[parallel tests](tests/parallel_tests.rs).

### Milestone 5 — Testing and CI: existing coverage

- [x] Lexer/parser/AST tests for structure, precedence, positions, and invalid syntax.
- [x] Semantic tests for scopes, contracts, nominal identity, and invalid control flow.
- [x] Interpreter tests for execution, errors, built-ins, and limits.
- [x] Networking tests for local HTTP, TCP, UDP, bytes, timeouts, retries, and concurrency.
- [x] GitHub Actions configuration for Ubuntu 24.04/macOS 15 host builds,
  debug/release tests, and CLI smoke checks.
- [x] Formatting, strict Clippy, actionlint/ShellCheck, pinned actions, and weekly Actions Dependabot updates.

**Acceptance met:** 138 integration tests pass locally in debug and release;
inspected workflows configure both host platforms. This is not a claim that a new
hosted CI run was performed during this documentation milestone. Windows CI and
cross-compiling Net-lang programs are unimplemented. Passing coverage is a baseline,
not exhaustive testing.

Evidence: [tests](tests/), [CI](.github/workflows/ci.yml), [Dependabot](.github/dependabot.yml).

## Current Milestone

### Documentation reorganization and verified roadmap

- [x] Concise README with verified quick start and network-oriented example.
- [x] Separate roadmap for completed, partial, upcoming, and long-term work.
- [x] Preserved technical reference in `docs-internal/LANGUAGE_DESIGN.md`.
- [x] Local Markdown links, examples, and source-backed status reviewed.
- [x] Documentation-only changes; compiler, runtime, tests, examples, and workflows unchanged.

**Acceptance:** the three documents have distinct purposes, technical content and
unfinished TODOs remain available, examples are verified, and language behavior is
unchanged. This handoff is complete. Milestone 6 is next and has not started; no
website, extension, or language server is created by this milestone.

## Upcoming Milestones

### Milestone 6 — Documentation Website

- [ ] Astro Starlight website inside `docs/`.
- [ ] Getting Started, installation, and CLI reference.
- [ ] Language syntax reference distinguishing implemented behavior from proposals.
- [ ] HTTP, TCP, UDP, and SEND/RECEIVE documentation.
- [ ] Runtime/standard-library reference and compiler architecture documentation.
- [ ] Working Net-lang code-block highlighting using a reusable TextMate grammar.
- [ ] LLM-friendly `llms.txt`, consolidated `llms-full.txt`, and machine-readable Markdown.
- [ ] GitHub Pages deployment and dedicated documentation CI/deployment workflow.

**Acceptance:** the site builds, links/examples are checked, current syntax is
highlighted, exports are generated, and deployment is verified. Organize the
internal reference into public docs without losing proposals.

### Milestone 7 — VS Code Extension

- [ ] Official extension with `.net` association and syntax highlighting.
- [ ] Shared TextMate grammar reused by the website and extension.
- [ ] Bracket/quote auto-closing, comment toggling, and code snippets.
- [ ] Static keyword completion.
- [ ] Run Net-lang file and Check Net-lang file commands.
- [ ] Local VSIX packaging and extension documentation.

**Acceptance:** a packaged VSIX installs and correctly edits, checks, and runs
sample files using the installed compiler. Semantic editor features belong to LSP.

### Milestone 8 — Language Server

- [ ] Rust Language Server Protocol implementation reusing lexer/parser/semantic analysis.
- [ ] Live syntax and semantic diagnostics.
- [ ] Identifier/function autocomplete, function signature help, and hover information.
- [ ] Go to definition and basic type-aware completion.
- [ ] Network-aware completion where compiler information supports it.
- [ ] Parser recovery suitable for incomplete editor input.
- [ ] VS Code extension integration.

**Acceptance:** editor integration tests exercise these capabilities on valid and
incomplete files. Reuse compiler logic; do not reimplement parsing in TypeScript.

### Milestone 9 — Remaining Language and Runtime Improvements

Deliver these as separate follow-up milestones. Basic transports, nominal structs,
and break/continue already work and are not awaiting reimplementation.

#### 9A — Types and semantic diagnostics

- [ ] Richer static typing, generic annotations, typed network responses, and collection schemas.
- [ ] Nominal-type extensions: local declarations, optional/default fields, and constructible recursive types.
- [ ] Return-path analysis, match exhaustiveness, and improved dynamic-callee analysis.
- [ ] Static SEND/RECEIVE capability validation; current capability checks are runtime-only.
- [ ] Multiple-source diagnostics and wide/combining-character display widths.

**Acceptance:** each selected extension has explicit semantics, positive/negative
tests, runtime contracts where needed, and updated reference docs. Primitive and
top-level nominal annotations are the implemented starting point.

#### 9B — Language ergonomics and modular programs

- [ ] String interpolation and constants.
- [ ] Imports, modules, module loader, visibility (`pub`/private), and package/module namespaces.
- [ ] Richer match patterns, including ranges and destructuring.
- [ ] First-class error handling; try/catch versus a result-oriented model remains undecided.
- [ ] Lower-priority enums, tuples, closures/anonymous functions, optional values, maps/sets, and macros.

**Acceptance:** scope and test each feature's parsing, semantics, execution, and
diagnostics separately. Named-function lexical captures already work; anonymous
syntax does not. Classes and inheritance remain outside current priorities.

#### 9C — Runtime stability and transport reliability

- [ ] Runtime API stabilization and standard library improvements beyond current built-ins.
- [ ] End-to-end deadlines including DNS/connection setup; retry backoff/policy improvements.
- [ ] Binary HTTP response bodies, explicit record encoding, framing, and typed receives.
- [ ] Additional networking/concurrency tests, including a local HTTPS certificate fixture.
- [ ] Long-running interpreter memory reclamation and broader platform coverage, including Windows CI.

**Acceptance:** document effects/failure contracts, verify them with deterministic
local tests, and retain host-runtime separation. Per-operation socket timeouts and
basic HTTP retries already work; they are not end-to-end deadlines.

#### 9D — Concurrency and additional network services

- [ ] Additional concurrency constructs (`spawn`, `await`, or a language-specific synchronization model).
- [ ] Cancellation, shared-listener/task ownership semantics, and TCP half-close where supported.
- [ ] WebSocket runtime/connections and streaming syntax.
- [ ] Reusable timeout/retry/rate-limit/connection policies.
- [ ] Server and route declarations; high-level syntax remains TBD.
- [ ] Request pipelines and network data pipelines; syntax is not finalized.

**Acceptance:** define ownership, ordering, cleanup, cancellation, and errors, then
verify each service with local tests. Current TCP listeners offer serial acceptance,
not server/route declarations.

#### 9E — Protocol-aware networking

- [ ] Protocol definitions/state machines, including resolution of `using Protocol`.
- [ ] Protocol-state checking as advanced semantic analysis.
- [ ] Binary packet/protocol layouts and typed structured SEND/RECEIVE across suitable transports.

**Acceptance:** define wire formats/transitions, reject invalid operations, and test
local protocol exchanges. Nominal records alone supply neither encoding nor protocol
behavior. SEND/RECEIVE remain language operators, not ordinary library methods.

## Long-Term / Endgame Goals

### Milestone — Net-lang Intermediate Representation

- [ ] Stabilize interpreter semantics/runtime APIs sufficiently to define a Net-lang-specific IR.
- [ ] Lower checked ASTs into IR independent of the machine-code backend.
- [ ] Backend abstraction and IR validation against interpreter behavior.

**Acceptance:** documented IR invariants, tested lowering, and agreement with the
interpreter on the supported subset. The frontend must not become tied to LLVM.

### Milestone — LLVM Native Backend

- [ ] Lower Net-lang IR into LLVM IR and generate target object code.
- [ ] Link object code with a target-specific Net-lang runtime into native executables.
- [ ] Hide OS-specific networking/system behavior behind a portable runtime boundary.

LLVM is preferred, not finalized. Reevaluate after the interpreter and IR are
stable; Cranelift or another backend remain alternatives. No IR/native backend exists.

```text
Net-lang Source
    ↓
Flex Lexer
    ↓
Rust Parser
    ↓
AST
    ↓
Semantic Analysis
    ↓
Net-lang IR
    ↓
LLVM IR
    ↓
LLVM Native Code Generation
    ↓
Target Object Code
    +
Target-Specific Net-lang Runtime
    ↓
Linker
    ↓
Native Executable
```

**Acceptance:** native programs match the interpreter on the supported subset and
call tested runtime operations. Conceptual lowering targets include `net_http_get`,
`net_tcp_connect`, `net_udp_bind`, `net_send`, and `net_receive`; these are not current APIs.

### Milestone — Cross-Platform Compilation

- [ ] Target selection and friendly names mapped to platform target triples/configuration.
- [ ] Target-specific runtime builds, potentially written in Rust and shipped prebuilt.
- [ ] Cross-platform linking, native executable generation, and runtime linking.
- [ ] Cross-compilation CLI and standalone executable packaging where practical.
- [ ] Compiler/runtime portability and Tier 1 support: Windows x64, Linux x64, macOS ARM64.
- [ ] Possible Tier 2 support: Windows ARM64, Linux ARM64, macOS x64.

The goal is Go-style cross-compilation: one toolchain builds supported OS/architecture
binaries without running the build on the target OS, subject to platform and target
toolchain constraints. Start with a small Tier 1 set, not every platform.

Future commands, **not implemented**:

```sh
netlang build app.net --target windows-x64
netlang build app.net --target linux-x64
netlang build app.net --target linux-arm64
netlang build app.net --target macos-arm64
```

**Acceptance:** select the target, lower IR to target LLVM IR/object code, select or
build the runtime, link/package, and verify execution on supported targets. Native
compilation and cross-compilation are not prerequisites for today's interpreter.
Existing Ubuntu/macOS host builds do not satisfy these criteria.

### Later ecosystem work

- [ ] Package manager after imports/modules and package namespaces are established.

**Acceptance:** define resolution/reproducibility and test package installation/use
before claiming support. No package manager is implemented today.
