#![allow(unused_assignments)] // FIXME: https://github.com/zkat/miette/pull/459
//! array 的初始化

use std::collections::HashMap;

use miette::Diagnostic;
use syntax::ast::{Expr, InitVal};
use syntax::{AirycLanguage, AstNode};
use thiserror::Error;
use tools::TextRange;

use crate::{
    error::AnalyzeError,
    module::{Module, StructID},
    r#type::Ty,
    value::Value,
};

#[derive(Clone, Debug, PartialEq)]
pub enum ArrayTreeValue {
    Expr(TextRange),
    Struct {
        struct_id: StructID,
        init_list: TextRange,
    },
    Empty,
}

impl ArrayTreeValue {
    pub fn get_const_value<'a>(
        &self,
        value_table: &'a HashMap<TextRange, Value>,
    ) -> Option<&'a Value> {
        match self {
            Self::Expr(r) => value_table.get(r),
            Self::Struct { init_list: r, .. } => value_table.get(r),
            Self::Empty => None,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum ArrayTree {
    Children(Vec<ArrayTree>),
    Val(ArrayTreeValue),
}

pub trait ArrayTreeTrait: AstNode<Language = AirycLanguage> + Sized {
    /// Node -> Expr
    fn try_expr(&self) -> Option<ArrayTreeValue>;
    /// Node -> {Node, Node, Node}，期望叶子节点
    fn is_subtree(&self) -> bool {
        self.syntax().children().any(|x| Self::can_cast(x.kind()))
    }

    fn first_child(&self) -> Option<Self> {
        let kind = self.syntax().kind();
        let first_child = self.syntax().first_child_by_kind(&|k| k == kind);
        first_child.and_then(|s| Self::cast(s))
    }

    fn next_sibling(&self) -> Option<Self> {
        let kind = self.syntax().kind();
        let sibling = self.syntax().next_sibling_by_kind(&|k| k == kind);
        sibling.and_then(|s| Self::cast(s))
    }
}

impl ArrayTreeTrait for InitVal {
    fn try_expr(&self) -> Option<ArrayTreeValue> {
        self.syntax()
            .children()
            .find_map(|x| Expr::cast(x).map(|x| ArrayTreeValue::Expr(x.text_range())))
    }
}

#[derive(Debug, Error, Diagnostic)]
pub enum ArrayInitError {
    #[error("Cannot assign array to scalar")]
    #[diagnostic(code(array::assign_array_to_number))]
    AssignArrayToNumber,

    #[error("Array index out of bound")]
    #[diagnostic(code(array::index_out_of_bound))]
    IndexOutOfBound,

    #[error("Index and type mismatch")]
    #[diagnostic(code(array::mismatch_index_and_type))]
    MisMatchIndexAndType,

    #[error("Struct initialization error: {0}")]
    #[diagnostic(code(array::initial_struct_value))]
    InitialStructValue(#[from] AnalyzeError),

    #[error("expected {expected}, found {found}")]
    #[diagnostic(code(array::type_mismatch))]
    TypeMismatch {
        expected: Ty,
        found: Ty,
        #[label("here")]
        range: TextRange,
    },
}

impl ArrayTree {
    /// 由调用方处理常量表
    pub fn new(
        m: &mut Module,
        ty: &Ty,
        init_val: InitVal,
    ) -> Result<(ArrayTree, bool), ArrayInitError> {
        let Some(first_child) = init_val.first_child() else {
            return Ok((ArrayTree::Val(ArrayTreeValue::Empty), true));
        };
        let mut is_const = true;

        match Self::build(m, ty, &mut Some(first_child), &mut is_const) {
            Ok(array_tree) => Ok((array_tree, is_const)),
            Err(e) => Err(e),
        }
    }

    fn build(
        m: &mut Module,
        ty: &Ty,
        cursor: &mut Option<InitVal>,
        is_const: &mut bool,
    ) -> Result<ArrayTree, ArrayInitError> {
        match ty {
            Ty::I32 | Ty::I8 | Ty::Bool | Ty::Pointer { .. } => {
                let Some(u) = cursor else { unreachable!() };
                if let Some(expr) = u.try_expr() {
                    let range = u.text_range();

                    // 检查表达式类型是否与数组元素类型匹配
                    if let Some(expr_ty) = m.get_expr_type(range)
                        && !ty.assign_to_me_is_ok(expr_ty)
                    {
                        return Err(ArrayInitError::TypeMismatch {
                            expected: ty.clone(),
                            found: expr_ty.clone(),
                            range: utils::trim_node_text_range(u),
                        });
                    }

                    *is_const &= m.value_table.contains_key(&range);
                    *cursor = u.next_sibling();
                    return Ok(ArrayTree::Val(expr));
                }
                Err(ArrayInitError::AssignArrayToNumber)
            }
            Ty::Struct { id: struct_id, .. } => {
                let Some(u) = cursor else {
                    return Err(ArrayInitError::MisMatchIndexAndType);
                };

                let v = m
                    .process_struct_init_value(*struct_id, u.clone())
                    .map_err(ArrayInitError::InitialStructValue)?;
                *is_const &= v.is_some();
                if let Some(v) = v {
                    m.value_table.insert(u.text_range(), v);
                }
                Ok(ArrayTree::Val(ArrayTreeValue::Struct {
                    init_list: u.text_range(),
                    struct_id: *struct_id,
                }))
            }
            Ty::Array(inner, count) => {
                // 如果 count 为 None，说明还没有常量折叠，暂时返回错误
                let count_val = count.ok_or(ArrayInitError::MisMatchIndexAndType)?;
                let mut children_vec = Vec::with_capacity(count_val as usize);
                for _ in 0..count_val {
                    let Some(u) = cursor else {
                        break;
                    };
                    if u.is_subtree() {
                        let sibling = u.next_sibling();
                        let subtree = if inner.is_array() {
                            let mut first_child = u.first_child();
                            // 可能有多余元素，直接忽略
                            Self::build(m, inner, &mut first_child, is_const)?
                        } else {
                            // 否则应该是 Struct
                            Self::build(m, inner, cursor, is_const)?
                        };
                        children_vec.push(subtree);
                        *cursor = sibling;
                    } else if u.try_expr().is_some() {
                        let subtree = Self::build(m, inner, cursor, is_const)?;
                        children_vec.push(subtree);
                    } else {
                        // {}
                        if inner.is_array() {
                            children_vec.push(ArrayTree::Val(ArrayTreeValue::Empty));
                            *cursor = u.next_sibling();
                        } else {
                            return Err(ArrayInitError::AssignArrayToNumber);
                        }
                    }
                }
                Ok(ArrayTree::Children(children_vec))
            }
            Ty::Const(inner) => Self::build(m, inner, cursor, is_const),
            Ty::Void => unreachable!(),
        }
    }

    /// 获取叶子节点
    pub fn get_leaf(&self, indices: &[i32]) -> Result<ArrayTreeValue, ArrayInitError> {
        let mut u = self;
        for i in indices {
            u = match u {
                ArrayTree::Children(children) => {
                    let Some(child) = children.get(*i as usize) else {
                        return Ok(ArrayTreeValue::Empty);
                    };
                    child
                }
                ArrayTree::Val(ArrayTreeValue::Empty) => {
                    return Ok(ArrayTreeValue::Empty);
                }
                _ => return Err(ArrayInitError::MisMatchIndexAndType),
            };
        }
        match u {
            ArrayTree::Val(v) => Ok(v.clone()),
            _ => Err(ArrayInitError::MisMatchIndexAndType),
        }
    }
}
