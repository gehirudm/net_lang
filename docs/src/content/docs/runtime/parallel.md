---
title: "Parallel execution"
description: "Parallel execution in the Net-lang language and toolchain."
---

Implemented behavior, with limitations and future work called out below.

`parallel item in collection { ... }` evaluates an array once and executes its
iterations in isolated workers. The statement joins all started workers before
the parent continues. Captured outer bindings are read-only snapshots, including
when accessed through inherited functions. Loop variables and worker-local
bindings remain mutable; copying a captured array/object into a local variable
allows modifying the local copy.

```netlang run
let settings = { multiplier: 2 };
parallel n in [1, 2, 3] {
    let result = n * settings.multiplier;
    print(result);
}
```

There are four concurrent interpreter workers by default, configurable from 1
through 64 using `Limits::parallel_workers`. Work runs in bounded batches.
Nested parallel blocks preserve fresh isolation boundaries but run their
iterations sequentially within the existing worker, preventing thread growth
and nested pool deadlocks. HTTP calls within separate workers can overlap.

Each iteration buffers print output, limited to 1 MiB by default through
`Limits::parallel_output_bytes`. After a batch joins, output is replayed in input
order, including output produced before a worker error. Network effects are real
and are neither ordered nor rolled back. If workers fail, the first error in
input order is reported after the current batch joins; later batches are not
started. Running requests are not cancelled and may finish or time out first.
Worker panics become runtime errors; buffered output from a panicked worker is
not recoverable.

Custom runtimes implement `Runtime::fork` to supply a `Send`-capable host for each
iteration. Host requests run in workers, while buffered printing is delivered
through the parent host. The standard runtime supports forks; a custom runtime
without this capability reports an explicit error for nonempty parallel blocks.

See [parallel_compute.net](https://github.com/gehirudm/net_lang/blob/main/examples/parallel_compute.net) for an offline example.
An integration test also executes the full success-criteria example against two
local HTTP endpoints that require concurrent requests before replying.
