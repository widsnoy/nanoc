use analyzer::{module::Module, project::Project};
use tower_lsp_server::ls_types::{Location, Position, Uri};
use vfs::{FileID, Vfs};

use crate::utils::{
    get_at_position::{get_function_id_at_position, get_variable_id_at_position},
    position_trans::text_range_to_ls_range,
};

pub(crate) fn get_references<F>(
    source_uri: Uri,
    pos: Position,
    module: &Module,
    _project: &Project,
    vfs: &Vfs,
    get_uri_by_file_id: F,
) -> Option<Vec<Location>>
where
    F: Fn(FileID) -> Option<Uri>,
{
    // 获取当前文件的 line_index
    let line_index = &vfs.get_file_by_file_id(&module.file_id)?.line_index;
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
                    let target_line_index =
                        &vfs.get_file_by_file_id(&citer_info.file_id)?.line_index;

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
                    let target_line_index =
                        &vfs.get_file_by_file_id(&citer_info.file_id)?.line_index;

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
