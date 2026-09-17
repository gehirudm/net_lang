use super::{ParseError, Parser};
use crate::{
    ast::{Program, Stmt},
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
        if self.matches(K::Fn) {
            self.parse_function()
        } else if self.matches(K::Let) {
            let name = self
                .consume(K::Identifier, "expected variable name")?
                .lexeme;
            self.consume(K::Equal, "expected '=' after variable name")?;
            let value = self.parse_expression()?;
            self.consume(K::Semicolon, "expected ';' after variable declaration")?;
            Ok(Stmt::Let { name, value })
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
                parameters.push(
                    self.consume(K::Identifier, "expected parameter name")?
                        .lexeme,
                );
                if !self.matches(K::Comma) || self.check(K::RightParen) {
                    break;
                }
            }
        }
        self.consume(K::RightParen, "expected ')' after parameters")?;
        let body = Box::new(self.parse_block()?);
        Ok(Stmt::Function {
            name,
            parameters,
            body,
        })
    }
    fn parse_statement(&mut self) -> Result<Stmt, ParseError> {
        if self.check(K::LeftBrace) {
            self.parse_block()
        } else if self.matches(K::If) {
            self.parse_if()
        } else if self.matches(K::While) {
            let condition = self.parse_expression()?;
            let body = Box::new(self.parse_block()?);
            Ok(Stmt::While { condition, body })
        } else if self.matches(K::For) {
            let variable = self
                .consume(K::Identifier, "expected loop variable")?
                .lexeme;
            self.consume(K::In, "expected 'in' after loop variable")?;
            let iterable = self.parse_expression()?;
            let body = Box::new(self.parse_block()?);
            Ok(Stmt::For {
                variable,
                iterable,
                body,
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
