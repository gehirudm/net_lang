---
title: "Semantic analysis"
description: "Semantic analysis in the Net-lang language and toolchain."
---

Implemented behavior, with limitations and future work called out below.

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
- `break` requires an enclosing serial loop; `continue` requires an enclosing
  serial or parallel loop. Neither can target a loop outside the current function.
  A `break` cannot cross a parallel worker boundary.
- Calls to directly named user functions must supply the declared argument count.
  Direct calls to the byte built-ins, `close`, `accept`, and `local_address`
  require exactly one argument.
  `set_timeout` requires two arguments: a connection and a duration.
  The built-in `print` is recognized, with no argument-count restriction in this
  initial pass. Signatures of dynamic callees, including variables holding
  functions, are not inferred yet.
- Direct assignment to a captured outer variable inside `parallel` is rejected,
  including assignments through a captured object's properties or array indexes.
  Indirect writes through inherited functions are caught by the runtime.
- A `return` cannot cross a parallel worker boundary. Functions called or declared
  inside a worker may return normally.

Request URLs, request option values, object values, and all other expression
positions are traversed. Property names and object keys are not variable uses.
Request option validity, general type inference, dynamic callee types, match exhaustiveness, and
runtime initialization order are not checked yet. A successful check therefore
does not guarantee that a program will run successfully.

Semantic errors identify the filename, relevant expression or statement range, and AST scope
context. `examples/match.net`
is a syntax fragment
using an undeclared `response`; it parses but intentionally fails semantic
checking. The complete example declares that binding and passes.
