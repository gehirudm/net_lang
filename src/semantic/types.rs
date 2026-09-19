use super::{Analyzer, SemanticErrorKind, Symbol};
use crate::ast::{BinaryOp, Expr, PrimitiveType, TypeAnnotation, UnaryOp};

impl Analyzer {
    pub(super) fn resolve_type(&mut self, annotation: &TypeAnnotation) -> Option<PrimitiveType> {
        let ty = PrimitiveType::from_name(&annotation.name);
        if ty.is_none() {
            let previous = self.current_span;
            self.current_span = annotation.span.or(previous);
            self.error(
                SemanticErrorKind::UnknownType,
                format!("unknown type '{}'", annotation.name),
            );
            self.current_span = previous;
        }
        ty
    }

    pub(super) fn check_type(&mut self, expression: &Expr, expected: PrimitiveType) {
        if let Some(actual) = self.known_type(expression)
            && actual != expected.name()
        {
            let previous = self.current_span;
            self.current_span = expression.span().or(previous);
            self.error(
                SemanticErrorKind::TypeMismatch,
                format!("expected {}, got {actual}", expected.name()),
            );
            self.current_span = previous;
        }
    }

    // Only describe results known without executing code or constraining an
    // unannotated binding. Unknown results are checked at the runtime boundary.
    fn known_type(&self, expression: &Expr) -> Option<&'static str> {
        match expression.unspanned() {
            Expr::Integer(_) => Some("int"),
            Expr::Float(_) => Some("float"),
            Expr::Boolean(_) => Some("bool"),
            Expr::String(_) => Some("string"),
            Expr::Duration(_) => Some("duration"),
            Expr::Null => Some("null"),
            Expr::Array(_) => Some("array"),
            Expr::Object(_) => Some("object"),
            Expr::Identifier(name) => match self.scopes.resolve(name) {
                Some(
                    Symbol::Variable {
                        annotation: Some(ty),
                    }
                    | Symbol::Parameter {
                        annotation: Some(ty),
                    },
                ) => Some(ty.name()),
                Some(Symbol::Function { .. } | Symbol::Builtin) => Some("function"),
                _ => None,
            },
            Expr::Assignment { value, .. } => self.known_type(value),
            Expr::Call { callee, .. } => {
                if let Expr::Identifier(name) = callee.unspanned()
                    && let Some(Symbol::Function {
                        return_type: Some(ty),
                        ..
                    }) = self.scopes.resolve(name)
                {
                    Some(ty.name())
                } else {
                    None
                }
            }
            Expr::Unary {
                operator: UnaryOp::Not,
                ..
            } => Some("bool"),
            Expr::Unary {
                operator: UnaryOp::Negate,
                expression,
            } => self
                .known_type(expression)
                .filter(|ty| matches!(*ty, "int" | "float")),
            Expr::Binary {
                left,
                operator,
                right,
            } => {
                use BinaryOp::*;
                if matches!(
                    operator,
                    Equal | NotEqual | Less | LessEqual | Greater | GreaterEqual | And | Or
                ) {
                    return Some("bool");
                }
                let left = self.known_type(left)?;
                let right = self.known_type(right)?;
                match (operator, left, right) {
                    (Add, "string", _) | (Add, _, "string") => Some("string"),
                    (Add, "bytes", "bytes") => Some("bytes"),
                    (Add | Subtract, "duration", "duration") => Some("duration"),
                    (_, "int", "int") => Some("int"),
                    (_, "int" | "float", "int" | "float") => Some("float"),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}
