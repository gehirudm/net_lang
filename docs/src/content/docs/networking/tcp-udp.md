---
title: "TCP, UDP, and bytes"
description: "TCP, UDP, and bytes in the Net-lang language and toolchain."
---

Implemented behavior, with limitations and future work called out below.

For a TCP client, connect to an existing server, send bytes, and read a chunk:

```netlang check
let conn = TCP "127.0.0.1:9000";
conn SEND "hello";
let response = conn RECEIVE;
print(response);
close(conn);
```

For connected UDP, each receive reads one datagram from the selected peer:

```netlang check
let socket = UDP "127.0.0.1:5000";
socket SEND "hello";
let packet = socket RECEIVE;
print(packet);
close(socket);
```

These programs need a listening peer to execute successfully. For an offline,
self-contained TCP demonstration, run `netlang run examples/tcp_loopback.net`
from the repository root.

### Frontend-only protocol syntax

The frontend parses TCP/UDP connection expressions, optional `using Protocol`
metadata, and untyped communication operators:

The following is a **parser-only example**, preserved from the earlier design
discussion: the standard runtime rejects `using Login` and rejects `TO` on a
connected UDP socket. Working unconnected UDP and listener examples follow below.

```netlang parse
let conn = TCP "example.com:9000" using Login;
conn SEND "hello";
let response = conn RECEIVE;

let socket = UDP "192.168.1.20:5000";
socket SEND response TO "192.168.1.20:5001";
```

`RECEIVE` is a postfix operator. `SEND` binds below logical OR and above
assignment; nested sends require parentheses. Connection addresses accept
expressions; use `(TCP address) RECEIVE` when receiving directly from a new
connection. `TO` remains provisional syntax.

The standard runtime executes TCP clients/listeners and connected/unconnected UDP sockets.
`using Login` is parsed but rejected until protocol support is implemented.
`TO` requires an unconnected UDP socket. Address and destination values must be
strings. Semantic analysis checks operand names;
transport capabilities are checked at runtime, not statically.

### Unconnected UDP

An unconnected socket binds a local address using an object configuration:

```netlang check
let socket = UDP { bind: "0.0.0.0:5000" };
let packet = socket RECEIVE;
socket SEND packet.data TO packet.address;
close(socket);
```

Only the `bind` field is accepted; its value must be a string. Port `0` requests
an ephemeral port. Unconnected receives return `{ data: bytes, address: string }`,
including the numeric sender address (IPv6 addresses are bracketed). Each send
requires `TO`. Connected UDP retains byte-valued receives and rejects `TO`,
avoiding platform-dependent behavior when overriding a connected peer.

### Transport and byte contracts

The byte-oriented contract is:

- `SEND` accepts strings (encoded as UTF-8) or byte values and returns `null`.
  It adds no newline, length prefix, JSON encoding, or other framing.
- TCP `RECEIVE` returns an arbitrary nonempty byte chunk, at most 65,535 bytes,
  or `null` at EOF. A receive may contain part of a send or combine several sends.
- Connected UDP `RECEIVE` returns one complete datagram as bytes. Empty datagrams remain
  empty byte values, distinct from TCP EOF. UDP sockets bind an ephemeral local
  port and connect to the supplied peer; this is not a reliability handshake.
  Maximum outgoing datagram size depends on the OS; oversized sends are errors.
- Bytes can be stored, compared, printed, and sent again without UTF-8 decoding.
  Printing uses `bytes[255, 0, ...]`. Binary values are not implicitly text.
  `bytes([0, 255])` constructs bytes from integers in 0..255;
  `encode_utf8(text)` and `decode_utf8(data)` perform explicit conversion.
  Decoding rejects invalid or incomplete UTF-8, reporting the first invalid byte
  offset. `byte_len(data)` counts bytes; `data[index]` reads an integer byte.
  `left + right` joins two byte values without changing either input. Byte
  indexing is read-only. Join TCP chunks before decoding split UTF-8 characters.
  These one-argument built-ins can be shadowed like `print`; aliases retain
  runtime argument checks. No byte literal syntax or implicit framing is added.
- Connection copies alias the same opaque handle. A runtime owns its sockets
  until `close(conn)` or runtime drop, with a 1,024-open-connection limit.
  `close(conn)` returns `null`, releases the socket immediately, and invalidates
  all aliases. Repeated close, send, or receive on that handle reports an error.
  It is a full close, not TCP half-close; it does not wait for the peer to process
  sent data. Parallel workers must open and close their own
  connections; captured handles from another runtime are rejected.
- Connect attempts and blocking socket reads/writes use five-second timeouts.
  `set_timeout(conn, 250ms)` changes subsequent read/write timeouts on that socket
  and all its aliases, returning `null`. It accepts durations from 1ms through
  1440m (24 hours); zero does not disable timeouts. This works for TCP and both
  UDP modes, and respects runtime ownership and closed-handle checks.
  DNS resolution is synchronous and outside that timeout; multiple addresses
  and partial TCP writes can take longer overall. These are per-operation OS
  timeouts, not total request deadlines. Failed sends may have sent some bytes;
  the runtime does not retry them automatically.

Protocol resolution, framing, typed packets, typed receives, end-to-end deadlines,
and static transport capability
checks remain future work. Loopback tests cover binary traffic, EOF, datagram
boundaries, timeouts, socket cleanup, and independent parallel connections.

### TCP listeners

TCP listening uses an initial object configuration form:

```netlang check
let listener = TCP { listen: "127.0.0.1:9000" };
set_timeout(listener, 30s);
let peer = accept(listener);
print(peer.address);
peer.connection SEND "hello";
close(peer.connection);
close(listener);
```

Only a string-valued `listen` field is accepted. `accept` returns an object with
`connection` and numeric peer `address` fields; it defaults to a five-second
timeout. `set_timeout(listener, duration)` changes subsequent accept waits and
the initial read/write timeouts of newly accepted streams. Existing streams
retain their own timeout. Closing a listener leaves its accepted streams open.
`SEND` and `RECEIVE` reject listener handles: accept a connection first.

`local_address(handle)` returns the bound local address of a listener, TCP
stream, or UDP socket, including the OS-assigned port when bound to port `0`.
Wildcard addresses describe the binding, not necessarily an address a remote
peer can connect to. Listeners and streams share the runtime's 1,024-handle
limit and worker ownership rules. Parallel workers cannot accept on a captured
parent listener. This initial serial accept model does not add task spawning,
shared listeners, half-close, or protocol framing.

Run `cargo run -- run examples/tcp_loopback.net` for a complete local exchange
without an external server. Server declaration and route syntax remain separate
future language work.
