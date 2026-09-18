use netlang::{interpreter::execute, lexer::Lexer, parser::Parser, runtime::StandardRuntime};

fn run(source: &str) -> Result<String, String> {
    let program = Parser::new(Lexer::new(source).unwrap().tokenize().unwrap())
        .unwrap()
        .parse_program()
        .unwrap();
    let mut output = Vec::new();
    execute(&program, &mut StandardRuntime::new(&mut output)).map_err(|error| error.to_string())?;
    Ok(String::from_utf8(output).unwrap())
}

#[test]
fn byte_construction_indexing_and_explicit_utf8_round_trip() {
    assert_eq!(
        run(r#"
        let data = bytes([0, 127, 255]);
        print(data, data[0], data[2], byte_len(data));
        let text = "hé🌍";
        print(decode_utf8(encode_utf8(text)) == text);
        print(byte_len(encode_utf8(text)));
        print(bytes([]), byte_len(bytes([])), decode_utf8(bytes([])));
    "#)
        .unwrap(),
        "bytes[0, 127, 255] 0 255 3\ntrue\n7\nbytes[] 0 \n"
    );
}

#[test]
fn split_codepoints_can_be_reassembled_without_implicit_decoding() {
    assert_eq!(
        run(r#"
        let first = bytes([226, 130]);
        let second = bytes([172]);
        let complete = first + second;
        print(decode_utf8(complete), first, second);
        print(complete == encode_utf8("€"));
    "#)
        .unwrap(),
        "€ bytes[226, 130] bytes[172]\ntrue\n"
    );
    assert!(
        run("decode_utf8(bytes([226, 130]));")
            .unwrap_err()
            .contains("invalid UTF-8 at byte 0")
    );
    assert!(
        run("decode_utf8(bytes([65, 255]));")
            .unwrap_err()
            .contains("invalid UTF-8 at byte 1")
    );
}

#[test]
fn byte_helpers_validate_types_ranges_and_dynamic_arity() {
    for source in [
        "bytes([-1]);",
        "bytes([256]);",
        "bytes([1.0]);",
        "bytes([true]);",
        "bytes(1);",
        "encode_utf8(bytes([]));",
        "decode_utf8(1);",
        "byte_len(\"text\");",
        "bytes([1])[-1];",
        "bytes([1])[1];",
        "bytes([1])[false];",
        "let b = bytes([1]); b[0] = 2;",
        "\"text\" + bytes([1]);",
        "bytes([1]) + \"text\";",
        "bytes([1]) - bytes([1]);",
        "let f = decode_utf8; f();",
        "let f = bytes; f([], []);",
    ] {
        assert!(run(source).is_err(), "accepted {source}");
    }
}

#[test]
fn builtin_bindings_check_arity_allow_shadowing_and_work_in_workers() {
    assert!(run("bytes();").unwrap_err().contains("expects 1 argument"));
    assert!(run("bytes = 1;").is_err());
    assert_eq!(
        run("fn bytes(x, y) { return x + y; } print(bytes(1, 2));").unwrap(),
        "3\n"
    );
    assert_eq!(
        run(r#"
        let convert = decode_utf8;
        parallel n in [65, 66] { print(convert(bytes([n]))); }
    "#)
        .unwrap(),
        "A\nB\n"
    );
}
