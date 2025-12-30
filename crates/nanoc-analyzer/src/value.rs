use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i32),
    Float(f32),
    Array(Vec<Value>),
    Struct(BTreeMap<String, Value>),
    Symbol(String, i32),
}

pub enum EvalError {
    TypeMismatch,
    UnsupportedOperation(String),
}

impl Value {
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
            // 指针运算: Symbol + Int
            (Value::Symbol(s, off), Value::Int(i)) => match op {
                "+" => Ok(Value::Symbol(s.to_string(), off + i)),
                "-" => Ok(Value::Symbol(s.to_string(), off - i)),
                _ => Err(EvalError::UnsupportedOperation(op.to_string())),
            },
            // 指针运算: Int + Symbol
            (Value::Int(i), Value::Symbol(s, off)) => match op {
                "+" => Ok(Value::Symbol(s.to_string(), off + i)),
                _ => Err(EvalError::UnsupportedOperation(op.to_string())),
            },
            // 指针运算: Symbol - Symbol (计算偏移量差值，仅当指向同一符号时有效)
            (Value::Symbol(s1, off1), Value::Symbol(s2, off2)) => match op {
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
