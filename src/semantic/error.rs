use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticErrorKind {
    UndefinedName,
    DuplicateDeclaration,
    ReturnOutsideFunction,
    ImmutableAssignment,
    InvalidAssignmentTarget,
    ArgumentCount,
    CapturedAssignment,
    ReturnAcrossParallel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticError {
    pub kind: SemanticErrorKind,
    pub message: String,
    /// AST scope/statement context. Source positions await spanned AST nodes.
    pub context: Vec<String>,
}

impl fmt::Display for SemanticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (in {})", self.message, self.context.join(" > "))
    }
}

impl std::error::Error for SemanticError {}
