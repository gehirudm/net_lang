//! Shared, per-program type names. Definitions are hoisted only at top level.
use crate::ast::{PrimitiveType, Program, Stmt, TypeField};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResolvedType {
    Primitive(PrimitiveType),
    Named(usize),
}

#[derive(Clone)]
pub(crate) struct TypeRegistry {
    pub identity: Arc<()>,
    pub definitions: Vec<(String, Vec<TypeField>)>,
}

impl TypeRegistry {
    pub fn has_required_cycle(&self, name: &str) -> bool {
        let Some(ResolvedType::Named(start)) = self.resolve(name) else {
            return false;
        };
        let mut pending = vec![start];
        let mut seen = std::collections::HashSet::new();
        while let Some(id) = pending.pop() {
            if !seen.insert(id) {
                continue;
            }
            for field in &self.definitions[id].1 {
                if let Some(ResolvedType::Named(next)) = self.resolve(&field.annotation.name) {
                    if next == start {
                        return true;
                    }
                    pending.push(next);
                }
            }
        }
        false
    }
    pub fn new(program: &Program) -> Self {
        let mut definitions = Vec::new();
        for statement in &program.statements {
            if let Stmt::Type { name, fields } = statement.unspanned()
                && !definitions.iter().any(|(existing, _)| existing == name)
            {
                definitions.push((name.clone(), fields.clone()));
            }
        }
        Self {
            identity: Arc::new(()),
            definitions,
        }
    }
    pub fn resolve(&self, name: &str) -> Option<ResolvedType> {
        PrimitiveType::from_name(name)
            .map(ResolvedType::Primitive)
            .or_else(|| {
                self.definitions
                    .iter()
                    .position(|(n, _)| n == name)
                    .map(ResolvedType::Named)
            })
    }
    pub fn name(&self, ty: ResolvedType) -> &str {
        match ty {
            ResolvedType::Primitive(ty) => ty.name(),
            ResolvedType::Named(id) => &self.definitions[id].0,
        }
    }
}
