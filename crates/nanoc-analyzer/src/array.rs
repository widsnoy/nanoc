use nanoc_parser::{
    ast::{AstNode, ConstExpr, ConstInitVal, Expr, InitVal},
    syntax_kind::NanocLanguage,
};

use crate::r#type::NType;

pub enum ArrayTreeValue {
    ConstExpr(ConstExpr),
    Expr(Expr),
}

pub enum ArrayTree {
    Children(Vec<ArrayTree>),
    Val(ArrayTreeValue),
    Empty,
}

pub trait ArrayTreeTrait: AstNode<Language = NanocLanguage> + Sized {
    /// Node -> Expr
    fn try_expr(&self) -> Option<ArrayTreeValue>;
    /// Node -> {Node, Node, Node}, expect leaf
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

impl ArrayTreeTrait for ConstInitVal {
    fn try_expr(&self) -> Option<ArrayTreeValue> {
        self.syntax().children().find_map(|x| {
            ConstExpr::cast(x.clone()).and_then(|y| Some(ArrayTreeValue::ConstExpr(y)))
        })
    }
}
impl ArrayTreeTrait for InitVal {
    fn try_expr(&self) -> Option<ArrayTreeValue> {
        self.syntax()
            .children()
            .find_map(|x| Expr::cast(x.clone()).and_then(|y| Some(ArrayTreeValue::Expr(y))))
    }
}

pub enum ArrayInitError {
    /// 用数组初始化标量
    AssignArrayToNumber,
}

impl ArrayTree {
    pub fn new(ty: &NType, init_val: impl ArrayTreeTrait) -> Result<ArrayTree, ArrayInitError> {
        let Some(first_child) = init_val.first_child() else {
            return Ok(ArrayTree::Empty);
        };

        let ty = if let NType::Const(inner) = ty {
            inner.as_ref()
        } else {
            ty
        };

        Self::build(ty, &mut Some(first_child))
    }

    fn build(
        ty: &NType,
        cursor: &mut Option<impl ArrayTreeTrait>,
    ) -> Result<ArrayTree, ArrayInitError> {
        match ty {
            NType::Int | NType::Float => {
                let Some(u) = cursor else {
                    return Ok(ArrayTree::Empty);
                };
                if let Some(expr) = u.try_expr() {
                    *cursor = u.next_sibling();
                    return Ok(ArrayTree::Val(expr));
                }
                Err(ArrayInitError::AssignArrayToNumber)
            }
            NType::Array(inner, count) => {
                let mut children_vec = Vec::with_capacity(*count);
                for _ in 0..*count {
                    let Some(u) = cursor else {
                        break;
                    };
                    if u.is_subtree() {
                        let mut first_child = u.first_child();
                        // 可能多了，直接忽略掉
                        let subtree = Self::build(inner, &mut first_child)?;
                        children_vec.push(subtree);
                        *cursor = u.next_sibling();
                    } else if u.try_expr().is_some() {
                        let subtree = Self::build(inner, cursor)?;
                        children_vec.push(subtree);
                    } else {
                        // {}
                        if inner.is_array() {
                            children_vec.push(ArrayTree::Empty);
                            *cursor = u.next_sibling();
                        } else {
                            return Err(ArrayInitError::AssignArrayToNumber);
                        }
                    }
                }
                Ok(ArrayTree::Children(children_vec))
            }
            _ => unreachable!(),
        }
    }
}
