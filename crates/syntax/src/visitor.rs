use rowan::WalkEvent;

use crate::{SyntaxKind, SyntaxNode, *};

macro_rules! def_visitor {
    ($($Node:ident, $Kind:ident, $enter:ident, $leave:ident);* $(;)?) => {
        /// 语法树访问者 trait
        pub trait Visitor: Sized {
            $(
                fn $enter(&mut self, _node: $Node) {}
                fn $leave(&mut self, _node: $Node) {}
            )*

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
                            match kind {
                                $(
                                    SyntaxKind::$Kind => {
                                        if let Some(n) = $Node::cast(node.clone()) {
                                            self.$enter(n);
                                        }
                                    }
                                )*
                                _ => {}
                            }
                        }
                        WalkEvent::Leave(node) => {
                            let kind = node.kind();
                            if kind == SyntaxKind::ERROR {
                                error_depth -= 1;
                            }
                            if error_depth > 0 {
                                continue;
                            }
                            match kind {
                                $(
                                    SyntaxKind::$Kind => {
                                        if let Some(n) = $Node::cast(node.clone()) {
                                            self.$leave(n);
                                        }
                                    }
                                )*
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    };
}

def_visitor! {
    CompUnit, COMP_UNIT, enter_comp_unit, leave_comp_unit;
    VarDef, VAR_DEF, enter_var_def, leave_var_def;
    InitVal, INIT_VAL, enter_init_val, leave_init_val;
    StructDef, STRUCT_DEF, enter_struct_def, leave_struct_def;
    StructField, STRUCT_FIELD, enter_struct_field, leave_struct_field;
    FuncDef, FUNC_DEF, enter_func_def, leave_func_def;
    FuncSign, FUNC_SIGN, enter_func_sign, leave_func_sign;
    FuncFParams, FUNC_F_PARAMS, enter_func_f_params, leave_func_f_params;
    FuncFParam, FUNC_F_PARAM, enter_func_f_param, leave_func_f_param;
    Block, BLOCK, enter_block, leave_block;
    AssignStmt, ASSIGN_STMT, enter_assign_stmt, leave_assign_stmt;
    ExprStmt, EXPR_STMT, enter_expr_stmt, leave_expr_stmt;
    IfStmt, IF_STMT, enter_if_stmt, leave_if_stmt;
    WhileStmt, WHILE_STMT, enter_while_stmt, leave_while_stmt;
    BreakStmt, BREAK_STMT, enter_break_stmt, leave_break_stmt;
    ContinueStmt, CONTINUE_STMT, enter_continue_stmt, leave_continue_stmt;
    ReturnStmt, RETURN_STMT, enter_return_stmt, leave_return_stmt;
    BinaryExpr, BINARY_EXPR, enter_binary_expr, leave_binary_expr;
    UnaryExpr, UNARY_EXPR, enter_unary_expr, leave_unary_expr;
    PostfixExpr, POSTFIX_EXPR, enter_postfix_expr, leave_postfix_expr;
    CallExpr, CALL_EXPR, enter_call_expr, leave_call_expr;
    FuncRParams, FUNC_R_PARAMS, enter_func_r_params, leave_func_r_params;
    ParenExpr, PAREN_EXPR, enter_paren_expr, leave_paren_expr;
    IndexVal, INDEX_VAL, enter_index_val, leave_index_val;
    FieldAccess, FIELD_ACCESS, enter_field_access, leave_field_access;
    Literal, LITERAL, enter_literal, leave_literal;
    Type, TYPE, enter_type, leave_type;
    Name, NAME, enter_name, leave_name;
    Pointer, POINTER, enter_pointer, leave_pointer;
}
