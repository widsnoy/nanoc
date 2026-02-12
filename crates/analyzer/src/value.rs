use syntax::SyntaxKind;

use crate::{
    array::ArrayTree,
    module::{Module, StructID},
    r#type::Ty,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    I32(i32),
    I8(i8),
    Bool(bool),
    String(String),
    Array(ArrayTree),
    Struct(StructID, Vec<Value>),
    StructZero(StructID),
    Null,
}

#[derive(Debug)]
pub enum EvalError {
    TypeMismatch,
    UnsupportedOperation(String),
}

impl Value {
    /// 将 Value 转换为 i32（用于常量折叠）
    pub fn cast_to_i32(&self) -> Result<Value, EvalError> {
        match self {
            Value::I32(v) => Ok(Value::I32(*v)),
            Value::I8(v) => Ok(Value::I32(*v as i32)),
            Value::Bool(v) => Ok(Value::I32(if *v { 1 } else { 0 })),
            _ => Err(EvalError::TypeMismatch),
        }
    }

    /// 将 Value 转换为 i8（用于常量折叠）
    pub fn cast_to_i8(&self) -> Result<Value, EvalError> {
        match self {
            Value::I8(v) => Ok(Value::I8(*v)),
            Value::I32(v) => Ok(Value::I8(*v as i8)),
            Value::Bool(v) => Ok(Value::I8(if *v { 1 } else { 0 })),
            _ => Err(EvalError::TypeMismatch),
        }
    }

    /// 将 Value 转换为 bool（用于常量折叠）
    pub fn cast_to_bool(&self) -> Result<Value, EvalError> {
        match self {
            Value::Bool(v) => Ok(Value::Bool(*v)),
            Value::I32(v) => Ok(Value::Bool(*v != 0)),
            Value::I8(v) => Ok(Value::Bool(*v != 0)),
            _ => Err(EvalError::TypeMismatch),
        }
    }

    /// 将 Value 转换为目标类型（对应 Ty::assign_to_me_is_ok 的转换逻辑）
    /// 用于常量传播时的隐式类型转换
    pub fn convert_to(&self, target_ty: &Ty, module: &Module) -> Result<Value, EvalError> {
        let target_unwrapped = target_ty.unwrap_const();
        let source_ty = self.get_type(module);

        // 如果类型已经匹配，直接返回
        if source_ty == target_unwrapped {
            return Ok(self.clone());
        }

        // 根据目标类型进行转换
        match target_unwrapped {
            Ty::I32 => self.cast_to_i32(),
            Ty::I8 => self.cast_to_i8(),
            Ty::Bool => self.cast_to_bool(),
            // 对于其他类型（数组、结构体、指针），不进行转换
            _ => Ok(self.clone()),
        }
    }

    pub fn get_type(&self, module: &Module) -> Ty {
        match self {
            Value::I32(_) => Ty::I32,
            Value::I8(_) => Ty::I8,
            Value::Bool(_) => Ty::Bool,
            Value::String(_) => Ty::Pointer {
                pointee: Box::new(Ty::I8),
                is_const: true,
            },
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
            Value::Null => Ty::Pointer {
                pointee: Box::new(Ty::Void),
                is_const: false,
            },
        }
    }
    pub fn eval(lhs: &Value, rhs: &Value, op: SyntaxKind) -> Result<Value, EvalError> {
        use SyntaxKind::*;

        // 对于算术运算，根据类型提升规则进行转换
        if matches!(op, PLUS | MINUS | STAR | SLASH | PERCENT) {
            match (lhs, rhs) {
                // 相同类型保持不变，直接运算
                (Value::I32(_), Value::I32(_)) => {}
                (Value::I8(_), Value::I8(_)) => {}

                // i32 混合类型：提升到 i32
                (Value::I32(_), Value::I8(_) | Value::Bool(_))
                | (Value::I8(_) | Value::Bool(_), Value::I32(_)) => {
                    let l = lhs.cast_to_i32()?;
                    let r = rhs.cast_to_i32()?;
                    return Self::eval(&l, &r, op);
                }

                // i8 + bool：提升到 i8
                (Value::I8(_), Value::Bool(_)) | (Value::Bool(_), Value::I8(_)) => {
                    let l = lhs.cast_to_i8()?;
                    let r = rhs.cast_to_i8()?;
                    return Self::eval(&l, &r, op);
                }

                // bool + bool：提升到 i32
                (Value::Bool(_), Value::Bool(_)) => {
                    let l = lhs.cast_to_i32()?;
                    let r = rhs.cast_to_i32()?;
                    return Self::eval(&l, &r, op);
                }

                _ => {}
            }
        }

        // 对于逻辑运算，如果不是 bool 则转换
        if matches!(op, AMPAMP | PIPEPIPE) {
            match (lhs, rhs) {
                (Value::Bool(_), Value::Bool(_)) => {} // 已经是 bool，不转换
                (
                    Value::I32(_) | Value::I8(_) | Value::Bool(_),
                    Value::I32(_) | Value::I8(_) | Value::Bool(_),
                ) => {
                    let l = lhs.cast_to_bool()?;
                    let r = rhs.cast_to_bool()?;
                    return Self::eval(&l, &r, op);
                }
                _ => {}
            }
        }

        // 对于比较运算，提升到更高类型
        if matches!(op, LT | GT | LTEQ | GTEQ | EQEQ | NEQ) {
            match (lhs, rhs) {
                // 相同类型直接比较
                (Value::Bool(_), Value::Bool(_)) => {}
                (Value::I32(_), Value::I32(_)) => {}
                (Value::I8(_), Value::I8(_)) => {}

                // i32 混合类型：提升到 i32
                (Value::I32(_), Value::I8(_) | Value::Bool(_))
                | (Value::I8(_) | Value::Bool(_), Value::I32(_)) => {
                    let l = lhs.cast_to_i32()?;
                    let r = rhs.cast_to_i32()?;
                    return Self::eval(&l, &r, op);
                }

                // i8 + bool：提升到 i8
                (Value::I8(_), Value::Bool(_)) | (Value::Bool(_), Value::I8(_)) => {
                    let l = lhs.cast_to_i8()?;
                    let r = rhs.cast_to_i8()?;
                    return Self::eval(&l, &r, op);
                }

                _ => {}
            }
        }

        match (lhs, rhs) {
            (Value::I32(l), Value::I32(r)) => match op {
                PLUS => Ok(Value::I32(l + r)),
                MINUS => Ok(Value::I32(l - r)),
                STAR => Ok(Value::I32(l * r)),
                SLASH => {
                    if *r == 0 {
                        Err(EvalError::UnsupportedOperation(
                            "Division by zero".to_string(),
                        ))
                    } else {
                        Ok(Value::I32(l / r))
                    }
                }
                PERCENT => {
                    if *r == 0 {
                        Err(EvalError::UnsupportedOperation(
                            "Modulo by zero".to_string(),
                        ))
                    } else {
                        Ok(Value::I32(l % r))
                    }
                }
                NEQ => Ok(Value::Bool(l != r)),
                EQEQ => Ok(Value::Bool(l == r)),
                LT => Ok(Value::Bool(l < r)),
                LTEQ => Ok(Value::Bool(l <= r)),
                GT => Ok(Value::Bool(l > r)),
                GTEQ => Ok(Value::Bool(l >= r)),
                _ => Err(EvalError::TypeMismatch),
            },
            (Value::I8(l), Value::I8(r)) => match op {
                PLUS => Ok(Value::I8(l + r)),
                MINUS => Ok(Value::I8(l - r)),
                STAR => Ok(Value::I8(l * r)),
                SLASH => {
                    if *r == 0 {
                        Err(EvalError::UnsupportedOperation(
                            "Division by zero".to_string(),
                        ))
                    } else {
                        Ok(Value::I8(l / r))
                    }
                }
                PERCENT => {
                    if *r == 0 {
                        Err(EvalError::UnsupportedOperation(
                            "Modulo by zero".to_string(),
                        ))
                    } else {
                        Ok(Value::I8(l % r))
                    }
                }
                NEQ => Ok(Value::Bool(l != r)),
                EQEQ => Ok(Value::Bool(l == r)),
                LT => Ok(Value::Bool(l < r)),
                LTEQ => Ok(Value::Bool(l <= r)),
                GT => Ok(Value::Bool(l > r)),
                GTEQ => Ok(Value::Bool(l >= r)),
                _ => Err(EvalError::TypeMismatch),
            },
            (Value::Bool(l), Value::Bool(r)) => match op {
                AMPAMP => Ok(Value::Bool(*l && *r)),
                PIPEPIPE => Ok(Value::Bool(*l || *r)),
                EQEQ => Ok(Value::Bool(l == r)),
                NEQ => Ok(Value::Bool(l != r)),
                _ => Err(EvalError::UnsupportedOperation(format!("{:?}", op))),
            },
            // null 与 null 比较
            (Value::Null, Value::Null) => match op {
                EQEQ => Ok(Value::Bool(true)),
                NEQ => Ok(Value::Bool(false)),
                _ => Err(EvalError::UnsupportedOperation(format!("{:?}", op))),
            },
            _ => Err(EvalError::TypeMismatch),
        }
    }

    pub fn eval_unary(val: Value, op: SyntaxKind) -> Result<Value, EvalError> {
        use SyntaxKind::*;

        match op {
            // 逻辑非：统一到 bool
            BANG => {
                let bool_val = val.cast_to_bool()?;
                if let Value::Bool(v) = bool_val {
                    Ok(Value::Bool(!v))
                } else {
                    unreachable!()
                }
            }
            // 算术运算：bool 提升到 i32
            PLUS | MINUS => match val {
                Value::I32(v) => match op {
                    PLUS => Ok(Value::I32(v)),
                    MINUS => Ok(Value::I32(-v)),
                    _ => unreachable!(),
                },
                Value::I8(v) => match op {
                    PLUS => Ok(Value::I8(v)),
                    MINUS => Ok(Value::I8(-v)),
                    _ => unreachable!(),
                },
                Value::Bool(_) => {
                    // bool 提升到 i32
                    let i32_val = val.cast_to_i32()?;
                    Self::eval_unary(i32_val, op)
                }
                _ => Err(EvalError::TypeMismatch),
            },
            _ => Err(EvalError::UnsupportedOperation(format!("{:?}", op))),
        }
    }
}
