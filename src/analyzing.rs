use analyzer::module::Module;
use rowan::GreenNode;

use crate::error::{CompilerError, Result};

/// 语义分析阶段
///
/// 对语法树进行语义分析，包括：
/// - 符号解析
/// - 类型检查
/// - 作用域分析
/// - 常量求值
///
/// # 参数
/// - `green_node`: 语法分析阶段生成的 green node
///
/// # 返回
/// - `Ok(Module)`: 成功时返回包含符号表和类型信息的 Module
/// - `Err(CompilerError)`: 如果有语义错误，返回包含所有错误的 CompilerError
///
/// # 错误处理
/// 如果存在语义错误，会立即返回错误，不继续后续编译阶段
pub fn analyze(green_node: GreenNode) -> Result<Module> {
    let mut analyzer = Module::new(green_node);
    analyzer.analyze();

    if !analyzer.semantic_errors.is_empty() {
        return Err(CompilerError::Semantic(analyzer.semantic_errors));
    }

    Ok(analyzer)
}
