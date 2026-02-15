//! 表达式相关的语义分析

use syntax::SyntaxKind;
use syntax::ast::*;
use syntax::visitor::ExprVisitor;

use crate::array::ArrayTreeValue;
use crate::error::AnalyzeError;
use crate::module::{Module, ReferenceTag};
use crate::r#type::{Ty, UnaryOpError};
use crate::value::Value;

impl ExprVisitor for Module {
    fn leave_call_expr(&mut self, node: CallExpr) {
        let Some((func_name, func_range)) =
            node.name().and_then(|n| utils::extract_name_and_range(&n))
        else {
            return;
        };

        // 检查函数是否已定义
        let Some(func_id) = self.get_function_id_by_name(&func_name) else {
            self.new_error(AnalyzeError::FunctionUndefined {
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

        let actual_args: Vec<_> = node
            .args()
            .map(|args| args.args().collect())
            .unwrap_or_default();

        let expected_params = &func.meta_types;
        let is_variadic = func.is_variadic;

        // 检查参数个数
        if is_variadic {
            // 可变参数函数：实际参数个数 >= 固定参数个数
            if actual_args.len() < expected_params.len() {
                self.new_error(AnalyzeError::ArgumentCountMismatch {
                    function_name: func_name.clone(),
                    expected: expected_params.len(),
                    found: actual_args.len(),
                    range: node.args().map(|a| a.text_range()).unwrap_or(func_range),
                });
                // 即使参数个数不匹配，也设置返回类型以便后续分析
                self.set_expr_type(node.text_range(), func.ret_type.clone());
                return;
            }
        } else {
            // 普通函数：实际参数个数 == 固定参数个数
            if actual_args.len() != expected_params.len() {
                self.new_error(AnalyzeError::ArgumentCountMismatch {
                    function_name: func_name.clone(),
                    expected: expected_params.len(),
                    found: actual_args.len(),
                    range: node.args().map(|a| a.text_range()).unwrap_or(func_range),
                });
                self.set_expr_type(node.text_range(), func.ret_type.clone());
                return;
            }
        }

        // 检查固定参数的类型（可变参数部分不检查类型）
        for (i, (actual_arg, (param_name, expected_ty))) in
            actual_args.iter().zip(expected_params.iter()).enumerate()
        {
            if let Some(actual_ty) = self.get_expr_type(actual_arg.text_range())
                && !expected_ty.assign_to_me_is_ok(actual_ty)
            {
                self.new_error(AnalyzeError::ArgumentTypeMismatch(Box::new(
                    crate::error::ArgumentTypeMismatchData {
                        function_name: func_name.clone(),
                        param_name: param_name.clone(),
                        arg_index: i + 1,
                        expected: expected_ty.clone(),
                        found: actual_ty.clone(),
                        range: actual_arg.text_range(),
                    },
                )));
            }
        }

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
            match Ty::compute_binary_result_type(l, r, op_kind) {
                Some(result_ty) => {
                    self.set_expr_type(node.text_range(), result_ty);
                }
                None => {
                    self.new_error(AnalyzeError::BinaryOpTypeMismatch {
                        op: op.op_str(),
                        lhs: l.clone(),
                        rhs: r.clone(),
                        range: node.text_range(),
                    });
                    return;
                }
            }
        }

        if self.is_compile_time_constant(lhs.text_range())
            && self.is_compile_time_constant(rhs.text_range())
        {
            let lhs_val = self.value_table.get(&lhs.text_range()).unwrap();
            let rhs_val = self.value_table.get(&rhs.text_range()).unwrap();

            match Value::calc_binary_expr(lhs_val, rhs_val, op.op().kind(), self) {
                Ok(val) => {
                    self.value_table.insert(node.text_range(), val);
                }
                Err(crate::value::EvalError::Overflow(msg)) => {
                    // 常量表达式溢出，报告错误
                    self.new_error(AnalyzeError::ConstArithmeticOverflow {
                        message: msg,
                        range: node.text_range(),
                    });
                }
                Err(_) => {
                    // 其他错误（类型不匹配等），忽略（不存储常量值）
                }
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
            // 特殊处理取地址操作（需要检查左值）
            if op_kind == SyntaxKind::AMP && !self.is_lvalue_expr(&expr) {
                self.new_error(AnalyzeError::AddressOfRight {
                    range: utils::trim_node_text_range(&expr),
                });
                return;
            }

            match inner_ty.validate_unary_op(op_kind) {
                Ok(result_ty) => {
                    self.set_expr_type(node.text_range(), result_ty);
                }
                Err(err) => {
                    let trimmed_range = utils::trim_node_text_range(&expr);
                    match err {
                        UnaryOpError::VoidPointerDeref => {
                            self.new_error(AnalyzeError::VoidPointerDeref {
                                range: trimmed_range,
                            });
                        }
                        UnaryOpError::InvalidOp => {
                            self.new_error(AnalyzeError::ApplyOpOnType {
                                ty: inner_ty.clone(),
                                op: op.op_str(),
                                range: trimmed_range,
                            });
                        }
                    }
                    return;
                }
            }
        }

        if matches!(op_kind, SyntaxKind::STAR | SyntaxKind::AMP) {
            return;
        }

        if self.is_compile_time_constant(expr.text_range()) {
            let val = self.value_table.get(&expr.text_range()).unwrap().clone();

            match Value::eval_unary(val.clone(), op.op().kind()) {
                Ok(res) => {
                    self.value_table.insert(node.text_range(), res);
                }
                Err(crate::value::EvalError::Overflow(msg)) => {
                    // 常量表达式溢出，报告错误
                    // 但需要检查是否是 -128i8 这样的特例
                    let expr_range = expr.text_range();
                    if op_kind == SyntaxKind::MINUS
                        && self
                            .analyzing
                            .overflowing_literals
                            .contains_key(&expr_range)
                    {
                        // 这是字面量溢出的情况，已经在字面量溢出检测中处理
                        // 不报告一元运算溢出
                    } else {
                        // 普通的一元运算溢出
                        self.new_error(AnalyzeError::ConstArithmeticOverflow {
                            message: msg,
                            range: node.text_range(),
                        });
                    }
                }
                Err(_) => {
                    // 其他错误，忽略
                }
            }

            // 检查溢出的字面量：如果操作数是溢出的字面量，且操作符是负号
            if op_kind == SyntaxKind::MINUS {
                let expr_range = expr.text_range();
                if let Some(literal_text) = self.analyzing.overflowing_literals.get(&expr_range) {
                    // 检查是否是特例（如 -128i8）
                    // 特例：字面量截断后的值取负后等于自身（即最小负数）
                    let is_min_value = match val {
                        Value::I8(v) => v == i8::MIN,
                        Value::I32(v) => v == i32::MIN,
                        Value::I64(v) => v == i64::MIN,
                        _ => false,
                    };

                    if !is_min_value {
                        // 不是特例，报告溢出错误
                        let ty_str = match val {
                            Value::I8(_) => "i8",
                            Value::U8(_) => "u8",
                            Value::I32(_) => "i32",
                            Value::U32(_) => "u32",
                            Value::I64(_) => "i64",
                            Value::U64(_) => "u64",
                            _ => unreachable!(),
                        };
                        self.new_error(AnalyzeError::IntegerLiteralOverflow {
                            literal: literal_text.clone(),
                            ty: ty_str.to_string(),
                            range: expr_range,
                        });
                    }
                    // 从溢出列表中移除
                    self.analyzing.overflowing_literals.remove(&expr_range);
                }
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
            self.new_error(AnalyzeError::VariableUndefined {
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

                let Some(index) = v.get_array_size() else {
                    self.new_error(AnalyzeError::TypeMismatch {
                        expected: Ty::I32,
                        found: v.get_type(self),
                        range: utils::trim_node_text_range(&indice),
                    });
                    return;
                };
                indices.push(index);
            }
            let leaf = match tree.get_leaf(&indices) {
                Ok(s) => s,
                Err(e) => {
                    self.new_error(AnalyzeError::ArrayError {
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
                    self.new_error(AnalyzeError::NotAStruct {
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
                    self.new_error(AnalyzeError::NotAStructPointer {
                        ty: base_ty.clone(),
                        range: utils::trim_node_text_range(&base_expr),
                    });
                    return;
                }
            }
            _ => {
                self.new_error(AnalyzeError::ApplyOpOnType {
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
            let result_ty = match Self::compute_indexed_type(
                &field.ty,
                indices.len(),
                utils::trim_node_text_range(&field_access_node),
            ) {
                Ok(ty) => ty,
                Err(e) => {
                    self.new_error(e);
                    return;
                }
            };

            // 如果 base_ty 是 const，需要继承
            let result_ty = if base_ty.is_const() && !result_ty.is_const() {
                Ty::Const(Box::new(result_ty))
            } else {
                result_ty
            };

            self.set_expr_type(range, result_ty);

            self.new_reference(member_range, ReferenceTag::FieldRead(field_id));

            // 常量处理：如果基础表达式是常量 struct，提取字段值
            if let Some(Value::Struct(_, field_values)) = self.value_table.get(&base_range).cloned()
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
                        let idx = match self.get_value_by_range(idx_range) {
                            Some(Value::I32(i)) => *i,
                            Some(Value::I8(i)) => *i as i32,
                            _ => return,
                        };
                        idx_values.push(idx);
                    }
                    if let Ok(leaf) = tree.get_leaf(&idx_values)
                        && let Some(v) = leaf.get_const_value(&self.value_table)
                    {
                        self.value_table.insert(range, v.clone());
                    }
                }
            }
        } else {
            self.new_error(AnalyzeError::FieldNotFound {
                struct_name: struct_def.name.clone(),
                field_name: member_name,
                range: utils::trim_node_text_range(&node),
            });
        }
    }

    fn enter_literal(&mut self, node: Literal) {
        let range = node.text_range();
        let v = if node.null_token().is_some() {
            self.set_expr_type(
                range,
                Ty::Pointer {
                    pointee: Box::new(Ty::Void),
                    is_const: false,
                },
            );
            Value::Null
        } else if node.string_token().is_some() {
            // 字符串字面量类型为 *const u8（修改：从 i8 改为 u8）
            self.set_expr_type(
                range,
                Ty::Pointer {
                    pointee: Box::new(Ty::U8),
                    is_const: true,
                },
            );
            // 获取字符串内容（去掉引号）
            let string_token = node.string_token().unwrap();
            let s = string_token.text().to_string();
            let content = match snailquote::unescape(&s) {
                Ok(s) => s,
                Err(e) => {
                    self.new_error(AnalyzeError::UnescapeError {
                        err: Box::new(e),
                        range: utils::trim_node_text_range(&node),
                    });
                    return;
                }
            };
            Value::String(content)
        } else if node.char_token().is_some() {
            // 字符字面量类型为 u8
            self.set_expr_type(range, Ty::U8);
            // 获取字符内容（去掉单引号）
            let char_token = node.char_token().unwrap();
            let s = char_token.text().to_string();
            // 去掉单引号，转换为双引号格式供 snailquote 处理
            let inner = &s[1..s.len() - 1];
            let quoted = format!("\"{}\"", inner);
            let content = match snailquote::unescape(&quoted) {
                Ok(s) => s,
                Err(e) => {
                    self.new_error(AnalyzeError::InvalidCharLiteral {
                        literal: s.clone(),
                        reason: format!("Invalid escape sequence: {}", e),
                        range: utils::trim_node_text_range(&node),
                    });
                    return;
                }
            };

            // 验证是单个 ASCII 字符
            let chars: Vec<char> = content.chars().collect();
            if chars.len() != 1 {
                self.new_error(AnalyzeError::InvalidCharLiteral {
                    literal: s.clone(),
                    reason: format!(
                        "Character literal must contain exactly one character, found {}",
                        chars.len()
                    ),
                    range: utils::trim_node_text_range(&node),
                });
                return;
            }

            let ch = chars[0];
            if !ch.is_ascii() {
                self.new_error(AnalyzeError::InvalidCharLiteral {
                    literal: s.clone(),
                    reason: format!(
                        "Character literal must be ASCII (0-127), found '{}' (U+{:04X}). \
                        Consider using a string literal or byte array for multi-byte characters.",
                        ch, ch as u32
                    ),
                    range: utils::trim_node_text_range(&node),
                });
                return;
            }

            Value::U8(ch as u8)
        } else if node.true_token().is_some() {
            self.set_expr_type(range, Ty::Bool);
            Value::Bool(true)
        } else if node.false_token().is_some() {
            self.set_expr_type(range, Ty::Bool);
            Value::Bool(false)
        } else {
            let Some(n) = node.int_token() else {
                return;
            };
            let s = n.text();

            // 分离后缀
            let (num_part, suffix) = if let Some(ss) = s.strip_suffix("i64") {
                (ss, Some("i64"))
            } else if let Some(ss) = s.strip_suffix("u64") {
                (ss, Some("u64"))
            } else if let Some(ss) = s.strip_suffix("i32") {
                (ss, Some("i32"))
            } else if let Some(ss) = s.strip_suffix("u32") {
                (ss, Some("u32"))
            } else if let Some(ss) = s.strip_suffix("i8") {
                (ss, Some("i8"))
            } else if let Some(ss) = s.strip_suffix("u8") {
                (ss, Some("u8"))
            } else {
                (s, None)
            };

            // 解析进制
            let (num_str, radix) = match num_part.chars().next() {
                Some('0') => match num_part.chars().nth(1) {
                    Some('x') | Some('X') => (&num_part[2..], 16),
                    Some('o') | Some('O') => (&num_part[2..], 8),
                    Some('b') | Some('B') => (&num_part[2..], 2),
                    _ => (num_part, 10),
                },
                _ => (num_part, 10),
            };

            // 根据后缀确定类型（默认 i32）
            let ty = match suffix {
                Some("i8") => Ty::I8,
                Some("u8") => Ty::U8,
                Some("i32") | None => Ty::I32,
                Some("u32") => Ty::U32,
                Some("i64") => Ty::I64,
                Some("u64") => Ty::U64,
                _ => unreachable!(),
            };

            // 使用 u128 解析，然后截断到目标类型
            let value_u128 = match u128::from_str_radix(num_str, radix) {
                Ok(v) => v,
                Err(_) => {
                    // 解析失败（数字太大或格式错误）
                    self.semantic_errors
                        .push(crate::error::AnalyzeError::IntegerLiteralOverflow {
                            literal: s.to_string(),
                            ty: match ty {
                                Ty::I8 => "i8",
                                Ty::U8 => "u8",
                                Ty::I32 => "i32",
                                Ty::U32 => "u32",
                                Ty::I64 => "i64",
                                Ty::U64 => "u64",
                                _ => unreachable!(),
                            }
                            .to_string(),
                            range,
                        });
                    return;
                }
            };

            // 检查是否溢出
            let overflows = match ty {
                Ty::I8 => value_u128 > i8::MAX as u128,
                Ty::U8 => value_u128 > u8::MAX as u128,
                Ty::I32 => value_u128 > i32::MAX as u128,
                Ty::U32 => value_u128 > u32::MAX as u128,
                Ty::I64 => value_u128 > i64::MAX as u128,
                Ty::U64 => value_u128 > u64::MAX as u128,
                _ => unreachable!(),
            };

            // 截断到目标类型
            let value = match ty {
                Ty::I8 => Value::I8(value_u128 as i8),
                Ty::U8 => Value::U8(value_u128 as u8),
                Ty::I32 => Value::I32(value_u128 as i32),
                Ty::U32 => Value::U32(value_u128 as u32),
                Ty::I64 => Value::I64(value_u128 as i64),
                Ty::U64 => Value::U64(value_u128 as u64),
                _ => unreachable!(),
            };

            // 如果溢出，标记此字面量（后续在一元负号处理时检查）
            if overflows {
                // 记录溢出的字面量，用于后续检测 -128i8 特例
                self.analyzing
                    .overflowing_literals
                    .insert(range, s.to_string());
            }

            self.set_expr_type(range, ty);
            value
        };
        self.value_table.insert(range, v);
    }
}
