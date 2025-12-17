#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NType {
    Int,
    Float,
    Void,
    Array(Box<NType>, usize),
    Pointer(Box<NType>),
    Struct(String),
    Const(Box<NType>),
}

pub enum Value {
    Int(i32),
    Float(f32),
}

pub enum EvalError {
    TypeMismatch,
    UnsupportedOperation(String),
}

impl Value {
    pub fn eval(lhs: Value, rhs: Value, op: &str) -> Result<Value, EvalError> {
        match (lhs, rhs) {
            (Value::Int(l), Value::Int(r)) => match op {
                "+" => Ok(Value::Int(l + r)),
                "-" => Ok(Value::Int(l - r)),
                "*" => Ok(Value::Int(l * r)),
                "/" => Ok(Value::Int(l / r)),
                "%" => Ok(Value::Int(l % r)),
                "!=" => Ok(Value::Int((l != r) as i32)),
                "==" => Ok(Value::Int((l == r) as i32)),
                "<" => Ok(Value::Int((l < r) as i32)),
                "<=" => Ok(Value::Int((l <= r) as i32)),
                ">" => Ok(Value::Int((l > r) as i32)),
                ">=" => Ok(Value::Int((l >= r) as i32)),
                "||" => Ok(Value::Int(((l != 0) || (r != 0)) as i32)),
                "&&" => Ok(Value::Int(((l != 0) && (r != 0)) as i32)),
                _ => Err(EvalError::TypeMismatch),
            },
            (Value::Float(l), Value::Float(r)) => match op {
                "+" => Ok(Value::Float(l + r)),
                "-" => Ok(Value::Float(l - r)),
                "*" => Ok(Value::Float(l * r)),
                "/" => Ok(Value::Float(l / r)),
                "%" => Ok(Value::Float(l % r)),
                _ => Err(EvalError::UnsupportedOperation(op.to_string())),
            },
            _ => Err(EvalError::TypeMismatch),
        }
    }
}
