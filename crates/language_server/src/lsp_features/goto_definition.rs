use analyzer::module::Module;
use rowan::TextSize;
use tools::{LineIndex, TextRange};
use tower_lsp_server::ls_types::{GotoDefinitionResponse, Location, Position, Uri};

use crate::utils::position_trans::{ls_position_to_offset, text_range_to_ls_range};

pub(crate) fn goto_definition(
    source_uri: Uri,
    pos: Position,
    line_index: &LineIndex,
    module: &Module,
) -> Option<GotoDefinitionResponse> {
    let offset = ls_position_to_offset(line_index, &pos);
    let text_size = TextSize::from(offset);

    let it = module
        .reference_map
        .range(..TextRange::new(offset, u32::MAX));

    let target = it
        .rev()
        .take(2)
        .find(|(range, _)| range.contains_inclusive(text_size));

    // 先找是不是分析好的引用
    if let Some((_, ref_id)) = target
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
        tracing::info!("woria: target_range: {range:?}");
        range.map(|range| {
            GotoDefinitionResponse::Scalar(Location::new(
                source_uri,
                text_range_to_ls_range(line_index, range),
            ))
        })
    } else {
        // TODO: 也可以是 struct Name
        None
    }
}
