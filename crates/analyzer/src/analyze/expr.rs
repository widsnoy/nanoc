//! 表达式相关的语义分析

use syntax::SyntaxKind;
use syntax::ast::*;
use syntax::visitor::ExprVisitor;

use crate::array::ArrayTreeValue;
use crate::error::SemanticError;
use crate::module::{Module, ReferenceTag};
use crate::r#type::NType;
use crate::value::Value;

impl ExprVisitor for Module {
    fn leave_call_expr(&mut self, node: CallExpr) {
        let Some((func_name, func_range)) =
            node.name().and_then(|n| utils::extract_name_and_range(&n))
        else {
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
        let Some(func_id) = self.get_function_id_by_name(&func_name) else {
            self.new_error(SemanticError::FunctionUndefined {
                name: func_name,
                range: func_range,
            });
            return;
        };

        self.new_reference(func_range, ReferenceTag::FuncCall(func_id));

        // 获取函数定义（支持跨模块访问）
        let Some(func) = self.get_function_by_id(func_id) else {
            // 理论上不应该发生（函数 ID 存在但找不到定义）
            debug_assert!(false, "Function {:?} not found", func_id);
            return;
        };

        // 设置返回类型
        self.set_expr_type(node.text_range(), func.ret_type.clone());
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
                        range: utils::trim_node_text_range(&expr),
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
        let Some((var_name, var_range)) =
            node.name().and_then(|n| utils::extract_name_and_range(&n))
        else {
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

        self.new_reference(var_range, ReferenceTag::VarRead(var_id));

        let var = self.variables.get(*var_id).unwrap();
        let index_count = node.indices().count();
        let result_ty = match Self::compute_indexed_type(
            &var.ty,
            index_count,
            utils::trim_node_text_range(&node),
        ) {
            Ok(ty) => ty,
            Err(e) => {
                self.new_error(e);
                return;
            }
        };
        let is_const = var.ty.is_const() && result_ty.is_const();
        let var_range = var.range;
        let const_zero = var.ty.const_zero();
        self.set_expr_type(node.text_range(), result_ty);

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
                        range: utils::trim_node_text_range(&indice),
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
                        range: utils::trim_node_text_range(&node),
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
        let Some((member_name, member_range)) = field_access_node
            .name()
            .and_then(|n| utils::extract_name_and_range(&n))
        else {
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
                        range: utils::trim_node_text_range(&base_expr),
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
                        range: utils::trim_node_text_range(&base_expr),
                    });
                    return;
                }
            }
            _ => {
                self.new_error(SemanticError::ApplyOpOnType {
                    ty: base_ty.clone(),
                    op: op_node.op().text().to_string(),
                    range: utils::trim_node_text_range(&base_expr),
                });
                return;
            }
        };

        // 查找 struct 定义
        let struct_def = self.get_struct_by_id(struct_id).unwrap();

        // 查找字段并设置类型
        if let Some(field_id) = struct_def.field(self, &member_name) {
            let field = self.get_field_by_id(field_id).unwrap();
            // 计算索引后的类型（如果有数组索引）
            let indices: Vec<_> = field_access_node.indices().collect();
            let result_ty = if indices.is_empty() {
                field.ty.clone()
            } else {
                // 有数组索引，需要计算索引后的类型
                match Self::compute_indexed_type(
                    &field.ty,
                    indices.len(),
                    utils::trim_node_text_range(&field_access_node),
                ) {
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

            self.set_expr_type(member_range, result_ty);
            // 不再记录字段访问为引用，因为 FieldID 不是 VariableID
            // self.new_reference(member_range, ReferenceTag::VarRead(field_id));

            // 常量处理：如果基础表达式是常量 struct，提取字段值
            if let Some(Value::Struct(_struct_id, field_values)) =
                self.value_table.get(&base_range).cloned()
                && let Some(field_idx) = struct_def.field_index(self, &member_name)
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
                struct_name: struct_def.name.clone(),
                field_name: member_name,
                range: utils::trim_node_text_range(&node),
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
}
