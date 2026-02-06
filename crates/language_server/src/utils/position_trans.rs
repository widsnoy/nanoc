//! position <-> line_position

use tools::{LineIndex, TextRange};
use tower_lsp_server::ls_types::{Position, Range};

pub(crate) fn ls_position_to_offset(line_index: &LineIndex, pos: &Position) -> u32 {
    line_index.get_offset(pos.line, pos.character)
}

pub(crate) fn ls_position_to_range(line_index: &LineIndex, pos: &Position) -> TextRange {
    let p = ls_position_to_offset(line_index, pos);
    TextRange::new(p, p + 1)
}

pub(crate) fn offset_to_ls_position(line_index: &LineIndex, offset: u32) -> Position {
    let (r, c) = line_index.get_row_column(offset);
    Position::new(r, c)
}

pub(crate) fn text_range_to_ls_range(line_index: &LineIndex, text_range: TextRange) -> Range {
    Range::new(
        offset_to_ls_position(line_index, text_range.start().into()),
        offset_to_ls_position(line_index, text_range.end().into()),
    )
}
