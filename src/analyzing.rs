use std::collections::HashMap;
use std::path::PathBuf;

use analyzer::{checker::RecursiveTypeChecker, project::Project};
use vfs::Vfs;

use crate::error::{CompilerError, Result};

/// 分析项目中的所有文件
pub fn analyze_project(input_paths: &[PathBuf], vfs: &Vfs) -> Result<Project> {
    for input_path in input_paths {
        let text = std::fs::read_to_string(input_path).map_err(CompilerError::Io)?;
        let absolute_path = input_path
            .canonicalize()
            .unwrap_or_else(|_| input_path.clone());
        vfs.new_file(absolute_path, text);
    }

    // 初始化并分析项目
    let mut project = Project::new().with_checker::<RecursiveTypeChecker>();
    project.full_initialize(vfs);

    // 按文件收集错误
    let mut errors_by_file = HashMap::new();
    for module in project.modules.values() {
        if !module.semantic_errors.is_empty() {
            errors_by_file.insert(module.file_id, module.semantic_errors.clone());
        }
    }

    if !errors_by_file.is_empty() {
        return Err(CompilerError::Analyze(errors_by_file));
    }

    Ok(project)
}
