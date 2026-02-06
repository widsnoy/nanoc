//! 语句相关的语义分析

use syntax::ast::*;
use syntax::visitor::StmtVisitor;

use crate::error::SemanticError;
use crate::module::Module;
use crate::r#type::NType;

impl StmtVisitor for Module {
    fn enter_block(&mut self, _node: Block) {
        self.analyzing.current_scope = self.new_scope(Some(self.analyzing.current_scope));
    }

    fn leave_block(&mut self, _node: Block) {
        self.analyzing.current_scope = self
            .scopes
            .get(*self.analyzing.current_scope)
            .unwrap()
            .parent
            .unwrap();
    }

    fn enter_while_stmt(&mut self, _node: WhileStmt) {
        self.analyzing.loop_depth += 1;
    }

    fn leave_while_stmt(&mut self, _node: WhileStmt) {
        self.analyzing.loop_depth -= 1;
    }

    fn leave_assign_stmt(&mut self, node: AssignStmt) {
        let Some(lhs) = node.lhs() else {
            return;
        };
        let Some(rhs) = node.rhs() else {
            return;
        };

        let lhs_range = lhs.text_range();
        let rhs_range = rhs.text_range();

        // 检查是否是左值
        let is_valid_lvalue = self.is_lvalue_expr(&lhs);
        if !is_valid_lvalue {
            self.new_error(SemanticError::NotALValue { range: lhs_range });
            return;
        }

        // 检查左值是否可赋值（const 检测）
        if !self.check_lvalue_assignable(&lhs) {
            return;
        }

        // 类型检查
        let Some(lhs_ty) = self.get_expr_type(lhs_range) else {
            return;
        };
        let Some(rhs_ty) = self.get_expr_type(rhs_range) else {
            return;
        };

        if !lhs_ty.assign_to_me_is_ok(rhs_ty) {
            self.new_error(SemanticError::TypeMismatch {
                expected: lhs_ty.clone(),
                found: rhs_ty.clone(),
                range: rhs_range,
            });
        }
    }

    fn enter_break_stmt(&mut self, node: BreakStmt) {
        if self.analyzing.loop_depth == 0 {
            self.new_error(SemanticError::BreakOutsideLoop {
                range: node.text_range(),
            });
        }
    }

    fn enter_continue_stmt(&mut self, node: ContinueStmt) {
        if self.analyzing.loop_depth == 0 {
            self.new_error(SemanticError::ContinueOutsideLoop {
                range: node.text_range(),
            });
        }
    }

    fn leave_return_stmt(&mut self, node: ReturnStmt) {
        let range = node.text_range();

        // 获取当前函数的返回类型
        let Some(expected_ret_type) = &self.analyzing.current_function_ret_type else {
            return;
        };

        // 获取 return 表达式的类型
        let actual_ret_type = if let Some(expr) = node.expr() {
            let expr_range = expr.text_range();
            match self.get_expr_type(expr_range) {
                Some(v) => v,
                None => return,
            }
        } else {
            &NType::Void
        };

        // 检查返回类型是否匹配
        if !expected_ret_type.assign_to_me_is_ok(actual_ret_type) {
            self.new_error(SemanticError::ReturnTypeMismatch {
                expected: expected_ret_type.clone(),
                found: actual_ret_type.clone(),
                range,
            });
        }
    }
}
