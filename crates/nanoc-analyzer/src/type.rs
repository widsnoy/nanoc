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
    pub fn is_const(&self) -> bool {
        matches!(self, Self::Const(_))
    }
    pub fn unwrap_const(&self) -> &Self {
        match self {
            Self::Const(inner) => inner,
            _ => self,
        }
    }
}
