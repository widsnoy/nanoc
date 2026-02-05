use analyzer::module::Module;
use codegen::llvm_ir::Program;
use inkwell::builder::Builder;
use inkwell::context::Context as LlvmContext;
use inkwell::module::Module as LlvmModule;
use inkwell::targets::TargetMachine;
use parser::SyntaxNode;
use parser::ast::{AstNode, CompUnit};
use rowan::GreenNode;

use crate::error::{CompilerError, Result};

/// LLVM 代码生成上下文
///
/// 包含 LLVM IR 生成所需的所有上下文信息
#[allow(dead_code)]
pub struct CodegenContext<'ctx> {
    pub context: &'ctx LlvmContext,
    pub module: LlvmModule<'ctx>,
    pub builder: Builder<'ctx>,
}

/// LLVM IR 代码生成阶段
///
/// 将语义分析后的 AST 转换为 LLVM IR
///
/// # 参数
/// - `context`: LLVM 上下文
/// - `module_name`: 模块名称（通常是源文件名）
/// - `green_node`: 语法树的 green node
/// - `analyzer`: 语义分析结果
///
/// # 返回
/// - `Ok(CodegenContext)`: 成功时返回包含生成的 LLVM module 的上下文
/// - `Err(CompilerError)`: 代码生成失败时返回错误
pub fn generate_ir<'ctx>(
    context: &'ctx LlvmContext,
    module_name: &str,
    green_node: GreenNode,
    analyzer: &Module,
) -> Result<CodegenContext<'ctx>> {
    let module = context.create_module(module_name);
    let builder = context.create_builder();

    let mut program = Program {
        context,
        builder: &builder,
        module: &module,
        analyzer,
        symbols: Default::default(),
    };

    let root = SyntaxNode::new_root(green_node);
    let comp_unit = CompUnit::cast(root).ok_or(CompilerError::InvalidRoot)?;

    program.compile_comp_unit(comp_unit)?;

    Ok(CodegenContext {
        context,
        module,
        builder,
    })
}

/// 优化和验证 LLVM IR
///
/// 设置目标机器的 triple 和 data layout，并验证生成的 LLVM IR
///
/// # 参数
/// - `module`: LLVM module
/// - `machine`: 目标机器
///
/// # 返回
/// - `Ok(())`: 验证成功
/// - `Err(CompilerError)`: 验证失败时返回错误
pub fn optimize_and_verify(module: &LlvmModule, machine: &TargetMachine) -> Result<()> {
    module.set_triple(&machine.get_triple());
    module.set_data_layout(&machine.get_target_data().get_data_layout());

    module
        .verify()
        .map_err(|e| CompilerError::LlvmVerification(e.to_string_lossy().to_string()))?;

    Ok(())
}
