use netlang::lexer::{Lexer, TokenKind::*};
#[test]
fn type_keyword_preserves_identifier_boundaries() {
    let tokens = Lexer::new("type User type_name Type")
        .unwrap()
        .tokenize()
        .unwrap();
    assert_eq!(
        tokens.iter().map(|t| t.kind).collect::<Vec<_>>(),
        vec![Type, Identifier, Identifier, Identifier, Eof]
    );
}
#[test]
fn return_arrow_is_distinct_from_match_arrow_and_separated_operators() {
    let tokens = Lexer::new("-> => - > int float bool string duration bytes")
        .unwrap()
        .tokenize()
        .unwrap();
    assert_eq!(
        tokens.iter().map(|t| t.kind).collect::<Vec<_>>(),
        vec![
            Arrow, FatArrow, Minus, Greater, Identifier, Identifier, Identifier, Identifier,
            Identifier, Identifier, Eof
        ]
    );
    assert_eq!(tokens[0].kind.name(), "ARROW");
}
#[test]
fn loop_control_keywords_preserve_identifier_boundaries_and_positions() {
    let tokens = Lexer::new("break;\ncontinue; breaker continue_loop BREAK Continue")
        .unwrap()
        .tokenize()
        .unwrap();
    assert_eq!(
        tokens.iter().map(|token| token.kind).collect::<Vec<_>>(),
        vec![
            Break, Semicolon, Continue, Semicolon, Identifier, Identifier, Identifier, Identifier,
            Eof
        ]
    );
    assert_eq!((tokens[2].line, tokens[2].column), (2, 1));
    assert_eq!(tokens[0].kind.name(), "BREAK");
    assert_eq!(tokens[2].kind.name(), "CONTINUE");
}
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
    assert_eq!(
        tokens.iter().map(|t| t.kind).collect::<Vec<_>>(),
        vec![
            Let,
            Fn,
            Return,
            If,
            Else,
            While,
            For,
            In,
            Match,
            Parallel,
            True,
            False,
            Null,
            Get,
            Post,
            Put,
            Patch,
            Delete,
            Head,
            Plus,
            Minus,
            Star,
            Slash,
            Percent,
            Equal,
            EqualEqual,
            Bang,
            BangEqual,
            Less,
            LessEqual,
            Greater,
            GreaterEqual,
            AndAnd,
            OrOr,
            LeftParen,
            RightParen,
            LeftBrace,
            RightBrace,
            LeftBracket,
            RightBracket,
            Comma,
            Colon,
            Semicolon,
            Dot,
            FatArrow,
            Identifier,
            Identifier,
            Identifier,
            Eof,
        ]
    );
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

#[test]
fn transport_keywords_are_case_sensitive_and_respect_identifier_boundaries() {
    let tokens = Lexer::new("TCP UDP SEND RECEIVE TO using\ntcp udp send receive to USING TCPstream SENDER RECEIVE_more using_protocol")
        .unwrap().tokenize().unwrap();
    assert_eq!(
        tokens[..6].iter().map(|t| t.kind).collect::<Vec<_>>(),
        vec![Tcp, Udp, Send, Receive, To, Using]
    );
    assert!(tokens[6..16].iter().all(|t| t.kind == Identifier));
    assert_eq!(tokens[16].kind, Eof);
    assert_eq!((tokens[6].line, tokens[6].column), (2, 1));
}
