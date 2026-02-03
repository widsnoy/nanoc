pub mod parse;
pub mod visitor;

#[cfg(test)]
mod test;

pub use lexer::{Lexer, LexerError};
pub use syntax::{AstNode, SyntaxKind, SyntaxNode, SyntaxToken, *};

/// 解析器错误
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    LexerError(#[from] LexerError),
}
