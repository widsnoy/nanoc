use std::collections::HashMap;
use std::path::Path;

use analyzer::error::SemanticError;
use parser::parse::Parser;
use rowan::GreenNode;
use vfs::Vfs;

use crate::error::{CompilerError, Result};

/// 解析源代码为语法树
pub fn parse(input_path: &Path, input: String) -> Result<GreenNode> {
    let parser = Parser::new(input.as_str());
    let (green_node, errors) = parser.parse();

    if !errors.is_empty() {
        // 创建临时 VFS 用于错误报告
        let vfs = Vfs::default();
        let absolute_path = input_path
            .canonicalize()
            .unwrap_or_else(|_| input_path.to_path_buf());
        let file_id = vfs.new_file(absolute_path, input);

        // 将 ParserError 包装为 SemanticError
        let semantic_errors: Vec<SemanticError> =
            errors.into_iter().map(SemanticError::ParserError).collect();

        let mut errors_by_file = HashMap::new();
        errors_by_file.insert(file_id, semantic_errors);

        return Err(CompilerError::Semantic(errors_by_file));
    }

    Ok(green_node)
}
