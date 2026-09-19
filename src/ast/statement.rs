use super::Expr;
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Stmt>,
}
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Break,
    Continue,
    Located {
        span: crate::source::Span,
        statement: Box<Stmt>,
    },
    Parallel {
        variable: String,
        iterable: Expr,
        body: Box<Stmt>,
    },
    Match {
        expression: Expr,
        arms: Vec<MatchArm>,
    },
    Let {
        name: String,
        annotation: Option<super::TypeAnnotation>,
        value: Expr,
    },
    Expression(Expr),
    Block(Vec<Stmt>),
    If {
        condition: Expr,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
    },
    While {
        condition: Expr,
        body: Box<Stmt>,
    },
    For {
        variable: String,
        iterable: Expr,
        body: Box<Stmt>,
    },
    Function {
        name: String,
        parameters: Vec<super::Parameter>,
        return_annotation: Option<super::TypeAnnotation>,
        body: Box<Stmt>,
    },
    Return {
        value: Option<Expr>,
    },
}

impl Stmt {
    pub fn unspanned(&self) -> &Self {
        let mut statement = self;
        while let Self::Located {
            statement: inner, ..
        } = statement
        {
            statement = inner;
        }
        statement
    }

    pub fn span(&self) -> Option<crate::source::Span> {
        match self {
            Self::Located { span, .. } => Some(*span),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Stmt,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pattern {
    Located {
        span: crate::source::Span,
        pattern: Box<Pattern>,
    },
    Integer(i64),
    String(String),
    Boolean(bool),
    Null,
    Wildcard,
}
impl Pattern {
    pub fn unspanned(&self) -> &Self {
        let mut pattern = self;
        while let Self::Located { pattern: inner, .. } = pattern {
            pattern = inner;
        }
        pattern
    }
    pub fn span(&self) -> Option<crate::source::Span> {
        match self {
            Self::Located { span, .. } => Some(*span),
            _ => None,
        }
    }
}
