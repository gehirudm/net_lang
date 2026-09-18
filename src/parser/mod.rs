mod error;
mod expression;
use crate::lexer::{Token, TokenKind};
pub use error::ParseError;

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}
impl Parser {
    /// The token stream must contain exactly one EOF token, at its end.
    pub fn new(tokens: Vec<Token>) -> Result<Self, ParseError> {
        if tokens.last().is_none_or(|t| t.kind != TokenKind::Eof)
            || tokens[..tokens.len().saturating_sub(1)]
                .iter()
                .any(|t| t.kind == TokenKind::Eof)
        {
            let (line, column) = tokens.last().map_or((1, 1), |t| (t.line, t.column));
            return Err(ParseError {
                message: "token stream must end with a single EOF token".into(),
                line,
                column,
                span: tokens
                    .last()
                    .map_or(crate::source::Span::point(line, column), Token::span),
            });
        }
        Ok(Self { tokens, current: 0 })
    }
    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }
    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }
    fn is_at_end(&self) -> bool {
        self.check(TokenKind::Eof)
    }
    fn advance(&mut self) -> Token {
        let token = self.peek().clone();
        if !self.is_at_end() {
            self.current += 1;
        }
        token
    }
    fn check(&self, kind: TokenKind) -> bool {
        self.peek().kind == kind
    }
    fn matches(&mut self, kind: TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }
    fn consume(&mut self, kind: TokenKind, message: &str) -> Result<Token, ParseError> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            Err(self.error(message))
        }
    }
    fn error(&self, message: &str) -> ParseError {
        Self::error_at(self.peek(), message)
    }
    fn error_at(token: &Token, message: &str) -> ParseError {
        ParseError {
            message: message.into(),
            line: token.line,
            column: token.column,
            span: token.span(),
        }
    }
}
mod statement;
