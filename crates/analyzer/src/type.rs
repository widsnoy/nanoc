use crate::{module::StructID, value::Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NType {
    Int,
    Float,
    Void,
    Array(Box<NType>, i32),
    Pointer { pointee: Box<NType>, is_const: bool },
    Struct(StructID),
    Const(Box<NType>),
}

impl NType {
    /// 检查是否为数组类型（包括 Const(Array(...))）
    pub fn is_array(&self) -> bool {
        match self {
            Self::Array(_, _) => true,
            Self::Const(inner) => inner.is_array(),
            _ => false,
        }
    }

    /// 检查是否为指针类型（包括 Const(Pointer {...})）
    pub fn is_pointer(&self) -> bool {
        self.pointer_inner().is_some()
    }

    /// 检查是否为结构体类型
    pub fn is_struct(&self) -> bool {
        self.as_struct_id().is_some()
    }

    /// 提取指针类型的内部类型，处理 Pointer {...} 和 Const(Pointer {...}) 两种情况
    pub fn pointer_inner(&self) -> Option<&NType> {
        match self {
            Self::Pointer { pointee, .. } => Some(pointee.as_ref()),
            Self::Const(inner) => {
                if let Self::Pointer { pointee, .. } = inner.as_ref() {
                    Some(pointee.as_ref())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn is_const(&self) -> bool {
        match self {
            Self::Array(inner, _) => inner.is_const(),
            Self::Const(_) => true,
            Self::Pointer { is_const, .. } => *is_const,
            _ => false,
        }
    }

    /// 去掉 Const 包装，返回内部类型
    pub fn unwrap_const(&self) -> NType {
        match self {
            Self::Const(inner) => inner.as_ref().clone(),
            _ => self.clone(),
        }
    }

    /// 提取 struct ID（处理 Struct 和 Const(Struct) 两种情况）
    pub fn as_struct_id(&self) -> Option<StructID> {
        match self {
            Self::Struct(id) => Some(*id),
            Self::Const(inner) => inner.as_struct_id(),
            _ => None,
        }
    }

    /// 提取 struct 指针的 struct ID（处理 Pointer{Struct} 和 Const(Pointer{Struct})）
    pub fn as_struct_pointer_id(&self) -> Option<StructID> {
        match self {
            Self::Pointer { pointee, .. } => pointee.as_struct_id(),
            Self::Const(inner) => inner.as_struct_pointer_id(),
            _ => None,
        }
    }

    /// 返回标量零值（int / float）
    pub fn const_zero(&self) -> Value {
        match self {
            NType::Int => Value::Int(0),
            NType::Float => Value::Float(0.0),
            NType::Void => Value::Int(0),
            NType::Array(ntype, _) => ntype.const_zero(),
            NType::Pointer { .. } => Value::Int(0), // null pointer
            NType::Struct(id) => Value::StructZero(*id),
            NType::Const(ntype) => ntype.const_zero(),
        }
    }

    /// 判断两种类型是否兼容
    pub fn assign_to_me_is_ok(&self, other: &Self) -> bool {
        match (self, other) {
            (NType::Void, NType::Void) => true,
            (NType::Int, NType::Int) => true,
            (NType::Float, NType::Float) => true,
            (NType::Int, NType::Float) => true,
            (NType::Float, NType::Int) => true,
            (NType::Pointer { .. }, NType::Pointer { .. }) => true,
            (NType::Pointer { .. }, NType::Int) => true,
            (NType::Struct(id1), NType::Struct(id2)) => id1 == id2,
            (NType::Const(inner), NType::Const(r_inner)) => inner.assign_to_me_is_ok(r_inner),
            (NType::Const(inner), _) => inner.assign_to_me_is_ok(other),
            (_, NType::Const(inner)) => self.assign_to_me_is_ok(inner),
            _ => false,
        }
    }
}
