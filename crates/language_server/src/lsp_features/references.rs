use analyzer::module::Module;
use tools::LineIndex;
use tower_lsp_server::ls_types::{Location, Position, Uri};

use crate::utils::{
    get_at_position::{get_function_id_at_postition, get_variable_id_at_position},
    position_trans::text_range_to_ls_range,
};

pub(crate) fn get_references(
    source_uri: Uri,
    pos: Position,
    line_index: &LineIndex,
    module: &Module,
) -> Option<Vec<Location>> {
    if let Some(var_id) = get_variable_id_at_position(module, line_index, &pos) {
        let refer_list = module.index.variable_reference.get(var_id)?;
        return Some(
            refer_list
                .iter()
                .flat_map(|id| module.get_reference_by_id(*id))
                .map(|r| {
                    Location::new(
                        source_uri.clone(),
                        text_range_to_ls_range(line_index, r.range),
                    )
                })
                .collect::<Vec<_>>(),
        );
    }

    if let Some(func_id) = get_function_id_at_postition(module, line_index, &pos) {
        let refer_list = module.index.function_reference.get(&func_id)?;
        return Some(
            refer_list
                .iter()
                .flat_map(|id| module.get_reference_by_id(*id))
                .map(|r| {
                    Location::new(
                        source_uri.clone(),
                        text_range_to_ls_range(line_index, r.range),
                    )
                })
                .collect::<Vec<_>>(),
        );
    }

    None
}

