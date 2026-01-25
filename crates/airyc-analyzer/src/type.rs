use crate::value::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NType {
    Int,
    Float,
    Void,
    Array(Box<NType>, i32),
    Pointer(Box<NType>),
    Struct(String),
    Const(Box<NType>),
}

impl NType {
    /// array 不可能被 const wrap
    pub fn is_array(&self) -> bool {
        matches!(self, Self::Array(_, _))
    }

    /// 检查是否为指针类型（包括 Const(Pointer(...))）
    pub fn is_pointer(&self) -> bool {
        self.pointer_inner().is_some()
    }

    /// 提取指针类型的内部类型，处理 Pointer(...) 和 Const(Pointer(...)) 两种情况
    pub fn pointer_inner(&self) -> Option<&NType> {
        match self {
            Self::Pointer(inner) => Some(inner.as_ref()),
            Self::Const(inner) => {
                if let Self::Pointer(p) = inner.as_ref() {
                    Some(p.as_ref())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn is_const(&self) -> bool {
        matches!(self, Self::Const(_))
    }

    /// 去掉 Const 包装，返回内部类型
    pub fn unwrap_const(&self) -> NType {
        match self {
            Self::Const(inner) => inner.as_ref().clone(),
            _ => self.clone(),
        }
    }

    /// 返回标量零值（int / float）
    pub fn const_zero(&self) -> Value {
        match self {
            NType::Int => Value::Int(0),
            NType::Float => Value::Float(0.0),
            NType::Void => Value::Int(0),
            NType::Array(ntype, _) => ntype.const_zero(),
            NType::Pointer(_ntype) => Value::Int(0), // null pointer
            NType::Struct(_) => todo!(),
            NType::Const(ntype) => ntype.const_zero(),
        }
    }
}
