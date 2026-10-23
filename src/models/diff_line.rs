#[derive(Debug, Clone, PartialEq)]
pub enum LineStatus {
    Equal,
    Added,
    Removed,
    Modified,
    Moved,
    /// 空行無視・行フィルタで比較対象から外れた行（保存で消さないために残す）
    Ignored,
}

impl LineStatus {
    /// 差分としてユーザーに見せる行か（Equal / Ignored は差分ではない）
    pub fn is_change(&self) -> bool {
        !matches!(self, Self::Equal | Self::Ignored)
    }

    pub fn as_i32(&self) -> i32 {
        match self {
            Self::Equal => 0,
            Self::Added => 1,
            Self::Removed => 2,
            Self::Modified => 3,
            Self::Moved => 4,
            Self::Ignored => 5,
        }
    }
}

/// A segment of word-level diff within a modified line.
/// `changed` = true means this segment differs between left and right.
#[derive(Debug, Clone)]
pub struct WordDiffSegment {
    pub text: String,
    pub changed: bool,
}

#[derive(Debug, Clone)]
pub struct DiffLine {
    pub left_line_no: Option<u32>,
    pub right_line_no: Option<u32>,
    pub left_text: String,
    pub right_text: String,
    pub status: LineStatus,
    /// Word-level diff segments for left side (only for Modified lines)
    pub left_word_segments: Vec<WordDiffSegment>,
    /// Word-level diff segments for right side (only for Modified lines)
    pub right_word_segments: Vec<WordDiffSegment>,
}

#[derive(Debug, Clone)]
pub struct DiffResult {
    pub lines: Vec<DiffLine>,
    pub diff_count: u32,
    pub diff_positions: Vec<usize>,
}
