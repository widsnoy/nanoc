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

impl NType {
    pub fn is_array(&self) -> bool {
        matches!(self, Self::Array(_, _))
    }
    pub fn is_const(&self) -> bool {
        matches!(self, Self::Const(_))
    }
}
