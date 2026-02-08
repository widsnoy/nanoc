use rowan::WalkEvent;

use crate::ast::*;
use crate::{SyntaxKind, SyntaxNode};

mod decl;
mod expr;
mod func;
mod stmt;

pub use decl::DeclVisitor;
pub use expr::ExprVisitor;
pub use func::FuncVisitor;
pub use stmt::StmtVisitor;

/// 语法树访问者 trait
///
/// 要求实现类型同时实现所有子 trait：
/// - `DeclVisitor`: 声明相关节点
/// - `FuncVisitor`: 函数相关节点
/// - `StmtVisitor`: 语句相关节点
/// - `ExprVisitor`: 表达式相关节点
pub trait Visitor: DeclVisitor + FuncVisitor + StmtVisitor + ExprVisitor + Sized {
    /// 遍历语法树
    fn walk(&mut self, root: &SyntaxNode) {
        let mut error_depth = 0usize;
        for event in root.preorder() {
            match event {
                WalkEvent::Enter(node) => {
                    let kind = node.kind();
                    if kind == SyntaxKind::ERROR {
                        error_depth += 1;
                    }
                    if error_depth > 0 {
                        continue;
                    }
                    self.dispatch_enter(node);
                }
                WalkEvent::Leave(node) => {
                    let kind = node.kind();
                    if kind == SyntaxKind::ERROR {
                        error_depth -= 1;
                    }
                    if error_depth > 0 {
                        continue;
                    }
                    self.dispatch_leave(node);
                }
            }
        }
    }

    /// 分发 enter 事件到对应的子 trait 方法
    fn dispatch_enter(&mut self, node: SyntaxNode) {
        dispatch_node!(self, node, enter);
    }

    /// 分发 leave 事件到对应的子 trait 方法
    fn dispatch_leave(&mut self, node: SyntaxNode) {
        dispatch_node!(self, node, leave);
    }
}

/// 宏：生成节点分发代码
macro_rules! dispatch_node {
    ($self:expr, $node:expr, enter) => {{
        use crate::*;
        match $node.kind() {
            SyntaxKind::COMP_UNIT => CompUnit::cast($node).map(|n| $self.enter_comp_unit(n)),
            SyntaxKind::VAR_DEF => VarDef::cast($node).map(|n| $self.enter_var_def(n)),
            SyntaxKind::INIT_VAL => InitVal::cast($node).map(|n| $self.enter_init_val(n)),
            SyntaxKind::STRUCT_DEF => StructDef::cast($node).map(|n| $self.enter_struct_def(n)),
            SyntaxKind::STRUCT_FIELD => {
                StructField::cast($node).map(|n| $self.enter_struct_field(n))
            }
            SyntaxKind::FUNC_DEF => FuncDef::cast($node).map(|n| $self.enter_func_def(n)),
            SyntaxKind::FUNC_SIGN => FuncSign::cast($node).map(|n| $self.enter_func_sign(n)),
            SyntaxKind::FUNC_ATTACH => FuncAttach::cast($node).map(|n| $self.enter_func_attach(n)),
            SyntaxKind::FUNC_F_PARAMS => {
                FuncFParams::cast($node).map(|n| $self.enter_func_f_params(n))
            }
            SyntaxKind::FUNC_F_PARAM => {
                FuncFParam::cast($node).map(|n| $self.enter_func_f_param(n))
            }
            SyntaxKind::BLOCK => Block::cast($node).map(|n| $self.enter_block(n)),
            SyntaxKind::ASSIGN_STMT => AssignStmt::cast($node).map(|n| $self.enter_assign_stmt(n)),
            SyntaxKind::EXPR_STMT => ExprStmt::cast($node).map(|n| $self.enter_expr_stmt(n)),
            SyntaxKind::IF_STMT => IfStmt::cast($node).map(|n| $self.enter_if_stmt(n)),
            SyntaxKind::WHILE_STMT => WhileStmt::cast($node).map(|n| $self.enter_while_stmt(n)),
            SyntaxKind::BREAK_STMT => BreakStmt::cast($node).map(|n| $self.enter_break_stmt(n)),
            SyntaxKind::CONTINUE_STMT => {
                ContinueStmt::cast($node).map(|n| $self.enter_continue_stmt(n))
            }
            SyntaxKind::RETURN_STMT => ReturnStmt::cast($node).map(|n| $self.enter_return_stmt(n)),
            SyntaxKind::BINARY_EXPR => BinaryExpr::cast($node).map(|n| $self.enter_binary_expr(n)),
            SyntaxKind::UNARY_EXPR => UnaryExpr::cast($node).map(|n| $self.enter_unary_expr(n)),
            SyntaxKind::POSTFIX_EXPR => {
                PostfixExpr::cast($node).map(|n| $self.enter_postfix_expr(n))
            }
            SyntaxKind::CALL_EXPR => CallExpr::cast($node).map(|n| $self.enter_call_expr(n)),
            SyntaxKind::FUNC_R_PARAMS => {
                FuncRParams::cast($node).map(|n| $self.enter_func_r_params(n))
            }
            SyntaxKind::PAREN_EXPR => ParenExpr::cast($node).map(|n| $self.enter_paren_expr(n)),
            SyntaxKind::INDEX_VAL => IndexVal::cast($node).map(|n| $self.enter_index_val(n)),
            SyntaxKind::FIELD_ACCESS => {
                FieldAccess::cast($node).map(|n| $self.enter_field_access(n))
            }
            SyntaxKind::LITERAL => Literal::cast($node).map(|n| $self.enter_literal(n)),
            _ => None,
        };
    }};
    ($self:expr, $node:expr, leave) => {{
        use crate::*;
        match $node.kind() {
            SyntaxKind::COMP_UNIT => CompUnit::cast($node).map(|n| $self.leave_comp_unit(n)),
            SyntaxKind::VAR_DEF => VarDef::cast($node).map(|n| $self.leave_var_def(n)),
            SyntaxKind::INIT_VAL => InitVal::cast($node).map(|n| $self.leave_init_val(n)),
            SyntaxKind::STRUCT_DEF => StructDef::cast($node).map(|n| $self.leave_struct_def(n)),
            SyntaxKind::STRUCT_FIELD => {
                StructField::cast($node).map(|n| $self.leave_struct_field(n))
            }
            SyntaxKind::FUNC_DEF => FuncDef::cast($node).map(|n| $self.leave_func_def(n)),
            SyntaxKind::FUNC_SIGN => FuncSign::cast($node).map(|n| $self.leave_func_sign(n)),
            SyntaxKind::FUNC_ATTACH => FuncAttach::cast($node).map(|n| $self.leave_func_attach(n)),
            SyntaxKind::FUNC_F_PARAMS => {
                FuncFParams::cast($node).map(|n| $self.leave_func_f_params(n))
            }
            SyntaxKind::FUNC_F_PARAM => {
                FuncFParam::cast($node).map(|n| $self.leave_func_f_param(n))
            }
            SyntaxKind::BLOCK => Block::cast($node).map(|n| $self.leave_block(n)),
            SyntaxKind::ASSIGN_STMT => AssignStmt::cast($node).map(|n| $self.leave_assign_stmt(n)),
            SyntaxKind::EXPR_STMT => ExprStmt::cast($node).map(|n| $self.leave_expr_stmt(n)),
            SyntaxKind::IF_STMT => IfStmt::cast($node).map(|n| $self.leave_if_stmt(n)),
            SyntaxKind::WHILE_STMT => WhileStmt::cast($node).map(|n| $self.leave_while_stmt(n)),
            SyntaxKind::BREAK_STMT => BreakStmt::cast($node).map(|n| $self.leave_break_stmt(n)),
            SyntaxKind::CONTINUE_STMT => {
                ContinueStmt::cast($node).map(|n| $self.leave_continue_stmt(n))
            }
            SyntaxKind::RETURN_STMT => ReturnStmt::cast($node).map(|n| $self.leave_return_stmt(n)),
            SyntaxKind::BINARY_EXPR => BinaryExpr::cast($node).map(|n| $self.leave_binary_expr(n)),
            SyntaxKind::UNARY_EXPR => UnaryExpr::cast($node).map(|n| $self.leave_unary_expr(n)),
            SyntaxKind::POSTFIX_EXPR => {
                PostfixExpr::cast($node).map(|n| $self.leave_postfix_expr(n))
            }
            SyntaxKind::CALL_EXPR => CallExpr::cast($node).map(|n| $self.leave_call_expr(n)),
            SyntaxKind::FUNC_R_PARAMS => {
                FuncRParams::cast($node).map(|n| $self.leave_func_r_params(n))
            }
            SyntaxKind::PAREN_EXPR => ParenExpr::cast($node).map(|n| $self.leave_paren_expr(n)),
            SyntaxKind::INDEX_VAL => IndexVal::cast($node).map(|n| $self.leave_index_val(n)),
            SyntaxKind::FIELD_ACCESS => {
                FieldAccess::cast($node).map(|n| $self.leave_field_access(n))
            }
            SyntaxKind::LITERAL => Literal::cast($node).map(|n| $self.leave_literal(n)),
            _ => None,
        };
    }};
}

use dispatch_node;
