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
    pub fn is_array(&self) -> bool {
        matches!(self, Self::Array(_, _))
    }

    pub fn is_pointer(&self) -> bool {
        matches!(self, Self::Pointer(_))
    }
    pub fn is_const(&self) -> bool {
        match self {
            Self::Const(_) => true,
            Self::Array(inner, _) => inner.is_const(),
            Self::Pointer(_) => false,
            Self::Struct(_) => todo!(),
            _ => false,
        }
    }

    /// 只返回标量, 比如 int / float
    pub fn const_zero(&self) -> Value {
        match self {
            NType::Int => Value::Int(0),
            NType::Float => Value::Float(0.0),
            NType::Void => Value::Int(0),
            NType::Array(ntype, _) => ntype.const_zero(),
            NType::Pointer(_ntype) => todo!(),
            NType::Struct(_) => todo!(),
            NType::Const(ntype) => ntype.const_zero(),
        }
    }
}
