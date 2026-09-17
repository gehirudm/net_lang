use netlang::{
    ast::HttpMethod,
    interpreter::execute,
    lexer::Lexer,
    parser::Parser,
    runtime::{Runtime, StandardRuntime, Value},
};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    thread,
    time::{Duration, Instant},
};

fn read_request(stream: &mut TcpStream) -> String {
    // Accepted sockets can inherit the listener's nonblocking mode on macOS.
    stream.set_nonblocking(false).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let mut data = Vec::new();
    let mut byte = [0];
    while !data.ends_with(b"\r\n\r\n") {
        stream.read_exact(&mut byte).unwrap();
        data.push(byte[0]);
        assert!(data.len() < 65536);
    }
    let headers = String::from_utf8(data.clone()).unwrap();
    let length = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().unwrap())
        })
        .unwrap_or(0);
    let mut body = vec![0; length];
    stream.read_exact(&mut body).unwrap();
    data.extend(body);
    String::from_utf8(data).unwrap()
}

fn server(
    replies: Vec<(u16, &'static str, Duration)>,
) -> (String, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    let handle = thread::spawn(move || {
        let mut requests = Vec::new();
        for (status, body, delay) in replies {
            let deadline = Instant::now() + Duration::from_secs(5);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(
                            Instant::now() < deadline,
                            "expected HTTP request was not received"
                        );
                        thread::sleep(Duration::from_millis(2));
                    }
                    Err(error) => panic!("{error}"),
                }
            };
            let request = read_request(&mut stream);
            let head = request.starts_with("HEAD ");
            requests.push(request);
            thread::sleep(delay);
            let response = format!(
                "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nX-Test: one\r\nX-Test: two\r\nConnection: close\r\n\r\n{}",
                body.len(),
                if head { "" } else { body }
            );
            // A timeout test may close the client before the response is written.
            let _ = stream.write_all(response.as_bytes());
        }
        requests
    });
    (url, handle)
}

fn object(fields: Vec<(&str, Value)>) -> Value {
    Value::Object(
        fields
            .into_iter()
            .map(|(key, value)| (key.into(), value))
            .collect(),
    )
}

#[test]
fn all_methods_execute_and_responses_preserve_status_body_and_headers() {
    for method in [
        HttpMethod::Get,
        HttpMethod::Post,
        HttpMethod::Put,
        HttpMethod::Patch,
        HttpMethod::Delete,
        HttpMethod::Head,
    ] {
        let (url, handle) = server(vec![(404, "missing", Duration::ZERO)]);
        let response = StandardRuntime::new(Vec::new())
            .request(method, &url, &object(vec![]))
            .unwrap();
        let Value::Object(fields) = response else {
            panic!()
        };
        assert_eq!(fields["status"], Value::Integer(404));
        assert_eq!(
            fields["body"],
            Value::String(
                if method == HttpMethod::Head {
                    ""
                } else {
                    "missing"
                }
                .into()
            )
        );
        let Value::Object(headers) = &fields["headers"] else {
            panic!()
        };
        assert_eq!(
            headers["x-test"],
            Value::Array(vec![
                Value::String("one".into()),
                Value::String("two".into())
            ])
        );
        let requests = handle.join().unwrap();
        assert!(requests[0].starts_with(&format!(
            "{} / HTTP/1.1",
            format!("{method:?}").to_uppercase()
        )));
    }
}

#[test]
fn interpreter_sends_json_headers_and_encoded_query() {
    let (url, handle) = server(vec![(201, "created", Duration::ZERO)]);
    let source = format!(
        r#"fn main() {{
        let response = POST "{url}/users?existing=1" {{
            headers: {{ "X-Token": "abc" }},
            query: {{ name: "Alice Smith", page: 2, active: true }},
            json: {{ name: "Alice", age: 24, active: true, tags: ["a", null] }},
            timeout: 2s
        }};
        print(response.status, response.body);
    }}"#
    );
    let program = Parser::new(Lexer::new(&source).unwrap().tokenize().unwrap())
        .unwrap()
        .parse_program()
        .unwrap();
    let mut output = Vec::new();
    execute(&program, &mut StandardRuntime::new(&mut output)).unwrap();
    assert_eq!(output, b"201 created\n");
    let requests = handle.join().unwrap();
    assert!(
        requests[0]
            .starts_with("POST /users?existing=1&active=true&name=Alice+Smith&page=2 HTTP/1.1")
    );
    assert!(requests[0].to_lowercase().contains("x-token: abc\r\n"));
    assert!(
        requests[0]
            .to_lowercase()
            .contains("content-type: application/json\r\n")
    );
    let json: serde_json::Value =
        serde_json::from_str(requests[0].split_once("\r\n\r\n").unwrap().1).unwrap();
    assert_eq!(
        json,
        serde_json::json!({"name":"Alice", "age":24, "active":true, "tags":["a",null]})
    );
}

#[test]
fn retries_are_explicit_additional_attempts() {
    let (url, handle) = server(vec![
        (503, "busy", Duration::ZERO),
        (200, "ready", Duration::ZERO),
    ]);
    let value = StandardRuntime::new(Vec::new())
        .request(
            HttpMethod::Get,
            &url,
            &object(vec![("retry", Value::Integer(1))]),
        )
        .unwrap();
    assert!(
        matches!(value, Value::Object(fields) if fields["body"] == Value::String("ready".into()))
    );
    assert_eq!(handle.join().unwrap().len(), 2);
    let (url, handle) = server(vec![(503, "busy", Duration::ZERO)]);
    let value = StandardRuntime::new(Vec::new())
        .request(HttpMethod::Get, &url, &object(vec![]))
        .unwrap();
    assert!(matches!(value, Value::Object(fields) if fields["status"] == Value::Integer(503)));
    assert_eq!(handle.join().unwrap().len(), 1);
}

#[test]
fn timeout_is_enforced() {
    let (url, handle) = server(vec![(200, "late", Duration::from_millis(200))]);
    let error = StandardRuntime::new(Vec::new())
        .request(
            HttpMethod::Get,
            &url,
            &object(vec![("timeout", Value::Duration(20))]),
        )
        .unwrap_err();
    assert!(error.contains("HTTP request failed"), "{error}");
    handle.join().unwrap();
}

#[test]
fn invalid_configuration_is_rejected_before_network_io() {
    for config in [
        Value::Null,
        object(vec![("unknown", Value::Null)]),
        object(vec![("timeout", Value::Integer(1))]),
        object(vec![("timeout", Value::Duration(0))]),
        object(vec![("timeout", Value::Duration(u64::MAX))]),
        object(vec![("retry", Value::Integer(-1))]),
        object(vec![("retry", Value::Integer(11))]),
        object(vec![("headers", Value::Null)]),
        object(vec![(
            "headers",
            object(vec![("bad\nheader", Value::String("x".into()))]),
        )]),
        object(vec![(
            "headers",
            object(vec![("X-Test", Value::Integer(1))]),
        )]),
        object(vec![("query", object(vec![("x", Value::Array(vec![]))]))]),
        object(vec![("json", Value::Duration(1000))]),
    ] {
        let error = StandardRuntime::new(Vec::new())
            .request(HttpMethod::Get, "http://127.0.0.1:1", &config)
            .unwrap_err();
        assert!(!error.contains("HTTP request failed"), "{error}");
    }
    for url in ["/relative", "file:///tmp/example", "ftp://example.com"] {
        assert!(
            StandardRuntime::new(Vec::new())
                .request(HttpMethod::Get, url, &Value::Object(BTreeMap::new()))
                .unwrap_err()
                .contains("absolute HTTP(S)")
        );
    }
}

fn raw_server(response: Vec<u8>, body_delay: Duration) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    let handle = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(Instant::now() < deadline);
                    thread::sleep(Duration::from_millis(2));
                }
                Err(error) => panic!("{error}"),
            }
        };
        read_request(&mut stream);
        let split = response.windows(4).position(|v| v == b"\r\n\r\n").unwrap() + 4;
        let _ = stream.write_all(&response[..split]);
        thread::sleep(body_delay);
        let written = stream.write_all(&response[split..]);
        if body_delay.is_zero() {
            written.unwrap();
        }
    });
    (url, handle)
}

#[test]
fn response_body_limit_and_utf8_are_enforced() {
    for (body, expected) in [
        (vec![255], "not UTF-8"),
        (vec![b'a'; 8 * 1024 * 1024 + 1], "exceeds 8 MiB"),
    ] {
        let mut response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .into_bytes();
        response.extend(body);
        let (url, handle) = raw_server(response, Duration::ZERO);
        let error = StandardRuntime::new(Vec::new())
            .request(HttpMethod::Get, &url, &object(vec![]))
            .unwrap_err();
        assert!(error.contains(expected), "expected {expected}: {error}");
        handle.join().unwrap();
    }
}

#[test]
fn timeout_also_covers_the_response_body() {
    let (url, handle) = raw_server(
        b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\nlate".to_vec(),
        Duration::from_millis(200),
    );
    let error = StandardRuntime::new(Vec::new())
        .request(
            HttpMethod::Get,
            &url,
            &object(vec![("timeout", Value::Duration(20))]),
        )
        .unwrap_err();
    assert!(
        error.contains("HTTP request failed") || error.contains("cannot read HTTP response body"),
        "{error}"
    );
    handle.join().unwrap();
}

#[test]
fn redirects_are_returned_and_exhausted_retries_preserve_the_last_response() {
    let (url, handle) = raw_server(
        b"HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:1/never\r\nContent-Length: 0\r\n\r\n"
            .to_vec(),
        Duration::ZERO,
    );
    let response = StandardRuntime::new(Vec::new())
        .request(HttpMethod::Get, &url, &object(vec![]))
        .unwrap();
    assert!(matches!(response, Value::Object(fields) if fields["status"] == Value::Integer(302)));
    handle.join().unwrap();
    let (url, handle) = server(vec![
        (503, "busy", Duration::ZERO),
        (503, "still busy", Duration::ZERO),
    ]);
    let response = StandardRuntime::new(Vec::new())
        .request(
            HttpMethod::Get,
            &url,
            &object(vec![("retry", Value::Integer(1))]),
        )
        .unwrap();
    assert!(
        matches!(response, Value::Object(fields) if fields["body"] == Value::String("still busy".into()))
    );
    assert_eq!(handle.join().unwrap().len(), 2);
}

#[test]
fn complete_program_runs_over_concurrent_local_http_requests() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut pending = Vec::new();
        // Neither request receives a response until both have arrived. A serial
        // interpreter cannot pass this fixture, regardless of machine speed.
        while pending.len() < 2 {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let request = read_request(&mut stream);
                    pending.push((stream, request));
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(
                        Instant::now() < deadline,
                        "parallel requests did not overlap"
                    );
                    thread::sleep(Duration::from_millis(2));
                }
                Err(error) => panic!("{error}"),
            }
        }
        for (mut stream, request) in pending.into_iter().rev() {
            assert!(request.to_lowercase().contains("accept: application/json"));
            let (status, body) = if request.starts_with("GET /1 ") {
                (200, "first body")
            } else {
                assert!(request.starts_with("GET /2 "));
                (404, "missing")
            };
            write!(
                stream,
                "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        }
    });
    let source = include_str!("../examples/complete.net")
        .replace("https://api1.example.com", &format!("http://{address}/1"))
        .replace("https://api2.example.com", &format!("http://{address}/2"));
    let program = Parser::new(Lexer::new(&source).unwrap().tokenize().unwrap())
        .unwrap()
        .parse_program()
        .unwrap();
    let mut output = Vec::new();
    let result = execute(&program, &mut StandardRuntime::new(&mut output));
    server.join().unwrap();
    result.unwrap();
    assert_eq!(output, b"first body\nNot found\n");
}
