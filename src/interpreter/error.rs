use crate::semantic::SemanticError;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeError {
    pub message: String,
    pub call_stack: Vec<String>,
    pub span: Option<crate::source::Span>,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)?;
        for name in self.call_stack.iter().rev() {
            write!(f, "\n  in function '{name}'")?;
        }
        Ok(())
    }
}
impl std::error::Error for RuntimeError {}

#[derive(Debug)]
pub enum ExecutionError {
    Semantic(Vec<SemanticError>),
    Runtime(RuntimeError),
}

impl fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Semantic(errors) => {
                for (i, error) in errors.iter().enumerate() {
                    if i != 0 {
                        writeln!(f)?;
                    }
                    write!(f, "{error}")?;
                }
                Ok(())
            }
            Self::Runtime(error) => write!(f, "{error}"),
        }
    }
}
impl std::error::Error for ExecutionError {}
