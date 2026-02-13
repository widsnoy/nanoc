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
    U8(u8),
    U32(u32),
    I64(i64),
    U64(u64),
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
    Overflow(String),
}

/// 宏：为整数类型生成二元运算的实现
macro_rules! impl_binary_ops {
    ($l:expr, $r:expr, $op:expr, $val_variant:ident, $type_name:expr) => {
        match $op {
            PLUS => {
                let (result, overflow) = $l.overflowing_add(*$r);
                if overflow {
                    Err(EvalError::Overflow(format!(
                        "{} + {} overflows {}",
                        $l, $r, $type_name
                    )))
                } else {
                    Ok(Value::$val_variant(result))
                }
            }
            MINUS => {
                let (result, overflow) = $l.overflowing_sub(*$r);
                if overflow {
                    Err(EvalError::Overflow(format!(
                        "{} - {} overflows {}",
                        $l, $r, $type_name
                    )))
                } else {
                    Ok(Value::$val_variant(result))
                }
            }
            STAR => {
                let (result, overflow) = $l.overflowing_mul(*$r);
                if overflow {
                    Err(EvalError::Overflow(format!(
                        "{} * {} overflows {}",
                        $l, $r, $type_name
                    )))
                } else {
                    Ok(Value::$val_variant(result))
                }
            }
            SLASH => {
                if *$r == 0 {
                    Err(EvalError::UnsupportedOperation(
                        "Division by zero".to_string(),
                    ))
                } else {
                    Ok(Value::$val_variant($l / $r))
                }
            }
            PERCENT => {
                if *$r == 0 {
                    Err(EvalError::UnsupportedOperation(
                        "Modulo by zero".to_string(),
                    ))
                } else {
                    Ok(Value::$val_variant($l % $r))
                }
            }
            NEQ => Ok(Value::Bool($l != $r)),
            EQEQ => Ok(Value::Bool($l == $r)),
            LT => Ok(Value::Bool($l < $r)),
            LTEQ => Ok(Value::Bool($l <= $r)),
            GT => Ok(Value::Bool($l > $r)),
            GTEQ => Ok(Value::Bool($l >= $r)),
            _ => Err(EvalError::TypeMismatch),
        }
    };
}

/// 宏：为整数类型生成一元运算的实现
macro_rules! impl_unary_ops {
    ($v:expr, $op:expr, $val_variant:ident) => {
        match $op {
            PLUS => Ok(Value::$val_variant($v)),
            MINUS => Ok(Value::$val_variant($v.wrapping_neg())),
            _ => unreachable!(),
        }
    };
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
            Value::I64(v) => Ok(Value::Bool(*v != 0)),
            Value::U32(v) => Ok(Value::Bool(*v != 0)),
            Value::U8(v) => Ok(Value::Bool(*v != 0)),
            Value::U64(v) => Ok(Value::Bool(*v != 0)),
            _ => Err(EvalError::TypeMismatch),
        }
    }

    /// 将 Value 转换为 u8
    pub fn cast_to_u8(&self) -> Result<Value, EvalError> {
        match self {
            Value::U8(v) => Ok(Value::U8(*v)),
            Value::Bool(v) => Ok(Value::U8(if *v { 1 } else { 0 })),
            _ => Err(EvalError::TypeMismatch),
        }
    }

    /// 将 Value 转换为 u32
    pub fn cast_to_u32(&self) -> Result<Value, EvalError> {
        match self {
            Value::U32(v) => Ok(Value::U32(*v)),
            Value::U8(v) => Ok(Value::U32(*v as u32)),
            Value::Bool(v) => Ok(Value::U32(if *v { 1 } else { 0 })),
            _ => Err(EvalError::TypeMismatch),
        }
    }

    /// 将 Value 转换为 i64
    pub fn cast_to_i64(&self) -> Result<Value, EvalError> {
        match self {
            Value::I64(v) => Ok(Value::I64(*v)),
            Value::I32(v) => Ok(Value::I64(*v as i64)),
            Value::I8(v) => Ok(Value::I64(*v as i64)),
            Value::Bool(v) => Ok(Value::I64(if *v { 1 } else { 0 })),
            _ => Err(EvalError::TypeMismatch),
        }
    }

    /// 将 Value 转换为 u64
    pub fn cast_to_u64(&self) -> Result<Value, EvalError> {
        match self {
            Value::U64(v) => Ok(Value::U64(*v)),
            Value::U32(v) => Ok(Value::U64(*v as u64)),
            Value::U8(v) => Ok(Value::U64(*v as u64)),
            Value::Bool(v) => Ok(Value::U64(if *v { 1 } else { 0 })),
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
            Ty::U8 => self.cast_to_u8(),
            Ty::U32 => self.cast_to_u32(),
            Ty::I64 => self.cast_to_i64(),
            Ty::U64 => self.cast_to_u64(),
            Ty::Bool => self.cast_to_bool(),
            // 对于其他类型（数组、结构体、指针），不进行转换
            _ => Ok(self.clone()),
        }
    }

    pub fn get_type(&self, module: &Module) -> Ty {
        match self {
            Value::I32(_) => Ty::I32,
            Value::I8(_) => Ty::I8,
            Value::U8(_) => Ty::U8,
            Value::U32(_) => Ty::U32,
            Value::I64(_) => Ty::I64,
            Value::U64(_) => Ty::U64,
            Value::Bool(_) => Ty::Bool,
            Value::String(_) => Ty::Pointer {
                pointee: Box::new(Ty::U8),
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
                is_const: true,
            },
        }
    }
    pub fn calc_binary_expr(
        lhs: &Value,
        rhs: &Value,
        op: SyntaxKind,
        module: &Module,
    ) -> Result<Value, EvalError> {
        use SyntaxKind::*;

        // 特殊情况：null 与 null 比较
        if matches!((lhs, rhs), (Value::Null, Value::Null)) {
            return match op {
                EQEQ => Ok(Value::Bool(true)),
                NEQ => Ok(Value::Bool(false)),
                _ => Err(EvalError::UnsupportedOperation(format!("{:?}", op))),
            };
        }

        // 获取两边的类型
        let lhs_ty = lhs.get_type(module);
        let rhs_ty = rhs.get_type(module);

        // 对于逻辑运算，转换为 bool
        if matches!(op, AMPAMP | PIPEPIPE) {
            let l = lhs.cast_to_bool()?;
            let r = rhs.cast_to_bool()?;
            return match op {
                AMPAMP => {
                    if let (Value::Bool(lv), Value::Bool(rv)) = (l, r) {
                        Ok(Value::Bool(lv && rv))
                    } else {
                        unreachable!()
                    }
                }
                PIPEPIPE => {
                    if let (Value::Bool(lv), Value::Bool(rv)) = (l, r) {
                        Ok(Value::Bool(lv || rv))
                    } else {
                        unreachable!()
                    }
                }
                _ => unreachable!(),
            };
        }

        // 对于比较运算，需要先提升到共同类型再比较
        if matches!(op, LT | GT | LTEQ | GTEQ | EQEQ | NEQ) {
            let promoted_ty =
                Ty::compute_promotion_type(&lhs_ty, &rhs_ty).ok_or(EvalError::TypeMismatch)?;

            if lhs_ty.unwrap_const() != promoted_ty || rhs_ty.unwrap_const() != promoted_ty {
                let l = lhs.cast_to_type(&promoted_ty)?;
                let r = rhs.cast_to_type(&promoted_ty)?;
                return Self::calc_binary_expr(&l, &r, op, module);
            }
            // 否则继续执行下面的同类型运算
        }

        // 对于算术运算，提升到目标类型
        if matches!(op, PLUS | MINUS | STAR | SLASH | PERCENT) {
            // 使用 Ty::compute_binary_result_type 计算目标类型
            let target_ty = Ty::compute_binary_result_type(&lhs_ty, &rhs_ty, op)
                .ok_or(EvalError::TypeMismatch)?;
            if lhs_ty.unwrap_const() != target_ty || rhs_ty.unwrap_const() != target_ty {
                let l = lhs.cast_to_type(&target_ty)?;
                let r = rhs.cast_to_type(&target_ty)?;
                return Self::calc_binary_expr(&l, &r, op, module);
            }
            // 否则继续执行下面的同类型运算
        }

        // 执行同类型运算
        match (lhs, rhs) {
            (Value::I32(l), Value::I32(r)) => impl_binary_ops!(l, r, op, I32, "i32"),
            (Value::I8(l), Value::I8(r)) => impl_binary_ops!(l, r, op, I8, "i8"),
            (Value::U8(l), Value::U8(r)) => impl_binary_ops!(l, r, op, U8, "u8"),
            (Value::U32(l), Value::U32(r)) => impl_binary_ops!(l, r, op, U32, "u32"),
            (Value::I64(l), Value::I64(r)) => impl_binary_ops!(l, r, op, I64, "i64"),
            (Value::U64(l), Value::U64(r)) => impl_binary_ops!(l, r, op, U64, "u64"),
            (Value::Bool(l), Value::Bool(r)) => match op {
                EQEQ => Ok(Value::Bool(l == r)),
                NEQ => Ok(Value::Bool(l != r)),
                _ => Err(EvalError::UnsupportedOperation(format!("{:?}", op))),
            },
            _ => Err(EvalError::TypeMismatch),
        }
    }

    /// 将 Value 转换为指定类型
    fn cast_to_type(&self, target_ty: &Ty) -> Result<Value, EvalError> {
        match target_ty.unwrap_const() {
            Ty::I32 => self.cast_to_i32(),
            Ty::I8 => self.cast_to_i8(),
            Ty::U8 => self.cast_to_u8(),
            Ty::U32 => self.cast_to_u32(),
            Ty::I64 => self.cast_to_i64(),
            Ty::U64 => self.cast_to_u64(),
            Ty::Bool => self.cast_to_bool(),
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
                Value::I32(v) => impl_unary_ops!(v, op, I32),
                Value::I8(v) => impl_unary_ops!(v, op, I8),
                Value::U8(v) => impl_unary_ops!(v, op, U8),
                Value::U32(v) => impl_unary_ops!(v, op, U32),
                Value::I64(v) => impl_unary_ops!(v, op, I64),
                Value::U64(v) => impl_unary_ops!(v, op, U64),
                Value::Bool(_) => {
                    let i32_val = val.cast_to_i32()?;
                    Self::eval_unary(i32_val, op)
                }
                _ => Err(EvalError::TypeMismatch),
            },
            _ => Err(EvalError::UnsupportedOperation(format!("{:?}", op))),
        }
    }

    pub fn get_array_size(&self) -> Option<i32> {
        match self {
            Value::I32(v) => Some(*v),
            Value::U32(v) => Some(*v as i32),
            Value::I8(v) => Some(*v as i32),
            Value::U8(v) => Some(*v as i32),
            Value::U64(v) => Some(*v as i32),
            Value::I64(v) => Some(*v as i32),
            _ => None,
        }
    }
}
