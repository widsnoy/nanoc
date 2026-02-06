use analyzer::module::Module;
use tools::LineIndex;
use tower_lsp_server::ls_types::{GotoDefinitionResponse, Location, Position, Uri};

use crate::utils::get_at_position::{get_function_id_at_postition, get_struct_id_at_postition};
use crate::utils::{
    get_at_position::{get_reference_id_at_position, get_variable_id_at_position},
    position_trans::text_range_to_ls_range,
};

pub(crate) fn goto_definition(
    source_uri: Uri,
    pos: Position,
    line_index: &LineIndex,
    module: &Module,
) -> Option<GotoDefinitionResponse> {
    // 先找是不是分析好的引用
    if let Some(ref_id) = get_reference_id_at_position(module, line_index, &pos)
        && let Some(refer) = module.get_reference_by_id(*ref_id)
    {
        let range = match refer.tag {
            analyzer::module::ReferenceTag::VarRead(variable_id) => {
                module.get_varaible_by_id(variable_id).map(|v| v.range)
            }
            analyzer::module::ReferenceTag::FuncCall(function_id) => {
                module.get_function_by_id(function_id).map(|v| v.range)
            }
        };
        return range.map(|range| {
            GotoDefinitionResponse::Scalar(Location::new(
                source_uri,
                text_range_to_ls_range(line_index, range),
            ))
        });
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

    if let Some(func_id) = get_function_id_at_postition(module, line_index, &pos)
        && let Some(f) = module.get_function_by_id(func_id)
    {
        return Some(GotoDefinitionResponse::Scalar(Location::new(
            source_uri,
            text_range_to_ls_range(line_index, f.range),
        )));
    }

    if let Some(struct_id) = get_struct_id_at_postition(module, line_index, &pos)
        && let Some(s) = module.get_struct_by_id(struct_id)
    {
        return Some(GotoDefinitionResponse::Scalar(Location::new(
            source_uri,
            text_range_to_ls_range(line_index, s.range),
        )));
    }
    None
}
