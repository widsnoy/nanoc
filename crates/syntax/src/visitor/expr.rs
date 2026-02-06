use crate::ast::*;

/// 表达式相关的访问者 trait
pub trait ExprVisitor {
    fn enter_binary_expr(&mut self, _node: BinaryExpr) {}
    fn leave_binary_expr(&mut self, _node: BinaryExpr) {}

    fn enter_unary_expr(&mut self, _node: UnaryExpr) {}
    fn leave_unary_expr(&mut self, _node: UnaryExpr) {}

    fn enter_postfix_expr(&mut self, _node: PostfixExpr) {}
    fn leave_postfix_expr(&mut self, _node: PostfixExpr) {}

    fn enter_call_expr(&mut self, _node: CallExpr) {}
    fn leave_call_expr(&mut self, _node: CallExpr) {}

    fn enter_func_r_params(&mut self, _node: FuncRParams) {}
    fn leave_func_r_params(&mut self, _node: FuncRParams) {}

    fn enter_paren_expr(&mut self, _node: ParenExpr) {}
    fn leave_paren_expr(&mut self, _node: ParenExpr) {}

    fn enter_index_val(&mut self, _node: IndexVal) {}
    fn leave_index_val(&mut self, _node: IndexVal) {}

    fn enter_field_access(&mut self, _node: FieldAccess) {}
    fn leave_field_access(&mut self, _node: FieldAccess) {}

    fn enter_literal(&mut self, _node: Literal) {}
    fn leave_literal(&mut self, _node: Literal) {}
}
