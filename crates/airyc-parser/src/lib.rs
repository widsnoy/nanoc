pub mod ast;
mod lexer;
pub mod parser;
pub mod visitor;

pub mod syntax_kind;

#[cfg(test)]
mod test;

/// 解析器错误
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    LexerError(#[from] lexer::LexerError),
}
