use analyzer::error::SemanticError;
use lexer::LexerError;
use miette::Diagnostic as _;
use parser::parse::ParserError;
use tools::TextRange;

/// Language Server 统一错误类型
#[derive(Debug, Clone)]
pub enum LspError {
    Lexer(LexerError),
    Parser(ParserError),
    Semantic(SemanticError),
}

impl LspError {
    /// 获取错误消息（使用 thiserror 的 Display）
    pub fn message(&self) -> String {
        match self {
            Self::Lexer(e) => e.to_string(),
            Self::Parser(e) => e.to_string(),
            Self::Semantic(e) => e.to_string(),
        }
    }

    /// 获取错误代码（使用 miette 的 Diagnostic trait）
    pub fn code(&self) -> Option<String> {
        match self {
            Self::Lexer(e) => e.code().map(|c| c.to_string()),
            Self::Parser(e) => e.code().map(|c| c.to_string()),
            Self::Semantic(e) => e.code().map(|c| c.to_string()),
        }
    }

    /// 获取错误的位置范围
    pub fn range(&self) -> &TextRange {
        match self {
            Self::Lexer(e) => e.range(),
            Self::Parser(e) => e.range(),
            Self::Semantic(e) => e.range(),
        }
    }
}

impl From<ParserError> for LspError {
    fn from(e: ParserError) -> Self {
        Self::Parser(e)
    }
}

impl From<SemanticError> for LspError {
    fn from(e: SemanticError) -> Self {
        Self::Semantic(e)
    }
}

impl From<LexerError> for LspError {
    fn from(e: LexerError) -> Self {
        Self::Lexer(e)
    }
}
