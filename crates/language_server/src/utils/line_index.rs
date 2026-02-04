//! position <-> line_position
#[derive(Debug)]
pub struct LineIndex {
    spilit_points: Vec<usize>, // 开区间
}

impl LineIndex {
    pub fn new(newline_token_end_position: Vec<usize>) -> Self {
        Self {
            spilit_points: newline_token_end_position,
        }
    }

    pub fn get_row_column(&self, offset: usize) -> (usize, usize) {
        let row_num = self.spilit_points.partition_point(|x| *x <= offset);
        let col_num = if row_num == 0 {
            offset
        } else {
            offset - self.spilit_points[row_num - 1]
        };
        (row_num, col_num)
    }

    pub fn get_offset(&self, row: usize, col: usize) -> usize {
        if row == 0 {
            col
        } else {
            self.spilit_points[row - 1] + col
        }
    }
}
