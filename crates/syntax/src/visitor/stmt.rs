use crate::ast::*;

/// 语句相关的访问者 trait
pub trait StmtVisitor {
    fn enter_block(&mut self, _node: Block) {}
    fn leave_block(&mut self, _node: Block) {}

    fn enter_assign_stmt(&mut self, _node: AssignStmt) {}
    fn leave_assign_stmt(&mut self, _node: AssignStmt) {}

    fn enter_expr_stmt(&mut self, _node: ExprStmt) {}
    fn leave_expr_stmt(&mut self, _node: ExprStmt) {}

    fn enter_if_stmt(&mut self, _node: IfStmt) {}
    fn leave_if_stmt(&mut self, _node: IfStmt) {}

    fn enter_while_stmt(&mut self, _node: WhileStmt) {}
    fn leave_while_stmt(&mut self, _node: WhileStmt) {}

    fn enter_break_stmt(&mut self, _node: BreakStmt) {}
    fn leave_break_stmt(&mut self, _node: BreakStmt) {}

    fn enter_continue_stmt(&mut self, _node: ContinueStmt) {}
    fn leave_continue_stmt(&mut self, _node: ContinueStmt) {}

    fn enter_return_stmt(&mut self, _node: ReturnStmt) {}
    fn leave_return_stmt(&mut self, _node: ReturnStmt) {}
}
