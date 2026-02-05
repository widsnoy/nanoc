use std::path::Path;

use inkwell::module::Module as LlvmModule;
use inkwell::targets::{FileType, TargetMachine};

use crate::error::{CompilerError, Result};

/// 写入 LLVM IR 到文件
///
/// 将生成的 LLVM IR 以文本格式写入 .ll 文件
///
/// # 参数
/// - `module`: LLVM module
/// - `output_path`: 输出文件路径（.ll 文件）
///
/// # 返回
/// - `Ok(())`: 写入成功
/// - `Err(CompilerError)`: 写入失败时返回错误
pub fn write_ir(module: &LlvmModule, output_path: &Path) -> Result<()> {
    module
        .print_to_file(output_path)
        .map_err(|e| CompilerError::LlvmWrite(e.to_string()))?;
    Ok(())
}

/// 链接生成可执行文件
///
/// 将 LLVM IR 编译为目标文件（.o），然后使用 clang 链接运行时库生成可执行文件
///
/// # 参数
/// - `module`: LLVM module
/// - `machine`: 目标机器
/// - `output_dir`: 输出目录
/// - `module_name`: 模块名称（用于生成文件名）
/// - `runtime_path`: 运行时库路径
///
/// # 返回
/// - `Ok(())`: 链接成功
/// - `Err(CompilerError)`: 链接失败时返回错误
///
/// # 过程
/// 1. 生成目标文件（.o）
/// 2. 使用 clang 链接运行时库
/// 3. 生成最终可执行文件
pub fn link_executable(
    module: &LlvmModule,
    machine: &TargetMachine,
    output_dir: &Path,
    module_name: &str,
    runtime_path: &Path,
) -> Result<()> {
    // 生成目标文件路径
    let object_path = output_dir.join(format!("{}.o", module_name));
    let output_path = output_dir.join(module_name);

    // 写入目标文件
    machine
        .write_to_file(module, FileType::Object, &object_path)
        .map_err(|e| CompilerError::LlvmWrite(e.to_string()))?;

    // 使用 clang 链接
    let status = std::process::Command::new("clang")
        .arg(&object_path)
        .arg(runtime_path)
        .arg("-o")
        .arg(&output_path)
        .status()
        .map_err(|e| CompilerError::Link(e.to_string()))?;

    if !status.success() {
        return Err(CompilerError::LinkerFailed);
    }

    Ok(())
}
