use std::collections::{HashMap, hash_map::Entry};

#[derive(Debug, Clone, Copy)]
pub(super) enum Symbol {
    Variable,
    Parameter,
    LoopVariable,
    Function { arity: usize },
    Builtin,
}

impl Symbol {
    pub fn is_mutable(self) -> bool {
        matches!(self, Self::Variable | Self::Parameter | Self::LoopVariable)
    }
}

/// Lookup walks from the innermost lexical scope to the built-in scope.
pub(super) struct Scopes {
    stack: Vec<HashMap<String, Symbol>>,
}

impl Scopes {
    pub fn new() -> Self {
        Self {
            stack: vec![HashMap::from([("print".into(), Symbol::Builtin)])],
        }
    }

    pub fn enter(&mut self) {
        self.stack.push(HashMap::new());
    }

    pub fn leave(&mut self) {
        assert!(self.stack.len() > 1, "cannot remove built-in scope");
        self.stack.pop();
    }

    /// Keep the first symbol on a duplicate, so subsequent checks are stable.
    pub fn declare(&mut self, name: &str, symbol: Symbol) -> bool {
        match self.stack.last_mut().unwrap().entry(name.into()) {
            Entry::Vacant(entry) => {
                entry.insert(symbol);
                true
            }
            Entry::Occupied(_) => false,
        }
    }

    pub fn resolve(&self, name: &str) -> Option<Symbol> {
        self.stack
            .iter()
            .rev()
            .find_map(|scope| scope.get(name))
            .copied()
    }
}
