use airyc_parser::ast::{AstNode as _, Expr, FuncType, Name, Pointer, Type};

use crate::{module::Module, r#type::NType, value::Value};

impl Module {
    pub(crate) fn build_basic_type(node: &Type) -> NType {
        if node.int_token().is_some() {
            NType::Int
        } else if node.float_token().is_some() {
            NType::Float
        } else if node.struct_token().is_some() {
            let name = Self::extract_name(&node.name().unwrap());
            NType::Struct(name)
        } else {
            unreachable!("unknown type node")
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
    /// 从 Name 节点提取变量名
    pub(crate) fn extract_name(node: &Name) -> String {
        node.ident()
            .map(|t| t.text().to_string())
            .expect("failed to get identifier")
    }

    pub(crate) fn build_func_type(node: &FuncType) -> NType {
        if node.void_token().is_some() {
            NType::Void
        } else {
            let base_type = Self::build_basic_type(&node.ty().unwrap());

            if let Some(pointer_node) = node.pointer() {
                Self::build_pointer_type(&pointer_node, base_type)
            } else {
                base_type
            }
        }
    }

    pub(crate) fn build_array_type(
        &self,
        mut ty: NType,
        indices_iter: impl Iterator<Item = Expr>,
    ) -> Option<NType> {
        let indices = indices_iter.collect::<Vec<Expr>>();
        for expr in indices.iter().rev() {
            let x = self.get_value(expr.syntax().text_range()).cloned()?;
            let Value::Int(y) = x else {
                return None;
            };
            ty = NType::Array(Box::new(ty), y);
        }
        Some(ty)
    }

    /// 计算索引后的类型：去掉 index_count 层数组/指针
    pub(crate) fn compute_indexed_type(ty: &NType, index_count: usize) -> NType {
        let mut current = ty.clone();
        for _ in 0..index_count {
            current = match current {
                NType::Array(inner, _) => *inner,
                NType::Pointer(inner) => *inner,
                NType::Const(inner) => Self::compute_indexed_type(&inner, 1),
                _ => current,
            };
        }
        current
    }
}
