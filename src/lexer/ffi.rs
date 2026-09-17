use super::{Token, TokenKind};
use std::{
    ffi::{c_char, c_int, c_void},
    fmt,
    ptr::NonNull,
};

#[repr(C)]
struct NetToken {
    kind: c_int,
    lexeme: *const c_char,
    length: usize,
    line: usize,
    column: usize,
}
unsafe extern "C" {
    fn netlang_lexer_create(source: *const c_char, length: usize) -> *mut c_void;
    fn netlang_lexer_next(lexer: *mut c_void, token: *mut NetToken) -> c_int;
    fn netlang_lexer_destroy(lexer: *mut c_void);
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexerError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}
impl fmt::Display for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at {}:{}", self.message, self.line, self.column)
    }
}
impl std::error::Error for LexerError {}

pub struct Lexer {
    raw: NonNull<c_void>,
    terminal: Option<Result<Token, LexerError>>,
}
impl Lexer {
    pub fn new(source: &str) -> Result<Self, LexerError> {
        // The bridge copies these bytes before returning; no Rust borrow is retained.
        let raw = unsafe { netlang_lexer_create(source.as_ptr().cast(), source.len()) };
        let raw = NonNull::new(raw).ok_or_else(|| LexerError {
            message: "could not create lexer (source may be too large)".into(),
            line: 1,
            column: 1,
        })?;
        Ok(Self {
            raw,
            terminal: None,
        })
    }
    pub fn next_token(&mut self) -> Result<Token, LexerError> {
        if let Some(result) = &self.terminal {
            return result.clone();
        }
        let mut token = NetToken {
            kind: 0,
            lexeme: std::ptr::null(),
            length: 0,
            line: 0,
            column: 0,
        };
        // raw owns a live scanner; token is a valid writable bridge struct.
        unsafe {
            netlang_lexer_next(self.raw.as_ptr(), &mut token);
        }
        let result = if let Some(kind) = TokenKind::from_raw(token.kind) {
            // The scanner text remains valid until the next call. Copy it now.
            let bytes =
                unsafe { std::slice::from_raw_parts(token.lexeme.cast::<u8>(), token.length) };
            Ok(Token {
                kind,
                lexeme: String::from_utf8_lossy(bytes).into_owned(),
                line: token.line,
                column: token.column,
            })
        } else {
            Err(LexerError {
                message: match token.kind {
                    -2 => "unterminated block comment",
                    -3 => "unsupported string escape",
                    -4 => "unterminated string",
                    _ => "unexpected character",
                }
                .into(),
                line: token.line,
                column: token.column,
            })
        };
        if token.kind <= 0 {
            self.terminal = Some(result.clone());
        }
        result
    }
    pub fn tokenize(mut self) -> Result<Vec<Token>, LexerError> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token()?;
            let done = token.kind == TokenKind::Eof;
            tokens.push(token);
            if done {
                return Ok(tokens);
            }
        }
    }
}
impl Drop for Lexer {
    fn drop(&mut self) {
        // This wrapper uniquely owns the scanner and destroys it exactly once.
        unsafe {
            netlang_lexer_destroy(self.raw.as_ptr());
        }
    }
}
