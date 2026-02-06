//! 类型相关的语义分析
use syntax::ast::*;
use syntax::visitor::TypeVisitor;

use crate::error::SemanticError;
use crate::module::Module;
use crate::r#type::NType;
use crate::value::Value;

impl TypeVisitor for Module {
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
                            range: utils::trim_node_text_range(&expr_node),
                        });
                        return;
                    }
                } else {
                    self.new_error(SemanticError::ConstantExprExpected {
                        range: utils::trim_node_text_range(&expr_node),
                    });
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
                        if let Some(sid) = self.get_struct_by_name(&name) {
                            NType::Struct(sid)
                        } else {
                            self.new_error(SemanticError::StructUndefined {
                                name,
                                range: utils::trim_node_text_range(&node),
                            });
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
