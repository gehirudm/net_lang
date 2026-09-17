use super::{Environment, Interpreter, Result};
use crate::{
    ast::{BinaryOp, Expr, UnaryOp},
    runtime::{Runtime, Value},
};

enum Selector {
    Property(String),
    Index(Value),
}

impl<R: Runtime> Interpreter<'_, R> {
    pub(super) fn expression(&mut self, expr: &Expr, env: &Environment) -> Result<Value> {
        self.tick()?;
        if self.expression_depth >= self.limits.expression_depth {
            return Err(self.error("expression depth limit exceeded"));
        }
        self.expression_depth += 1;
        let result = self.expression_inner(expr, env);
        self.expression_depth -= 1;
        result
    }

    fn expression_inner(&mut self, expr: &Expr, env: &Environment) -> Result<Value> {
        Ok(match expr {
            Expr::Integer(v) => Value::Integer(*v),
            Expr::Float(v) => Value::Float(*v),
            Expr::String(v) => Value::String(v.clone()),
            Expr::Boolean(v) => Value::Boolean(*v),
            Expr::Null => Value::Null,
            Expr::Duration(v) => Value::Duration(*v),
            Expr::Identifier(name) => self.read(self.resolve(env, name)?)?,
            Expr::Array(values) => Value::Array(
                values
                    .iter()
                    .map(|v| self.expression(v, env))
                    .collect::<Result<_>>()?,
            ),
            Expr::Object(fields) => {
                let mut values = std::collections::BTreeMap::new();
                for field in fields {
                    values.insert(field.key.clone(), self.expression(&field.value, env)?);
                }
                Value::Object(values)
            }
            Expr::Unary {
                operator,
                expression,
            } => {
                let value = self.expression(expression, env)?;
                match (operator, value) {
                    (UnaryOp::Not, Value::Boolean(v)) => Value::Boolean(!v),
                    (UnaryOp::Negate, Value::Integer(v)) => Value::Integer(
                        v.checked_neg()
                            .ok_or_else(|| self.error("integer overflow"))?,
                    ),
                    (UnaryOp::Negate, Value::Float(v)) => Value::Float(-v),
                    (_, v) => {
                        return Err(
                            self.error(format!("invalid unary operation on {}", v.type_name()))
                        );
                    }
                }
            }
            Expr::Binary {
                left,
                operator,
                right,
            } => {
                let left = self.expression(left, env)?;
                if matches!(operator, BinaryOp::And | BinaryOp::Or) {
                    let left = self.boolean(left)?;
                    if (*operator == BinaryOp::And && !left) || (*operator == BinaryOp::Or && left)
                    {
                        Value::Boolean(left)
                    } else {
                        let right = self.expression(right, env)?;
                        Value::Boolean(self.boolean(right)?)
                    }
                } else {
                    let right = self.expression(right, env)?;
                    self.binary(left, *operator, right)?
                }
            }
            Expr::Call { callee, arguments } => {
                let callee = self.expression(callee, env)?;
                let arguments = arguments
                    .iter()
                    .map(|a| self.expression(a, env))
                    .collect::<Result<_>>()?;
                self.call(callee, arguments)?
            }
            Expr::Property { object, name } => {
                let object = self.expression(object, env)?;
                self.get(&object, &Selector::Property(name.clone()))
                    .map_err(|m| self.error(m))?
                    .clone()
            }
            Expr::Index { object, index } => {
                let object = self.expression(object, env)?;
                let index = self.expression(index, env)?;
                self.get(&object, &Selector::Index(index))
                    .map_err(|m| self.error(m))?
                    .clone()
            }
            Expr::Assignment { target, value } => {
                let mut selectors = Vec::new();
                let id = self.target(target, env, &mut selectors)?;
                if !self.bindings[id].mutable {
                    return Err(self.error("cannot assign to a function binding"));
                }
                let value = self.expression(value, env)?;
                if selectors.is_empty() {
                    self.bindings[id].value = Some(value.clone());
                } else {
                    let mut root = self.read(id)?;
                    Self::set(&mut root, &selectors, value.clone()).map_err(|m| self.error(m))?;
                    self.bindings[id].value = Some(root);
                }
                value
            }
            Expr::Request {
                method,
                url,
                config,
            } => {
                let url = self.expression(url, env)?;
                let Value::String(url) = url else {
                    return Err(self.error("request URL must be a string"));
                };
                let config = match config {
                    Some(c) => self.expression(c, env)?,
                    None => Value::Object(Default::default()),
                };
                self.runtime
                    .request(*method, &url, &config)
                    .map_err(|message| self.error(message))?
            }
        })
    }

    fn target(
        &mut self,
        expr: &Expr,
        env: &Environment,
        selectors: &mut Vec<Selector>,
    ) -> Result<usize> {
        self.tick()?;
        match expr {
            Expr::Identifier(name) => self.resolve(env, name),
            Expr::Property { object, name } => {
                let root = self.target(object, env, selectors)?;
                selectors.push(Selector::Property(name.clone()));
                Ok(root)
            }
            Expr::Index { object, index } => {
                let root = self.target(object, env, selectors)?;
                selectors.push(Selector::Index(self.expression(index, env)?));
                Ok(root)
            }
            _ => Err(self.error("assignment must be rooted in a mutable variable")),
        }
    }

    fn get<'a>(
        &self,
        value: &'a Value,
        selector: &Selector,
    ) -> std::result::Result<&'a Value, String> {
        match (value, selector) {
            (
                Value::Object(fields),
                Selector::Property(key) | Selector::Index(Value::String(key)),
            ) => fields
                .get(key)
                .ok_or_else(|| format!("object has no field '{key}'")),
            (Value::Array(values), Selector::Index(Value::Integer(index))) => {
                let index = usize::try_from(*index)
                    .map_err(|_| "array index must be nonnegative".to_string())?;
                values.get(index).ok_or_else(|| {
                    format!(
                        "array index {index} is out of bounds (length {})",
                        values.len()
                    )
                })
            }
            _ => Err(format!(
                "invalid property/index access on {}",
                value.type_name()
            )),
        }
    }

    fn set(
        root: &mut Value,
        selectors: &[Selector],
        value: Value,
    ) -> std::result::Result<(), String> {
        let Some((selector, rest)) = selectors.split_first() else {
            *root = value;
            return Ok(());
        };
        match (root, selector) {
            (
                Value::Object(fields),
                Selector::Property(key) | Selector::Index(Value::String(key)),
            ) => {
                if rest.is_empty() {
                    fields.insert(key.clone(), value);
                    Ok(())
                } else {
                    let child = fields
                        .get_mut(key)
                        .ok_or_else(|| format!("object has no field '{key}'"))?;
                    Self::set(child, rest, value)
                }
            }
            (Value::Array(values), Selector::Index(Value::Integer(index))) => {
                let index = usize::try_from(*index)
                    .map_err(|_| "array index must be nonnegative".to_string())?;
                let length = values.len();
                let child = values.get_mut(index).ok_or_else(|| {
                    format!("array index {index} is out of bounds (length {length})")
                })?;
                Self::set(child, rest, value)
            }
            _ => Err("invalid assignment property/index".into()),
        }
    }
}
