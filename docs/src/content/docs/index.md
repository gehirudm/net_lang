---
title: Net-lang
description: Learn the experimental language built around network communication.
---

Network operations are part of the language. Net-lang combines a Flex lexer and
Rust frontend with a checked tree-walking interpreter.

```netlang check
fn main() {
    let response = GET "https://example.com" {
        timeout: 5s,
        retry: 1
    };
    print(response.status);
}
```

**Working today:** functions and control flow, dynamic values with optional checked
annotations, nominal records, HTTP(S), TCP clients/listeners, UDP, SEND/RECEIVE, bytes,
and bounded parallel iteration. This is an experimental personal project, not a
production-ready networking platform.

- [Get started offline](/net_lang/getting-started/) with the interpreter.
- [Learn the syntax](/net_lang/language/syntax/) and [type contracts](/net_lang/language/types/).
- [Make HTTP requests](/net_lang/networking/http/) or use [TCP and UDP](/net_lang/networking/tcp-udp/).
- [Inspect the compiler](/net_lang/project/compiler/) and [roadmap](/net_lang/project/roadmap/).

**Planned:** typed wire protocols, WebSockets, modules, the editor extension/LSP,
Net-lang IR, native code generation, and cross-compilation. Proposed examples are
labeled; syntax highlighting does not imply a feature is executable.

For automated readers: [llms.txt](/net_lang/llms.txt),
[llms-full.txt](/net_lang/llms-full.txt), and the
[Markdown manifest](/net_lang/markdown/index.json).
