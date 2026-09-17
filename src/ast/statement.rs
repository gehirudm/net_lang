use super::Expr;
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Stmt>,
}
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
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
