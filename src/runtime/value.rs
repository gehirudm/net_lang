use std::{collections::BTreeMap, fmt};

/// Function handles are local to one interpreter execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FunctionId(pub(crate) usize);

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Null,
    Duration(u64),
    Array(Vec<Value>),
    Object(BTreeMap<String, Value>),
    Function(FunctionId),
    Print,
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Integer(_) => "integer",
            Self::Float(_) => "float",
            Self::String(_) => "string",
            Self::Boolean(_) => "boolean",
            Self::Null => "null",
            Self::Duration(_) => "duration",
            Self::Array(_) => "array",
            Self::Object(_) => "object",
            Self::Function(_) | Self::Print => "function",
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Integer(v) => write!(f, "{v}"),
            Self::Float(v) => write!(f, "{v}"),
            Self::String(v) => f.write_str(v),
            Self::Boolean(v) => write!(f, "{v}"),
            Self::Null => f.write_str("null"),
            Self::Duration(v) => write!(f, "{v}ms"),
            Self::Function(_) | Self::Print => f.write_str("<function>"),
            Self::Array(values) => {
                f.write_str("[")?;
                for (i, value) in values.iter().enumerate() {
                    if i != 0 {
                        f.write_str(", ")?;
                    }
                    nested(value, f)?;
                }
                f.write_str("]")
            }
            Self::Object(fields) => {
                f.write_str("{")?;
                for (i, (key, value)) in fields.iter().enumerate() {
                    if i != 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{key:?}: ")?;
                    nested(value, f)?;
                }
                f.write_str("}")
            }
        }
    }
}

fn nested(value: &Value, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if let Value::String(text) = value {
        write!(f, "{text:?}")
    } else {
        write!(f, "{value}")
    }
}
