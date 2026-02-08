use std::collections::HashMap;
use std::path::PathBuf;

use analyzer::project::Project;
use vfs::Vfs;

use crate::error::{CompilerError, Result, SemanticErrors};

/// 分析项目中的所有文件
pub fn analyze_project(input_paths: &[PathBuf]) -> Result<Project> {
    if input_paths.is_empty() {
        return Err(CompilerError::Semantic(Box::new(SemanticErrors {
            errors_by_file: HashMap::new(),
            vfs: Vfs::default(),
        })));
    }

    // 构建 VFS
    let mut vfs = Vfs::default();
    for input_path in input_paths {
        let text = std::fs::read_to_string(input_path).map_err(CompilerError::Io)?;
        let absolute_path = input_path
            .canonicalize()
            .unwrap_or_else(|_| input_path.clone());
        vfs.new_file(absolute_path, text);
    }

    // 初始化并分析项目
    let mut project = Project::default();
    project.initialize(vfs);

    // 按文件收集错误
    let mut errors_by_file = HashMap::new();
    for (file_id, &module_id) in project.file_index.iter() {
        let module = project.modules.get(module_id.0).unwrap();
        if !module.semantic_errors.is_empty() {
            errors_by_file.insert(*file_id, module.semantic_errors.clone());
        }
    }

    if !errors_by_file.is_empty() {
        return Err(CompilerError::Semantic(Box::new(SemanticErrors {
            errors_by_file,
            vfs: project.vfs,
        })));
    }

    Ok(project)
}
