---
title: Built-in functions
description: Current runtime functions, distinct from a future standard library.
---

These built-ins are implemented. They can be shadowed by local bindings; aliases
retain runtime argument checks. A broader standard library is planned.

| Function | Arguments and result |
| --- | --- |
| `print(...)` | Space-separated display values and a newline; returns null. |
| `bytes(values)` | Array of integers in 0..255 → bytes. |
| `encode_utf8(text)` | String → bytes. |
| `decode_utf8(data)` | Bytes → string; invalid/incomplete UTF-8 is an error. |
| `byte_len(data)` | Bytes → integer byte count. |
| `close(handle)` | Releases the socket/listener, invalidates aliases; returns null. |
| `set_timeout(handle, duration)` | 1ms through 24h; changes read/write or listener accept waits. |
| `accept(listener)` | Returns `{ connection, address }`, with a bounded wait. |
| `local_address(handle)` | Returns the bound local address, including an assigned port. |

```netlang run
let data: bytes = encode_utf8("hello");
print(byte_len(data), decode_utf8(data));
print(bytes([0, 255])[1]);
```

This prints `5 hello` and `255`. Byte indexing is read-only. Repeated close and
operations on closed/foreign handles fail. Accepted streams remain open when
their listener closes. Timeout changes apply to aliases of the same handle.

[Interpreter behavior](/net_lang/runtime/interpreter/),
[parallel execution](/net_lang/runtime/parallel/), and
[TCP/UDP contracts](/net_lang/networking/tcp-udp/) define resource and error semantics.
