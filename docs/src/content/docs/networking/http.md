---
title: "HTTP and HTTPS"
description: "HTTP and HTTPS in the Net-lang language and toolchain."
---

Implemented behavior, with limitations and future work called out below.

```netlang check
let response = GET "https://example.com" {
    headers: { "Accept": "text/html" },
    timeout: 5s,
    retry: 1
};
print(response.status);
print(response.body);
```

Save this as `request.net` and run `netlang run request.net` with an installed
compiler. This performs a real request; `netlang check request.net` only validates
the program.

`GET`, `POST`, `PUT`, `PATCH`, `DELETE`, and `HEAD` execute through a reusable
blocking HTTP client. Absolute `http://` or `https://` URLs are required. HTTPS
uses Rustls with certificate verification enabled. Interpreter code only calls
the runtime interface; HTTP implementation details stay in `src/runtime/http.rs`.

Supported request options:

- `headers`: an object with string header values.
- `query`: an object whose values are strings, numbers, or booleans. Values are
  URL-encoded and appended to any existing query parameters.
- `json`: JSON-compatible literals, arrays, and objects. Functions and durations
  are rejected. The runtime serializes the value and sets the JSON content type.
- `timeout`: a positive duration up to 24 hours, defaulting to 30 seconds per
  attempt. The timeout covers connecting and reading the response body.
- `retry`: zero to ten additional attempts, defaulting to zero. Connection errors,
  timeouts before response headers, and statuses 429/502/503/504 can trigger a
  retry. Body-read errors are reported immediately. Retries currently have no
  backoff; explicitly retrying a write request can repeat its remote side effects.

Unknown options or invalid option types are runtime errors. Automatic redirects,
implicit client retries, and system proxies are disabled, so redirect responses
and the configured attempt count remain visible to the program.

A response is an object with `status` (integer), `body` (UTF-8 string), and
`headers` (an object keyed by lowercase header names). Repeated header values
become arrays of strings. Bodies are limited to 8 MiB; binary body support is
future work. HTTP error statuses are ordinary responses, including the final
response after exhausted retries. Transport failures are runtime errors.

HTTP integration tests use local loopback servers; they make no public-network
requests. TLS is supplied by the verified client configuration, but a local
certificate-based HTTPS integration fixture remains future test work.
