use parser::ast::{AstNode as _, InitVal};

use crate::{
    array::ArrayTree,
    module::{Module, SemanticError, StructField, StructID},
    r#type::NType,
    value::Value,
};

impl Module {
    /// 计算索引后的类型：去掉 index_count 层数组/指针
    /// 如果结果是数组类型，自动 decay 成指向元素的指针
    pub(crate) fn compute_indexed_type(
        ty: &NType,
        index_count: usize,
    ) -> Result<NType, SemanticError> {
        let mut current = ty.clone();
        for _ in 0..index_count {
            current = match current {
                NType::Array(inner, _) => *inner,
                NType::Pointer { pointee, .. } => *pointee,
                NType::Const(inner) => Self::compute_indexed_type(&inner, 1)?,
                _ => {
                    return Err(SemanticError::CantApplyOpOnType {
                        ty: current,
                        op: "[]",
                    });
                }
            };
        }
        // 如果结果是数组类型，decay 成指向元素的指针
        if let NType::Array(inner, _) = current {
            Ok(NType::Pointer {
                pointee: inner,
                is_const: false,
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
        let range = init_val_node.syntax().text_range();
        // 获取 struct 定义
        let struct_def = self
            .get_struct(struct_id)
            .ok_or(SemanticError::TypeUndefined { range })?;

        // 否则是初始化列表 { init1, init2, ... }
        let inits: Vec<_> = init_val_node.inits().collect();

        // 检查初始化列表长度是否与字段数匹配
        if inits.len() != struct_def.fields.len() {
            return Err(SemanticError::StructInitFieldCountMismatch {
                expected: struct_def.fields.len(),
                found: inits.len(),
                range,
            });
        }

        // 按顺序解析每个字段的初始化值
        let mut field_values = Vec::with_capacity(struct_def.fields.len());
        let fields: *const [StructField] = &struct_def.fields[..];
        let mut all_const = true;
        for (init, field) in inits.into_iter().zip(unsafe { &*fields }.iter()) {
            let value = self.process_field_init_value(&field.ty, init)?;
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
        let range = init_val_node.syntax().text_range();
        // 去掉 Const 包装
        let inner_ty = field_ty.unwrap_const();

        match &inner_ty {
            // 标量类型：期望一个表达式
            NType::Int | NType::Float | NType::Pointer { .. } => {
                let Some(expr) = init_val_node.expr() else {
                    // 期望表达式，但得到了初始化列表
                    return Err(SemanticError::ConstantExprExpected { range });
                };
                let expr_range = expr.syntax().text_range();
                Ok(self.value_table.get(&expr_range).cloned())
            }

            // 数组类型：使用 ArrayTree 解析
            NType::Array(_, _) => {
                let range = init_val_node.syntax().text_range();
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
}
