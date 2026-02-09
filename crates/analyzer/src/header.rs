//! 头文件分析器：解析 import 语句，导入符号

use dashmap::DashMap;
use syntax::{AstNode as _, SyntaxNode, ast::CompUnit};
use tools::TextRange;
use vfs::{FileID, Vfs};

use crate::{
    error::SemanticError,
    module::{FunctionID, Module, StructID},
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
    pub file_id: FileID,
    pub imports: Vec<(ImportInfo, TextRange)>,
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
        modules: &DashMap<FileID, Module>,
    ) -> ModuleImports {
        let mut imports = Vec::new();
        let mut errors = Vec::new();
        let root = SyntaxNode::new_root(module.green_tree.clone());

        if let Some(comp_unit) = CompUnit::cast(root) {
            for header in comp_unit.headers() {
                if let Some(path_node) = header.path() {
                    let path_node_range_trimmed = utils::trim_node_text_range(&path_node);
                    match Self::resolve_import_path(&path_node, current_file_id, vfs) {
                        Ok((target_file_id, symbol_name)) => {
                            match Self::collect_import_info(
                                target_file_id,
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
            file_id: current_file_id,
            imports,
            errors,
        }
    }

    /// 将单个模块的导入信息应用到该模块（写入操作）
    pub fn apply_module_imports(module: &mut Module, module_imports: ModuleImports) {
        module.semantic_errors.extend(module_imports.errors);

        for (import_info, range) in module_imports.imports {
            if let Err(e) = Self::apply_imports_to_module(module, import_info, range) {
                module.semantic_errors.push(e);
            }
        }
    }

    /// 解析 import 路径，返回目标文件 ID 和可选的符号名
    fn resolve_import_path(
        path_node: &syntax::ast::Path,
        current_file_id: FileID,
        vfs: &Vfs,
    ) -> Result<(FileID, Option<String>), SemanticError> {
        let path_node_range_trimmed = utils::trim_node_text_range(path_node);
        let path_token =
            path_node
                .string_literal()
                .ok_or_else(|| SemanticError::ImportPathNotFound {
                    path: "<invalid>".to_string(),
                    range: path_node_range_trimmed,
                })?;

        let path_with_quotes = path_token.text();
        let path_text = path_with_quotes.trim_matches('"');

        let symbol_name = path_node.symbol().map(|s| s.text().to_string());

        let current_file = vfs.get_file_by_file_id(&current_file_id).ok_or_else(|| {
            SemanticError::ImportPathNotFound {
                path: path_text.to_string(),
                range: path_node_range_trimmed,
            }
        })?;

        let current_dir = current_file
            .path
            .parent()
            .unwrap_or(std::path::Path::new(""));

        let target_path = if path_text.ends_with(".airy") {
            current_dir.join(path_text)
        } else {
            current_dir.join(format!("{}.airy", path_text))
        };

        let target_file_id = vfs.get_file_id_by_path(&target_path).ok_or_else(|| {
            SemanticError::ImportPathNotFound {
                path: format!("{} (resolved to {:?})", path_text, target_path),
                range: path_node_range_trimmed,
            }
        })?;

        Ok((*target_file_id, symbol_name))
    }

    /// 从目标模块收集需要导入的符号信息
    fn collect_import_info(
        target_file_id: FileID,
        symbol_name: Option<&str>,
        range: TextRange,
        modules: &DashMap<FileID, Module>,
    ) -> Result<ImportInfo, SemanticError> {
        let target_module =
            modules
                .get(&target_file_id)
                .ok_or_else(|| SemanticError::ImportPathNotFound {
                    path: format!("{:?}", target_file_id),
                    range,
                })?;

        if let Some(symbol) = symbol_name {
            Self::collect_specific_symbol(&target_module, symbol, range)
        } else {
            Self::collect_all_symbols(&target_module)
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

        if let Some(&func_id) = target_module.function_map.get(symbol_name) {
            import_info
                .functions
                .push((symbol_name.to_string(), func_id));
            return Ok(import_info);
        }

        if let Some(&struct_id) = target_module.struct_map.get(symbol_name) {
            import_info
                .structs
                .push((symbol_name.to_string(), struct_id));
            return Ok(import_info);
        }

        Err(SemanticError::ImportSymbolNotFound {
            symbol: symbol_name.to_string(),
            module_path: format!("{:?}", target_module.file_id),
            range,
        })
    }

    /// 收集所有符号
    fn collect_all_symbols(target_module: &Module) -> Result<ImportInfo, SemanticError> {
        let mut import_info = ImportInfo {
            functions: Vec::new(),
            structs: Vec::new(),
        };

        for (name, &func_id) in &target_module.function_map {
            import_info.functions.push((name.clone(), func_id));
        }

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
        for (name, func_id) in import_info.functions {
            if module.function_map.contains_key(&name) {
                return Err(SemanticError::ImportSymbolConflict {
                    symbol: name,
                    range,
                });
            }
            module.function_map.insert(name, func_id);
        }

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
