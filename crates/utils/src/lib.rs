//! 辅助函数

use syntax::{AirycLanguage, AstNode, SyntaxNode};

use tools::TextRange;

pub fn find_node_by_range<N>(root: &SyntaxNode, range: TextRange) -> Option<N>
where
    N: AstNode<Language = AirycLanguage>,
{
    let element = root.covering_element(*range);

    element.ancestors().find_map(|n| N::cast(n))
}

/// 获取去掉首尾的换行、空格的范围
pub fn trim_node_text_range(node: &impl AstNode<Language = AirycLanguage>) -> TextRange {
    let mut l = u32::MAX;
    let mut r = 0u32;
    node.syntax().children_with_tokens().for_each(|x| match x {
        rowan::NodeOrToken::Node(x) if !x.kind().is_trivia() => {
            l = l.min(x.text_range().start().into());
            r = r.max(x.text_range().end().into());
        }
        rowan::NodeOrToken::Token(x) if !x.kind().is_trivia() => {
            l = l.min(x.text_range().start().into());
            r = r.max(x.text_range().end().into());
        }
        _ => {}
    });
    if l > r {
        TextRange::new(0, 0)
    } else {
        TextRange::new(l, r)
    }
}

/// 从 Name 节点中提取变量名和范围
/// 返回 Some((name, range)) 如果两者都存在，否则返回 None
pub fn extract_name_and_range(name_node: &syntax::ast::Name) -> Option<(String, TextRange)> {
    let name = name_node.var_name()?;
    let range = name_node.var_range()?;
    Some((name, range))
}

/// 定义 ID 包装类型的宏，用于 arena 索引
#[macro_export]
macro_rules! define_id_type {
    ($name:ident) => {
        #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
        pub struct $name(pub thunderdome::Index);

        impl $name {
            pub fn none() -> Self {
                $name(thunderdome::Index::DANGLING)
            }
        }

        impl From<thunderdome::Index> for $name {
            fn from(index: thunderdome::Index) -> Self {
                $name(index)
            }
        }

        impl Deref for $name {
            type Target = thunderdome::Index;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
    };
}

#[macro_export]
macro_rules! define_module_id_type {
    ($name:ident) => {
        #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
        pub struct $name {
            pub module: FileID,
            pub index: thunderdome::Index,
        }

        impl $name {
            pub fn none() -> Self {
                $name {
                    module: FileID::none(),
                    index: thunderdome::Index::DANGLING,
                }
            }

            pub fn new(module: FileID, index: thunderdome::Index) -> Self {
                $name { module, index }
            }
        }

        impl From<(FileID, thunderdome::Index)> for $name {
            fn from((module, index): (FileID, thunderdome::Index)) -> Self {
                $name { module, index }
            }
        }
    };
}
