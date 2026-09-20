---
title: "Design proposals and status"
description: "Design proposals and status in the Net-lang language and toolchain."
---

Mixed status: every proposal below is explicitly labeled.

Milestone checklists now live in [ROADMAP.md](https://github.com/gehirudm/net_lang/blob/main/ROADMAP.md). These examples
preserve the earlier design discussion; status labels describe their current scope.

### Language syntax examples and proposals

- **Implemented:** **Optional primitive annotations on `let` bindings**
- **Implemented:** **Optional primitive function parameter and return annotations**
- **Implemented:** **Named-type annotations on bindings, parameters, returns, and fields**
- **Proposed:** **Generic type annotations**, including typed request responses

  The primitive annotation below works; `Response` requires a declaration and
  the ellipsis abbreviates a body. Generic `Response<User>` remains unimplemented.

  ```netlang
  let port: int = 8080;

  fn fetch_user(id: int) -> Response {
      ...
  }
  ```

- **Implemented:** **User-defined types and structs** — top-level nominal types with explicit construction

  ```netlang
  type User {
      id: int,
      name: string,
      active: bool
  }
  ```

- **Proposed:** **Typed request responses**

  ```netlang
  let user: Response<User> = GET "/users/1";
  ```

- **Proposed:** **String interpolation**

  ```netlang
  let response = GET "/users/${id}";
  ```

  String concatenation is currently required:

  ```netlang
  GET "/users/" + id;
  ```

- **Proposed:** **Imports and modules**

  ```netlang
  import net.http;
  import "./utils.net";
  ```

- **Proposed:** **Constants**

  ```netlang
  const API_URL = "https://api.example.com";
  ```

- **Implemented:** **`break` and `continue`** — parsed, checked, and executed

  ```netlang
  for item in items {
      if item == target {
          break;
      }
  }
  ```

- **Proposed:** **Richer match patterns**, including ranges and eventually destructuring

  ```netlang
  match response.status {
      200..299 => { ... }
      404      => { ... }
      _        => { ... }
  }
  ```

- **Proposed:** **First-class error handling**

  The syntax is not finalized. One possible form is:

  ```netlang
  try {
      ...
  } catch error {
      ...
  }
  ```

  A result-oriented model is also under consideration.

- **Proposed:** **Additional concurrency syntax**, potentially including `spawn`, `await`,
  or a Net-lang-specific synchronization model

  ```netlang
  spawn {
      ...
  }
  ```

### Network-specific syntax

- **Partial / evolving:** **First-class `SEND` and `RECEIVE` operators**

  Untyped operators now execute on TCP clients and connected/unconnected UDP sockets.
  The broader typed and protocol-aware forms below remain planned.

  `SEND` and `RECEIVE` are Net-lang language constructs rather than ordinary
  library methods. They are intended to work across suitable transports,
  including TCP connections, WebSockets, connected UDP sockets, and custom
  protocol connections, depending on the target's type. Conventional method
  forms such as `conn.send(data)` and `conn.receive()` are not planned.

  ```netlang
  conn SEND data;
  let data = conn RECEIVE;
  ```

  An unconnected transport can include a destination:

  ```netlang
  socket SEND packet TO address;
  ```

  Explicit named constructors are implemented, but record wire encoding is absent
  and a typed RECEIVE suffix is not parsed. These networking forms remain proposals:

  ```netlang
  conn SEND LoginPacket {
      username: "alice",
      token: token
  };

  let response = conn RECEIVE LoginResponse;
  ```

  Semantic analysis will verify whether the target transport or type supports
  `SEND`, `RECEIVE`, or both.

- **Partial / evolving:** **TCP connections**

  Basic clients and `TCP { listen: address }` listeners are implemented;
  protocol-aware connections remain planned.

  ```netlang
  let conn = TCP "example.com:9000";

  conn SEND "hello";

  let response = conn RECEIVE;
  ```

  A future protocol-aware form is:

  ```netlang
  let conn = TCP "example.com:9000" using MyProtocol;

  conn SEND Credentials {
      username: "alice",
      password: password
  };

  let result = conn RECEIVE;
  ```

  The initial listening form is `TCP { listen: address }`, followed by
  `accept(listener)`. Higher-level server declaration syntax remains future work.

- **Partial / evolving:** **UDP communication**

  Connected UDP, unconnected `UDP { bind: address }`, and `TO` sends are implemented.

  A connected UDP socket can use the standard `SEND` and `RECEIVE` operators:

  ```netlang
  let socket = UDP "192.168.1.20:5000";

  socket SEND data;

  let packet = socket RECEIVE;
  ```

  For an unconnected UDP socket, the implemented, provisional syntax supplies a
  destination with `TO`:

  ```netlang
  socket SEND data TO "192.168.1.20:5000";

  let packet = socket RECEIVE;
  ```

  `TO` is implemented but provisional and may change.

- **Proposed:** **WebSocket connections**

  ```netlang
  let socket = WS "wss://example.com/events";
  ```

- **Proposed:** **Streaming syntax**

  ```netlang
  for message in WS "wss://example.com/events" {
      print(message);
  }
  ```

- **Proposed:** **Reusable network policies** for timeout, retry, rate-limit, and
  connection behavior

  ```netlang
  policy external_api {
      timeout: 5s,
      retry: 3
  }
  ```

- **Proposed:** **Native server and route declarations**

  ```netlang
  server 8080 {
      GET "/users/:id" {
          ...
      }

      POST "/users" {
          ...
      }
  }
  ```

- **Proposed:** **Protocol definitions and protocol state machines**

  Protocol declarations may describe state transitions with the same `SEND` and
  `RECEIVE` language operators:

  ```netlang
  protocol Login {
      state Connected {
          SEND Credentials -> Waiting
      }

      state Waiting {
          RECEIVE Success -> Authenticated
          RECEIVE Failure -> Connected
      }
  }
  ```

  Normal Net-lang code using that protocol should conceptually look like:

  ```netlang
  let conn = TCP "server.example.com:9000" using Login;

  conn SEND Credentials {
      username: "alice",
      password: password
  };

  let result = conn RECEIVE;
  ```

- **Proposed:** **Binary packet and protocol structures**

  ```netlang
  packet Header {
      version: u8,
      length: u16be
  }
  ```

- **Proposed:** **Request pipelines and network data pipelines**

  ```netlang
  GET "/events"
      |> decode json
      |> process;
  ```

### Possible general language features

These are lower priority and should be added only when they serve networking
programs:

- **Proposed:** Enums
- **Proposed:** Generics
- **Proposed:** Tuples
- **Proposed:** Closures and anonymous functions
- **Proposed:** Optional values
- **Proposed:** Standard collection types such as maps and sets
- **Proposed:** Visibility with `pub` and private declarations
- **Proposed:** Package and module namespaces
- **Proposed:** Macros

Classes and inheritance are intentionally not a current priority. Net-lang does
not aim to reproduce every feature of a general-purpose object-oriented language.
