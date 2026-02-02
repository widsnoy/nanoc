use crate::{array::ArrayTree, module::StructID, r#type::NType};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i32),
    Float(f32),
    Array(ArrayTree),
    /// Struct 初始化值，按字段顺序存储
    Struct(StructID, Vec<Value>),
    StructZero(StructID),
    Pointee(String, i32),
}

pub enum EvalError {
    TypeMismatch,
    UnsupportedOperation(String),
}

impl Value {
    pub fn get_type(&self) -> NType {
        match self {
            Value::Int(_) => NType::Int,
            Value::Float(_) => NType::Float,
            Value::Array(_) => NType::Array(Box::new(NType::Void), 0),
            Value::Struct(struct_id, _) => NType::Struct(*struct_id),
            Value::StructZero(struct_id) => NType::Struct(*struct_id),
            Value::Pointee(_, _) => NType::Pointer(Box::new(NType::Void)),
        }
    }
    pub fn eval(lhs: &Value, rhs: &Value, op: &str) -> Result<Value, EvalError> {
        match (lhs, rhs) {
            (Value::Int(l), Value::Int(r)) => match op {
                "+" => Ok(Value::Int(l + r)),
                "-" => Ok(Value::Int(l - r)),
                "*" => Ok(Value::Int(l * r)),
                "/" => {
                    if *r == 0 {
                        Err(EvalError::UnsupportedOperation(
                            "Division by zero".to_string(),
                        ))
                    } else {
                        Ok(Value::Int(l / r))
                    }
                }
                "%" => {
                    if *r == 0 {
                        Err(EvalError::UnsupportedOperation(
                            "Modulo by zero".to_string(),
                        ))
                    } else {
                        Ok(Value::Int(l % r))
                    }
                }
                "!=" => Ok(Value::Int((l != r) as i32)),
                "==" => Ok(Value::Int((l == r) as i32)),
                "<" => Ok(Value::Int((l < r) as i32)),
                "<=" => Ok(Value::Int((l <= r) as i32)),
                ">" => Ok(Value::Int((l > r) as i32)),
                ">=" => Ok(Value::Int((l >= r) as i32)),
                "||" => Ok(Value::Int(((*l != 0) || (*r != 0)) as i32)),
                "&&" => Ok(Value::Int(((*l != 0) && (*r != 0)) as i32)),
                _ => Err(EvalError::TypeMismatch),
            },
            (Value::Float(l), Value::Float(r)) => match op {
                "+" => Ok(Value::Float(l + r)),
                "-" => Ok(Value::Float(l - r)),
                "*" => Ok(Value::Float(l * r)),
                "/" => Ok(Value::Float(l / r)),
                "%" => Ok(Value::Float(l % r)),
                "!=" => Ok(Value::Int((l != r) as i32)),
                "==" => Ok(Value::Int((l == r) as i32)),
                "<" => Ok(Value::Int((l < r) as i32)),
                "<=" => Ok(Value::Int((l <= r) as i32)),
                ">" => Ok(Value::Int((l > r) as i32)),
                ">=" => Ok(Value::Int((l >= r) as i32)),
                _ => Err(EvalError::UnsupportedOperation(op.to_string())),
            },
            // Pointer arithmetic: Symbol + Int
            (Value::Pointee(s, off), Value::Int(i)) => match op {
                "+" => Ok(Value::Pointee(s.to_string(), off + i)),
                "-" => Ok(Value::Pointee(s.to_string(), off - i)),
                _ => Err(EvalError::UnsupportedOperation(op.to_string())),
            },
            // Pointer arithmetic: Int + Symbol
            (Value::Int(i), Value::Pointee(s, off)) => match op {
                "+" => Ok(Value::Pointee(s.to_string(), off + i)),
                _ => Err(EvalError::UnsupportedOperation(op.to_string())),
            },
            // Pointer arithmetic: Symbol - Symbol (offset diff, only valid for same symbol)
            (Value::Pointee(s1, off1), Value::Pointee(s2, off2)) => match op {
                "-" => {
                    if s1 == s2 {
                        Ok(Value::Int(off1 - off2))
                    } else {
                        Err(EvalError::UnsupportedOperation(
                            "Pointer subtraction with different symbols".to_string(),
                        ))
                    }
                }
                "==" => Ok(Value::Int((s1 == s2 && off1 == off2) as i32)),
                "!=" => Ok(Value::Int((s1 != s2 || off1 != off2) as i32)),
                _ => Err(EvalError::UnsupportedOperation(op.to_string())),
            },
            _ => Err(EvalError::TypeMismatch),
        }
    }

    pub fn eval_unary(val: Value, op: &str) -> Result<Value, EvalError> {
        match val {
            Value::Int(v) => match op {
                "+" => Ok(Value::Int(v)),
                "-" => Ok(Value::Int(-v)),
                "!" => Ok(Value::Int((v == 0) as i32)),
                _ => Err(EvalError::UnsupportedOperation(op.to_string())),
            },
            Value::Float(v) => match op {
                "+" => Ok(Value::Float(v)),
                "-" => Ok(Value::Float(-v)),
                _ => Err(EvalError::UnsupportedOperation(op.to_string())),
            },
            _ => Err(EvalError::TypeMismatch),
        }
    }
}
