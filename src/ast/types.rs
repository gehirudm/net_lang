use crate::source::Span;

/// Source-level annotation. Names are resolved by semantic analysis, not parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeAnnotation {
    pub name: String,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveType {
    Int,
    Float,
    Bool,
    String,
    Duration,
    Bytes,
}

impl PrimitiveType {
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "int" => Self::Int,
            "float" => Self::Float,
            "bool" => Self::Bool,
            "string" => Self::String,
            "duration" => Self::Duration,
            "bytes" => Self::Bytes,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Int => "int",
            Self::Float => "float",
            Self::Bool => "bool",
            Self::String => "string",
            Self::Duration => "duration",
            Self::Bytes => "bytes",
        }
    }
}
