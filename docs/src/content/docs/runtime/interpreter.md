---
title: "Interpreter behavior"
description: "Interpreter behavior in the Net-lang language and toolchain."
---

Implemented behavior, with limitations and future work called out below.

`netlang run file.net` performs semantic checks before executing top-level
statements, then invokes a top-level `fn main()` if one exists. `main` must have
no parameters. Scripts without `main` execute just their top-level statements.
An explicit call to `main` in top-level code is an ordinary call and does not
suppress the automatic entry-point call. Return values do not set process exit
codes; runtime errors cause a failure exit.

The interpreter supports literals, arithmetic, short-circuit logical operators,
mutable bindings, arrays/objects, functions, lexical captures, recursion,
conditionals, while/for loops, parallel iteration, match, return, `break`, and
`continue`. `print` writes its arguments
separated by spaces followed by a newline. Output already written is retained
when a later runtime error occurs.

Current runtime decisions:

- `break;` exits the nearest `while` or `for` loop. `continue;` skips the rest
  of the current iteration; a `while` loop then reevaluates its condition.
  In a `parallel` body, `continue;` finishes only that worker iteration. Other
  iterations still run. A nested serial loop consumes its own `break`/`continue`;
  breaking out of a parallel loop is rejected because cancellation is not defined.
  See [examples/loop_control.net](https://github.com/gehirudm/net_lang/blob/main/examples/loop_control.net).
- Conditions and logical operators require booleans; there is no implicit
  truthiness. Integer division truncates toward zero. Arithmetic overflow,
  division by zero, and non-finite float results are errors. Mixed integer/float
  arithmetic requires the integer to be exactly representable as a float.
- String concatenation accepts scalar values, including integers and durations.
  Duration addition/subtraction is checked and results remain in milliseconds.
- Arrays and objects copy by value, including arguments and return values.
  Mutation must be rooted in a mutable variable. Object assignment may add a
  field; array assignment requires an existing nonnegative integer index.
- Functions capture lexical binding identities, so later shadowing does not
  change earlier captures. A forward call that reaches a binding before its
  initializer executes is a runtime error. Calls through function-valued
  variables also check argument counts at runtime.
- Match uses the first matching literal or wildcard arm; no matching arm does
  nothing. A function that finishes without returning a value returns `null`.
- Each run defaults to one million evaluation steps, 128 active function calls,
  and 256 nested expressions. Embedders can set `interpreter::Limits` through
  `execute_with_limits`. The step budget is shared across the parent and all
  workers. These are execution limits, not a security sandbox.

The runtime interface separates printing and network effects from AST evaluation.
Embedders can provide a custom `runtime::Runtime`; the standard runtime supports
printing, HTTP(S), TCP, and UDP. Runtime errors include function call stacks and
the relevant expression or statement location when parsed with spans. Binding/function arenas
are released
after each run but retain expired scopes during a run; memory reclamation for
long-running programs remains future work.
