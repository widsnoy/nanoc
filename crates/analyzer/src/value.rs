use crate::{
    array::ArrayTree,
    module::{Module, StructID},
    r#type::Ty,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i32),
    Float(f32),
    Array(ArrayTree),
    /// Struct 初始化值，按字段顺序存储
    Struct(StructID, Vec<Value>),
    StructZero(StructID),
    Pointee(String, i32),
    /// 空指针（null）
    Null,
}

pub enum EvalError {
    TypeMismatch,
    UnsupportedOperation(String),
}

impl Value {
    pub fn get_type(&self, module: &Module) -> Ty {
        match self {
            Value::Int(_) => Ty::I32,
            Value::Float(_) => Ty::F32,
            Value::Array(_) => Ty::Array(Box::new(Ty::Void), None),
            Value::Struct(struct_id, _) => {
                let name = module
                    .get_struct_by_id(*struct_id)
                    .map(|s| s.name)
                    .unwrap_or_else(|| format!("struct#{:?}", struct_id.index));
                Ty::Struct {
                    id: *struct_id,
                    name,
                }
            }
            Value::StructZero(struct_id) => {
                let name = module
                    .get_struct_by_id(*struct_id)
                    .map(|s| s.name)
                    .unwrap_or_else(|| format!("struct#{:?}", struct_id.index));
                Ty::Struct {
                    id: *struct_id,
                    name,
                }
            }
            Value::Pointee(_, _) => Ty::Pointer {
                pointee: Box::new(Ty::Void),
                is_const: false,
            },
            Value::Null => Ty::Pointer {
                pointee: Box::new(Ty::Void),
                is_const: true,
            },
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
            // null 与 null 比较
            (Value::Null, Value::Null) => match op {
                "==" => Ok(Value::Int(1)),
                "!=" => Ok(Value::Int(0)),
                _ => Err(EvalError::UnsupportedOperation(op.to_string())),
            },
            // null 与指针比较
            (Value::Null, Value::Pointee(_, _)) | (Value::Pointee(_, _), Value::Null) => match op {
                "==" => Ok(Value::Int(0)), // null != 非空指针
                "!=" => Ok(Value::Int(1)),
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
