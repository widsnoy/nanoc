use airyc_parser::ast::*;
use airyc_parser::syntax_kind::SyntaxKind;
use airyc_parser::visitor::Visitor;

use crate::array::{ArrayTree, ArrayTreeValue};
use crate::module::{ConstKind, Function, Module, SemanticError, VariableTag};
use crate::r#type::NType;
use crate::value::Value;

impl Visitor for Module {
    fn enter_comp_unit(&mut self, _node: CompUnit) {
        self.analyzing.current_scope = self.new_scope(None);
        self.global_scope = self.analyzing.current_scope;
    }

    fn enter_const_decl(&mut self, node: ConstDecl) {
        self.analyzing.current_base_type = Some(NType::Const(Box::new(Self::build_basic_type(
            &node.ty().unwrap(),
        ))));
    }

    fn leave_const_def(&mut self, const_def: ConstDef) {
        let base_type = self.analyzing.current_base_type.clone().unwrap();
        let var_type = if let Some(pointer_node) = const_def.pointer() {
            Self::build_pointer_type(&pointer_node, base_type)
        } else {
            base_type
        };

        let index_val_node = const_def.const_index_val().unwrap();
        let var_type = self
            .build_array_type(var_type, index_val_node.indices())
            .unwrap();
        let name_node = index_val_node.name().unwrap();
        let name = Self::extract_name(&name_node);

        let scope = self.scopes.get_mut(*self.analyzing.current_scope).unwrap();

        let range = name_node.ident().unwrap().text_range();

        if scope.have_variable(&name) {
            self.analyzing
                .errors
                .push(SemanticError::VariableDefined { name, range });
            return;
        }

        let Some(const_init_val_node) = const_def.init() else {
            self.analyzing
                .errors
                .push(SemanticError::ExpectInitialVal { name, range });
            return;
        };

        match &var_type {
            NType::Array(_, _) => {
                let init_range = const_init_val_node.syntax().text_range();
                let array_tree = ArrayTree::new(&var_type, const_init_val_node).unwrap(); // Error handling TODO
                self.expand_array.insert(init_range, array_tree.clone());
                self.value_table.insert(range, Value::Array(array_tree));
            }
            _ => {
                // 如果初始值是编译时常量，存入 value_table 用于常量折叠
                // 如果不是编译时常量（如 &a），仍然允许声明，只是不能用于常量折叠
                if let Some(init_value) = self
                    .value_table
                    .get(&const_init_val_node.syntax().text_range())
                    .cloned()
                {
                    self.value_table.insert(range, init_value);
                }
            }
        }

        let scope = self.scopes.get_mut(*self.analyzing.current_scope).unwrap();
        let _ = scope.new_variable(
            &mut self.variables,
            &mut self.variable_map,
            name,
            var_type,
            range,
            VariableTag::Define,
        );
    }

    fn leave_const_init_val(&mut self, node: ConstInitVal) {
        self.check_and_mark_constant(
            node.syntax().text_range(),
            node.expr().map(|e| e.syntax().text_range()),
            node.inits().map(|c| c.syntax().text_range()),
        );
    }

    fn enter_var_decl(&mut self, node: VarDecl) {
        self.analyzing.current_base_type = Some(Self::build_basic_type(&node.ty().unwrap()));
    }

    // 待办：首先检查左值和右值是否匹配
    fn leave_var_def(&mut self, def: VarDef) {
        let base_type = self.analyzing.current_base_type.clone().unwrap();
        let var_type = if let Some(pointer_node) = def.pointer() {
            Self::build_pointer_type(&pointer_node, base_type)
        } else {
            base_type
        };

        let const_index_val_node = def.const_index_val().unwrap();
        let var_type = self
            .build_array_type(var_type, const_index_val_node.indices())
            .unwrap();
        let name_node = const_index_val_node.name().unwrap();
        let name = Self::extract_name(&name_node);
        let range = name_node.ident().unwrap().text_range();
        let scope = self.scopes.get_mut(*self.analyzing.current_scope).unwrap();

        if scope.have_variable(&name) {
            self.analyzing
                .errors
                .push(SemanticError::VariableDefined { name, range });
            return;
        }

        if let Some(init_val_node) = def.init() {
            let range = init_val_node.syntax().text_range();
            if self.global_scope == self.analyzing.current_scope
                && !self.constant_nodes.contains_key(&range)
            {
                self.analyzing
                    .errors
                    .push(SemanticError::ConstantExprExpected { range });
                return;
            }

            if var_type.is_array() {
                let array_tree = ArrayTree::new(&var_type, init_val_node).unwrap();
                self.expand_array.insert(range, array_tree);
            }
        }

        let _ = scope.new_variable(
            &mut self.variables,
            &mut self.variable_map,
            name,
            var_type,
            range,
            VariableTag::Define,
        );
    }

    fn leave_init_val(&mut self, node: InitVal) {
        self.check_and_mark_constant(
            node.syntax().text_range(),
            node.expr().map(|e| e.syntax().text_range()),
            node.inits().map(|c| c.syntax().text_range()),
        );
    }

    fn enter_func_def(&mut self, _node: FuncDef) {
        self.analyzing.current_scope = self.new_scope(Some(self.analyzing.current_scope));
    }

    fn leave_func_def(&mut self, node: FuncDef) {
        let mut param_list = Vec::new();

        let scope = self.scopes.get(*self.analyzing.current_scope).unwrap();
        let parent_scope = scope.parent.unwrap();

        if let Some(params) = node.params() {
            for param in params.params() {
                let name_node = param.name().unwrap();
                let name = name_node.ident().unwrap();
                let Some(v) = scope.look_up(self, name.text(), VariableTag::Define) else {
                    return;
                }; // 函数定义是一个作用域
                param_list.push(v);
            }
        }

        let func_type_node = node.func_type().unwrap();
        let ret_type = Self::build_func_type(&func_type_node);
        // 将函数返回类型存储到 type_table，供 codegen 使用
        self.set_expr_type(func_type_node.syntax().text_range(), ret_type.clone());

        let name = node.name().unwrap().ident().unwrap().text().to_string();
        self.functions.insert(Function {
            name,
            params: param_list,
            ret_type,
        });

        self.analyzing.current_scope = parent_scope;
    }

    fn leave_func_f_param(&mut self, node: FuncFParam) {
        let base_type = Self::build_basic_type(&node.ty().unwrap());

        // 处理指针声明 int *arr
        let mut param_type = if let Some(pointer_node) = node.pointer() {
            Self::build_pointer_type(&pointer_node, base_type)
        } else {
            base_type
        };

        // 处理数组参数 int arr[] 或 int arr[][3]
        if node.is_array() {
            let Some(ty) = self.build_array_type(param_type, node.indices()) else {
                return;
            };
            param_type = NType::Pointer(Box::new(ty));
        }

        let name_node = node.name().unwrap();
        let name = Self::extract_name(&name_node);
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
            &mut self.variable_map,
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
        // 待办：检查类型是否匹配
    }

    fn leave_assign_stmt(&mut self, _node: AssignStmt) {
        // 待办
    }

    fn enter_break_stmt(&mut self, _node: BreakStmt) {
        // 待办
    }

    fn leave_break_stmt(&mut self, _node: BreakStmt) {
        // 待办
    }

    fn enter_continue_stmt(&mut self, _node: ContinueStmt) {
        // 待办
    }

    fn leave_continue_stmt(&mut self, _node: ContinueStmt) {
        // 待办
    }

    // 检查返回类型
    fn leave_return_stmt(&mut self, _node: ReturnStmt) {}

    fn leave_binary_expr(&mut self, node: BinaryExpr) {
        let lhs = node.lhs().unwrap();
        let rhs = node.rhs().unwrap();
        let op = node.op().unwrap();
        let op_kind = op.op().kind();

        // 类型推导
        let lhs_ty = self.get_expr_type(lhs.syntax().text_range()).cloned();
        let rhs_ty = self.get_expr_type(rhs.syntax().text_range()).cloned();
        if let (Some(l), Some(r)) = (&lhs_ty, &rhs_ty) {
            let result_ty = match (l, r, op_kind) {
                (NType::Pointer(p), NType::Int, SyntaxKind::PLUS | SyntaxKind::MINUS) => {
                    NType::Pointer(p.clone())
                }
                (NType::Int, NType::Pointer(p), SyntaxKind::PLUS) => NType::Pointer(p.clone()),
                (NType::Pointer(_), NType::Pointer(_), SyntaxKind::MINUS) => NType::Int,
                (
                    _,
                    _,
                    SyntaxKind::LT
                    | SyntaxKind::GT
                    | SyntaxKind::LTEQ
                    | SyntaxKind::GTEQ
                    | SyntaxKind::EQEQ
                    | SyntaxKind::NEQ
                    | SyntaxKind::AMPAMP
                    | SyntaxKind::PIPEPIPE,
                ) => NType::Int,
                _ => l.clone(),
            };
            self.set_expr_type(node.syntax().text_range(), result_ty);
        }

        if self.is_constant(lhs.syntax().text_range())
            && self.is_constant(rhs.syntax().text_range())
        {
            let lhs_val = self.value_table.get(&lhs.syntax().text_range()).unwrap();
            let rhs_val = self.value_table.get(&rhs.syntax().text_range()).unwrap();

            if let Ok(val) = Value::eval(lhs_val, rhs_val, &op.op_str()) {
                self.mark_constant(node.syntax().text_range(), ConstKind::CompileTime);
                self.value_table.insert(node.syntax().text_range(), val);
            }
        }
    }

    fn leave_unary_expr(&mut self, node: UnaryExpr) {
        let expr = node.expr().unwrap();
        let op = node.op().unwrap();
        let op_kind = op.op().kind();

        // 类型推导
        if let Some(inner_ty) = self.get_expr_type(expr.syntax().text_range()) {
            let result_ty = if op_kind == SyntaxKind::AMP {
                NType::Pointer(Box::new(inner_ty.clone()))
            } else {
                inner_ty.clone()
            };
            self.set_expr_type(node.syntax().text_range(), result_ty);
        }

        // 取地址操作 & 是运行时常量
        if op_kind == SyntaxKind::AMP {
            self.mark_constant(node.syntax().text_range(), ConstKind::Runtime);
            return;
        }

        if self.is_constant(expr.syntax().text_range()) {
            let val = self
                .value_table
                .get(&expr.syntax().text_range())
                .unwrap()
                .clone();
            if let Ok(res) = Value::eval_unary(val, &op.op_str()) {
                self.mark_constant(node.syntax().text_range(), ConstKind::CompileTime);
                self.value_table.insert(node.syntax().text_range(), res);
            }
        }
    }

    fn leave_paren_expr(&mut self, node: ParenExpr) {
        let expr = node.expr().unwrap();
        // 类型传递
        if let Some(ty) = self.get_expr_type(expr.syntax().text_range()) {
            self.set_expr_type(node.syntax().text_range(), ty.clone());
        }
        if self.is_constant(expr.syntax().text_range()) {
            self.mark_constant(
                node.syntax().text_range(),
                self.get_const_kind(expr.syntax().text_range())
                    .unwrap_or(ConstKind::CompileTime),
            );
            let val = self
                .value_table
                .get(&expr.syntax().text_range())
                .unwrap()
                .clone();
            self.value_table.insert(node.syntax().text_range(), val);
        }
    }

    /// 解引用表达式得到的都不是常量
    fn leave_deref_expr(&mut self, node: DerefExpr) {
        let inner = node.expr().unwrap();
        if let Some(ty) = self.get_expr_type(inner.syntax().text_range()) {
            // 处理 Const(Pointer(...)) 和 Pointer(...) 两种情况
            let pointee: Option<NType> = match ty {
                NType::Pointer(pointee) => Some((*pointee).as_ref().clone()),
                NType::Const(inner) => {
                    if let NType::Pointer(pointee) = inner.as_ref() {
                        Some(pointee.as_ref().clone())
                    } else {
                        None
                    }
                }
                _ => None,
            };
            if let Some(pointee) = pointee {
                self.set_expr_type(node.syntax().text_range(), pointee);
            }
        }
    }

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

        // 计算索引后的类型
        let var = self.variables.get(*vid).unwrap();
        let index_count = node.indices().count();
        let result_ty = Self::compute_indexed_type(&var.ty, index_count);
        let is_const = var.is_const();
        let var_range = var.range;
        let const_zero = var.ty.const_zero();
        self.set_expr_type(node.syntax().text_range(), result_ty);

        if !is_const {
            return;
        }
        // const 变量可能没有编译时常量值（如 int *const p = &a）
        let Some(mut value) = self.value_table.get(&var_range) else {
            return;
        };

        if let Value::Array(tree) = value {
            let mut indices = Vec::new();
            for indice in node.indices() {
                let range = indice.syntax().text_range();
                let Some(v) = self.get_value(range) else {
                    return;
                };
                // 待办：如果非常量，应该进行类型检查
                let Value::Int(index) = v else {
                    self.analyzing.errors.push(SemanticError::TypeMismatch {
                        expected: NType::Int,
                        found: NType::Float,
                        range,
                    });
                    return;
                };
                indices.push(*index);
            }
            let leaf = match tree.get_leaf(&indices) {
                Ok(s) => s,
                Err(e) => {
                    self.analyzing.errors.push(SemanticError::ArrayError {
                        message: e,
                        range: node.syntax().text_range(),
                    });
                    return;
                }
            };
            value = match leaf {
                ArrayTreeValue::ConstExpr(expr) => {
                    // 运行时常量可能没有在 value_table 中
                    let Some(v) = self.value_table.get(&expr.syntax().text_range()) else {
                        return;
                    };
                    v
                }
                ArrayTreeValue::Empty => &const_zero,
                _ => unreachable!(),
            };
        }
        let range = node.syntax().text_range();
        self.value_table.insert(range, value.clone());
        self.mark_constant(range, ConstKind::CompileTime);
    }

    fn leave_const_expr(&mut self, node: ConstExpr) {
        let expr = node.expr().unwrap();
        let range = expr.syntax().text_range();

        // 检查父节点类型，只有在数组大小声明中才要求编译时常量
        // ConstInitVal 中的 ConstExpr 允许运行时值
        let parent = node.syntax().parent();
        let is_array_size = parent
            .as_ref()
            .map(|p| {
                p.kind() == SyntaxKind::CONST_INDEX_VAL || p.kind() == SyntaxKind::FUNC_F_PARAM
            })
            .unwrap_or(false);

        if is_array_size && !self.is_constant(range) {
            self.analyzing
                .errors
                .push(SemanticError::ConstantExprExpected { range });
        }
    }

    fn enter_literal(&mut self, node: Literal) {
        self.mark_constant(node.syntax().text_range(), ConstKind::CompileTime);
        let range = node.syntax().text_range();
        let v = if let Some(n) = node.float_token() {
            let s = n.text();
            self.set_expr_type(range, NType::Float);
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
            self.set_expr_type(range, NType::Int);
            Value::Int(i32::from_str_radix(num_str, radix).unwrap())
        };
        self.value_table.insert(range, v);
    }
}
