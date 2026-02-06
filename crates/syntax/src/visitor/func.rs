use crate::ast::*;

/// 函数相关的访问者 trait
pub trait FuncVisitor {
    fn enter_func_def(&mut self, _node: FuncDef) {}
    fn leave_func_def(&mut self, _node: FuncDef) {}

    fn enter_func_sign(&mut self, _node: FuncSign) {}
    fn leave_func_sign(&mut self, _node: FuncSign) {}

    fn enter_func_attach(&mut self, _node: FuncAttach) {}
    fn leave_func_attach(&mut self, _node: FuncAttach) {}

    fn enter_func_f_params(&mut self, _node: FuncFParams) {}
    fn leave_func_f_params(&mut self, _node: FuncFParams) {}

    fn enter_func_f_param(&mut self, _node: FuncFParam) {}
    fn leave_func_f_param(&mut self, _node: FuncFParam) {}
}
