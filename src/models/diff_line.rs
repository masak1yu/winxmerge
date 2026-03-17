#[derive(Debug, Clone, PartialEq)]
pub enum LineStatus {
    Equal,
    Added,
    Removed,
    Modified,
}

#[derive(Debug, Clone)]
pub struct DiffLine {
    pub left_line_no: Option<u32>,
    pub right_line_no: Option<u32>,
    pub left_text: String,
    pub right_text: String,
    pub status: LineStatus,
}

#[derive(Debug, Clone)]
pub struct DiffResult {
    pub lines: Vec<DiffLine>,
    pub diff_count: u32,
    pub diff_positions: Vec<usize>,
}
