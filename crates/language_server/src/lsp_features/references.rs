use analyzer::{module::Module, project::Project};
use tools::LineIndex;
use tower_lsp_server::ls_types::{Location, Position, Uri};
use vfs::FileID;

use crate::utils::{
    get_at_position::{get_function_id_at_position, get_variable_id_at_position},
    position_trans::text_range_to_ls_range,
};

pub(crate) fn get_references<F>(
    source_uri: Uri,
    pos: Position,
    line_index: &LineIndex,
    module: &Module,
    project: &Project,
    get_uri_by_file_id: F,
) -> Option<Vec<Location>>
where
    F: Fn(FileID) -> Option<Uri>,
{
    if let Some(var_id) = get_variable_id_at_position(module, line_index, &pos) {
        let refer_list = module.index.variable_reference.get(var_id)?;
        return Some(
            refer_list
                .iter()
                .filter_map(|citer_info| {
                    // 获取引用所在文件的 URI
                    let target_uri = if citer_info.file_id == module.file_id {
                        source_uri.clone()
                    } else {
                        get_uri_by_file_id(citer_info.file_id)?
                    };

                    // 获取引用所在文件的 LineIndex
                    let target_line_index = project.line_indexes.get(&citer_info.file_id)?;

                    Some(Location::new(
                        target_uri,
                        text_range_to_ls_range(target_line_index, citer_info.range),
                    ))
                })
                .collect::<Vec<_>>(),
        );
    }

    if let Some(func_id) = get_function_id_at_position(module, line_index, &pos) {
        let refer_list = module.index.function_reference.get(&func_id)?;
        return Some(
            refer_list
                .iter()
                .filter_map(|citer_info| {
                    // 获取引用所在文件的 URI
                    let target_uri = if citer_info.file_id == module.file_id {
                        source_uri.clone()
                    } else {
                        get_uri_by_file_id(citer_info.file_id)?
                    };

                    // 获取引用所在文件的 LineIndex
                    let target_line_index = project.line_indexes.get(&citer_info.file_id)?;

                    Some(Location::new(
                        target_uri,
                        text_range_to_ls_range(target_line_index, citer_info.range),
                    ))
                })
                .collect::<Vec<_>>(),
        );
    }

    None
}
