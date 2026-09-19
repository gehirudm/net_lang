use super::*;
use std::fmt::{self, Write};

/// A small presentation tree keeps indentation separate from AST traversal.
struct Tree {
    label: String,
    children: Vec<Tree>,
}
impl Tree {
    fn new(label: impl Into<String>, children: Vec<Tree>) -> Self {
        Self {
            label: label.into(),
            children,
        }
    }
    fn leaf(label: impl Into<String>) -> Self {
        Self::new(label, vec![])
    }
    fn write(&self, output: &mut String, prefix: &str, last: bool) {
        writeln!(
            output,
            "{prefix}{}{}",
            if last { "└── " } else { "├── " },
            self.label
        )
        .unwrap();
        let prefix = format!("{prefix}{}", if last { "    " } else { "│   " });
        for (i, child) in self.children.iter().enumerate() {
            child.write(output, &prefix, i + 1 == self.children.len());
        }
    }
}
impl Program {
    pub fn pretty(&self) -> String {
        let mut output = String::from("Program\n");
        for (i, stmt) in self.statements.iter().enumerate() {
            statement(stmt).write(&mut output, "", i + 1 == self.statements.len());
        }
        output
    }
}
impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.pretty())
    }
}
fn statement(stmt: &Stmt) -> Tree {
    match stmt {
        Stmt::Break => Tree::leaf("Break"),
        Stmt::Continue => Tree::leaf("Continue"),
        Stmt::Located {
            statement: inner, ..
        } => statement(inner),
        Stmt::Let {
            name,
            annotation,
            value,
        } => {
            let suffix = annotation
                .as_ref()
                .map(|a| format!(": {}", a.name))
                .unwrap_or_default();
            Tree::new(format!("Let {name}{suffix}"), vec![expression(value)])
        }
        Stmt::Expression(expr) => Tree::new("Expression", vec![expression(expr)]),
        Stmt::Block(statements) => Tree::new("Block", statements.iter().map(statement).collect()),
        Stmt::Function {
            name,
            parameters,
            body,
        } => {
            let mut children = vec![Tree::new(
                "Parameters",
                parameters.iter().map(Tree::leaf).collect(),
            )];
            children.push(statement(body));
            Tree::new(format!("Function {name}"), children)
        }
        Stmt::Return { value } => Tree::new("Return", value.iter().map(expression).collect()),
        Stmt::If {
            condition,
            then_branch,
            else_branch,
        } => {
            let mut children = vec![
                Tree::new("Condition", vec![expression(condition)]),
                Tree::new("Then", vec![statement(then_branch)]),
            ];
            if let Some(branch) = else_branch {
                children.push(Tree::new("Else", vec![statement(branch)]));
            }
            Tree::new("If", children)
        }
        Stmt::While { condition, body } => Tree::new(
            "While",
            vec![
                Tree::new("Condition", vec![expression(condition)]),
                statement(body),
            ],
        ),
        Stmt::For {
            variable,
            iterable,
            body,
        }
        | Stmt::Parallel {
            variable,
            iterable,
            body,
        } => {
            let keyword = if matches!(stmt, Stmt::For { .. }) {
                "For"
            } else {
                "Parallel"
            };
            Tree::new(
                format!("{keyword} {variable}"),
                vec![
                    Tree::new("Iterable", vec![expression(iterable)]),
                    statement(body),
                ],
            )
        }
        Stmt::Match {
            expression: expr,
            arms,
        } => {
            let mut children = vec![Tree::new("Value", vec![expression(expr)])];
            children.extend(arms.iter().map(|arm| {
                let pattern = match arm.pattern.unspanned() {
                    Pattern::Located { .. } => unreachable!("pattern location was removed"),
                    Pattern::Integer(value) => value.to_string(),
                    Pattern::String(value) => format!("{value:?}"),
                    Pattern::Boolean(value) => value.to_string(),
                    Pattern::Null => "null".into(),
                    Pattern::Wildcard => "_".into(),
                };
                Tree::new(format!("Arm {pattern}"), vec![statement(&arm.body)])
            }));
            Tree::new("Match", children)
        }
    }
}
fn expression(expr: &Expr) -> Tree {
    match expr {
        Expr::Located {
            expression: inner, ..
        } => expression(inner),
        Expr::Connection {
            transport,
            address,
            protocol,
        } => {
            let transport = match transport {
                Transport::Tcp => "TCP",
                Transport::Udp => "UDP",
            };
            let mut children = vec![Tree::new("Address", vec![expression(address)])];
            if let Some(protocol) = protocol {
                children.push(Tree::leaf(format!("Protocol {protocol}")));
            }
            Tree::new(format!("Connection {transport}"), children)
        }
        Expr::Send {
            connection,
            data,
            destination,
        } => {
            let mut children = vec![
                Tree::new("Connection", vec![expression(connection)]),
                Tree::new("Data", vec![expression(data)]),
            ];
            if let Some(destination) = destination {
                children.push(Tree::new("Destination", vec![expression(destination)]));
            }
            Tree::new("Send", children)
        }
        Expr::Receive { connection } => Tree::new("Receive", vec![expression(connection)]),
        Expr::Integer(value) => Tree::leaf(format!("Integer {value}")),
        Expr::Float(value) => Tree::leaf(format!("Float {value}")),
        Expr::String(value) => Tree::leaf(format!("String {value:?}")),
        Expr::Boolean(value) => Tree::leaf(format!("Boolean {value}")),
        Expr::Null => Tree::leaf("Null"),
        Expr::Duration(value) => Tree::leaf(format!("Duration {value}ms")),
        Expr::Identifier(name) => Tree::leaf(format!("Identifier {name}")),
        Expr::Array(values) => Tree::new("Array", values.iter().map(expression).collect()),
        Expr::Object(fields) => Tree::new(
            "Object",
            fields
                .iter()
                .map(|field| {
                    Tree::new(
                        format!("Field {:?}", field.key),
                        vec![expression(&field.value)],
                    )
                })
                .collect(),
        ),
        Expr::Unary {
            operator,
            expression: expr,
        } => {
            let op = match operator {
                UnaryOp::Not => "!",
                UnaryOp::Negate => "-",
            };
            Tree::new(format!("Unary {op}"), vec![expression(expr)])
        }
        Expr::Binary {
            left,
            operator,
            right,
        } => {
            let op = match operator {
                BinaryOp::Add => "+",
                BinaryOp::Subtract => "-",
                BinaryOp::Multiply => "*",
                BinaryOp::Divide => "/",
                BinaryOp::Remainder => "%",
                BinaryOp::Less => "<",
                BinaryOp::LessEqual => "<=",
                BinaryOp::Greater => ">",
                BinaryOp::GreaterEqual => ">=",
                BinaryOp::Equal => "==",
                BinaryOp::NotEqual => "!=",
                BinaryOp::And => "&&",
                BinaryOp::Or => "||",
            };
            Tree::new(
                format!("Binary {op}"),
                vec![expression(left), expression(right)],
            )
        }
        Expr::Assignment { target, value } => Tree::new(
            "Assignment",
            vec![
                Tree::new("Target", vec![expression(target)]),
                Tree::new("Value", vec![expression(value)]),
            ],
        ),
        Expr::Call { callee, arguments } => Tree::new(
            "Call",
            vec![
                Tree::new("Callee", vec![expression(callee)]),
                Tree::new("Arguments", arguments.iter().map(expression).collect()),
            ],
        ),
        Expr::Property { object, name } => {
            Tree::new(format!("Property {name}"), vec![expression(object)])
        }
        Expr::Index { object, index } => Tree::new(
            "Index",
            vec![
                Tree::new("Object", vec![expression(object)]),
                Tree::new("Subscript", vec![expression(index)]),
            ],
        ),
        Expr::Request {
            method,
            url,
            config,
        } => {
            let method = match method {
                HttpMethod::Get => "GET",
                HttpMethod::Post => "POST",
                HttpMethod::Put => "PUT",
                HttpMethod::Patch => "PATCH",
                HttpMethod::Delete => "DELETE",
                HttpMethod::Head => "HEAD",
            };
            let mut children = vec![Tree::new("URL", vec![expression(url)])];
            if let Some(config) = config {
                children.push(Tree::new("Config", vec![expression(config)]));
            }
            Tree::new(format!("Request {method}"), children)
        }
    }
}
