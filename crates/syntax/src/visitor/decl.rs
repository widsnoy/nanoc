use crate::ast::*;

/// 声明相关的访问者 trait
pub trait DeclVisitor {
    fn enter_comp_unit(&mut self, _node: CompUnit) {}
    fn leave_comp_unit(&mut self, _node: CompUnit) {}

    fn enter_var_def(&mut self, _node: VarDef) {}
    fn leave_var_def(&mut self, _node: VarDef) {}

    fn enter_init_val(&mut self, _node: InitVal) {}
    fn leave_init_val(&mut self, _node: InitVal) {}

    fn enter_struct_def(&mut self, _node: StructDef) {}
    fn leave_struct_def(&mut self, _node: StructDef) {}

    fn enter_struct_field(&mut self, _node: StructField) {}
    fn leave_struct_field(&mut self, _node: StructField) {}
}
