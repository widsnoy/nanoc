#![allow(unused_assignments)]

use lexer::LexerError;
use miette::Diagnostic;
use syntax::SyntaxKind;
use thiserror::Error;
use tools::TextRange;

/// 格式化 SyntaxKind 列表为字符串
fn format_kinds(kinds: &[SyntaxKind]) -> String {
    kinds
        .iter()
        .map(|k| format!("{:?}", k))
        .collect::<Vec<_>>()
        .join(", ")
}

#[derive(Debug, Clone, Error, Diagnostic)]
pub enum ParserError {
    #[error("expected one of: {}", format_kinds(expected))]
    #[diagnostic(code(parser::expected_token))]
    Expected {
        expected: Vec<SyntaxKind>,
        #[label("here")]
        range: TextRange,
    },

    #[error(transparent)]
    #[diagnostic(transparent)]
    LexerError(#[from] LexerError),
}

impl ParserError {
    /// 获取错误的位置范围
    pub fn range(&self) -> &TextRange {
        match self {
            Self::Expected { range, .. } => range,
            Self::LexerError(e) => e.range(),
        }
    }
}
