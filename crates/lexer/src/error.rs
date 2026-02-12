#![allow(unused_assignments)]

use miette::Diagnostic;
use thiserror::Error;
use tools::TextRange;

/// Lexer 错误类型（用于 logos）
#[derive(Debug, Default, Clone, PartialEq)]
pub enum LexerErrorKind {
    InvalidInteger,
    #[default]
    Unknown,
}

/// Lexer 错误（带位置信息，用于诊断）
#[derive(Debug, Clone, PartialEq, Error, Diagnostic)]
pub enum LexerError {
    #[error("invalid integer literal: {text}")]
    #[diagnostic(code(lexer::invalid_integer))]
    InvalidInteger {
        text: String,
        #[label("here")]
        range: TextRange,
    },

    #[error("unknown lexer error")]
    #[diagnostic(code(lexer::unknown))]
    Unknown {
        #[label("here")]
        range: TextRange,
    },
}

impl LexerError {
    pub fn range(&self) -> &TextRange {
        match self {
            LexerError::InvalidInteger { range, .. } | LexerError::Unknown { range } => range,
        }
    }
}
