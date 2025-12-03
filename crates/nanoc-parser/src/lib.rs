mod ast;
mod lexer;
mod parser;

// #[cfg(test)]
// mod tests;

/// Parser errors
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    LexerError(#[from] lexer::LexerError),
}
