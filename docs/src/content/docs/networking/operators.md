---
title: SEND and RECEIVE
description: First-class communication operators and their current limits.
---

SEND and RECEIVE are language operators, not methods on connection objects.
These are syntax fragments; `conn`, `data`, and `address` must already be bound:

```netlang
conn SEND data;
let data = conn RECEIVE;
socket SEND packet TO address;
```

The current runtime supports TCP streams and connected/unconnected UDP. SEND
accepts strings (UTF-8 encoded) or bytes and returns `null`. TCP RECEIVE returns
arbitrary byte chunks or `null` at EOF; connected UDP returns a complete datagram.
Unconnected UDP returns `{ data: bytes, address: string }` and requires TO when sending.
Connected UDP rejects TO. TCP listener handles must be accepted before communication.

No framing, newline, or JSON encoding is implicit. SEND may partially affect a
remote peer before failing; it is not retried automatically. Connection handles
belong to their runtime, and worker captures cannot share parent sockets.

RECEIVE is postfix. SEND binds below logical OR and above assignment; parenthesize
nested sends. See [TCP/UDP and byte handling](/net_lang/networking/tcp-udp/) for
working constructors, timeout behavior, and built-ins.

**Not implemented:** typed RECEIVE suffixes, structured record wire encoding,
WebSockets, and protocol-state execution. `using Protocol` parses as metadata but
is rejected by the standard runtime. TO works today but its spelling is provisional.
