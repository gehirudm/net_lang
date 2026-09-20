---
title: "Compiler development"
description: "Compiler development in the Net-lang language and toolchain."
---

Implemented behavior, with limitations and future work called out below.

[GitHub Actions CI](https://github.com/gehirudm/net_lang/blob/main/.github/workflows/ci.yml) runs on pushes, pull requests,
and manual dispatches. Ubuntu 24.04 and macOS 15 run the complete debug and
release test suites, including doctests and loopback networking tests, plus CLI
smoke checks. A separate Ubuntu job checks formatting, strict Clippy, and workflow
syntax with actionlint. Builds use stable Rust and the committed Cargo.lock
(`--locked`), with Flex, C tools, and CMake available on each runner.

Actions are pinned to commit hashes and updated through weekly Dependabot PRs.
Compiler CI uses read-only repository permissions, cancels superseded runs, and caches
Cargo downloads/builds by OS, architecture, Rust version, and build inputs.
Windows CI remains deferred until the native lexer build supports that toolchain.
The workflows become active when these files are pushed to GitHub; branch
protection settings are managed separately in the repository settings.

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
Local milestone notes live in `devlogs/`, which is ignored by Git. Each note
describes behavior, design decisions, tradeoffs, verification, and remaining work.

## Documentation development

The [documentation workflow](https://github.com/gehirudm/net_lang/blob/main/.github/workflows/docs.yml)
checks content, syntax highlighting, annotated examples, built links, and LLM
exports. Pull requests only validate; the `main` branch can deploy to GitHub Pages.
Only the deployment job receives Pages write and OIDC permissions. The site has
its own npm lockfile and weekly grouped Dependabot updates.

See [docs/README.md](https://github.com/gehirudm/net_lang/blob/main/docs/README.md)
for local preview, generation, and deployment setup. Node.js is needed only for
documentation tooling, not for the Net-lang compiler.
