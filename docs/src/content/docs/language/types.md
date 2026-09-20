---
title: "Annotations and named types"
description: "Annotations and named types in the Net-lang language and toolchain."
---

Implemented behavior, with limitations and future work called out below.

Unannotated code stays dynamic, including bindings that change value types.
Primitive `let` annotations opt into a checked contract:

```netlang run
let port: int = 8080;
let timeout: duration = 5s;
let name: string = "Alice";
let payload: bytes = encode_utf8(name);

fn next_port(port: int) -> int {
    return port + 1;
}
```

The primitive names are `int`, `float`, `bool`, `string`, `duration`, and `bytes`.
Annotations may also name a declared type, as described below.
These are ordinary identifiers outside annotation positions. `check` rejects
unknown type names and provable initializer/reassignment mismatches. Dynamic
values are checked when entering an annotated binding and on every later write,
including writes through captured functions. No implicit numeric conversion is
performed: `float` requires a float value.

Function parameters and return values may use the same optional annotations.
Annotated parameters remain mutable but must keep their declared type; unannotated
parameters remain dynamic. Direct calls check known argument and result types
statically. Calls through function-valued variables also enforce contracts at
runtime. Argument values are evaluated before runtime argument checks; an invalid
argument prevents the function body from running. Returned dynamic values are
checked before leaving the function, with its call stack and source location.
Falling through a function returns `null`, which violates any currently supported
return annotation. Missing-return path analysis is not implemented; fallthrough
is checked at runtime. Explicit `return;` in an annotated function is a semantic
error. Unannotated functions keep their existing behavior.

Unannotated collections remain heterogeneous; collection schemas, optional types,
and transport types are later work. A successful semantic check still
cannot guarantee runtime success. Run [examples/annotations.net](https://github.com/gehirudm/net_lang/blob/main/examples/annotations.net)
for a complete example.

### Distinct named types

Named types require explicit construction. A plain object with the same fields
does not satisfy a named annotation, and two different named types remain
incompatible even when their fields match:

```netlang run
type User {
    id: int,
    name: string
}

fn rename(user: User, name: string) -> User {
    user.name = name;
    return user;
}

let user: User = User { id: 1, name: "Alice" };
let renamed = rename(user, "Bob");
```

All fields are required and annotated with a primitive or another named type.
Construction rejects missing, extra, duplicate, or incorrectly typed fields;
dynamic field values are checked at runtime. Fields may be reordered and lists
may have trailing commas. `user.name` and `user["name"]` support reads and checked
writes, including when `user` itself is unannotated. Adding fields is not allowed.
Records copy by value, so the example leaves `user.name` equal to `"Alice"`.
Equality requires the same named type as well as equal field values.

Type declarations are currently top-level only and are visible throughout the
program, including before their declaration. Type names have a separate namespace
from variable/function names; primitive and runtime category names are reserved.
Named identity survives copying, function calls, and parallel worker snapshots.
Runtime hosts can inspect record names and fields but cannot construct or mutate
record internals to bypass contracts. Named values from separate executions have
different identities.

In a control-flow header, parenthesize a direct constructor, for example
`if (Flag { active: true }).active { ... }`. Call arguments and array elements
already delimit constructors. Request URL/address parsing also preserves existing
configuration and block boundaries; parentheses make a constructor explicit there.

Local type declarations, optional/default fields, generics, recursive required-field
cycles, methods, and inheritance are not implemented. No wire encoding or automatic
JSON conversion is added: named values do not yet make typed networking executable.
See [examples/named_types.net](https://github.com/gehirudm/net_lang/blob/main/examples/named_types.net).
