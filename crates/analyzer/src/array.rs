//! array 的初始化
use std::collections::HashMap;

use syntax::{AirycLanguage, AstNode, Expr, InitVal};
use text_size::TextRange;

use crate::{
    module::{Module, SemanticError, StructID},
    r#type::NType,
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
            .find_map(|x| Expr::cast(x).map(|x| ArrayTreeValue::Expr(x.syntax().text_range())))
    }
}

#[derive(Debug)]
pub enum ArrayInitError {
    /// 将数组赋值给标量
    AssignArrayToNumber,
    /// 数组索引越界
    IndexOutOfBound,
    /// 索引和类型不匹配
    MisMatchIndexAndType,
    /// 初始化 struct 出错
    InitialStructValue(SemanticError),
}

impl ArrayTree {
    /// 由调用方处理常量表
    pub fn new(
        m: &mut Module,
        ty: &NType,
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
        ty: &NType,
        cursor: &mut Option<InitVal>,
        is_const: &mut bool,
    ) -> Result<ArrayTree, ArrayInitError> {
        match ty {
            NType::Int | NType::Float | NType::Pointer { .. } => {
                let Some(u) = cursor else { unreachable!() };
                if let Some(expr) = u.try_expr() {
                    let range = u.syntax().text_range();
                    *is_const &= m.value_table.contains_key(&range);
                    *cursor = u.next_sibling();
                    return Ok(ArrayTree::Val(expr));
                }
                Err(ArrayInitError::AssignArrayToNumber)
            }
            NType::Struct(struct_id) => {
                let Some(u) = cursor else {
                    return Err(ArrayInitError::MisMatchIndexAndType);
                };

                let v = m
                    .process_struct_init_value(*struct_id, u.clone())
                    .map_err(ArrayInitError::InitialStructValue)?;
                *is_const &= v.is_some();
                if let Some(v) = v {
                    m.value_table.insert(u.syntax().text_range(), v);
                }
                Ok(ArrayTree::Val(ArrayTreeValue::Struct {
                    init_list: u.syntax().text_range(),
                    struct_id: *struct_id,
                }))
            }
            NType::Array(inner, count) => {
                let mut children_vec = Vec::with_capacity(*count as usize);
                for _ in 0..*count {
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
            NType::Const(inner) => Self::build(m, inner, cursor, is_const),
            NType::Void => unreachable!(),
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
