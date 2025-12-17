use nanoc_parser::ast::*;
use nanoc_parser::visitor::Visitor;

use crate::module::{Module, SemanticError};
use crate::ntype::NType;

impl Visitor for Module {
    fn enter_comp_unit(&mut self, _node: CompUnit) {
        self.analyzing.current_scope = self.new_scope(None);
        self.global_scope = self.analyzing.current_scope;
    }

    fn leave_comp_unit(&mut self, _node: CompUnit) {
        todo!()
    }

    fn leave_const_decl(&mut self, node: ConstDecl) {
        let base_type = Self::eval_type_node(&node.ty().unwrap());

        for const_def in node.const_defs() {
            let pointer_node = const_def.pointer().unwrap();
            let var_type = Self::eval_pointer_node(&pointer_node, base_type.clone());

            let index_val_node = const_def.const_index_val().unwrap();

            // todo 这里需要先把常数下标算出来

            let name_node = index_val_node.name().unwrap();
            let name = Self::eval_name(&name_node);

            let scope = self.scopes.get_mut(*self.analyzing.current_scope).unwrap();
            let var = scope.new_variable(&mut self.variables, name, var_type);

            // todo 处理初始化值
        }
    }

    fn leave_var_decl(&mut self, node: VarDecl) {
        let base_type = Self::eval_type_node(&node.ty().unwrap());

        for def in node.var_defs() {
            let pointer_node = def.pointer().unwrap();
            let var_type = Self::eval_pointer_node(&pointer_node, base_type.clone());

            let index_val_node = def.const_index_val().unwrap();

            // todo 这里需要先把常数下标算出来

            let name_node = index_val_node.name().unwrap();
            let name = Self::eval_name(&name_node);

            let scope = self.scopes.get_mut(*self.analyzing.current_scope).unwrap();
            let var = scope.new_variable(&mut self.variables, name, var_type);

            // todo 处理初始化值
        }
    }

    fn enter_func_def(&mut self, node: FuncDef) {
        let ret_ty = Self::eval_func_type_node(&node.func_type().unwrap());
        self.analyzing.current_scope = self.new_scope(Some(self.analyzing.current_scope));
        let scope = self.scopes.get_mut(*self.analyzing.current_scope).unwrap();
        let params = node.params().unwrap();
        for param in params.params() {
            let param_base_type = Self::eval_type_node(&param.ty().unwrap());
            let pointer_node = param.pointer().unwrap();
            let param_type = Self::eval_pointer_node(&pointer_node, param_base_type);

            // todo

            let name_node = param.name().unwrap();
            let name = Self::eval_name(&name_node);
        }
    }

    fn leave_func_def(&mut self, _node: FuncDef) {
        self.analyzing.current_scope = self
            .scopes
            .get(*self.analyzing.current_scope)
            .unwrap()
            .parent
            .unwrap();
    }

    fn enter_func_type(&mut self, _node: FuncType) {
        todo!()
    }

    fn leave_func_type(&mut self, _node: FuncType) {
        todo!()
    }

    fn enter_func_f_params(&mut self, _node: FuncFParams) {
        todo!()
    }

    fn leave_func_f_params(&mut self, _node: FuncFParams) {
        todo!()
    }

    fn enter_func_f_param(&mut self, _node: FuncFParam) {
        todo!()
    }

    fn leave_func_f_param(&mut self, _node: FuncFParam) {
        todo!()
    }

    fn enter_block(&mut self, _node: Block) {
        todo!()
    }

    fn leave_block(&mut self, _node: Block) {
        todo!()
    }

    fn enter_assign_stmt(&mut self, _node: AssignStmt) {
        todo!()
    }

    fn leave_assign_stmt(&mut self, _node: AssignStmt) {
        todo!()
    }

    fn enter_expr_stmt(&mut self, _node: ExprStmt) {
        todo!()
    }

    fn leave_expr_stmt(&mut self, _node: ExprStmt) {
        todo!()
    }

    fn enter_if_stmt(&mut self, _node: IfStmt) {
        todo!()
    }

    fn leave_if_stmt(&mut self, _node: IfStmt) {
        todo!()
    }

    fn enter_while_stmt(&mut self, _node: WhileStmt) {
        todo!()
    }

    fn leave_while_stmt(&mut self, _node: WhileStmt) {
        todo!()
    }

    fn enter_break_stmt(&mut self, _node: BreakStmt) {
        todo!()
    }

    fn leave_break_stmt(&mut self, _node: BreakStmt) {
        todo!()
    }

    fn enter_continue_stmt(&mut self, _node: ContinueStmt) {
        todo!()
    }

    fn leave_continue_stmt(&mut self, _node: ContinueStmt) {
        todo!()
    }

    fn enter_return_stmt(&mut self, _node: ReturnStmt) {
        todo!()
    }

    fn leave_return_stmt(&mut self, _node: ReturnStmt) {
        todo!()
    }

    fn enter_binary_expr(&mut self, _node: BinaryExpr) {
        todo!()
    }

    fn leave_binary_expr(&mut self, _node: BinaryExpr) {
        todo!()
    }

    fn enter_unary_expr(&mut self, _node: UnaryExpr) {
        todo!()
    }

    fn leave_unary_expr(&mut self, _node: UnaryExpr) {
        todo!()
    }

    fn enter_binary_op(&mut self, _node: BinaryOp) {
        todo!()
    }

    fn leave_binary_op(&mut self, _node: BinaryOp) {
        todo!()
    }

    fn enter_unary_op(&mut self, _node: UnaryOp) {
        todo!()
    }

    fn leave_unary_op(&mut self, _node: UnaryOp) {
        todo!()
    }

    fn enter_call_expr(&mut self, _node: CallExpr) {
        todo!()
    }

    fn leave_call_expr(&mut self, _node: CallExpr) {
        todo!()
    }

    fn enter_func_r_params(&mut self, _node: FuncRParams) {
        todo!()
    }

    fn leave_func_r_params(&mut self, _node: FuncRParams) {
        todo!()
    }

    fn enter_paren_expr(&mut self, _node: ParenExpr) {
        todo!()
    }

    fn leave_paren_expr(&mut self, _node: ParenExpr) {
        todo!()
    }

    fn enter_deref_expr(&mut self, _node: DerefExpr) {
        todo!()
    }

    fn leave_deref_expr(&mut self, _node: DerefExpr) {
        todo!()
    }

    fn enter_index_val(&mut self, _node: IndexVal) {
        todo!()
    }

    fn leave_index_val(&mut self, _node: IndexVal) {
        todo!()
    }

    fn enter_const_index_val(&mut self, _node: ConstIndexVal) {
        todo!()
    }

    fn leave_const_index_val(&mut self, _node: ConstIndexVal) {
        todo!()
    }

    fn leave_const_expr(&mut self, node: ConstExpr) {
        let expr = node.expr().unwrap();
        let range = expr.syntax().text_range();
        if !self.is_constant(range) {
            self.analyzing
                .errors
                .push(SemanticError::ConstantExprExpected { range });
        }
    }

    fn enter_literal(&mut self, node: Literal) {}

    fn leave_literal(&mut self, _node: Literal) {
        todo!()
    }

    fn enter_type(&mut self, node: Type) {}

    fn enter_name(&mut self, _node: Name) {
        todo!()
    }

    fn leave_name(&mut self, _node: Name) {
        todo!()
    }

    fn enter_pointer(&mut self, _node: Pointer) {
        todo!()
    }

    fn leave_pointer(&mut self, _node: Pointer) {
        todo!()
    }
}

impl Module {
    fn eval_type_node(node: &Type) -> NType {
        if node.int_token().is_some() {
            NType::Int
        } else if node.float_token().is_some() {
            NType::Float
        } else if node.struct_token().is_some() {
            let name = Self::eval_name(&node.name().unwrap());
            NType::Struct(name)
        } else {
            unreachable!("未知类型节点")
        }
    }

    fn eval_pointer_node(node: &Pointer, base_type: NType) -> NType {
        let res = node.stars();
        let mut ty = base_type;
        for b in res {
            ty = NType::Pointer(Box::new(ty));
            if !b {
                ty = NType::Const(Box::new(ty));
            }
        }
        ty
    }

    fn eval_name(node: &Name) -> String {
        node.ident()
            .map(|t| t.text().to_string())
            .expect("获取标识符失败")
    }

    fn eval_func_type_node(node: &FuncType) -> NType {
        if node.void_token().is_some() {
            NType::Void
        } else {
            let base_type = Self::eval_type_node(&node.ty().unwrap());
            let pointer_node = node.pointer().unwrap();
            Self::eval_pointer_node(&pointer_node, base_type)
        }
    }
}
