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
