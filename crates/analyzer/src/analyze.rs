//! 主要进行类型推导和常量计算, 以及基本的检查
use parser::visitor::Visitor;
use syntax::{SyntaxKind, *};

use crate::array::{ArrayTree, ArrayTreeValue};
use crate::error::SemanticError;
use crate::module::{Module, ReferenceTag};
use crate::r#type::NType;
use crate::value::Value;

impl Visitor for Module {
    fn enter_comp_unit(&mut self, _node: CompUnit) {
        self.analyzing.current_scope = self.new_scope(None);
        self.global_scope = self.analyzing.current_scope;
    }

    fn enter_struct_def(&mut self, node: StructDef) {
        let Some(name_node) = node.name() else {
            return;
        };
        let Some(name) = name_node.var_name() else {
            return;
        };
        let Some(range) = name_node.var_range() else {
            return;
        };
        // 检查是否重复定义
        if self.find_struct(&name).is_some() {
            self.new_error(SemanticError::StructDefined {
                name: name.clone(),
                range,
            });
            return;
        }

        self.analyzing.current_scope = self.new_scope(Some(self.global_scope));

        // 提前创建占位，以支持自引用结构体
        let struct_id = self.new_struct(name.clone(), vec![], range);
        self.struct_map.insert(name, struct_id);
    }

    fn leave_struct_def(&mut self, node: StructDef) {
        // 获取 struct 名称
        let Some(Some(name)) = node.name().map(|n| n.var_name()) else {
            return;
        };
        // 获取已创建的 struct ID（在 enter_struct_def 中创建）
        let Some(struct_id) = self.find_struct(&name) else {
            return;
        };

        // 收集字段信息（先不创建变量）
        let mut field_infos = Vec::new();
        let mut field_names = std::collections::HashSet::new();

        for field_node in node.fields() {
            let Some(field_name_node) = field_node.name() else {
                continue;
            };
            let Some(field_name) = field_name_node.var_name() else {
                continue;
            };
            let Some(field_range) = field_name_node.var_range() else {
                continue;
            };
            let Some(ty_node) = field_node.ty() else {
                continue;
            };

            // 检查字段名是否重复
            if !field_names.insert(field_name.clone()) {
                self.new_error(SemanticError::VariableDefined {
                    name: field_name.clone(),
                    range: field_range,
                });
                continue;
            }

            // 获取字段类型（已经在 leave_type 中构建好）
            let field_ty = if let Some(ty) = self.get_expr_type(ty_node.text_range()) {
                ty.clone()
            } else {
                self.new_error(SemanticError::TypeUndefined {
                    range: ty_node.text_range(),
                });
                continue;
            };

            field_infos.push((field_name, field_ty, field_range));
        }

        // 创建字段变量
        let Some(scope) = self.scopes.get_mut(*self.analyzing.current_scope) else {
            return;
        };
        let Some(parent_scope) = scope.parent else {
            return;
        };
        let mut field_ids = Vec::new();
        for (field_name, field_ty, field_range) in field_infos {
            let field_id = scope.new_variable(
                &mut self.variables,
                &mut self.variable_map,
                field_name,
                field_ty,
                field_range,
            );
            field_ids.push(field_id);
        }

        // 更新 struct 定义的字段
        let struct_def = self.get_struct_mut_by_id(struct_id).unwrap();
        struct_def.fields = field_ids;

        self.analyzing.current_scope = parent_scope;
    }

    fn leave_var_def(&mut self, def: VarDef) {
        let Some(name_node) = def.name() else {
            return;
        };
        let Some(var_name) = name_node.var_name() else {
            return;
        };
        let Some(var_range) = name_node.var_range() else {
            return;
        };
        let Some(ty_node) = def.ty() else {
            return;
        };

        let var_type = if let Some(ty) = self.get_expr_type(ty_node.text_range()) {
            ty.clone()
        } else {
            self.new_error(SemanticError::TypeUndefined {
                range: ty_node.text_range(),
            });
            return;
        };

        let current_scope = self.analyzing.current_scope;
        let scope = self.scopes.get_mut(*current_scope).unwrap();
        let is_global = current_scope == self.global_scope;
        let is_const = var_type.is_const();
        if scope.have_variable_def(&var_name) {
            self.new_error(SemanticError::VariableDefined {
                name: var_name,
                range: var_range,
            });
            return;
        }

        // 处理初始值
        if let Some(init_val_node) = def.init() {
            // 如果是表达式，已经在 expr 处理，所以只用考虑 Array 和 Struct 类型
            let init_range = init_val_node.text_range();
            // 如果 InitVal 包含一个表达式，使用表达式的范围
            let expr_range = init_val_node
                .expr()
                .map(|e| e.text_range())
                .unwrap_or(init_range);
            if var_type.is_array() {
                let (array_tree, is_const_list) =
                    match ArrayTree::new(self, &var_type, init_val_node) {
                        Ok(s) => s,
                        Err(e) => {
                            self.new_error(SemanticError::ArrayError {
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
                        self.new_error(e);
                        return;
                    }
                }
            }

            match self.value_table.get(&expr_range) {
                Some(v) => {
                    // 如果是 const ，给变量设置一下初值
                    if is_const {
                        self.value_table.insert(var_range, v.clone());
                    }
                }
                None => {
                    // global 变量必须编译时能求值
                    if is_global {
                        self.new_error(SemanticError::ConstantExprExpected { range: init_range });
                        return;
                    }
                }
            }
        } else if is_const {
            // 如果是 const 必须要有初始化列表:w
            self.new_error(SemanticError::ExpectInitialVal {
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
        );
    }

    fn enter_func_def(&mut self, _: FuncDef) {
        self.analyzing.current_scope = self.new_scope(Some(self.analyzing.current_scope));
    }

    fn leave_func_def(&mut self, _: FuncDef) {
        let Some(scope) = self.scopes.get(*self.analyzing.current_scope) else {
            return;
        };
        let Some(parent_scope) = scope.parent else {
            return;
        };
        self.analyzing.current_scope = parent_scope;
        self.analyzing.current_function_ret_type = None;
    }

    fn leave_func_sign(&mut self, node: FuncSign) {
        let mut param_list = Vec::new();

        let Some(scope) = self.scopes.get(*self.analyzing.current_scope) else {
            return;
        };

        if let Some(params) = node.params() {
            for param in params.params() {
                let Some(name_node) = param.name() else {
                    return;
                };
                let Some(ident) = name_node.ident() else {
                    return;
                };
                let name = ident.text();
                let Some(v) = scope.look_up_variable(self, name) else {
                    return;
                };
                param_list.push(v);
            }
        }

        let ret_type = if let Some(ty_node) = node.ret_type() {
            if let Some(ty) = self.get_expr_type(ty_node.text_range()) {
                ty.clone()
            } else {
                self.new_error(SemanticError::TypeUndefined {
                    range: ty_node.text_range(),
                });
                return;
            }
        } else {
            NType::Void
        };

        let Some(name_node) = node.name() else {
            return;
        };
        let Some(name) = name_node.var_name() else {
            return;
        };
        let Some(range) = name_node.var_range() else {
            return;
        };

        let func_id = self.new_function(name.clone(), param_list, ret_type.clone(), range);
        self.function_map.insert(name, func_id);

        self.analyzing.current_function_ret_type = Some(ret_type);
    }

    fn leave_func_f_param(&mut self, node: FuncFParam) {
        let Some(ty_node) = node.ty() else {
            return;
        };

        let param_type = if let Some(ty) = self.get_expr_type(ty_node.text_range()) {
            ty.clone()
        } else {
            self.new_error(SemanticError::TypeUndefined {
                range: ty_node.text_range(),
            });
            return;
        };

        let Some(name_node) = node.name() else {
            return;
        };
        let Some(name) = name_node.var_name() else {
            return;
        };
        let Some(range) = name_node.var_range() else {
            return;
        };
        let scope = self.scopes.get_mut(*self.analyzing.current_scope).unwrap();

        if scope.have_variable_def(&name) {
            self.new_error(SemanticError::VariableDefined { name, range });
            return;
        }

        scope.new_variable(
            &mut self.variables,
            &mut self.variable_map,
            name,
            param_type,
            range,
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

    fn leave_call_expr(&mut self, node: CallExpr) {
        let Some(name_node) = node.name() else {
            return;
        };
        let Some(func_name) = name_node.var_name() else {
            return;
        };
        let Some(func_range) = name_node.var_range() else {
            return;
        };

        // FIXME: 内置函数列表（运行时库提供）
        let builtin_functions = [
            ("getint", 0, NType::Int),
            ("getch", 0, NType::Int),
            ("getfloat", 0, NType::Float),
            ("getarray", 1, NType::Int),
            ("getfarray", 1, NType::Int),
            ("putint", 1, NType::Void),
            ("putch", 1, NType::Void),
            ("putfloat", 1, NType::Void),
            ("putarray", 2, NType::Void),
            ("putfarray", 2, NType::Void),
            ("putf", -1, NType::Void), // 可变参数
            ("starttime", 0, NType::Void),
            ("stoptime", 0, NType::Void),
            ("_sysy_starttime", 1, NType::Void),
            ("_sysy_stoptime", 1, NType::Void),
        ];

        // 获取实际参数数量
        let arg_count = node.args().map(|args| args.args().count()).unwrap_or(0);

        // 检查是否为内置函数
        if let Some((_, expected_args, ret_type)) = builtin_functions
            .iter()
            .find(|(name, _, _)| *name == func_name)
        {
            // 设置返回类型
            self.set_expr_type(node.text_range(), ret_type.clone());

            // 检查参数数量（-1 表示可变参数，不检查）
            if *expected_args >= 0 && arg_count != *expected_args as usize {
                self.new_error(SemanticError::ArgumentCountMismatch {
                    function_name: func_name.clone(),
                    expected: *expected_args as usize,
                    found: arg_count,
                    range: func_range,
                });
            }
            return;
        }

        // 查找用户定义的函数
        let Some(func_id) = self.find_function(&func_name) else {
            self.new_error(SemanticError::FunctionUndefined {
                name: func_name,
                range: func_range,
            });
            return;
        };

        self.new_reference(func_range, ReferenceTag::FuncCall(func_id));

        let func = self.get_function_by_id(func_id).unwrap();
        let expected_arg_count = func.params.len();
        let ret_type = func.ret_type.clone();

        // 设置返回类型
        self.set_expr_type(node.text_range(), ret_type);

        // 检查参数数量（跳过正在定义的函数，因为参数列表还未完成）
        if arg_count != expected_arg_count {
            self.new_error(SemanticError::ArgumentCountMismatch {
                function_name: func_name,
                expected: expected_arg_count,
                found: arg_count,
                range: func_range,
            });
        }
    }

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

        let lhs_ty = self.get_expr_type(lhs.text_range()).cloned();
        let rhs_ty = self.get_expr_type(rhs.text_range()).cloned();
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
            self.set_expr_type(node.text_range(), result_ty);
        }

        if self.is_compile_time_constant(lhs.text_range())
            && self.is_compile_time_constant(rhs.text_range())
        {
            let lhs_val = self.value_table.get(&lhs.text_range()).unwrap();
            let rhs_val = self.value_table.get(&rhs.text_range()).unwrap();

            if let Ok(val) = Value::eval(lhs_val, rhs_val, &op.op_str()) {
                self.value_table.insert(node.text_range(), val);
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

        if let Some(inner_ty) = self.get_expr_type(expr.text_range()) {
            let result_ty = if op_kind == SyntaxKind::AMP {
                if !self.is_lvalue_expr(&expr) {
                    self.new_error(SemanticError::AddressOfRight {
                        range: expr.text_range(),
                    });

                    return;
                }
                // 取地址操作，默认生成 *mut 指针
                NType::Pointer {
                    pointee: Box::new(inner_ty.clone()),
                    is_const: false,
                }
            } else if op_kind == SyntaxKind::STAR {
                let pointee: Option<NType> = match inner_ty {
                    NType::Pointer { pointee, .. } => Some((*pointee).as_ref().clone()),
                    NType::Const(inner) => {
                        if let NType::Pointer { pointee, .. } = inner.as_ref() {
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
                    self.new_error(SemanticError::ApplyOpOnType {
                        ty: inner_ty.clone(),
                        op: "*".to_string(),
                        range: expr.text_range(),
                    });
                    return;
                }
            } else {
                inner_ty.clone()
            };
            self.set_expr_type(node.text_range(), result_ty);
        }

        if matches!(op_kind, SyntaxKind::STAR | SyntaxKind::AMP) {
            return;
        }

        if self.is_compile_time_constant(expr.text_range()) {
            let val = self.value_table.get(&expr.text_range()).unwrap().clone();
            if let Ok(res) = Value::eval_unary(val, &op.op_str()) {
                self.value_table.insert(node.text_range(), res);
            }
        }
    }

    fn leave_paren_expr(&mut self, node: ParenExpr) {
        let Some(expr) = node.expr() else {
            return;
        };
        if let Some(ty) = self.get_expr_type(expr.text_range()) {
            self.set_expr_type(node.text_range(), ty.clone());
        }
        let expr_range = expr.text_range();
        if self.is_compile_time_constant(expr_range) {
            let val = self.value_table.get(&expr_range).unwrap().clone();
            self.value_table.insert(node.text_range(), val);
        }
    }

    fn leave_index_val(&mut self, node: IndexVal) {
        let Some(name_node) = node.name() else {
            return;
        };
        let Some(var_name) = name_node.var_name() else {
            return;
        };
        let Some(var_range) = name_node.var_range() else {
            return;
        };

        // 查找变量定义
        let Some(var_id) = self.find_variable_def(&var_name) else {
            self.new_error(SemanticError::VariableUndefined {
                name: var_name.to_string(),
                range: var_range,
            });
            return;
        };

        let node_range = node.text_range();
        // 记录 Read 引用
        self.new_reference(node_range, ReferenceTag::VarRead(var_id));

        let var = self.variables.get(*var_id).unwrap();
        let index_count = node.indices().count();
        let result_ty = match Self::compute_indexed_type(&var.ty, index_count, node_range) {
            Ok(ty) => ty,
            Err(e) => {
                self.new_error(e);
                return;
            }
        };
        let is_const = var.ty.is_const() && result_ty.is_const();
        let var_range = var.range;
        let const_zero = var.ty.const_zero();
        self.set_expr_type(node_range, result_ty);

        if !is_const {
            return;
        }
        let Some(mut value) = self.value_table.get(&var_range) else {
            return;
        };

        if let Value::Array(tree) = value {
            let mut indices = Vec::new();
            for indice in node.indices() {
                let range = indice.text_range();
                let Some(v) = self.get_value_by_range(range) else {
                    return;
                };
                let Value::Int(index) = v else {
                    self.new_error(SemanticError::TypeMismatch {
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
                    self.new_error(SemanticError::ArrayError {
                        message: Box::new(e),
                        range: node.text_range(),
                    });
                    return;
                }
            };
            value = match leaf {
                ArrayTreeValue::Expr(expr_range) => {
                    let Some(v) = self.value_table.get(&expr_range) else {
                        return;
                    };
                    v
                }
                ArrayTreeValue::Struct {
                    init_list: init_val_node_range,
                    ..
                } => {
                    let Some(v) = self.value_table.get(&init_val_node_range) else {
                        return;
                    };
                    v
                }
                ArrayTreeValue::Empty => &const_zero,
            };
        }
        let range = node.text_range();
        self.value_table.insert(range, value.clone());
    }

    fn leave_postfix_expr(&mut self, node: PostfixExpr) {
        let range = node.text_range();

        // 获取操作符类型
        let Some(op_node) = node.op() else {
            return;
        };
        let op_kind = op_node.op().kind();

        // 获取成员 FieldAccess（可能包含数组索引）
        let Some(field_access_node) = node.field() else {
            return;
        };
        let filed_access_range = field_access_node.text_range();
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

        let base_range = base_expr.text_range();
        let Some(base_ty) = self.get_expr_type(base_range) else {
            return;
        };

        // 根据操作符提取 struct ID
        let struct_id = match op_kind {
            SyntaxKind::DOT => {
                // 直接成员访问：左操作数必须是 struct 类型
                if let Some(id) = base_ty.as_struct_id() {
                    id
                } else {
                    self.new_error(SemanticError::NotAStruct {
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
                    self.new_error(SemanticError::NotAStructPointer {
                        ty: base_ty.clone(),
                        range: base_range,
                    });
                    return;
                }
            }
            _ => {
                self.new_error(SemanticError::ApplyOpOnType {
                    ty: base_ty.clone(),
                    op: op_node.op().text().to_string(),
                    range: base_range,
                });
                return;
            }
        };

        // 查找 struct 定义
        let struct_def = self.get_struct_by_id(struct_id).unwrap();
        let struct_def: *const crate::module::Struct = struct_def;

        // 查找字段并设置类型
        if let Some(field_id) = unsafe { &*struct_def }.field(self, &member_name) {
            let field = self.variables.get(*field_id).unwrap();
            // 计算索引后的类型（如果有数组索引）
            let indices: Vec<_> = field_access_node.indices().collect();
            let result_ty = if indices.is_empty() {
                field.ty.clone()
            } else {
                // 有数组索引，需要计算索引后的类型
                match Self::compute_indexed_type(&field.ty, indices.len(), filed_access_range) {
                    Ok(ty) => ty,
                    Err(e) => {
                        self.new_error(e);
                        return;
                    }
                }
            };
            // 如果 base_ty 是 const，需要继承
            let result_ty = if base_ty.is_const() && !result_ty.is_const() {
                NType::Const(Box::new(result_ty))
            } else {
                result_ty
            };

            self.set_expr_type(range, result_ty);

            self.new_reference(filed_access_range, ReferenceTag::VarRead(field_id));

            // 常量处理：如果基础表达式是常量 struct，提取字段值
            if let Some(Value::Struct(_struct_id, field_values)) =
                self.value_table.get(&base_range).cloned()
                && let Some(field_idx) = unsafe { &*struct_def }.field_index(self, &member_name)
                && let Some(field_value) = field_values.get(field_idx as usize)
            {
                if indices.is_empty() {
                    self.value_table.insert(range, field_value.clone());
                } else if let Value::Array(tree) = field_value {
                    // 处理数组索引
                    let mut idx_values = Vec::new();
                    for idx_expr in field_access_node.indices() {
                        let idx_range = idx_expr.text_range();
                        if let Some(Value::Int(idx)) = self.get_value_by_range(idx_range) {
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
            self.new_error(SemanticError::FieldNotFound {
                struct_name: unsafe { &*struct_def }.name.clone(),
                field_name: member_name,
                range,
            });
        }
    }

    fn enter_literal(&mut self, node: Literal) {
        let range = node.text_range();
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

    fn leave_type(&mut self, node: Type) {
        let range = node.text_range();

        let ntype = if node.l_brack_token().is_some() {
            // 数组类型: [Type; Expr]
            let inner_type_node = node.inner_type();
            let size_expr_node = node.size_expr();

            let inner = if let Some(inner_node) = inner_type_node {
                if let Some(ty) = self.get_expr_type(inner_node.text_range()) {
                    ty.clone()
                } else {
                    return;
                }
            } else {
                return;
            };

            let size = if let Some(expr_node) = size_expr_node {
                let expr_range = expr_node.text_range();
                if let Some(x) = self.get_value_by_range(expr_range).cloned() {
                    if let Value::Int(n) = x {
                        n
                    } else {
                        self.new_error(SemanticError::TypeMismatch {
                            expected: NType::Const(Box::new(NType::Int)),
                            found: x.get_type(),
                            range: expr_range,
                        });
                        return;
                    }
                } else {
                    return;
                }
            } else {
                return;
            };

            NType::Array(Box::new(inner), size)
        } else if let Some(pointer) = node.pointer() {
            // 指针类型: Pointer BaseType
            let inner_type_node = node.inner_type();

            let inner = if let Some(inner_node) = inner_type_node {
                if let Some(ty) = self.get_expr_type(inner_node.text_range()) {
                    ty.clone()
                } else {
                    return;
                }
            } else {
                return;
            };

            NType::Pointer {
                pointee: Box::new(inner),
                is_const: pointer.is_const(),
            }
        } else {
            // 原始类型: PrimitType
            let primit_type_node = node.primit_type();

            let ntype = if let Some(pt_node) = primit_type_node {
                if pt_node.int_token().is_some() {
                    NType::Int
                } else if pt_node.float_token().is_some() {
                    NType::Float
                } else if pt_node.void_token().is_some() {
                    NType::Void
                } else if pt_node.struct_token().is_some() {
                    let name_node = pt_node.name();
                    if let Some(Some(name)) = name_node.map(|n| n.var_name()) {
                        if let Some(sid) = self.find_struct(&name) {
                            NType::Struct(sid)
                        } else {
                            self.new_error(SemanticError::TypeUndefined { range });
                            return;
                        }
                    } else {
                        return;
                    }
                } else {
                    return;
                }
            } else {
                return;
            };

            if node.const_token().is_some() {
                NType::Const(Box::new(ntype))
            } else {
                ntype
            }
        };

        self.set_expr_type(range, ntype.clone());
    }
}
