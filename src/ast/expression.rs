#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Located {
        span: crate::source::Span,
        expression: Box<Expr>,
    },
    Connection {
        transport: Transport,
        address: Box<Expr>,
        protocol: Option<String>,
    },
    Send {
        connection: Box<Expr>,
        data: Box<Expr>,
        destination: Option<Box<Expr>>,
    },
    Receive {
        connection: Box<Expr>,
    },
    Request {
        method: HttpMethod,
        url: Box<Expr>,
        config: Option<Box<Expr>>,
    },
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Null,
    /// Duration normalized to milliseconds.
    Duration(u64),
    Identifier(String),
    Array(Vec<Expr>),
    Object(Vec<ObjectField>),
    Unary {
        operator: UnaryOp,
        expression: Box<Expr>,
    },
    Binary {
        left: Box<Expr>,
        operator: BinaryOp,
        right: Box<Expr>,
    },
    Assignment {
        target: Box<Expr>,
        value: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        arguments: Vec<Expr>,
    },
    Property {
        object: Box<Expr>,
        name: String,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
    },
}
impl Expr {
    pub fn unspanned(&self) -> &Self {
        let mut expression = self;
        while let Self::Located {
            expression: inner, ..
        } = expression
        {
            expression = inner;
        }
        expression
    }
    pub fn span(&self) -> Option<crate::source::Span> {
        match self {
            Self::Located { span, .. } => Some(*span),
            _ => None,
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectField {
    pub key: String,
    pub value: Expr,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
    Negate,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Equal,
    NotEqual,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    Tcp,
    Udp,
}
