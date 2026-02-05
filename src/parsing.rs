use lexer::LexerError;
use parser::parse::ParserError;
use rowan::GreenNode;

use crate::error::{CompilerError, Result};

/// 语法分析阶段
///
/// 对输入源代码进行词法分析和语法分析，生成 Green Node（lossless syntax tree）
///
/// # 参数
/// - `input`: 源代码字符串
///
/// # 返回
/// - `Ok((GreenNode, Vec<ParserError>, Vec<LexerError>))`: 成功时返回 green node 和错误列表
/// - `Err(CompilerError)`: 如果有 parser 错误，返回包含所有错误的 CompilerError
///
/// # 错误处理
/// 如果存在 parser 错误，会立即返回错误，不继续后续编译阶段
pub fn parse(input: &str) -> Result<(GreenNode, Vec<ParserError>, Vec<LexerError>)> {
    let parser = parser::parse::Parser::new(input);
    let (green_node, parser_errors, lexer_errors) = parser.parse();

    if !lexer_errors.is_empty() {
        return Err(CompilerError::Lexer(lexer_errors));
    }

    if !parser_errors.is_empty() {
        return Err(CompilerError::Parser(parser_errors));
    }

    Ok((green_node, parser_errors, lexer_errors))
}
