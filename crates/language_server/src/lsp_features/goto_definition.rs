use analyzer::{module::Module, project::Project};
use tools::LineIndex;
use tower_lsp_server::ls_types::{GotoDefinitionResponse, Location, Position, Uri};
use vfs::FileID;

use crate::utils::get_at_position::{get_function_id_at_position, get_struct_id_at_position};
use crate::utils::{
    get_at_position::{get_reference_id_at_position, get_variable_id_at_position},
    position_trans::text_range_to_ls_range,
};

pub(crate) fn goto_definition<F>(
    source_uri: Uri,
    pos: Position,
    line_index: &LineIndex,
    module: &Module,
    project: &Project,
    get_uri_by_file_id: F,
) -> Option<GotoDefinitionResponse>
where
    F: Fn(FileID) -> Option<Uri>,
{
    // 先找是不是分析好的引用
    if let Some(ref_id) = get_reference_id_at_position(module, line_index, &pos)
        && let Some(refer) = module.get_reference_by_id(*ref_id)
    {
        let (range, target_file_id) = match refer.tag {
            analyzer::module::ReferenceTag::VarRead(variable_id) => {
                // 变量是局部的，使用当前文件
                (
                    module.get_varaible_by_id(variable_id).map(|v| v.range),
                    module.file_id,
                )
            }
            analyzer::module::ReferenceTag::FuncCall(function_id) => {
                // 函数可能在其他文件，使用 function_id.module
                (
                    module.get_function_by_id(function_id).map(|v| v.range),
                    function_id.module,
                )
            }
            analyzer::module::ReferenceTag::FieldRead(field_id) => {
                // 字段可能在其他文件，使用 field_id.module
                (
                    module.get_field_by_id(field_id).map(|f| f.range),
                    field_id.module,
                )
            }
        };

        tracing::info!("aaa: target_uri");

        if let Some(range) = range {
            // 获取目标文件的 URI 和 LineIndex
            let target_uri = if target_file_id == module.file_id {
                source_uri
            } else {
                get_uri_by_file_id(target_file_id)?
            };

            let target_line_index = project.line_indexes.get(&target_file_id)?;

            tracing::info!(
                "aaabb: {:?}, {:?}",
                &target_uri,
                text_range_to_ls_range(target_line_index, range)
            );

            return Some(GotoDefinitionResponse::Scalar(Location::new(
                target_uri,
                text_range_to_ls_range(target_line_index, range),
            )));
        }
    }

    if let Some(var_id) = get_variable_id_at_position(module, line_index, &pos)
        && let Some(variable) = module.get_varaible_by_id(*var_id)
    {
        let range = variable.range;
        return Some(GotoDefinitionResponse::Scalar(Location::new(
            source_uri,
            text_range_to_ls_range(line_index, range),
        )));
    }

    if let Some(func_id) = get_function_id_at_position(module, line_index, &pos)
        && let Some(f) = module.get_function_by_id(func_id)
    {
        // 获取函数定义所在文件的 URI 和 LineIndex
        let target_uri = if func_id.module == module.file_id {
            source_uri
        } else {
            get_uri_by_file_id(func_id.module)?
        };

        let target_line_index = project.line_indexes.get(&func_id.module)?;

        return Some(GotoDefinitionResponse::Scalar(Location::new(
            target_uri,
            text_range_to_ls_range(target_line_index, f.range),
        )));
    }

    if let Some(struct_id) = get_struct_id_at_position(module, line_index, &pos)
        && let Some(s) = module.get_struct_by_id(struct_id)
    {
        // 获取结构体定义所在文件的 URI 和 LineIndex
        let target_uri = if struct_id.module == module.file_id {
            source_uri
        } else {
            get_uri_by_file_id(struct_id.module)?
        };

        let target_line_index = project.line_indexes.get(&struct_id.module)?;

        return Some(GotoDefinitionResponse::Scalar(Location::new(
            target_uri,
            text_range_to_ls_range(target_line_index, s.range),
        )));
    }
    None
}
