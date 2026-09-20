---
title: "Syntax and operators"
description: "Syntax and operators in the Net-lang language and toolchain."
---

Implemented behavior, with limitations and future work called out below.

- Functions, calls, `let`, assignment, and optional return values; optional
  primitive and named-type annotations on bindings, parameters, and function returns.
- Top-level named types and explicit construction with checked, required fields.
- Blocks, `if` / `else if` / `else`, `while`, `for item in items`, `break`, and `continue`.
- Integers, floats, strings, booleans, null, arrays, and objects.
- Property access, indexing, unary `!` and `-`, arithmetic, comparisons,
  equality, logical operators, and right-associative assignment.
- `GET`, `POST`, `PUT`, `PATCH`, `DELETE`, and `HEAD` request expressions.
- Duration literals in `ms`, `s`, and `m`, normalized to checked `u64`
  milliseconds in the AST.
- `parallel item in collection` statements.
- Match patterns: integer, string, boolean, null, and `_`.
- Line comments and non-nested block comments.

Simple statements (`let`, assignment, calls, `return`, `break`, `continue`, and other expressions)
require semicolons. Block-based constructs do not require a trailing semicolon,
as in the specification's examples. Lists of array elements, object fields,
arguments, and parameters accept an optional trailing comma.

Identifiers follow `[a-zA-Z_][a-zA-Z0-9_]*`. Object keys are identifiers or
strings; quote a keyword when using it as a key. `timeout`, `retry`, `headers`,
`query`, and `json` remain ordinary identifiers. Request options are validated
when a request executes, rather than during parsing or semantic checking.

Strings support `\n`, `\t`, `\r`, `\"`, and `\\`. Raw newlines are
not allowed inside strings. Strings may contain UTF-8 text. Interpolation is not
performed; use `"/users/" + id`.

Precedence from tightest to loosest is grouping, postfix operations, unary
operators, multiplication/division/remainder, addition/subtraction, comparisons,
equality, `&&`, `||`, `SEND`, assignment. Binary operators associate left; assignment
associates right. Assignment targets must be identifiers, properties, or indexes.

### Request expression boundaries

The grammar deliberately follows `HTTP_METHOD expression object?`:

```netlang
GET api + "/users" { timeout: 5s }
```

The full expression `api + "/users"` is the URL, and the following object is its
configuration. To access the **request result**, put the request in parentheses:

```netlang
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
