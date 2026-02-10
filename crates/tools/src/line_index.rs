#[derive(Debug)]
pub struct LineIndex {
    spilit_points: Vec<u32>, // 开区间
}

impl LineIndex {
    pub fn new(spilit_points: Vec<u32>) -> Self {
        Self { spilit_points }
    }

    /// 从文本创建 LineIndex，扫描所有换行符位置
    pub fn from_text(text: &str) -> Self {
        let spilit_points: Vec<u32> = text
            .char_indices()
            .filter_map(|(idx, ch)| {
                if ch == '\n' {
                    Some((idx + 1) as u32) // +1 因为是开区间
                } else {
                    None
                }
            })
            .collect();
        Self { spilit_points }
    }

    pub fn get_row_column(&self, offset: u32) -> (u32, u32) {
        let row_num = self.spilit_points.partition_point(|x| *x <= offset);
        let col_num = if row_num == 0 {
            offset
        } else {
            offset - self.spilit_points[row_num - 1]
        };
        (row_num as u32, col_num)
    }

    pub fn get_offset(&self, row: u32, col: u32) -> u32 {
        if row == 0 {
            col
        } else {
            self.spilit_points[row as usize - 1] + col
        }
    }
}
