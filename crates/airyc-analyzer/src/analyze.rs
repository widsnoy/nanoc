//! 主要进行类型推导和常量计算, 以及基本的检查
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

    fn leave_struct_def(&mut self, node: StructDef) {
        let range = node.syntax().text_range();

        // 获取 struct 名称
        let Some(Some(name)) = node.name().map(|n| n.var_name()) else {
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
            // 获取字段基本类型
            let base_ty = if let Some(ty_node) = field_node.ty() {
                let Some(base_ty) = self.build_basic_type(&ty_node) else {
                    self.analyzing.errors.push(SemanticError::TypeUndefined {
                        range: ty_node.syntax().text_range(),
                    });
                    continue;
                };
                if let Some(pointer_node) = field_node.pointer() {
                    Self::build_pointer_type(&pointer_node, base_ty)
                } else {
                    base_ty
                }
            } else {
                continue;
            };

            // 获取字段名称和数组维度
            if let Some(array_decl) = field_node.array_decl()
                && let Some(field_name_node) = array_decl.name()
            {
                let Some(field_name) = field_name_node.var_name() else {
                    return;
                };

                // 检查字段名是否重复
                if !field_names.insert(field_name.clone()) {
                    self.analyzing.errors.push(SemanticError::VariableDefined {
                        name: field_name.clone(),
                        range: field_name_node.syntax().text_range(),
                    });
                    continue;
                }

                // 处理数组维度，构建完整的字段类型
                let field_ty = match self.build_array_type(base_ty.clone(), array_decl.dimensions())
                {
                    Ok(ty) => ty,
                    Err(e) => {
                        self.analyzing.new_error(e);
                        return;
                    }
                };

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
        let Some(ty_node) = node.ty() else {
            return;
        };
        let Some(base_type) = self.build_basic_type(&ty_node) else {
            self.analyzing.errors.push(SemanticError::TypeUndefined {
                range: ty_node.syntax().text_range(),
            });
            return;
        };
        // 如果是 const 声明，将类型包装为 Const
        self.analyzing.current_base_type = if node.is_const() {
            Some(NType::Const(Box::new(base_type)))
        } else {
            Some(base_type)
        };
    }

    fn leave_var_def(&mut self, def: VarDef) {
        let Some(base_type) = self.analyzing.current_base_type.clone() else {
            return;
        };
        let var_type = if let Some(pointer_node) = def.pointer() {
            Self::build_pointer_type(&pointer_node, base_type)
        } else {
            base_type
        };

        let Some(array_decl_node) = def.array_decl() else {
            return;
        };

        let var_type = match self.build_array_type(var_type, array_decl_node.dimensions()) {
            Ok(ty) => ty,
            Err(e) => {
                self.analyzing.new_error(e);
                return;
            }
        };
        let Some(name_node) = array_decl_node.name() else {
            return;
        };
        let Some(var_name) = name_node.var_name() else {
            return;
        };
        let Some(var_range) = name_node.var_range() else {
            return;
        };

        let current_scope = self.analyzing.current_scope;
        let scope = self.scopes.get_mut(*current_scope).unwrap();
        let is_global = current_scope == self.global_scope;
        let is_const = var_type.is_const();
        if scope.have_variable(&var_name) {
            self.analyzing.errors.push(SemanticError::VariableDefined {
                name: var_name,
                range: var_range,
            });
            return;
        }

        // 处理初始值
        if let Some(init_val_node) = def.init() {
            // 如果是表达式，已经在 expr 处理，所以只用考虑 Array 和 Struct 类型
            let init_range = init_val_node.syntax().text_range();
            if var_type.is_array() {
                let (array_tree, is_const_list) =
                    match ArrayTree::new(self, &var_type, init_val_node) {
                        Ok(s) => s,
                        Err(e) => {
                            self.analyzing.errors.push(SemanticError::ArrayError {
                                message: Box::new(e),
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
            } else if var_type.is_struct() {
                let struct_id = var_type.as_struct_id().unwrap();
                match self.process_struct_init_value(struct_id, init_val_node) {
                    Ok(Some(value)) => {
                        self.value_table.insert(init_range, value);
                    }
                    Ok(None) => {}
                    Err(e) => {
                        self.analyzing.new_error(e);
                        return;
                    }
                }
            }

            match self.value_table.get(&init_range) {
                Some(v) => {
                    // 如果是 const ，给变量设置一下初值
                    if is_const {
                        self.value_table.insert(var_range, v.clone());
                    }
                }
                None => {
                    // global 变量必须编译时能求值
                    if is_global {
                        self.analyzing
                            .errors
                            .push(SemanticError::ConstantExprExpected { range: init_range });
                        return;
                    }
                }
            }
        } else if is_const {
            // 如果是 const 必须要有初始化列表:w
            self.analyzing.errors.push(SemanticError::ExpectInitialVal {
                name: var_name,
                range: var_range,
            });
            return;
        }

        let scope = self.scopes.get_mut(*self.analyzing.current_scope).unwrap();
        let _ = scope.new_variable(
            &mut self.variables,
            &mut self.variable_map,
            var_name,
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
                let Some(name_node) = param.name() else {
                    return;
                };
                let Some(ident) = name_node.ident() else {
                    return;
                };
                let name = ident.text();
                let Some(v) = scope.look_up(self, name, VariableTag::Define) else {
                    return;
                };
                param_list.push(v);
            }
        }

        let Some(func_type_node) = node.func_type() else {
            return;
        };
        let Some(ret_type) = self.build_func_type(&func_type_node) else {
            return;
        };
        self.set_expr_type(func_type_node.syntax().text_range(), ret_type.clone());

        let Some(name_node) = node.name() else {
            return;
        };
        let Some(ident) = name_node.ident() else {
            return;
        };
        let name = ident.text().to_string();
        self.functions.insert(Function {
            name,
            params: param_list,
            ret_type,
        });

        self.analyzing.current_scope = parent_scope;
    }

    fn leave_func_f_param(&mut self, node: FuncFParam) {
        let Some(ty_node) = node.ty() else {
            return;
        };
        let Some(base_type) = self.build_basic_type(&ty_node) else {
            return;
        };

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
            let ty = match self.build_array_type(param_type, node.indices()) {
                Ok(ty) => ty,
                Err(e) => {
                    self.analyzing.new_error(e);
                    return;
                }
            };
            param_type = NType::Pointer(Box::new(ty));
        }

        let Some(name_node) = node.name() else {
            return;
        };
        let Some(name) = name_node.var_name() else {
            return;
        };
        let Some(ident) = name_node.ident() else {
            return;
        };
        let range = ident.text_range();
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
        let Some(lhs) = node.lhs() else {
            return;
        };
        let Some(rhs) = node.rhs() else {
            return;
        };
        let Some(op) = node.op() else {
            return;
        };
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
        let Some(expr) = node.expr() else {
            return;
        };
        let Some(op) = node.op() else {
            return;
        };
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
        let Some(expr) = node.expr() else {
            return;
        };
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
        let result_ty = match Self::compute_indexed_type(&var.ty, index_count) {
            Ok(ty) => ty,
            Err(e) => {
                self.analyzing.new_error(e);
                return;
            }
        };
        let is_const = var.ty.is_const() && result_ty.is_const();
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
                        message: Box::new(e),
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
                ArrayTreeValue::Struct {
                    init_list: init_val_node,
                    ..
                } => {
                    let Some(v) = self.value_table.get(&init_val_node.syntax().text_range()) else {
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

        // 获取成员 FieldAccess（可能包含数组索引）
        let Some(field_access_node) = node.field() else {
            return;
        };
        let Some(name_node) = field_access_node.name() else {
            return;
        };
        let Some(member_name) = name_node.var_name() else {
            return;
        };

        // 获取左操作数的类型
        let Some(base_expr) = node.expr() else {
            return;
        };

        let base_range = base_expr.syntax().text_range();
        let Some(base_ty) = self.get_expr_type(base_range) else {
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
            _ => unreachable!(),
        };

        // 查找 struct 定义
        let struct_def = self.get_struct(struct_id).unwrap();

        let struct_def: *const crate::module::StructDef = struct_def;

        // 查找字段并设置类型
        if let Some(field) = unsafe { &*struct_def }.field(&member_name) {
            // 计算索引后的类型（如果有数组索引）
            let indices: Vec<_> = field_access_node.indices().collect();
            let result_ty = if indices.is_empty() {
                field.ty.clone()
            } else {
                // 有数组索引，需要计算索引后的类型
                match Self::compute_indexed_type(&field.ty, indices.len()) {
                    Ok(ty) => ty,
                    Err(e) => {
                        self.analyzing.new_error(e);
                        return;
                    }
                }
            };
            self.set_expr_type(range, result_ty.clone());

            // 常量处理：如果基础表达式是常量 struct，提取字段值
            if let Some(Value::Struct(_struct_id, field_values)) =
                self.value_table.get(&base_range).cloned()
                && let Some(field_idx) = unsafe { &*struct_def }.field_index(&member_name)
                && let Some(field_value) = field_values.get(field_idx as usize)
            {
                if indices.is_empty() {
                    self.value_table.insert(range, field_value.clone());
                } else if let Value::Array(tree) = field_value {
                    // 处理数组索引
                    let mut idx_values = Vec::new();
                    for idx_expr in field_access_node.indices() {
                        let idx_range = idx_expr.syntax().text_range();
                        if let Some(Value::Int(idx)) = self.get_value(idx_range) {
                            idx_values.push(*idx);
                        } else {
                            return; // 索引不是常量
                        }
                    }
                    if let Ok(leaf) = tree.get_leaf(&idx_values)
                        && let Some(v) = leaf.get_const_value(&self.value_table)
                    {
                        self.value_table.insert(range, v.clone());
                    }
                }
            }
        } else {
            self.analyzing.errors.push(SemanticError::FieldNotFound {
                struct_name: unsafe { &*struct_def }.name.clone(),
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
            let Some(n) = node.int_token() else {
                return;
            };
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
            Value::Int(match i32::from_str_radix(num_str, radix) {
                Ok(v) => v,
                Err(_) => return,
            })
        };
        self.value_table.insert(range, v);
    }
}
