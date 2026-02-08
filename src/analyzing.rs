use std::path::PathBuf;

use analyzer::project::Project;
use vfs::Vfs;

use crate::error::{CompilerError, Result};

/// 语义分析阶段
///
/// 对源文件进行语义分析，支持单文件和多文件（跨模块引用）
///
/// # 参数
/// - `input_paths`: 输入文件路径列表
///
/// # 返回
/// - `Ok(Project)`: 成功时返回包含所有模块的 Project
/// - `Err(CompilerError)`: 如果有语义错误，返回包含所有错误的 CompilerError
pub fn analyze_project(input_paths: &[PathBuf]) -> Result<Project> {
    if input_paths.is_empty() {
        return Err(CompilerError::Semantic(vec![]));
    }

    // 创建空的 VFS，然后只添加用户指定的文件（使用绝对路径）
    let mut vfs = Vfs::default();

    for input_path in input_paths {
        // 读取文件内容
        let text = std::fs::read_to_string(input_path).map_err(|e| {
            eprintln!("Error reading file {:?}: {}", input_path, e);
            CompilerError::Semantic(vec![analyzer::error::SemanticError::InvalidPath {
                range: tools::TextRange::default(),
            }])
        })?;

        // 转换为绝对路径
        let absolute_path = input_path
            .canonicalize()
            .unwrap_or_else(|_| input_path.clone());

        // 添加到 VFS（使用绝对路径）
        vfs.new_file(absolute_path, text);
    }

    // 创建 Project 并初始化（会自动进行三阶段分析）
    let mut project = Project::default();
    project.initialize(vfs);

    // 收集所有模块的错误
    let mut all_errors = Vec::new();
    for (_, module) in project.modules.iter() {
        if !module.semantic_errors.is_empty() {
            all_errors.extend(module.semantic_errors.clone());
        }
    }

    if !all_errors.is_empty() {
        return Err(CompilerError::Semantic(all_errors));
    }

    Ok(project)
}
