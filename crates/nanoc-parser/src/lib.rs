mod ast;
mod lexer;
pub mod parser;

pub mod syntax_kind;

#[cfg(test)]
mod test;

/// Parser errors
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    LexerError(#[from] lexer::LexerError),
}
