use nanoc_parser::ast::*;
use nanoc_parser::visitor::Visitor;

use crate::module::{Function, Module, SemanticError, VariableTag};
use crate::r#type::NType;
use crate::value::Value;

impl Visitor for Module {
    fn enter_comp_unit(&mut self, _node: CompUnit) {
        self.analyzing.current_scope = self.new_scope(None);
        self.global_scope = self.analyzing.current_scope;
    }

    fn enter_const_decl(&mut self, node: ConstDecl) {
        self.analyzing.current_base_type = Some(NType::Const(Box::new(Self::eval_type_node(
            &node.ty().unwrap(),
        ))));
    }

    fn leave_const_def(&mut self, const_def: ConstDef) {
        let base_type = self.analyzing.current_base_type.clone().unwrap();
        let var_type = if let Some(pointer_node) = const_def.pointer() {
            Self::eval_pointer_node(&pointer_node, base_type.clone())
        } else {
            base_type.clone()
        };

        let index_val_node = const_def.const_index_val().unwrap();

        // todo 这里需要先把常数下标算出来

        let name_node = index_val_node.name().unwrap();
        let name = Self::eval_name(&name_node);

        let scope = self.scopes.get_mut(*self.analyzing.current_scope).unwrap();

        let range = name_node.ident().unwrap().text_range();

        if scope.have_variable(&name) {
            self.analyzing
                .errors
                .push(SemanticError::VariableDefined { name, range });
            return;
        }

        let Some(init_val_node) = const_def.init() else {
            self.analyzing
                .errors
                .push(SemanticError::ExpectInitialVal { name, range });
            return;
        };

        if let Some(v) = self
            .value_table
            .get(&init_val_node.syntax().text_range())
            .cloned()
        {
            self.value_table.insert(range, v);
        }
        let _ = scope.new_variable(
            &mut self.variables,
            name,
            var_type,
            range,
            VariableTag::Define,
        );
    }

    fn leave_const_init_val(&mut self, _node: ConstInitVal) {}

    fn enter_var_decl(&mut self, node: VarDecl) {
        self.analyzing.current_base_type = Some(Self::eval_type_node(&node.ty().unwrap()));
    }

    fn leave_var_def(&mut self, def: VarDef) {
        let base_type = self.analyzing.current_base_type.clone().unwrap();
        let var_type = if let Some(pointer_node) = def.pointer() {
            Self::eval_pointer_node(&pointer_node, base_type.clone())
        } else {
            base_type.clone()
        };

        let index_val_node = def.const_index_val().unwrap();

        let name_node = index_val_node.name().unwrap();
        let name = Self::eval_name(&name_node);
        let range = name_node.ident().unwrap().text_range();
        let scope = self.scopes.get_mut(*self.analyzing.current_scope).unwrap();

        if scope.have_variable(&name) {
            self.analyzing
                .errors
                .push(SemanticError::VariableDefined { name, range });
            return;
        }

        let _ = scope.new_variable(
            &mut self.variables,
            name,
            var_type,
            range,
            VariableTag::Define,
        );

        // todo 处理初始化值，把数组列表展开
    }

    fn enter_func_def(&mut self, _node: FuncDef) {
        self.analyzing.current_scope = self.new_scope(Some(self.analyzing.current_scope));
    }

    fn leave_func_def(&mut self, node: FuncDef) {
        let scope = self.scopes.get(*self.analyzing.current_scope).unwrap();
        let mut param_list = Vec::new();

        if let Some(params) = node.params() {
            for param in params.params() {
                let name_node = param.name().unwrap();
                let name = name_node.ident().unwrap();
                let Some(v) = scope.look_up(self, name.text(), VariableTag::Define) else {
                    return;
                }; // 函数定义是一个 scope
                param_list.push(v);
            }
        }

        let ret_type = Self::eval_func_type_node(&node.func_type().unwrap());
        let name = node.name().unwrap().ident().unwrap().text().to_string();
        self.functions.insert(Function {
            name,
            params: param_list,
            ret_type,
        });

        self.analyzing.current_scope = scope.parent.unwrap();
    }

    fn leave_func_f_param(&mut self, node: FuncFParam) {
        let param_base_type = Self::eval_type_node(&node.ty().unwrap());

        let param_type = if let Some(pointer_node) = node.pointer() {
            Self::eval_pointer_node(&pointer_node, param_base_type)
        } else {
            param_base_type
        };

        // todo:

        let name_node = node.name().unwrap();
        let name = Self::eval_name(&name_node);
        let range = name_node.ident().unwrap().text_range();
        let scope = self.scopes.get_mut(*self.analyzing.current_scope).unwrap();

        if scope.have_variable(&name) {
            self.analyzing
                .errors
                .push(SemanticError::VariableDefined { name, range });
            return;
        }

        scope.new_variable(
            &mut self.variables,
            name,
            param_type,
            range,
            VariableTag::Define,
        );
    }

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

    fn enter_assign_stmt(&mut self, _node: AssignStmt) {
        // todo!("检查类型是否匹配")
    }

    fn leave_assign_stmt(&mut self, _node: AssignStmt) {
        // todo!()
    }

    fn enter_break_stmt(&mut self, _node: BreakStmt) {
        // todo!()
    }

    fn leave_break_stmt(&mut self, _node: BreakStmt) {
        // todo!()
    }

    fn enter_continue_stmt(&mut self, _node: ContinueStmt) {
        // todo!()
    }

    fn leave_continue_stmt(&mut self, _node: ContinueStmt) {
        // todo!()
    }

    // 检查返回类型
    fn leave_return_stmt(&mut self, _node: ReturnStmt) {}

    fn leave_binary_expr(&mut self, node: BinaryExpr) {
        let lhs = node.lhs().unwrap();
        let rhs = node.rhs().unwrap();
        let op = node.op().unwrap();
        let op_str = op.op_str();
        if self.is_constant(lhs.syntax().text_range())
            && self.is_constant(rhs.syntax().text_range())
        {
            let lhs_val = self.value_table.get(&lhs.syntax().text_range()).unwrap();
            let rhs_val = self.value_table.get(&rhs.syntax().text_range()).unwrap();

            if let Ok(val) = Value::eval(lhs_val, rhs_val, &op_str) {
                self.mark_constant(node.syntax().text_range());
                self.value_table.insert(node.syntax().text_range(), val);
            }
        }
    }

    fn leave_unary_expr(&mut self, node: UnaryExpr) {
        let expr = node.expr().unwrap();
        let op = node.op().unwrap();
        let op_str = op.op_str();

        if self.is_constant(expr.syntax().text_range()) {
            let val = self
                .value_table
                .get(&expr.syntax().text_range())
                .unwrap()
                .clone();
            if let Ok(res) = Value::eval_unary(val, &op_str) {
                self.mark_constant(node.syntax().text_range());
                self.value_table.insert(node.syntax().text_range(), res);
            }
        }
    }

    fn leave_paren_expr(&mut self, node: ParenExpr) {
        let expr = node.expr().unwrap();
        if self.is_constant(expr.syntax().text_range()) {
            self.mark_constant(node.syntax().text_range());
            let val = self
                .value_table
                .get(&expr.syntax().text_range())
                .unwrap()
                .clone();
            self.value_table.insert(node.syntax().text_range(), val);
        }
    }

    fn enter_deref_expr(&mut self, _node: DerefExpr) {
        todo!()
    }

    fn leave_deref_expr(&mut self, _node: DerefExpr) {
        todo!()
    }

    // fn enter_index_val(&mut self, _node: IndexVal) {
    //     todo!()
    // }

    fn leave_index_val(&mut self, node: IndexVal) {
        let ident_token = node.name().unwrap().ident().unwrap();
        let var_range = ident_token.text_range();
        let var_name = ident_token.text();
        let scope = self.scopes.get(*self.analyzing.current_scope).unwrap();
        let Some(vid) = scope.look_up(self, var_name, VariableTag::Define) else {
            self.analyzing
                .errors
                .push(SemanticError::VariableUndefined {
                    name: var_name.to_string(),
                    range: var_range,
                });
            return;
        };
        let v = self.variables.get(*vid).unwrap();
        if !v.is_const() {
            return;
        }
        let value = self.value_table.get(&v.range).unwrap();
        // todo: array...
        // 因为 trivia，所以需要把整个节点的值存入
        let range = node.syntax().text_range();
        self.value_table.insert(range, value.clone());
        self.mark_constant(range);
    }

    // fn enter_const_index_val(&mut self, _node: ConstIndexVal) {

    // }

    // fn leave_const_index_val(&mut self, _node: ConstIndexVal) {

    // }

    fn leave_const_expr(&mut self, node: ConstExpr) {
        let expr = node.expr().unwrap();
        let range = expr.syntax().text_range();
        if !self.is_constant(range) {
            self.analyzing
                .errors
                .push(SemanticError::ConstantExprExpected { range });
        }
    }

    fn enter_literal(&mut self, node: Literal) {
        self.mark_constant(node.syntax().text_range());
        let v = if let Some(n) = node.float_token() {
            let s = n.text();
            Value::Float(s.parse::<f32>().unwrap())
        } else {
            let n = node.int_token().unwrap();
            let s = n.text();
            let (num_str, radix) = match s.chars().next() {
                Some('0') => match s.chars().nth(1) {
                    Some('x') | Some('X') => (&s[2..], 16),
                    Some(_) => (&s[1..], 8),
                    None => (s, 10),
                },
                _ => (s, 10),
            };
            Value::Int(i32::from_str_radix(num_str, radix).unwrap())
        };
        self.value_table.insert(node.syntax().text_range(), v);
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
    /// 从 Name 节点提取变量名
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

            if let Some(pointer_node) = node.pointer() {
                Self::eval_pointer_node(&pointer_node, base_type)
            } else {
                base_type
            }
        }
    }
}
