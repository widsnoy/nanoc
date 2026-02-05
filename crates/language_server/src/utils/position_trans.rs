//! position <-> line_position

use tools::LineIndex;
use tower_lsp_server::ls_types::Position;

#[allow(dead_code)]
pub(crate) fn ls_position_to_offset(line_index: &LineIndex, pos: &Position) -> u32 {
    line_index.get_offset(pos.line as usize, pos.character as usize) as u32
}
