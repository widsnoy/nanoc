use std::collections::HashMap;

use nanoc_parser::{
    ast::{AstNode, ConstExpr, ConstInitVal, Expr, InitVal},
    syntax_kind::NanocLanguage,
};
use text_size::TextRange;

use crate::{r#type::NType, value::Value};

#[derive(Clone, Debug, PartialEq)]
pub enum ArrayTreeValue {
    ConstExpr(ConstExpr),
    Expr(Expr),
    Empty,
}

impl ArrayTreeValue {
    pub fn get_const_value<'a>(
        &self,
        value_table: &'a HashMap<TextRange, Value>,
    ) -> Option<&'a Value> {
        match self {
            Self::ConstExpr(node) => value_table.get(&node.syntax().text_range()),
            _ => None,
        }
    }
}

#[derive(Clone, PartialEq)]
pub enum ArrayTree {
    Children(Vec<ArrayTree>),
    Val(ArrayTreeValue),
}

impl std::fmt::Display for ArrayTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn fmt_inner(
            tree: &ArrayTree,
            f: &mut std::fmt::Formatter<'_>,
            prefix: &str,
            is_last: bool,
            is_root: bool,
        ) -> std::fmt::Result {
            let connector = if is_root {
                ""
            } else if is_last {
                "└── "
            } else {
                "├── "
            };

            match tree {
                ArrayTree::Children(children) => {
                    writeln!(f, "{}{}Mama", prefix, connector)?;

                    let new_prefix = if is_root {
                        "".to_string()
                    } else if is_last {
                        format!("{}    ", prefix)
                    } else {
                        format!("{}│   ", prefix)
                    };

                    for (i, child) in children.iter().enumerate() {
                        let is_last_child = i == children.len() - 1;
                        fmt_inner(child, f, &new_prefix, is_last_child, false)?;
                    }
                    Ok(())
                }
                ArrayTree::Val(v) => {
                    let text = match v {
                        ArrayTreeValue::ConstExpr(e) => e.syntax().text(),
                        ArrayTreeValue::Expr(e) => e.syntax().text(),
                        ArrayTreeValue::Empty => {
                            return writeln!(f, "{}{}Empty", prefix, connector);
                        }
                    }
                    .to_string()
                    .trim()
                    .to_string();
                    writeln!(f, "{}{}Val({})", prefix, connector, text)
                }
            }
        }
        fmt_inner(self, f, "", true, true)
    }
}

impl std::fmt::Debug for ArrayTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
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
        self.syntax()
            .children()
            .find_map(|x| ConstExpr::cast(x.clone()).map(ArrayTreeValue::ConstExpr))
    }
}
impl ArrayTreeTrait for InitVal {
    fn try_expr(&self) -> Option<ArrayTreeValue> {
        self.syntax()
            .children()
            .find_map(|x| Expr::cast(x.clone()).map(ArrayTreeValue::Expr))
    }
}

#[derive(Debug)]
pub enum ArrayInitError {
    /// 用数组初始化标量
    AssignArrayToNumber,
    /// 数组下标越界
    IndexOutOfBound,
    /// 下标和类型不匹配
    MisMatchIndexAndType,
    Unkown,
}

impl ArrayTree {
    pub fn new(ty: &NType, init_val: impl ArrayTreeTrait) -> Result<ArrayTree, ArrayInitError> {
        let Some(first_child) = init_val.first_child() else {
            return Ok(ArrayTree::Val(ArrayTreeValue::Empty));
        };

        Self::build(ty, &mut Some(first_child))
    }

    fn build(
        ty: &NType,
        cursor: &mut Option<impl ArrayTreeTrait>,
    ) -> Result<ArrayTree, ArrayInitError> {
        match ty {
            NType::Int | NType::Float => {
                let Some(u) = cursor else { unreachable!() };
                if let Some(expr) = u.try_expr() {
                    *cursor = u.next_sibling();
                    return Ok(ArrayTree::Val(expr));
                }
                Err(ArrayInitError::AssignArrayToNumber)
            }
            NType::Array(inner, count) => {
                let mut children_vec = Vec::with_capacity(*count as usize);
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
                            children_vec.push(ArrayTree::Val(ArrayTreeValue::Empty));
                            *cursor = u.next_sibling();
                        } else {
                            return Err(ArrayInitError::AssignArrayToNumber);
                        }
                    }
                }
                Ok(ArrayTree::Children(children_vec))
            }
            NType::Const(inner) => Self::build(inner, cursor),
            _ => unreachable!(),
        }
    }

    /// 获取叶子
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

#[cfg(test)]
mod test {
    use nanoc_parser::{
        ast::{AstNode, ConstIndexVal, ConstInitVal, SyntaxNode, Type},
        parser::Parser,
        syntax_kind::SyntaxKind,
        visitor::Visitor as _,
    };

    use crate::{
        array::{ArrayInitError, ArrayTree},
        module::Module,
        value::Value,
    };

    fn get_init_val_node(root: &SyntaxNode) -> SyntaxNode {
        let res = root
            .descendants()
            .find(|x| matches!(x.kind(), SyntaxKind::CONST_INIT_VAL));
        res.unwrap()
    }

    fn get_type_node(root: &SyntaxNode) -> SyntaxNode {
        let res = root.descendants().find(|x| x.kind() == SyntaxKind::TYPE);
        res.unwrap()
    }

    fn get_const_index_node(root: &SyntaxNode) -> SyntaxNode {
        let res = root
            .descendants()
            .find(|x| x.kind() == SyntaxKind::CONST_INDEX_VAL);
        res.unwrap()
    }

    fn generator(text: &str) -> Result<(String, Module, ArrayTree), ArrayInitError> {
        let p = Parser::new(text);
        let (tree, _) = p.parse();
        let root = Parser::new_root(tree);
        let init_val_node = ConstInitVal::cast(get_init_val_node(&root)).unwrap();
        // dbg!(init_val_node.syntax());
        let mut module = Module::default();
        module.walk(&root);
        let basic_ty = Module::build_basic_type(&Type::cast(get_type_node(&root)).unwrap());
        let ty = module
            .build_array_type(
                basic_ty,
                &ConstIndexVal::cast(get_const_index_node(&root)).unwrap(),
            )
            .unwrap();
        dbg!(&ty);
        let array_tree = ArrayTree::new(&ty, init_val_node)?;
        Ok((array_tree.to_string(), module, array_tree))
    }

    #[test]
    fn normal_array() {
        let text = "const int a[2] = {1, 2}";
        let (tree, _, _) = generator(text).unwrap();
        insta::assert_snapshot!(tree);
    }

    #[test]
    fn special_array() {
        let text = "const int arr[2][3][4] = {1, 2, 3, 4, {5}, {6}, {7, 8}};";
        let (tree, module, array_tree) = generator(text).unwrap();
        insta::assert_snapshot!(tree);
        let indices = [0, 1, 0];
        let leaf = array_tree.get_leaf(&indices).unwrap();
        dbg!(&leaf);
        dbg!(&module.value_table);
        let value = leaf.get_const_value(&module.value_table);
        assert_eq!(value, Some(&Value::Int(5)));
    }

    // #[test]
    // fn bad_test_case() {
    //     let text = "const int arr[2][2][2] = {{}, 1, {}};";
    //     let tree = generator(text);
    //     assert!(tree.is_err());
    // }
}
