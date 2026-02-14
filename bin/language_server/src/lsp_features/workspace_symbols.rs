use analyzer::project::Project;
use tower_lsp_server::ls_types::{
    Location, SymbolInformation, SymbolKind, Uri, WorkspaceSymbolResponse,
};
use vfs::{FileID, Vfs};

use crate::utils::position_trans::text_range_to_ls_range;

/// 在工作区中搜索符号
pub(crate) fn search_workspace_symbols<F>(
    query: &str,
    project: &Project,
    vfs: &Vfs,
    get_uri_by_file_id: F,
) -> Option<WorkspaceSymbolResponse>
where
    F: Fn(FileID) -> Option<Uri>,
{
    let mut symbols = Vec::new();
    let query_lower = query.to_lowercase();

    // 遍历所有模块
    for (file_id, module) in &project.modules {
        // 获取当前文件的 URI 和 LineIndex
        let uri = match get_uri_by_file_id(*file_id) {
            Some(u) => u,
            None => continue,
        };

        let file = match vfs.get_file_by_file_id(file_id) {
            Some(f) => f,
            None => continue,
        };

        let line_index = &file.line_index;

        // 1. 收集全局变量
        if let Some(global_scope) = module.scopes.get(*module.global_scope) {
            for var_id in global_scope.variables.values() {
                if let Some(variable) = module.variables.get(**var_id) {
                    // 模糊匹配
                    if !query.is_empty() && !variable.name.to_lowercase().contains(&query_lower) {
                        continue;
                    }

                    let kind = if variable.is_const() {
                        SymbolKind::CONSTANT
                    } else {
                        SymbolKind::VARIABLE
                    };

                    let range = text_range_to_ls_range(line_index, variable.range);

                    symbols.push(SymbolInformation {
                        name: variable.name.clone(),
                        kind,
                        tags: None,
                        #[allow(deprecated)]
                        deprecated: None,
                        location: Location::new(uri.clone(), range),
                        container_name: None,
                    });
                }
            }
        }

        // 2. 收集函数
        for (idx, function) in module.functions.iter() {
            let function_id = analyzer::module::FunctionID::new(*file_id, idx);

            // 只收集本地定义的函数
            if function_id.module == *file_id {
                // 模糊匹配
                if !query.is_empty() && !function.name.to_lowercase().contains(&query_lower) {
                    continue;
                }

                let range = text_range_to_ls_range(line_index, function.range);

                symbols.push(SymbolInformation {
                    name: function.name.clone(),
                    kind: SymbolKind::FUNCTION,
                    tags: None,
                    #[allow(deprecated)]
                    deprecated: None,
                    location: Location::new(uri.clone(), range),
                    container_name: None,
                });
            }
        }

        // 3. 收集结构体
        for (idx, struct_def) in module.structs.iter() {
            let struct_id = analyzer::module::StructID::new(*file_id, idx);

            // 只收集本地定义的结构体
            if struct_id.module == *file_id {
                // 模糊匹配
                if !query.is_empty() && !struct_def.name.to_lowercase().contains(&query_lower) {
                    continue;
                }

                let range = text_range_to_ls_range(line_index, struct_def.range);

                symbols.push(SymbolInformation {
                    name: struct_def.name.clone(),
                    kind: SymbolKind::STRUCT,
                    tags: None,
                    #[allow(deprecated)]
                    deprecated: None,
                    location: Location::new(uri.clone(), range),
                    container_name: None,
                });
            }
        }
    }

    Some(WorkspaceSymbolResponse::Flat(symbols))
}
