use super::{ParseError, Parser};
use crate::{
    ast::{BinaryOp, Expr, HttpMethod, ObjectField, UnaryOp},
    lexer::{Token, TokenKind as K},
};

impl Parser {
    /// Parse one expression, leaving subsequent tokens for the caller.
    pub fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        self.parse_assignment()
    }
    /// Parse an expression and require the entire input to be consumed.
    pub fn parse_expression_complete(&mut self) -> Result<Expr, ParseError> {
        let expr = self.parse_expression()?;
        self.consume(K::Eof, "expected end of expression")?;
        Ok(expr)
    }
    fn parse_assignment(&mut self) -> Result<Expr, ParseError> {
        let target = self.parse_or()?;
        if self.matches(K::Equal) {
            if !matches!(
                target,
                Expr::Identifier(_) | Expr::Property { .. } | Expr::Index { .. }
            ) {
                return Err(Self::error_at(self.previous(), "invalid assignment target"));
            }
            let value = self.parse_assignment()?;
            Ok(Expr::Assignment {
                target: Box::new(target),
                value: Box::new(value),
            })
        } else {
            Ok(target)
        }
    }
    // Each level delegates operands to the next tighter precedence level.
    fn parse_binary(
        &mut self,
        operand: fn(&mut Self) -> Result<Expr, ParseError>,
        operators: &[(K, BinaryOp)],
    ) -> Result<Expr, ParseError> {
        let mut expr = operand(self)?;
        while let Some((_, operator)) = operators.iter().find(|(kind, _)| self.check(*kind)) {
            let operator = *operator;
            self.advance();
            expr = Expr::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(operand(self)?),
            };
        }
        Ok(expr)
    }
    fn parse_or(&mut self) -> Result<Expr, ParseError> {
        self.parse_binary(Self::parse_and, &[(K::OrOr, BinaryOp::Or)])
    }
    fn parse_and(&mut self) -> Result<Expr, ParseError> {
        self.parse_binary(Self::parse_equality, &[(K::AndAnd, BinaryOp::And)])
    }
    fn parse_equality(&mut self) -> Result<Expr, ParseError> {
        self.parse_binary(
            Self::parse_comparison,
            &[
                (K::EqualEqual, BinaryOp::Equal),
                (K::BangEqual, BinaryOp::NotEqual),
            ],
        )
    }
    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        self.parse_binary(
            Self::parse_term,
            &[
                (K::Less, BinaryOp::Less),
                (K::LessEqual, BinaryOp::LessEqual),
                (K::Greater, BinaryOp::Greater),
                (K::GreaterEqual, BinaryOp::GreaterEqual),
            ],
        )
    }
    fn parse_term(&mut self) -> Result<Expr, ParseError> {
        self.parse_binary(
            Self::parse_factor,
            &[(K::Plus, BinaryOp::Add), (K::Minus, BinaryOp::Subtract)],
        )
    }
    fn parse_factor(&mut self) -> Result<Expr, ParseError> {
        self.parse_binary(
            Self::parse_unary,
            &[
                (K::Star, BinaryOp::Multiply),
                (K::Slash, BinaryOp::Divide),
                (K::Percent, BinaryOp::Remainder),
            ],
        )
    }
    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        if self.matches(K::Bang) || self.matches(K::Minus) {
            let operator = if self.previous().kind == K::Bang {
                UnaryOp::Not
            } else {
                UnaryOp::Negate
            };
            Ok(Expr::Unary {
                operator,
                expression: Box::new(self.parse_unary()?),
            })
        } else {
            self.parse_postfix()
        }
    }
    fn parse_postfix(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary()?;
        loop {
            if self.matches(K::LeftParen) {
                let arguments = self.expression_list(K::RightParen)?;
                expr = Expr::Call {
                    callee: Box::new(expr),
                    arguments,
                };
            } else if self.matches(K::LeftBracket) {
                let index = self.parse_expression()?;
                self.consume(K::RightBracket, "expected ']' after index")?;
                expr = Expr::Index {
                    object: Box::new(expr),
                    index: Box::new(index),
                };
            } else if self.matches(K::Dot) {
                let name = self
                    .consume(K::Identifier, "expected property name after '.'")?
                    .lexeme;
                expr = Expr::Property {
                    object: Box::new(expr),
                    name,
                };
            } else {
                return Ok(expr);
            }
        }
    }
    fn expression_list(&mut self, closing: K) -> Result<Vec<Expr>, ParseError> {
        let mut values = Vec::new();
        if !self.check(closing) {
            loop {
                values.push(self.parse_expression()?);
                if !self.matches(K::Comma) || self.check(closing) {
                    break;
                }
            }
        }
        self.consume(closing, "expected closing delimiter after expression list")?;
        Ok(values)
    }
    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let token = self.advance();
        match token.kind {
            K::Get | K::Post | K::Put | K::Patch | K::Delete | K::Head => {
                self.parse_request_expression(token.kind)
            }
            K::Integer => Ok(Expr::Integer(Self::integer(&token)?)),
            K::Float => {
                let value: f64 = token
                    .lexeme
                    .parse()
                    .map_err(|_| Self::error_at(&token, "invalid float"))?;
                if !value.is_finite() {
                    return Err(Self::error_at(&token, "float is out of range"));
                }
                Ok(Expr::Float(value))
            }
            K::String => Ok(Expr::String(Self::decode_string(&token)?)),
            K::True => Ok(Expr::Boolean(true)),
            K::False => Ok(Expr::Boolean(false)),
            K::Null => Ok(Expr::Null),
            K::Identifier => Ok(Expr::Identifier(token.lexeme)),
            K::Duration => {
                let (digits, multiplier) = if let Some(n) = token.lexeme.strip_suffix("ms") {
                    (n, 1)
                } else if let Some(n) = token.lexeme.strip_suffix('s') {
                    (n, 1000)
                } else if let Some(n) = token.lexeme.strip_suffix('m') {
                    (n, 60000)
                } else {
                    return Err(Self::error_at(&token, "invalid duration"));
                };
                let value = digits
                    .parse::<u64>()
                    .ok()
                    .and_then(|n| n.checked_mul(multiplier))
                    .ok_or_else(|| Self::error_at(&token, "duration is out of range"))?;
                Ok(Expr::Duration(value))
            }
            K::LeftParen => {
                let expr = self.parse_expression()?;
                self.consume(K::RightParen, "expected ')' after expression")?;
                Ok(expr)
            }
            K::LeftBracket => Ok(Expr::Array(self.expression_list(K::RightBracket)?)),
            K::LeftBrace => self.parse_object(),
            _ => Err(Self::error_at(&token, "expected expression")),
        }
    }
    fn parse_request_expression(&mut self, kind: K) -> Result<Expr, ParseError> {
        let method = match kind {
            K::Get => HttpMethod::Get,
            K::Post => HttpMethod::Post,
            K::Put => HttpMethod::Put,
            K::Patch => HttpMethod::Patch,
            K::Delete => HttpMethod::Delete,
            K::Head => HttpMethod::Head,
            _ => unreachable!("request parser is called only for HTTP method tokens"),
        };
        // A URL consumes a full expression. The following object, if any,
        // belongs to this request. Parenthesize a request to operate on its result.
        let url = Box::new(self.parse_expression()?);
        let config = if self.matches(K::LeftBrace) {
            Some(Box::new(self.parse_object()?))
        } else {
            None
        };
        Ok(Expr::Request {
            method,
            url,
            config,
        })
    }
    // Called after the opening brace has been consumed.
    fn parse_object(&mut self) -> Result<Expr, ParseError> {
        let mut fields = Vec::new();
        if !self.check(K::RightBrace) {
            loop {
                let key = self.advance();
                let key = match key.kind {
                    K::Identifier => key.lexeme,
                    K::String => Self::decode_string(&key)?,
                    _ => {
                        return Err(Self::error_at(
                            &key,
                            "expected identifier or string object key",
                        ));
                    }
                };
                self.consume(K::Colon, "expected ':' after object key")?;
                fields.push(ObjectField {
                    key,
                    value: self.parse_expression()?,
                });
                if !self.matches(K::Comma) || self.check(K::RightBrace) {
                    break;
                }
            }
        }
        self.consume(K::RightBrace, "expected '}' after object")?;
        Ok(Expr::Object(fields))
    }
    pub(super) fn integer(token: &Token) -> Result<i64, ParseError> {
        token
            .lexeme
            .parse()
            .map_err(|_| Self::error_at(token, "integer is out of range"))
    }
    pub(super) fn decode_string(token: &Token) -> Result<String, ParseError> {
        let content = token
            .lexeme
            .strip_prefix('"')
            .and_then(|s| s.strip_suffix('"'))
            .ok_or_else(|| Self::error_at(token, "invalid string"))?;
        let mut result = String::new();
        let mut chars = content.chars();
        while let Some(ch) = chars.next() {
            result.push(if ch == '\\' {
                match chars.next() {
                    Some('n') => '\n',
                    Some('t') => '\t',
                    Some('r') => '\r',
                    Some('"') => '"',
                    Some('\\') => '\\',
                    _ => return Err(Self::error_at(token, "unsupported string escape")),
                }
            } else {
                ch
            });
        }
        Ok(result)
    }
}
