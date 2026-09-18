use netlang::{
    diagnostic,
    lexer::Lexer,
    parser::Parser,
    source::{Position, Span},
};

#[test]
fn token_spans_use_raw_utf8_bytes_and_keep_eof_empty() {
    let tokens = Lexer::new("\tlet name = \"é\\n\";\r\nname")
        .unwrap()
        .tokenize()
        .unwrap();
    assert_eq!(
        tokens[0].span(),
        Span::from_text(Position { line: 1, column: 2 }, "let")
    );
    let string = &tokens[3];
    assert_eq!(
        string.span().end.column - string.span().start.column,
        "\"é\\n\"".len()
    );
    let name = &tokens[tokens.len() - 2];
    assert_eq!(
        name.span(),
        Span::from_text(Position { line: 2, column: 1 }, "name")
    );
    let eof = tokens.last().unwrap().span();
    assert_eq!(eof.start, eof.end);
    assert_eq!(eof.start, Position { line: 2, column: 5 });
}

#[test]
fn spans_advance_across_lines_without_decoding_escapes() {
    let span = Span::from_text(Position { line: 3, column: 5 }, "é\r\n\tZ");
    assert_eq!(span.end, Position { line: 4, column: 3 });
    assert_eq!(Span::from_text(span.start, "").end, span.start);
}

#[test]
fn parser_errors_cover_the_unexpected_token_and_preserve_start_fields() {
    let error = Parser::new(
        Lexer::new("let name unexpected;")
            .unwrap()
            .tokenize()
            .unwrap(),
    )
    .unwrap()
    .parse_program()
    .unwrap_err();
    assert_eq!((error.line, error.column), (1, 10));
    assert_eq!(
        error.span,
        Span::from_text(
            Position {
                line: 1,
                column: 10
            },
            "unexpected"
        )
    );
    let output = diagnostic::render_span(
        "test.net",
        "let name unexpected;",
        error.span,
        &error.message,
    );
    assert!(output.ends_with("|          ^~~~~~~~~~\n"), "{output}");
    let error = Parser::new(Vec::new()).err().unwrap();
    assert_eq!(error.span, Span::point(1, 1));
}

#[test]
fn renderer_handles_tabs_unicode_multiline_and_invalid_ranges() {
    let source = "\téxyz\nnext";
    let span = Span::from_text(Position { line: 1, column: 2 }, "éxy");
    let output = diagnostic::render_span("test.net", source, span, "bad token");
    assert!(output.ends_with("|     ^~~\n"), "{output}");
    let span = Span::from_text(Position { line: 1, column: 2 }, "éxyz\nnext");
    let output = diagnostic::render_span("test.net", source, span, "bad token");
    assert!(output.contains("|     ^~~~\n"));
    assert!(output.ends_with("span continues to 2:5\n"));
    let output = diagnostic::render_span(
        "test.net",
        "x",
        Span {
            start: Position {
                line: 1,
                column: usize::MAX,
            },
            end: Position { line: 1, column: 1 },
        },
        "bad",
    );
    assert!(output.ends_with("|  ^\n"));
    assert!(diagnostic::render_span("test.net", "", Span::point(1, 1), "empty").ends_with("| ^\n"));
}
