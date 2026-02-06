use crate::ast::*;

/// 类型相关的访问者 trait
pub trait TypeVisitor {
    fn enter_type(&mut self, _node: Type) {}
    fn leave_type(&mut self, _node: Type) {}

    fn enter_name(&mut self, _node: Name) {}
    fn leave_name(&mut self, _node: Name) {}

    fn enter_pointer(&mut self, _node: Pointer) {}
    fn leave_pointer(&mut self, _node: Pointer) {}
}
