use super::{Analyzer, SemanticErrorKind, Symbol};
use crate::ast::{BinaryOp, Expr, TypeAnnotation, UnaryOp};
use crate::types::ResolvedType;

impl Analyzer {
    pub(super) fn check_member(&mut self, object: &Expr, name: &str) {
        if let Some(known) = self.known_type(object)
            && let Some(ResolvedType::Named(id)) = self.types.resolve(&known)
            && !self.types.definitions[id]
                .1
                .iter()
                .any(|field| field.name == name)
        {
            self.error(
                SemanticErrorKind::TypeMismatch,
                format!("type '{known}' has no field '{name}'"),
            );
        }
    }

    pub(super) fn member_type(&self, target: &Expr) -> Option<ResolvedType> {
        let name = self.known_type(target)?;
        self.types.resolve(&name)
    }
    pub(super) fn type_declaration(&mut self, name: &str, fields: &[crate::ast::TypeField]) {
        if self.scopes.depth() != 2 || self.function_depth != 0 {
            self.error(
                SemanticErrorKind::InvalidTypeDeclaration,
                "type declarations are only supported at top level",
            );
        }
        if crate::ast::PrimitiveType::from_name(name).is_some()
            || matches!(
                name,
                "array" | "object" | "function" | "null" | "connection"
            )
        {
            self.error(
                SemanticErrorKind::InvalidTypeDeclaration,
                format!("type name '{name}' is reserved"),
            );
        }
        if !self.declared_types.insert(name.into()) {
            self.error(
                SemanticErrorKind::DuplicateDeclaration,
                format!("duplicate type declaration '{name}'"),
            );
        }
        let mut names = std::collections::HashSet::new();
        for field in fields {
            if !names.insert(&field.name) {
                self.error(
                    SemanticErrorKind::DuplicateDeclaration,
                    format!("duplicate field '{}' in type '{name}'", field.name),
                );
            }
            self.resolve_type(&field.annotation);
        }
        if self.types.has_required_cycle(name) {
            self.error(SemanticErrorKind::InvalidTypeDeclaration,
                format!("type '{name}' has a cycle of required fields; recursive fields are not supported yet"));
        }
    }

    pub(super) fn constructor(&mut self, name: &str, fields: &[crate::ast::ObjectField]) {
        let definition = match self.types.resolve(name) {
            Some(ResolvedType::Named(id)) => Some(self.types.definitions[id].1.clone()),
            _ => {
                self.error(
                    SemanticErrorKind::InvalidConstructor,
                    format!("'{name}' is not a declared named type"),
                );
                None
            }
        };
        let mut names = std::collections::HashSet::new();
        for field in fields {
            self.expression(&field.value);
            if !names.insert(&field.key) {
                self.error(
                    SemanticErrorKind::InvalidConstructor,
                    format!("duplicate constructor field '{}'", field.key),
                );
            }
            if let Some(definition) = &definition {
                if let Some(expected) = definition.iter().find(|f| f.name == field.key) {
                    if let Some(ty) = self.types.resolve(&expected.annotation.name) {
                        self.check_type(&field.value, ty);
                    }
                } else {
                    self.error(
                        SemanticErrorKind::InvalidConstructor,
                        format!("type '{name}' has no field '{}'", field.key),
                    );
                }
            }
        }
        if let Some(definition) = definition {
            for field in definition {
                if !names.contains(&field.name) {
                    self.error(
                        SemanticErrorKind::InvalidConstructor,
                        format!("missing field '{}' for type '{name}'", field.name),
                    );
                }
            }
        }
    }
    pub(super) fn resolve_type(&mut self, annotation: &TypeAnnotation) -> Option<ResolvedType> {
        let ty = self.types.resolve(&annotation.name);
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

    pub(super) fn check_type(&mut self, expression: &Expr, expected: ResolvedType) {
        if let Some(actual) = self.known_type(expression)
            && actual != self.types.name(expected)
        {
            let previous = self.current_span;
            self.current_span = expression.span().or(previous);
            self.error(
                SemanticErrorKind::TypeMismatch,
                format!("expected {}, got {actual}", self.types.name(expected)),
            );
            self.current_span = previous;
        }
    }

    // Only describe results known without executing code or constraining an
    // unannotated binding. Unknown results are checked at the runtime boundary.
    fn known_type(&self, expression: &Expr) -> Option<String> {
        match expression.unspanned() {
            Expr::Property { object, name } => self.field_type(object, name),
            Expr::Index { object, index } => match index.unspanned() {
                Expr::String(name) => self.field_type(object, name),
                _ => None,
            },
            Expr::Construct { name, .. } => self
                .types
                .resolve(name)
                .map(|ty| self.types.name(ty).to_owned()),
            Expr::Integer(_) => Some("int".into()),
            Expr::Float(_) => Some("float".into()),
            Expr::Boolean(_) => Some("bool".into()),
            Expr::String(_) => Some("string".into()),
            Expr::Duration(_) => Some("duration".into()),
            Expr::Null => Some("null".into()),
            Expr::Array(_) => Some("array".into()),
            Expr::Object(_) => Some("object".into()),
            Expr::Identifier(name) => match self.scopes.resolve(name) {
                Some(
                    Symbol::Variable {
                        annotation: Some(ty),
                    }
                    | Symbol::Parameter {
                        annotation: Some(ty),
                    },
                ) => Some(self.types.name(ty).to_owned()),
                Some(Symbol::Function { .. } | Symbol::Builtin) => Some("function".into()),
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
                    Some(self.types.name(ty).to_owned())
                } else {
                    None
                }
            }
            Expr::Unary {
                operator: UnaryOp::Not,
                ..
            } => Some("bool".into()),
            Expr::Unary {
                operator: UnaryOp::Negate,
                expression,
            } => self
                .known_type(expression)
                .filter(|ty| matches!(ty.as_str(), "int" | "float")),
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
                    return Some("bool".into());
                }
                let left = self.known_type(left)?;
                let right = self.known_type(right)?;
                match (operator, left.as_str(), right.as_str()) {
                    (Add, "string", _) | (Add, _, "string") => Some("string".into()),
                    (Add, "bytes", "bytes") => Some("bytes".into()),
                    (Add | Subtract, "duration", "duration") => Some("duration".into()),
                    (_, "int", "int") => Some("int".into()),
                    (_, "int" | "float", "int" | "float") => Some("float".into()),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn field_type(&self, object: &Expr, name: &str) -> Option<String> {
        let object = self.known_type(object)?;
        let ResolvedType::Named(id) = self.types.resolve(&object)? else {
            return None;
        };
        self.types.definitions[id]
            .1
            .iter()
            .find(|field| field.name == name)
            .map(|field| field.annotation.name.clone())
    }
}
