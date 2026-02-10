use std::fs;
use std::path::Path;

use crate::error::{CompilerError, Result};

/// 链接多个目标文件生成可执行文件
///
/// 将多个目标文件字节数据写入临时文件，然后使用 clang 链接运行时库生成可执行文件
///
/// # 参数
/// - `object_files`: (模块名, 目标文件字节) 的列表
/// - `output_dir`: 输出目录
/// - `output_name`: 输出可执行文件名称
/// - `runtime_path`: 运行时库路径
///
/// # 返回
/// - `Ok(())`: 链接成功
/// - `Err(CompilerError)`: 链接失败时返回错误
pub fn link_multiple_objects(
    object_files: &[(String, Vec<u8>)],
    output_dir: &Path,
    output_name: &str,
    runtime_path: &Path,
) -> Result<()> {
    let mut object_paths = Vec::new();

    // 写入所有目标文件
    for (module_name, object_bytes) in object_files {
        let object_path = output_dir.join(format!("{}.o", module_name));
        fs::write(&object_path, object_bytes)?;
        object_paths.push(object_path);
    }

    let output_path = output_dir.join(output_name);

    // 使用 clang 链接所有目标文件
    let mut cmd = std::process::Command::new("clang");
    for object_path in &object_paths {
        cmd.arg(object_path);
    }
    cmd.arg(runtime_path).arg("-o").arg(&output_path);

    let output = cmd
        .output()
        .map_err(|e| CompilerError::Link(e.to_string()))?;

    if !output.status.success() {
        let std_err = String::from_utf8_lossy(&output.stderr);
        return Err(CompilerError::Link(std_err.into_owned()));
    }

    // 删除临时目标文件
    for object_path in &object_paths {
        let _ = fs::remove_file(object_path);
    }

    Ok(())
}
