use super::Expr;
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Stmt>,
}
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
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
        parameters: Vec<String>,
        body: Box<Stmt>,
    },
    Return {
        value: Option<Expr>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Stmt,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pattern {
    Integer(i64),
    String(String),
    Boolean(bool),
    Null,
    Wildcard,
}
