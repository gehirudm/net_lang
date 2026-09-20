---
title: Getting started
description: Build Net-lang and execute an offline program.
---

Install [the build prerequisites](/net_lang/installation/), then run these commands
from the repository root:

```sh
cargo build --locked
cargo run --locked -- check examples/hello.net
cargo run --locked -- run examples/hello.net
```

The bundled program prints `Hello, Bob`. To write your own, save this as `hello.net`:

```netlang run
fn main() {
    let name = "Alice";
    print("Hello, " + name);
}
```

```sh
cargo run --locked -- run hello.net
```

This prints `Hello, Alice`. Statements end in semicolons, and blocks use braces.
Unannotated variables are mutable and may change value types.

To opt into checked contracts:

```netlang run
type User { name: string }
fn greet(user: User) -> string {
    return "Hello, " + user.name;
}
print(greet(User { name: "Alice" }));
```

`check` performs semantic analysis without network effects. `run` executes
top-level statements, then calls a top-level parameterless `main` if present.
Scripts without `main` are supported. See the [CLI reference](/net_lang/cli/).

The [network example on the home page](/net_lang/) makes a real HTTP(S) request
when run. Offline examples are useful while learning; HTTP/transport failures are
reported as runtime errors, and prior output is retained.
