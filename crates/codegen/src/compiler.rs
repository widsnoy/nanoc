use std::path::Path;

use analyzer::module::Module;
use analyzer::project::Project;
use inkwell::context::Context as LlvmContext;
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine,
};
use rowan::GreenNode;
use syntax::SyntaxNode;
use syntax::ast::{AstNode, CompUnit};

use crate::error::{CodegenError, Result};
use crate::llvm_ir::Program;

/// 优化级别
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum OptLevel {
    O0,
    O1,
    O2,
    O3,
}

impl From<OptLevel> for inkwell::OptimizationLevel {
    fn from(level: OptLevel) -> Self {
        match level {
            OptLevel::O0 => inkwell::OptimizationLevel::None,
            OptLevel::O1 => inkwell::OptimizationLevel::Less,
            OptLevel::O2 => inkwell::OptimizationLevel::Default,
            OptLevel::O3 => inkwell::OptimizationLevel::Aggressive,
        }
    }
}

/// 编译到 LLVM IR 字符串
///
/// 将语义分析后的 AST 转换为 LLVM IR 文本格式
///
/// # 参数
/// - `module_name`: 模块名称（通常是源文件名）
/// - `green_node`: 语法树的 green node
/// - `analyzer`: 语义分析结果
/// - `opt_level`: 优化级别
///
/// # 返回
/// - `Ok(String)`: 成功时返回 LLVM IR 字符串
/// - `Err(CodegenError)`: 代码生成失败时返回错误
pub fn compile_to_ir_string(
    module_name: &str,
    green_node: GreenNode,
    analyzer: &Module,
    opt_level: OptLevel,
) -> Result<String> {
    let context = LlvmContext::create();
    let module = generate_and_optimize(&context, module_name, green_node, analyzer, opt_level)?;
    Ok(module.print_to_string().to_string())
}

/// 编译到 LLVM IR 文件
///
/// 将语义分析后的 AST 转换为 LLVM IR 并写入 .ll 文件
///
/// # 参数
/// - `module_name`: 模块名称（通常是源文件名）
/// - `green_node`: 语法树的 green node
/// - `analyzer`: 语义分析结果
/// - `opt_level`: 优化级别
/// - `output_path`: 输出文件路径（.ll 文件）
///
/// # 返回
/// - `Ok(())`: 写入成功
/// - `Err(CodegenError)`: 代码生成或写入失败时返回错误
pub fn compile_to_ir_file(
    module_name: &str,
    green_node: GreenNode,
    analyzer: &Module,
    opt_level: OptLevel,
    output_path: &Path,
) -> Result<()> {
    let context = LlvmContext::create();
    let module = generate_and_optimize(&context, module_name, green_node, analyzer, opt_level)?;
    module
        .print_to_file(output_path)
        .map_err(|e| CodegenError::LlvmWrite(e.to_string()))?;
    Ok(())
}

/// 编译到目标文件字节数据
///
/// 将语义分析后的 AST 转换为目标文件（.o）的字节数据
///
/// # 参数
/// - `module_name`: 模块名称（通常是源文件名）
/// - `green_node`: 语法树的 green node
/// - `analyzer`: 语义分析结果
/// - `opt_level`: 优化级别
///
/// # 返回
/// - `Ok(Vec<u8>)`: 成功时返回目标文件的字节数据
/// - `Err(CodegenError)`: 代码生成失败时返回错误
pub fn compile_to_object_bytes(
    module_name: &str,
    green_node: GreenNode,
    analyzer: &Module,
    opt_level: OptLevel,
) -> Result<Vec<u8>> {
    let context = LlvmContext::create();
    let module = generate_and_optimize(&context, module_name, green_node, analyzer, opt_level)?;

    // 初始化目标机器
    let machine = create_target_machine(opt_level)?;

    // 生成目标文件到内存
    let buffer = machine
        .write_to_memory_buffer(&module, FileType::Object)
        .map_err(|e| CodegenError::LlvmWrite(e.to_string()))?;

    Ok(buffer.as_slice().to_vec())
}

/// 内部函数：生成并优化 LLVM IR
///
/// 执行以下步骤：
/// 1. 创建 LLVM module 和 builder
/// 2. 编译 AST 到 LLVM IR
/// 3. 设置目标机器信息（triple 和 data layout）
/// 4. 验证生成的 IR
fn generate_and_optimize<'ctx>(
    context: &'ctx LlvmContext,
    module_name: &str,
    green_node: GreenNode,
    analyzer: &Module,
    opt_level: OptLevel,
) -> Result<inkwell::module::Module<'ctx>> {
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
    let comp_unit = CompUnit::cast(root).ok_or(CodegenError::InvalidRoot)?;

    program.compile_comp_unit(comp_unit)?;

    // 设置目标机器信息
    let machine = create_target_machine(opt_level)?;
    module.set_triple(&machine.get_triple());
    module.set_data_layout(&machine.get_target_data().get_data_layout());

    // 验证
    module
        .verify()
        .map_err(|e| CodegenError::LlvmVerification(e.to_string_lossy().to_string()))?;

    Ok(module)
}

/// 编译多个模块到目标文件字节数据
///
/// 将 Project 中的所有模块分别编译为目标文件
///
/// # 参数
/// - `project`: 包含所有模块的项目
/// - `opt_level`: 优化级别
///
/// # 返回
/// - `Ok(Vec<(String, Vec<u8>)>)`: 成功时返回 (模块名, 目标文件字节) 的列表
/// - `Err(CodegenError)`: 代码生成失败时返回错误
pub fn compile_project_to_object_bytes(
    project: &Project,
    opt_level: OptLevel,
) -> Result<Vec<(String, Vec<u8>)>> {
    let mut object_files = Vec::new();

    for (file_id, module) in &project.modules {
        let module_name = project
            .vfs
            .get_file_by_file_id(file_id)
            .and_then(|file| {
                std::path::Path::new(&file.path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "unknown".to_string());

        let object_bytes =
            compile_to_object_bytes(&module_name, module.green_tree.clone(), module, opt_level)?;

        object_files.push((module_name, object_bytes));
    }

    Ok(object_files)
}

/// 内部函数：创建目标机器
///
/// 初始化 LLVM 目标并创建目标机器实例
fn create_target_machine(opt_level: OptLevel) -> Result<TargetMachine> {
    Target::initialize_all(&InitializationConfig::default());
    let triple = inkwell::targets::TargetMachine::get_default_triple();
    let target =
        Target::from_triple(&triple).map_err(|e| CodegenError::TargetMachine(e.to_string()))?;

    target
        .create_target_machine(
            &triple,
            "generic",
            "",
            opt_level.into(),
            RelocMode::Default,
            CodeModel::Default,
        )
        .ok_or_else(|| CodegenError::TargetMachine("failed to create target machine".to_string()))
}
