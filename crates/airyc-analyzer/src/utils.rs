use airyc_parser::ast::{AstNode as _, Expr, FuncType, InitVal, Pointer, Type};

use crate::{
    array::ArrayTree,
    module::{Module, SemanticError, StructField, StructID},
    r#type::NType,
    value::Value,
};

impl Module {
    pub(crate) fn build_basic_type(&self, node: &Type) -> Option<NType> {
        if node.int_token().is_some() {
            Some(NType::Int)
        } else if node.float_token().is_some() {
            Some(NType::Float)
        } else if node.struct_token().is_some() {
            let Some(Some(name)) = node.name().map(|n| n.var_name()) else {
                return None;
            };
            // 查找 struct 定义
            self.find_struct(&name).map(NType::Struct)
        } else {
            None
        }
    }

    pub(crate) fn build_pointer_type(node: &Pointer, base_type: NType) -> NType {
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

    pub(crate) fn build_func_type(&self, node: &FuncType) -> Option<NType> {
        if node.void_token().is_some() {
            Some(NType::Void)
        } else {
            let node_ty = node.ty()?;
            let base_type = self.build_basic_type(&node_ty)?;

            if let Some(pointer_node) = node.pointer() {
                Some(Self::build_pointer_type(&pointer_node, base_type))
            } else {
                Some(base_type)
            }
        }
    }

    pub(crate) fn build_array_type(
        &self,
        mut ty: NType,
        indices_iter: impl Iterator<Item = Expr>,
    ) -> Result<NType, SemanticError> {
        let indices = indices_iter.collect::<Vec<Expr>>();
        for expr in indices.iter().rev() {
            let range = expr.syntax().text_range();
            let Some(x) = self.get_value(range).cloned() else {
                return Err(SemanticError::ConstantExprExpected { range });
            };
            let Value::Int(y) = x else {
                return Err(SemanticError::TypeMismatch {
                    expected: NType::Const(Box::new(NType::Int)),
                    found: x.get_type(),
                    range,
                });
            };
            ty = NType::Array(Box::new(ty), y);
        }
        Ok(ty)
    }

    /// 计算索引后的类型：去掉 index_count 层数组/指针
    pub(crate) fn compute_indexed_type(
        ty: &NType,
        index_count: usize,
    ) -> Result<NType, SemanticError> {
        let mut current = ty.clone();
        for _ in 0..index_count {
            current = match current {
                NType::Array(inner, _) => *inner,
                NType::Pointer(inner) => *inner,
                NType::Const(inner) => Self::compute_indexed_type(&inner, 1)?,
                _ => {
                    return Err(SemanticError::CantApplyOpOnType {
                        ty: current,
                        op: "[]",
                    });
                }
            };
        }
        Ok(current)
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
            NType::Int | NType::Float | NType::Pointer(_) => {
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
