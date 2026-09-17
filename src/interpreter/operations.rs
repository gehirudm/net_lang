use super::{Interpreter, Result};
use crate::{
    ast::BinaryOp as Op,
    runtime::{Runtime, Value},
};
use std::cmp::Ordering;

impl<R: Runtime> Interpreter<'_, R> {
    pub(super) fn binary(&self, left: Value, op: Op, right: Value) -> Result<Value> {
        if matches!(op, Op::Equal | Op::NotEqual) {
            let equal = match (&left, &right) {
                (Value::Integer(a), Value::Float(b)) | (Value::Float(b), Value::Integer(a)) => {
                    compare_int_float(*a, *b) == Some(Ordering::Equal)
                }
                _ => left == right,
            };
            return Ok(Value::Boolean(if op == Op::Equal { equal } else { !equal }));
        }
        if matches!(
            op,
            Op::Less | Op::LessEqual | Op::Greater | Op::GreaterEqual
        ) {
            let order = match (&left, &right) {
                (Value::Integer(a), Value::Integer(b)) => Some(a.cmp(b)),
                (Value::Float(a), Value::Float(b)) => a.partial_cmp(b),
                (Value::Integer(a), Value::Float(b)) => compare_int_float(*a, *b),
                (Value::Float(a), Value::Integer(b)) => {
                    compare_int_float(*b, *a).map(Ordering::reverse)
                }
                (Value::String(a), Value::String(b)) => Some(a.cmp(b)),
                (Value::Duration(a), Value::Duration(b)) => Some(a.cmp(b)),
                _ => None,
            }
            .ok_or_else(|| self.error("values cannot be ordered"))?;
            return Ok(Value::Boolean(match op {
                Op::Less => order.is_lt(),
                Op::LessEqual => !order.is_gt(),
                Op::Greater => order.is_gt(),
                Op::GreaterEqual => !order.is_lt(),
                _ => unreachable!(),
            }));
        }
        if op == Op::Add && (matches!(left, Value::String(_)) || matches!(right, Value::String(_)))
        {
            if matches!(
                left,
                Value::Array(_)
                    | Value::Object(_)
                    | Value::Function(_)
                    | Value::Print
                    | Value::Bytes(_)
                    | Value::Connection(_)
            ) || matches!(
                right,
                Value::Array(_)
                    | Value::Object(_)
                    | Value::Function(_)
                    | Value::Print
                    | Value::Bytes(_)
                    | Value::Connection(_)
            ) {
                return Err(self.error("string concatenation requires scalar values"));
            }
            return Ok(Value::String(format!("{left}{right}")));
        }
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => {
                if matches!(op, Op::Divide | Op::Remainder) && b == 0 {
                    return Err(self.error("division by zero"));
                }
                let value = match op {
                    Op::Add => a.checked_add(b),
                    Op::Subtract => a.checked_sub(b),
                    Op::Multiply => a.checked_mul(b),
                    Op::Divide => a.checked_div(b),
                    Op::Remainder => a.checked_rem(b),
                    _ => None,
                }
                .ok_or_else(|| self.error("integer overflow or invalid integer operation"))?;
                Ok(Value::Integer(value))
            }
            (Value::Duration(a), Value::Duration(b)) => {
                let value = match op {
                    Op::Add => a.checked_add(b),
                    Op::Subtract => a.checked_sub(b),
                    _ => None,
                }
                .ok_or_else(|| self.error("duration overflow or invalid duration operation"))?;
                Ok(Value::Duration(value))
            }
            (Value::Float(a), Value::Float(b)) => self.float_binary(a, op, b),
            (Value::Integer(a), Value::Float(b)) => self.float_binary(self.exact_float(a)?, op, b),
            (Value::Float(a), Value::Integer(b)) => self.float_binary(a, op, self.exact_float(b)?),
            (a, b) => Err(self.error(format!(
                "invalid binary operation on {} and {}",
                a.type_name(),
                b.type_name()
            ))),
        }
    }

    fn exact_float(&self, value: i64) -> Result<f64> {
        let float = value as f64;
        if float as i128 != i128::from(value) {
            return Err(self.error("integer cannot be represented exactly as a float"));
        }
        Ok(float)
    }
    fn float_binary(&self, a: f64, op: Op, b: f64) -> Result<Value> {
        if matches!(op, Op::Divide | Op::Remainder) && b == 0.0 {
            return Err(self.error("division by zero"));
        }
        let value = match op {
            Op::Add => a + b,
            Op::Subtract => a - b,
            Op::Multiply => a * b,
            Op::Divide => a / b,
            Op::Remainder => a % b,
            _ => return Err(self.error("invalid float operation")),
        };
        if !value.is_finite() {
            return Err(self.error("float result is not finite"));
        }
        Ok(Value::Float(value))
    }
}

// Avoid rounding large integers when comparing them with floating-point values.
fn compare_int_float(integer: i64, float: f64) -> Option<Ordering> {
    if float.is_nan() {
        return None;
    }
    if float >= 9223372036854775808.0 {
        return Some(Ordering::Less);
    }
    if float < -9223372036854775808.0 {
        return Some(Ordering::Greater);
    }
    let truncated = float as i64;
    match integer.cmp(&truncated) {
        Ordering::Equal => 0.0f64.partial_cmp(&float.fract()),
        order => Some(order),
    }
}
