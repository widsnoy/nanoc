use nanoc_parser::ast::{AstNode as _, ConstExpr, ConstIndexVal, FuncType, Name, Pointer, Type};

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
            unreachable!("未知类型节点")
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
            .expect("获取标识符失败")
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
        basic_type: NType,
        node: &ConstIndexVal,
    ) -> Option<NType> {
        let mut ty = basic_type;
        let mut indices_rev = node.indices().collect::<Vec<ConstExpr>>();
        indices_rev.reverse();
        for expr in indices_rev {
            let x = self.get_value(expr.syntax().text_range()).cloned()?;
            let Value::Int(y) = x else {
                return None;
            };
            ty = NType::Array(Box::new(ty), y);
        }
        Some(ty)
    }
}
