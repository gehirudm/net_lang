use netlang::lexer::{Lexer, TokenKind::*};
#[test]
fn literals_comments_and_positions() {
    let tokens = Lexer::new("// hi\nlet x = [42, 3.14, 100ms, 5s, 2m, \"a\\n\\t\\r\\\"\\\\\", true, false, null]; /* hi\n */").unwrap().tokenize().unwrap();
    assert_eq!(
        tokens.iter().map(|t| t.kind).collect::<Vec<_>>(),
        vec![
            Let,
            Identifier,
            Equal,
            LeftBracket,
            Integer,
            Comma,
            Float,
            Comma,
            Duration,
            Comma,
            Duration,
            Comma,
            Duration,
            Comma,
            String,
            Comma,
            True,
            Comma,
            False,
            Comma,
            Null,
            RightBracket,
            Semicolon,
            Eof
        ]
    );
    assert_eq!((tokens[0].line, tokens[0].column), (2, 1));
    assert_eq!((tokens[1].line, tokens[1].column), (2, 5));
    assert_eq!(tokens[14].lexeme, "\"a\\n\\t\\r\\\"\\\\\"");
}
#[test]
fn keywords_and_operators() {
    let tokens = Lexer::new("let fn return if else while for in match parallel true false null GET POST PUT PATCH DELETE HEAD + - * / % = == ! != < <= > >= && || ( ) { } [ ] , : ; . => _ getter GETfoo").unwrap().tokenize().unwrap();
    assert_eq!(tokens.len(), 49);
    assert_eq!(tokens[18].kind, Head);
    assert_eq!(tokens[44].kind, FatArrow);
    assert!(tokens[45..48].iter().all(|t| t.kind == Identifier));
}
#[test]
fn errors_are_located_and_terminal() {
    for (source, message) in [
        ("@", "unexpected character"),
        ("/* x", "unterminated block comment"),
        ("\"abc", "unterminated string"),
        ("\"\\q\"", "unsupported string escape"),
        ("\0", "unexpected character"),
    ] {
        let mut lexer = Lexer::new(source).unwrap();
        let error = lexer.next_token().unwrap_err();
        assert_eq!(error.message, message);
        assert_eq!((error.line, error.column), (1, 1));
        assert_eq!(lexer.next_token().unwrap_err(), error);
    }
}
#[test]
fn scanners_are_independent_and_eof_is_stable() {
    let mut a = Lexer::new("one\ntwo").unwrap();
    let mut b = Lexer::new("42").unwrap();
    assert_eq!(a.next_token().unwrap().lexeme, "one");
    assert_eq!(b.next_token().unwrap().kind, Integer);
    assert_eq!(a.next_token().unwrap().line, 2);
    assert_eq!(b.next_token().unwrap(), b.next_token().unwrap());
}
