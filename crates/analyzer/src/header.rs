//! 头文件分析器：解析 import 语句，导入符号

use std::collections::HashMap;

use syntax::{AstNode as _, SyntaxNode, ast::CompUnit};
use thunderdome::Arena;
use tools::TextRange;
use vfs::{FileID, Vfs};

use crate::{
    error::SemanticError,
    module::{FunctionID, Module, ModuleID, StructID},
};

/// 导入信息
#[derive(Debug, Clone)]
pub struct ImportInfo {
    /// 要导入的函数：(名称, FunctionID)
    pub functions: Vec<(String, FunctionID)>,
    /// 要导入的结构体：(名称, StructID)
    pub structs: Vec<(String, StructID)>,
}

/// 单个模块的所有导入信息
#[derive(Debug)]
pub struct ModuleImports {
    pub module_id: ModuleID,
    pub imports: Vec<(ImportInfo, TextRange)>, // (导入信息, 错误位置)
    pub errors: Vec<SemanticError>,
}

/// Header 分析器（无状态，使用静态方法）
pub struct HeaderAnalyzer;

impl HeaderAnalyzer {
    /// 收集单个模块的导入信息
    pub fn collect_module_imports(
        module: &Module,
        current_file_id: FileID,
        vfs: &Vfs,
        file_index: &HashMap<FileID, ModuleID>,
        modules: &Arena<Module>,
    ) -> ModuleImports {
        let mut imports = Vec::new();
        let mut errors = Vec::new();
        let root = SyntaxNode::new_root(module.green_tree.clone());

        if let Some(comp_unit) = CompUnit::cast(root) {
            for header in comp_unit.headers() {
                if let Some(path_node) = header.path() {
                    let path_node_range_trimmed = utils::trim_node_text_range(&path_node);
                    // 解析路径
                    match Self::resolve_import_path(&path_node, current_file_id, vfs, file_index) {
                        Ok((target_module_id, symbol_name)) => {
                            // 收集需要导入的符号
                            match Self::collect_import_info(
                                target_module_id,
                                symbol_name.as_deref(),
                                path_node_range_trimmed,
                                modules,
                            ) {
                                Ok(import_info) => {
                                    imports.push((import_info, path_node_range_trimmed));
                                }
                                Err(e) => errors.push(e),
                            }
                        }
                        Err(e) => errors.push(e),
                    }
                }
            }
        }

        ModuleImports {
            module_id: module.module_id,
            imports,
            errors,
        }
    }

    /// 将单个模块的导入信息应用到该模块（写入操作）
    pub fn apply_module_imports(module: &mut Module, module_imports: ModuleImports) {
        // 添加错误
        module.semantic_errors.extend(module_imports.errors);

        // 应用导入
        for (import_info, range) in module_imports.imports {
            if let Err(e) = Self::apply_imports_to_module(module, import_info, range) {
                module.semantic_errors.push(e);
            }
        }
    }

    /// 解析 import 路径，返回目标模块 ID 和可选的符号名
    fn resolve_import_path(
        path_node: &syntax::ast::Path,
        current_file_id: FileID,
        vfs: &Vfs,
        file_index: &HashMap<FileID, ModuleID>,
    ) -> Result<(ModuleID, Option<String>), SemanticError> {
        let path_node_range_trimmed = utils::trim_node_text_range(path_node);
        // 获取字符串字面量（路径）
        let path_token =
            path_node
                .string_literal()
                .ok_or_else(|| SemanticError::ImportPathNotFound {
                    path: "<invalid>".to_string(),
                    range: path_node_range_trimmed,
                })?;

        // 获取文本并去掉引号
        let path_with_quotes = path_token.text();
        let path_text = path_with_quotes.trim_matches('"');

        // 获取可选的符号名
        let symbol_name = path_node.symbol().map(|s| s.text().to_string());

        // 获取当前文件路径
        let current_file = vfs.get_file_by_file_id(&current_file_id).ok_or_else(|| {
            SemanticError::ImportPathNotFound {
                path: path_text.to_string(),
                range: path_node_range_trimmed,
            }
        })?;

        // 解析相对路径
        // 注意：VFS 中存储的是相对于 workspace 的路径
        let current_dir = current_file
            .path
            .parent()
            .unwrap_or(std::path::Path::new(""));

        // 添加 .airy 扩展名（如果没有）
        let target_path = if path_text.ends_with(".airy") {
            current_dir.join(path_text)
        } else {
            current_dir.join(format!("{}.airy", path_text))
        };

        // 在 VFS 中查找文件（使用相对路径）
        let target_file_id = vfs.get_file_id_by_path(&target_path).ok_or_else(|| {
            SemanticError::ImportPathNotFound {
                path: format!("{} (resolved to {:?})", path_text, target_path),
                range: path_node_range_trimmed,
            }
        })?;

        // 通过 file_index 映射到 ModuleID
        let target_module_id = file_index.get(target_file_id).copied().ok_or_else(|| {
            SemanticError::ImportPathNotFound {
                path: path_text.to_string(),
                range: path_node_range_trimmed,
            }
        })?;

        Ok((target_module_id, symbol_name))
    }

    /// 从目标模块收集需要导入的符号信息
    fn collect_import_info(
        target_module_id: ModuleID,
        symbol_name: Option<&str>,
        range: TextRange,
        modules: &Arena<Module>,
    ) -> Result<ImportInfo, SemanticError> {
        // 获取目标模块
        let target_module =
            modules
                .get(target_module_id.0)
                .ok_or_else(|| SemanticError::ImportPathNotFound {
                    path: format!("{:?}", target_module_id),
                    range,
                })?;

        if let Some(symbol) = symbol_name {
            // 导入特定符号
            Self::collect_specific_symbol(target_module, symbol, range)
        } else {
            // 导入所有符号
            Self::collect_all_symbols(target_module)
        }
    }

    /// 收集特定符号
    fn collect_specific_symbol(
        target_module: &Module,
        symbol_name: &str,
        range: TextRange,
    ) -> Result<ImportInfo, SemanticError> {
        let mut import_info = ImportInfo {
            functions: Vec::new(),
            structs: Vec::new(),
        };

        // 查找函数
        if let Some(&func_id) = target_module.function_map.get(symbol_name) {
            import_info
                .functions
                .push((symbol_name.to_string(), func_id));
            return Ok(import_info);
        }

        // 查找结构体
        if let Some(&struct_id) = target_module.struct_map.get(symbol_name) {
            import_info
                .structs
                .push((symbol_name.to_string(), struct_id));
            return Ok(import_info);
        }

        // 符号未找到
        Err(SemanticError::ImportSymbolNotFound {
            symbol: symbol_name.to_string(),
            module_path: format!("{:?}", target_module.module_id),
            range,
        })
    }

    /// 收集所有符号
    fn collect_all_symbols(target_module: &Module) -> Result<ImportInfo, SemanticError> {
        let mut import_info = ImportInfo {
            functions: Vec::new(),
            structs: Vec::new(),
        };

        // 收集所有函数
        for (name, &func_id) in &target_module.function_map {
            import_info.functions.push((name.clone(), func_id));
        }

        // 收集所有结构体
        for (name, &struct_id) in &target_module.struct_map {
            import_info.structs.push((name.clone(), struct_id));
        }

        Ok(import_info)
    }

    /// 将导入信息应用到单个模块
    fn apply_imports_to_module(
        module: &mut Module,
        import_info: ImportInfo,
        range: TextRange,
    ) -> Result<(), SemanticError> {
        // 导入函数
        for (name, func_id) in import_info.functions {
            if module.function_map.contains_key(&name) {
                return Err(SemanticError::ImportSymbolConflict {
                    symbol: name,
                    range,
                });
            }
            module.function_map.insert(name, func_id);
        }

        // 导入结构体
        for (name, struct_id) in import_info.structs {
            if module.struct_map.contains_key(&name) {
                return Err(SemanticError::ImportSymbolConflict {
                    symbol: name,
                    range,
                });
            }
            module.struct_map.insert(name, struct_id);
        }

        Ok(())
    }
}
