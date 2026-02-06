use analyzer::module::{FunctionID, Module, ReferenceID, StructID, VariableID};
use rowan::TextSize;
use syntax::{
    AstNode, SyntaxNode,
    ast::{FuncSign, PrimitType},
};
use tools::{LineIndex, TextRange};
use tower_lsp_server::ls_types::Position;

use crate::utils::position_trans::{ls_position_to_offset, ls_position_to_range};

pub fn get_reference_id_at_position<'a>(
    module: &'a Module,
    line_index: &LineIndex,
    pos: &Position,
) -> Option<&'a ReferenceID> {
    let offset = ls_position_to_offset(line_index, pos);
    let text_size = TextSize::from(offset);

    let it = module
        .reference_map
        .range(..TextRange::new(offset, u32::MAX));

    it.rev()
        .take(2)
        .find(|(range, _)| range.contains_inclusive(text_size))
        .map(|x| x.1)
}

pub fn get_variable_id_at_position<'a>(
    module: &'a Module,
    line_index: &LineIndex,
    pos: &Position,
) -> Option<&'a VariableID> {
    let offset = ls_position_to_offset(line_index, pos);
    let text_size = TextSize::from(offset);

    let it = module
        .variable_map
        .range(..TextRange::new(offset, u32::MAX));

    it.rev()
        .take(2)
        .find(|(range, _)| range.contains_inclusive(text_size))
        .map(|x| x.1)
}

pub fn get_function_id_at_postition(
    module: &Module,
    line_index: &LineIndex,
    pos: &Position,
) -> Option<FunctionID> {
    let root = SyntaxNode::new_root(module.green_tree.clone());
    let range = ls_position_to_range(line_index, pos);
    let token = root.covering_element(*range);

    if let Some(node) = token.parent().and_then(|n| n.parent())
        && let Some(func_signature) = FuncSign::cast(node)
        && let Some(func_name) = func_signature.name().and_then(|n| n.var_name())
    {
        module.get_function_id_by_name(&func_name)
    } else {
        None
    }
}

pub fn get_struct_id_at_postition(
    module: &Module,
    line_index: &LineIndex,
    pos: &Position,
) -> Option<StructID> {
    let root = SyntaxNode::new_root(module.green_tree.clone());
    let range = ls_position_to_range(line_index, pos);
    let token = root.covering_element(*range);
    if let Some(node) = token.parent().and_then(|x| x.parent())
        && let Some(primitive_type_node) = PrimitType::cast(node)
        && primitive_type_node.struct_token().is_some()
        && let Some(name) = primitive_type_node.name().and_then(|n| n.var_name())
    {
        module.get_struct_id_by_name(&name)
    } else {
        None
    }
}
