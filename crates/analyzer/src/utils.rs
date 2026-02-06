use syntax::SyntaxKind;
use syntax::ast::{AstNode as _, Expr, IndexVal, InitVal, OpNode, PostfixExpr, UnaryExpr};
use tools::TextRange;

use crate::error::SemanticError;
use crate::{
    array::ArrayTree,
    module::{Module, StructID, VariableID},
    r#type::NType,
    value::Value,
};

impl Module {
    /// 计算索引后的类型：去掉 index_count 层数组/指针
    /// 如果结果是数组类型，自动 decay 成指向元素的指针
    pub(crate) fn compute_indexed_type(
        ty: &NType,
        index_count: usize,
        range: TextRange,
    ) -> Result<NType, SemanticError> {
        let mut current = ty.clone();
        for _ in 0..index_count {
            current = match current {
                NType::Array(inner, _) => *inner,
                NType::Pointer { pointee, .. } => *pointee,
                NType::Const(inner) => Self::compute_indexed_type(&inner, 1, range)?,
                _ => {
                    return Err(SemanticError::ApplyOpOnType {
                        ty: current,
                        op: "[]".to_string(),
                        range,
                    });
                }
            };
        }
        // 如果结果是数组类型，decay 成指向元素的指针
        if let NType::Array(inner, _) = current {
            Ok(NType::Pointer {
                pointee: inner,
                is_const: true,
            })
        } else {
            Ok(current)
        }
    }

    /// 解析 struct 初始化列表，返回 Value::Struct
    /// 如果非常量初始化列表，返回 None
    /// 一定要遍历所有子树，目的是初始化 ArrayTree
    /// 由调用方处理常量表
    pub(crate) fn process_struct_init_value(
        &mut self,
        struct_id: StructID,
        init_val_node: InitVal,
    ) -> Result<Option<Value>, SemanticError> {
        let range_trimmed = utils::trim_node_text_range(&init_val_node); // 获取 struct 定义
        let struct_def =
            self.get_struct_by_id(struct_id)
                .ok_or(SemanticError::StructUndefined {
                    name: "<uname>".to_string(),
                    range: range_trimmed,
                })?;

        // 否则是初始化列表 { init1, init2, ... }
        let inits: Vec<_> = init_val_node.inits().collect();

        // 检查初始化列表长度是否与字段数匹配
        if inits.len() != struct_def.fields.len() {
            return Err(SemanticError::StructInitFieldCountMismatch {
                expected: struct_def.fields.len(),
                found: inits.len(),
                range: range_trimmed,
            });
        }

        // 按顺序解析每个字段的初始化值
        let mut field_values = Vec::with_capacity(struct_def.fields.len());
        let field_ids: *const [VariableID] = &struct_def.fields[..];

        // 先收集所有字段类型（避免借用冲突）
        let field_types: Vec<NType> = unsafe { &*field_ids }
            .iter()
            .map(|field_id| self.variables.get(**field_id).unwrap().ty.clone())
            .collect();

        let mut all_const = true;
        for (init, field_ty) in inits.into_iter().zip(field_types.iter()) {
            let value = self.process_field_init_value(field_ty, init)?;
            if let Some(v) = value
                && all_const
            {
                field_values.push(v);
            } else {
                all_const = false;
            }
        }

        if !all_const {
            Ok(None)
        } else {
            let value = Value::Struct(struct_id, field_values);
            Ok(Some(value))
        }
    }

    /// 根据字段类型决定如何解析初始化值
    fn process_field_init_value(
        &mut self,
        field_ty: &NType,
        init_val_node: InitVal,
    ) -> Result<Option<Value>, SemanticError> {
        let range = init_val_node.text_range();
        // 去掉 Const 包装
        let inner_ty = field_ty.unwrap_const();

        match &inner_ty {
            // 标量类型：期望一个表达式
            NType::Int | NType::Float | NType::Pointer { .. } => {
                let Some(expr) = init_val_node.expr() else {
                    // 期望表达式，但得到了初始化列表
                    return Err(SemanticError::ConstantExprExpected { range });
                };
                let expr_range = expr.text_range();
                Ok(self.value_table.get(&expr_range).cloned())
            }

            // 数组类型：使用 ArrayTree 解析
            NType::Array(_, _) => {
                let range = init_val_node.text_range();
                let (array_tree, is_const) = ArrayTree::new(self, field_ty, init_val_node)
                    .map_err(|e| SemanticError::ArrayError {
                        message: Box::new(e),
                        range,
                    })?;

                if !is_const {
                    self.expand_array.insert(range, array_tree);
                    return Ok(None);
                }
                self.expand_array.insert(range, array_tree.clone());
                self.value_table
                    .insert(range, Value::Array(array_tree.clone()));

                Ok(Some(Value::Array(array_tree)))
            }

            // Struct 类型：递归解析
            NType::Struct(nested_struct_id) => {
                let result = self.process_struct_init_value(*nested_struct_id, init_val_node);
                if let Ok(Some(v)) = &result {
                    self.value_table.insert(range, v.clone()); // 将常量加入表中
                }
                result
            }

            NType::Void | NType::Const(_) => unreachable!(),
        }
    }

    /// 返回 true 如果是有效的左值
    pub(crate) fn is_lvalue_expr(&self, expr: &Expr) -> bool {
        match expr {
            Expr::IndexVal(_) => true,
            Expr::PostfixExpr(_) => true,
            Expr::UnaryExpr(unary) => unary.op().map(|x| x.op().kind()) == Some(SyntaxKind::STAR),
            // 其他表达式类型不是有效的左值（字面量、函数调用、二元表达式等）
            _ => false,
        }
    }

    /// 检查左值是否可赋值（检测 const 并报错）
    pub(crate) fn check_lvalue_assignable(&mut self, expr: &Expr) -> bool {
        match expr {
            Expr::IndexVal(index_val) => self.check_index_val_assignable(index_val),
            Expr::PostfixExpr(postfix) => self.check_postfix_assignable(postfix),
            Expr::UnaryExpr(unary) => self.check_unary_assignable(unary),
            _ => false,
        }
    }

    /// 检查 IndexVal 是否可赋值（检测 const 并报错）
    fn check_index_val_assignable(&mut self, node: &IndexVal) -> bool {
        let Some(name_node) = node.name() else {
            return false;
        };
        let Some(var_name) = name_node.var_name() else {
            return false;
        };
        let Some(var_range) = name_node.var_range() else {
            return false;
        };

        let Some(def_id) = self.find_variable_def(&var_name) else {
            return false;
        };

        let var = self.variables.get(*def_id).unwrap();

        let Some(result_ty) = self.get_expr_type(node.text_range()) else {
            return false;
        };

        // const 不可被赋值
        if var.ty.is_const() || result_ty.is_const() {
            self.new_error(SemanticError::AssignToConst {
                name: var_name.to_string(),
                range: var_range,
            });
            return false;
        }
        true
    }

    /// 检查 PostfixExpr 是否可赋值（检测 const 并报错）
    fn check_postfix_assignable(&mut self, node: &PostfixExpr) -> bool {
        if let Some(ty) = self.get_expr_type(node.text_range()) {
            if ty.is_const() {
                self.new_error(SemanticError::AssignToConst {
                    name: "field".to_string(),
                    range: utils::trim_node_text_range(node),
                });
                return false;
            }
            true
        } else {
            false
        }
    }

    /// 检查 UnaryExpr（解引用 *ptr）是否可赋值（检测 const 并报错）
    fn check_unary_assignable(&mut self, node: &UnaryExpr) -> bool {
        let Some(op) = node.op() else {
            return true;
        };

        // 只检查解引用操作
        if op.op().kind() != SyntaxKind::STAR {
            return true;
        }

        let Some(expr_ty) = self.get_expr_type(node.text_range()) else {
            return false;
        };

        if expr_ty.is_const() {
            self.new_error(SemanticError::AssignToConst {
                name: "*ptr".to_string(),
                range: utils::trim_node_text_range(node),
            });
            false
        } else {
            true
        }
    }
}
