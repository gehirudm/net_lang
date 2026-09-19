use super::{ParseError, Parser};
use crate::{
    ast::{MatchArm, Parameter, Pattern, Program, Stmt, TypeAnnotation},
    lexer::TokenKind as K,
};

impl Parser {
    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut statements = Vec::new();
        while !self.is_at_end() {
            statements.push(self.parse_declaration()?);
        }
        Ok(Program { statements })
    }
    fn parse_declaration(&mut self) -> Result<Stmt, ParseError> {
        let start = self.peek().span().start;
        let statement = self.parse_declaration_inner()?;
        if self.spans {
            Ok(Stmt::Located {
                span: crate::source::Span {
                    start,
                    end: self.previous().span().end,
                },
                statement: Box::new(statement),
            })
        } else {
            Ok(statement)
        }
    }
    fn parse_declaration_inner(&mut self) -> Result<Stmt, ParseError> {
        if self.matches(K::Fn) {
            self.parse_function()
        } else if self.matches(K::Let) {
            let name = self
                .consume(K::Identifier, "expected variable name")?
                .lexeme;
            let annotation = self.parse_annotation()?;
            self.consume(K::Equal, "expected '=' after variable name")?;
            let value = self.parse_expression()?;
            self.consume(K::Semicolon, "expected ';' after variable declaration")?;
            Ok(Stmt::Let {
                name,
                annotation,
                value,
            })
        } else {
            self.parse_statement()
        }
    }
    fn parse_function(&mut self) -> Result<Stmt, ParseError> {
        let name = self
            .consume(K::Identifier, "expected function name")?
            .lexeme;
        self.consume(K::LeftParen, "expected '(' after function name")?;
        let mut parameters = Vec::new();
        if !self.check(K::RightParen) {
            loop {
                let name = self
                    .consume(K::Identifier, "expected parameter name")?
                    .lexeme;
                let annotation = self.parse_annotation()?;
                parameters.push(Parameter { name, annotation });
                if !self.matches(K::Comma) || self.check(K::RightParen) {
                    break;
                }
            }
        }
        self.consume(K::RightParen, "expected ')' after parameters")?;
        let return_annotation = if self.matches(K::Arrow) {
            Some(self.parse_type_name()?)
        } else {
            None
        };
        let body = Box::new(self.parse_block()?);
        Ok(Stmt::Function {
            name,
            parameters,
            return_annotation,
            body,
        })
    }
    fn parse_annotation(&mut self) -> Result<Option<TypeAnnotation>, ParseError> {
        if !self.matches(K::Colon) {
            return Ok(None);
        }
        Ok(Some(self.parse_type_name()?))
    }
    fn parse_type_name(&mut self) -> Result<TypeAnnotation, ParseError> {
        let token = self.consume(K::Identifier, "expected type name")?;
        Ok(TypeAnnotation {
            span: self.spans.then(|| token.span()),
            name: token.lexeme,
        })
    }
    fn parse_statement(&mut self) -> Result<Stmt, ParseError> {
        if self.matches(K::Break) || self.matches(K::Continue) {
            let is_break = self.previous().kind == K::Break;
            self.consume(K::Semicolon, "expected ';' after loop control statement")?;
            Ok(if is_break {
                Stmt::Break
            } else {
                Stmt::Continue
            })
        } else if self.check(K::LeftBrace) {
            self.parse_block()
        } else if self.matches(K::If) {
            self.parse_if()
        } else if self.matches(K::While) {
            let condition = self.parse_expression()?;
            let body = Box::new(self.parse_block()?);
            Ok(Stmt::While { condition, body })
        } else if self.matches(K::Match) {
            self.parse_match()
        } else if self.matches(K::For) || self.matches(K::Parallel) {
            let parallel = self.previous().kind == K::Parallel;
            let variable = self
                .consume(K::Identifier, "expected loop variable")?
                .lexeme;
            self.consume(K::In, "expected 'in' after loop variable")?;
            let iterable = self.parse_expression()?;
            let body = Box::new(self.parse_block()?);
            Ok(if parallel {
                Stmt::Parallel {
                    variable,
                    iterable,
                    body,
                }
            } else {
                Stmt::For {
                    variable,
                    iterable,
                    body,
                }
            })
        } else if self.matches(K::Return) {
            let value = if self.check(K::Semicolon) {
                None
            } else {
                Some(self.parse_expression()?)
            };
            self.consume(K::Semicolon, "expected ';' after return")?;
            Ok(Stmt::Return { value })
        } else {
            let expression = self.parse_expression()?;
            self.consume(K::Semicolon, "expected ';' after expression")?;
            Ok(Stmt::Expression(expression))
        }
    }
    fn parse_match(&mut self) -> Result<Stmt, ParseError> {
        let expression = self.parse_expression()?;
        self.consume(K::LeftBrace, "expected '{' before match arms")?;
        let mut arms = Vec::new();
        while !self.check(K::RightBrace) && !self.is_at_end() {
            let token = self.advance();
            let pattern = match token.kind {
                K::Integer => Pattern::Integer(Self::integer(&token)?),
                K::String => Pattern::String(Self::decode_string(&token)?),
                K::True => Pattern::Boolean(true),
                K::False => Pattern::Boolean(false),
                K::Null => Pattern::Null,
                K::Identifier if token.lexeme == "_" => Pattern::Wildcard,
                _ => return Err(Self::error_at(&token, "expected literal pattern or '_'")),
            };
            let pattern = if self.spans {
                Pattern::Located {
                    span: token.span(),
                    pattern: Box::new(pattern),
                }
            } else {
                pattern
            };
            self.consume(K::FatArrow, "expected '=>' after match pattern")?;
            arms.push(MatchArm {
                pattern,
                body: self.parse_block()?,
            });
        }
        self.consume(K::RightBrace, "expected '}' after match arms")?;
        Ok(Stmt::Match { expression, arms })
    }
    fn parse_block(&mut self) -> Result<Stmt, ParseError> {
        self.consume(K::LeftBrace, "expected '{' before block")?;
        let mut statements = Vec::new();
        while !self.check(K::RightBrace) && !self.is_at_end() {
            statements.push(self.parse_declaration()?);
        }
        self.consume(K::RightBrace, "expected '}' after block")?;
        Ok(Stmt::Block(statements))
    }
    // The leading 'if' has already been consumed, including for else-if.
    fn parse_if(&mut self) -> Result<Stmt, ParseError> {
        let condition = self.parse_expression()?;
        let then_branch = Box::new(self.parse_block()?);
        let else_branch = if self.matches(K::Else) {
            Some(Box::new(if self.matches(K::If) {
                self.parse_if()?
            } else {
                self.parse_block()?
            }))
        } else {
            None
        };
        Ok(Stmt::If {
            condition,
            then_branch,
            else_branch,
        })
    }
}
