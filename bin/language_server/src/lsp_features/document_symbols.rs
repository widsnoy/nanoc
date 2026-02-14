use analyzer::module::Module;
use tools::LineIndex;
use tower_lsp_server::ls_types::{DocumentSymbol, DocumentSymbolResponse, SymbolKind};

use crate::utils::position_trans::text_range_to_ls_range;

/// 计算文档符号
pub(crate) fn compute_document_symbols(
    module: &Module,
    line_index: &LineIndex,
) -> Option<DocumentSymbolResponse> {
    let mut symbols = Vec::new();

    // 1. 收集全局变量
    if let Some(global_scope) = module.scopes.get(*module.global_scope) {
        for var_id in global_scope.variables.values() {
            if let Some(variable) = module.variables.get(**var_id) {
                let kind = if variable.is_const() {
                    SymbolKind::CONSTANT
                } else {
                    SymbolKind::VARIABLE
                };

                let range = text_range_to_ls_range(line_index, variable.range);

                symbols.push(DocumentSymbol {
                    name: variable.name.clone(),
                    detail: Some(variable.ty.to_string()),
                    kind,
                    tags: None,
                    #[allow(deprecated)]
                    deprecated: None,
                    range,
                    selection_range: range,
                    children: None,
                });
            }
        }
    }

    // 2. 收集函数
    for (idx, function) in module.functions.iter() {
        let function_id = analyzer::module::FunctionID::new(module.file_id, idx);

        // 只收集本地定义的函数
        if function_id.module == module.file_id {
            let range = text_range_to_ls_range(line_index, function.range);

            // 构建函数签名作为 detail
            let params = function
                .meta_types
                .iter()
                .map(|(name, ty)| format!("{}: {}", name, ty))
                .collect::<Vec<_>>()
                .join(", ");
            let detail = format!("fn {}({}) -> {}", function.name, params, function.ret_type);

            symbols.push(DocumentSymbol {
                name: function.name.clone(),
                detail: Some(detail),
                kind: SymbolKind::FUNCTION,
                tags: None,
                #[allow(deprecated)]
                deprecated: None,
                range,
                selection_range: range,
                children: None,
            });
        }
    }

    // 3. 收集结构体（包含字段作为子符号）
    for (idx, struct_def) in module.structs.iter() {
        let struct_id = analyzer::module::StructID::new(module.file_id, idx);

        // 只收集本地定义的结构体
        if struct_id.module == module.file_id {
            let range = text_range_to_ls_range(line_index, struct_def.range);

            // 收集字段作为子符号
            let mut children = Vec::new();
            for field_id in &struct_def.fields {
                if let Some(field) = module.fields.get(field_id.index) {
                    let field_range = text_range_to_ls_range(line_index, field.range);
                    children.push(DocumentSymbol {
                        name: field.name.clone(),
                        detail: Some(field.ty.to_string()),
                        kind: SymbolKind::FIELD,
                        tags: None,
                        #[allow(deprecated)]
                        deprecated: None,
                        range: field_range,
                        selection_range: field_range,
                        children: None,
                    });
                }
            }

            symbols.push(DocumentSymbol {
                name: struct_def.name.clone(),
                detail: None,
                kind: SymbolKind::STRUCT,
                tags: None,
                #[allow(deprecated)]
                deprecated: None,
                range,
                selection_range: range,
                children: if children.is_empty() {
                    None
                } else {
                    Some(children)
                },
            });
        }
    }

    Some(DocumentSymbolResponse::Nested(symbols))
}
