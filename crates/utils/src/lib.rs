//! 通用工具函数

use rowan::TextRange;
use syntax::{AirycLanguage, AstNode, SyntaxNode};

pub fn find_node_by_range<N>(root: &SyntaxNode, range: TextRange) -> Option<N>
where
    N: AstNode<Language = AirycLanguage>,
{
    let element = root.covering_element(range);

    element.ancestors().find_map(|n| N::cast(n))
}
