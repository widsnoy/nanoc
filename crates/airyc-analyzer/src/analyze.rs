use airyc_parser::ast::*;
use airyc_parser::syntax_kind::SyntaxKind;
use airyc_parser::visitor::Visitor;

use crate::array::{ArrayTree, ArrayTreeValue};
use crate::module::{Function, Module, SemanticError, VariableTag};
use crate::r#type::NType;
use crate::value::Value;

impl Visitor for Module {
    fn enter_comp_unit(&mut self, _node: CompUnit) {
        self.analyzing.current_scope = self.new_scope(None);
        self.global_scope = self.analyzing.current_scope;
    }

    fn enter_struct_def(&mut self, node: StructDef) {
        let range = node.syntax().text_range();

        // 获取 struct 名称
        let name = if let Some(name_node) = node.name() {
            Self::extract_name(&name_node)
        } else {
            self.analyzing.errors.push(SemanticError::StructUndefined {
                name: "<unnamed>".to_string(),
                range,
            });
            return;
        };

        // 检查是否重复定义
        if self.find_struct(&name).is_some() {
            self.analyzing.errors.push(SemanticError::StructDefined {
                name: name.clone(),
                range,
            });
            return;
        }

        // 收集字段
        let mut fields = Vec::new();
        let mut field_names = std::collections::HashSet::new();

        for field_node in node.fields() {
            // 获取字段类型
            let field_ty = if let Some(ty_node) = field_node.ty() {
                let base_ty = self.build_basic_type(&ty_node);
                if let Some(pointer_node) = field_node.pointer() {
                    Self::build_pointer_type(&pointer_node, base_ty)
                } else {
                    base_ty
                }
            } else {
                continue;
            };

            // 获取字段名称
            if let Some(index_val) = field_node.index_val()
                && let Some(field_name_node) = index_val.name()
            {
                let field_name = Self::extract_name(&field_name_node);

                // 检查字段名是否重复
                if !field_names.insert(field_name.clone()) {
                    self.analyzing.errors.push(SemanticError::VariableDefined {
                        name: field_name.clone(),
                        range: field_name_node.syntax().text_range(),
                    });
                    continue;
                }

                fields.push(crate::module::StructField {
                    name: field_name,
                    ty: field_ty,
                });
            }
        }

        // 添加 struct 定义
        let struct_id = self.new_struct(name.clone(), fields, range);
        self.struct_map.insert(name, struct_id);
    }

    fn enter_var_decl(&mut self, node: VarDecl) {
        let base_type = self.build_basic_type(&node.ty().unwrap());
        // 如果是 const 声明，将类型包装为 Const
        self.analyzing.current_base_type = if node.is_const() {
            Some(NType::Const(Box::new(base_type)))
        } else {
            Some(base_type)
        };
    }

    fn leave_var_def(&mut self, def: VarDef) {
        let base_type = self.analyzing.current_base_type.clone().unwrap();
        let is_const = base_type.is_const();
        let var_type = if let Some(pointer_node) = def.pointer() {
            Self::build_pointer_type(&pointer_node, base_type)
        } else {
            base_type
        };

        let index_val_node = def.index_val().unwrap();

        // 检查数组维度是否为编译时常量
        // FIXME: 如果是局部变量也可以不是
        for expr in index_val_node.indices() {
            let range = expr.syntax().text_range();
            if !self.is_compile_time_constant(range) {
                self.analyzing
                    .errors
                    .push(SemanticError::ConstantExprExpected { range });
                return;
            }
        }

        let var_type = self
            .build_array_type(var_type, index_val_node.indices())
            .unwrap();
        let index_val_node = def.index_val().unwrap();
        let name_node = index_val_node.name().unwrap();
        let name = Self::extract_name(&name_node);
        let var_range = name_node.ident().unwrap().text_range();

        let current_scope = self.analyzing.current_scope;
        let scope = self.scopes.get_mut(*current_scope).unwrap();
        let is_global = current_scope == self.global_scope;
        if scope.have_variable(&name) {
            self.analyzing.errors.push(SemanticError::VariableDefined {
                name,
                range: var_range,
            });
            return;
        }

        if let Some(init_val_node) = def.init() {
            // 如果是表达式，已经在 expr 处理。这里只用考虑数组初始列表
            let init_range = init_val_node.syntax().text_range();
            if var_type.is_array() {
                let (array_tree, is_const_list) =
                    match ArrayTree::new(self, &var_type, init_val_node) {
                        Ok(s) => s,
                        Err(e) => {
                            self.analyzing.errors.push(SemanticError::ArrayError {
                                message: e,
                                range: init_range,
                            });
                            return;
                        }
                    };
                if is_const_list {
                    self.value_table
                        .insert(init_range, Value::Array(array_tree.clone()));
                }
                self.expand_array.insert(init_range, array_tree);
            }
            if let Some(value) = self.value_table.get(&init_range) {
                self.value_table.insert(var_range, value.clone());
            } else if is_global {
                self.analyzing
                    .errors
                    .push(SemanticError::ConstantExprExpected { range: init_range });
                return;
            };
        } else if is_const {
            self.analyzing.errors.push(SemanticError::ExpectInitialVal {
                name,
                range: var_range,
            });
            return;
        }

        let scope = self.scopes.get_mut(*self.analyzing.current_scope).unwrap();
        let _ = scope.new_variable(
            &mut self.variables,
            &mut self.variable_map,
            name,
            var_type,
            var_range,
            VariableTag::Define,
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
                };
                param_list.push(v);
            }
        }

        let func_type_node = node.func_type().unwrap();
        let ret_type = self.build_func_type(&func_type_node);
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
        let base_type = self.build_basic_type(&node.ty().unwrap());

        let mut param_type = if let Some(pointer_node) = node.pointer() {
            Self::build_pointer_type(&pointer_node, base_type)
        } else {
            base_type
        };

        if node.is_array() {
            for expr in node.indices() {
                let range = expr.syntax().text_range();
                if !self.is_compile_time_constant(range) {
                    self.analyzing
                        .errors
                        .push(SemanticError::ConstantExprExpected { range });
                    return;
                }
            }
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

    fn enter_assign_stmt(&mut self, _node: AssignStmt) {}
    fn leave_assign_stmt(&mut self, _node: AssignStmt) {}
    fn enter_break_stmt(&mut self, _node: BreakStmt) {}
    fn leave_break_stmt(&mut self, _node: BreakStmt) {}
    fn enter_continue_stmt(&mut self, _node: ContinueStmt) {}
    fn leave_continue_stmt(&mut self, _node: ContinueStmt) {}
    fn leave_return_stmt(&mut self, _node: ReturnStmt) {}

    fn leave_binary_expr(&mut self, node: BinaryExpr) {
        let lhs = node.lhs().unwrap();
        let rhs = node.rhs().unwrap();
        let op = node.op().unwrap();
        let op_kind = op.op().kind();

        let lhs_ty = self.get_expr_type(lhs.syntax().text_range()).cloned();
        let rhs_ty = self.get_expr_type(rhs.syntax().text_range()).cloned();
        if let (Some(l), Some(r)) = (&lhs_ty, &rhs_ty) {
            let result_ty = match op_kind {
                SyntaxKind::PLUS | SyntaxKind::MINUS
                    if l.is_pointer() && matches!(r, NType::Int) =>
                {
                    l.clone()
                }
                SyntaxKind::PLUS if matches!(l, NType::Int) && r.is_pointer() => r.clone(),
                SyntaxKind::MINUS if l.is_pointer() && r.is_pointer() => NType::Int,
                SyntaxKind::LT
                | SyntaxKind::GT
                | SyntaxKind::LTEQ
                | SyntaxKind::GTEQ
                | SyntaxKind::EQEQ
                | SyntaxKind::NEQ
                | SyntaxKind::AMPAMP
                | SyntaxKind::PIPEPIPE => NType::Int,
                _ => l.clone(),
            };
            self.set_expr_type(node.syntax().text_range(), result_ty);
        }

        if self.is_compile_time_constant(lhs.syntax().text_range())
            && self.is_compile_time_constant(rhs.syntax().text_range())
        {
            let lhs_val = self.value_table.get(&lhs.syntax().text_range()).unwrap();
            let rhs_val = self.value_table.get(&rhs.syntax().text_range()).unwrap();

            if let Ok(val) = Value::eval(lhs_val, rhs_val, &op.op_str()) {
                self.value_table.insert(node.syntax().text_range(), val);
            }
        }
    }

    fn leave_unary_expr(&mut self, node: UnaryExpr) {
        let expr = node.expr().unwrap();
        let op = node.op().unwrap();
        let op_kind = op.op().kind();

        if let Some(inner_ty) = self.get_expr_type(expr.syntax().text_range()) {
            let result_ty = if op_kind == SyntaxKind::AMP {
                NType::Pointer(Box::new(inner_ty.clone()))
            } else if op_kind == SyntaxKind::STAR {
                let pointee: Option<NType> = match inner_ty {
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
                    pointee
                } else {
                    unreachable!("");
                }
            } else {
                inner_ty.clone()
            };
            self.set_expr_type(node.syntax().text_range(), result_ty);
        }

        if matches!(op_kind, SyntaxKind::STAR | SyntaxKind::AMP) {
            return;
        }

        if self.is_compile_time_constant(expr.syntax().text_range()) {
            let val = self
                .value_table
                .get(&expr.syntax().text_range())
                .unwrap()
                .clone();
            if let Ok(res) = Value::eval_unary(val, &op.op_str()) {
                self.value_table.insert(node.syntax().text_range(), res);
            }
        }
    }

    fn leave_paren_expr(&mut self, node: ParenExpr) {
        let expr = node.expr().unwrap();
        if let Some(ty) = self.get_expr_type(expr.syntax().text_range()) {
            self.set_expr_type(node.syntax().text_range(), ty.clone());
        }
        let expr_range = expr.syntax().text_range();
        if self.is_compile_time_constant(expr_range) {
            let val = self.value_table.get(&expr_range).unwrap().clone();
            self.value_table.insert(node.syntax().text_range(), val);
        }
    }

    fn leave_index_val(&mut self, node: IndexVal) {
        // 如果 IndexVal 是在变量声明中（父节点是 VarDef），则跳过
        // 因为变量定义由 leave_var_def 处理
        if let Some(parent) = node.syntax().parent()
            && parent.kind() == SyntaxKind::VAR_DEF
        {
            return;
        }

        // 如果 IndexVal 是在 struct 字段定义中（父节点是 STRUCT_FIELD），则跳过
        // 因为 struct 字段由 enter_struct_def 处理
        if let Some(parent) = node.syntax().parent()
            && parent.kind() == SyntaxKind::STRUCT_FIELD
        {
            return;
        }

        let Some(name_node) = node.name() else {
            return;
        };
        let Some(ident_token) = name_node.ident() else {
            return;
        };
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

        let var = self.variables.get(*vid).unwrap();
        let index_count = node.indices().count();
        let result_ty = Self::compute_indexed_type(&var.ty, index_count);
        let is_const = result_ty.is_const();
        let var_range = var.range;
        let const_zero = var.ty.const_zero();
        self.set_expr_type(node.syntax().text_range(), result_ty);

        if !is_const {
            return;
        }
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
                ArrayTreeValue::Expr(expr) => {
                    let Some(v) = self.value_table.get(&expr.syntax().text_range()) else {
                        return;
                    };
                    v
                }
                ArrayTreeValue::Empty => &const_zero,
            };
        }
        let range = node.syntax().text_range();
        self.value_table.insert(range, value.clone());
    }

    fn leave_postfix_expr(&mut self, node: PostfixExpr) {
        let range = node.syntax().text_range();

        // 获取操作符类型
        let Some(op_node) = node.op() else {
            return;
        };
        let op = op_node.op().kind();

        // 获取成员名称
        let Some(name_node) = node.name() else {
            return;
        };
        let member_name = Self::extract_name(&name_node);

        // 获取左操作数的类型
        let Some(base_expr) = node.expr() else {
            return;
        };

        let base_range = base_expr.syntax().text_range();
        let Some(base_ty) = self.get_expr_type(base_range).cloned() else {
            // 基础表达式类型未知，无法继续
            return;
        };

        // 根据操作符提取 struct ID
        let struct_id = match op {
            SyntaxKind::DOT => {
                // 直接成员访问：左操作数必须是 struct 类型
                if let Some(id) = base_ty.as_struct_id() {
                    id
                } else {
                    self.analyzing.errors.push(SemanticError::NotAStruct {
                        ty: base_ty.clone(),
                        range: base_range,
                    });
                    return;
                }
            }
            SyntaxKind::ARROW => {
                // 指针成员访问：左操作数必须是 struct 指针类型
                if let Some(id) = base_ty.as_struct_pointer_id() {
                    id
                } else {
                    self.analyzing
                        .errors
                        .push(SemanticError::NotAStructPointer {
                            ty: base_ty.clone(),
                            range: base_range,
                        });
                    return;
                }
            }
            _ => return,
        };

        // 查找 struct 定义
        let Some(struct_def) = self.get_struct(struct_id).cloned() else {
            // Struct 未定义（可能是前向引用）
            return;
        };

        // 查找字段并设置类型
        if let Some(field) = struct_def.field(&member_name) {
            self.set_expr_type(range, field.ty.clone());
        } else {
            self.analyzing.errors.push(SemanticError::FieldNotFound {
                struct_name: struct_def.name.clone(),
                field_name: member_name,
                range,
            });
        }
    }

    fn enter_literal(&mut self, node: Literal) {
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
