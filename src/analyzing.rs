use std::path::PathBuf;

use analyzer::{module::Module, project::Project};
use vfs::Vfs;

use crate::error::{CompilerError, Result};

/// 语义分析阶段（多文件支持）
///
/// 对多个源文件进行语义分析，支持跨模块引用
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

    // 确定工作目录（使用第一个文件的父目录）
    let workspace = input_paths[0]
        .parent()
        .filter(|p| !p.as_os_str().is_empty()) // 过滤掉空路径
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    // 创建 VFS 并添加所有文件
    let vfs = Vfs::new(&workspace).map_err(|e| {
        eprintln!("DEBUG: VFS creation failed: {}", e);
        CompilerError::Semantic(vec![analyzer::error::SemanticError::InvalidPath {
            range: tools::TextRange::default(),
        }])
    })?;

    // 创建 Project 并初始化（会自动进行三阶段分析）
    let mut project = Project::default();
    project.initialize(workspace, vfs);

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

/// 单文件分析（向后兼容）
pub fn analyze_single(input_path: &PathBuf) -> Result<Module<'static>> {
    // 单文件分析（不支持跨模块）
    let text = std::fs::read_to_string(input_path).map_err(|_| {
        CompilerError::Semantic(vec![analyzer::error::SemanticError::InvalidPath {
            range: tools::TextRange::default(),
        }])
    })?;

    let parser = parser::parse::Parser::new(&text);
    let (green_node, _, _) = parser.parse();

    let mut analyzer = Module::new(green_node);
    analyzer.analyze();

    if !analyzer.semantic_errors.is_empty() {
        return Err(CompilerError::Semantic(analyzer.semantic_errors));
    }

    Ok(analyzer)
}
