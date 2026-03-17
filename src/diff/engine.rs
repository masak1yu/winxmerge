use similar::{ChangeTag, TextDiff};

use crate::models::diff_line::{DiffLine, DiffResult, LineStatus};

pub fn compute_diff(left_text: &str, right_text: &str) -> DiffResult {
    let diff = TextDiff::from_lines(left_text, right_text);
    let mut lines = Vec::new();
    let mut diff_positions = Vec::new();
    let mut diff_count: u32 = 0;
    let mut left_line_no: u32 = 0;
    let mut right_line_no: u32 = 0;

    let changes: Vec<_> = diff.iter_all_changes().collect();
    let mut i = 0;

    while i < changes.len() {
        let change = &changes[i];
        match change.tag() {
            ChangeTag::Equal => {
                left_line_no += 1;
                right_line_no += 1;
                let text = change.value().trim_end_matches('\n').to_string();
                lines.push(DiffLine {
                    left_line_no: Some(left_line_no),
                    right_line_no: Some(right_line_no),
                    left_text: text.clone(),
                    right_text: text,
                    status: LineStatus::Equal,
                });
                i += 1;
            }
            ChangeTag::Delete => {
                // Look ahead: if next change is Insert, treat as Modified pair
                if i + 1 < changes.len() && changes[i + 1].tag() == ChangeTag::Insert {
                    left_line_no += 1;
                    right_line_no += 1;
                    let left = change.value().trim_end_matches('\n').to_string();
                    let right = changes[i + 1].value().trim_end_matches('\n').to_string();
                    diff_positions.push(lines.len());
                    diff_count += 1;
                    lines.push(DiffLine {
                        left_line_no: Some(left_line_no),
                        right_line_no: Some(right_line_no),
                        left_text: left,
                        right_text: right,
                        status: LineStatus::Modified,
                    });
                    i += 2;
                } else {
                    left_line_no += 1;
                    let text = change.value().trim_end_matches('\n').to_string();
                    diff_positions.push(lines.len());
                    diff_count += 1;
                    lines.push(DiffLine {
                        left_line_no: Some(left_line_no),
                        right_line_no: None,
                        left_text: text,
                        right_text: String::new(),
                        status: LineStatus::Removed,
                    });
                    i += 1;
                }
            }
            ChangeTag::Insert => {
                right_line_no += 1;
                let text = change.value().trim_end_matches('\n').to_string();
                diff_positions.push(lines.len());
                diff_count += 1;
                lines.push(DiffLine {
                    left_line_no: None,
                    right_line_no: Some(right_line_no),
                    left_text: String::new(),
                    right_text: text,
                    status: LineStatus::Added,
                });
                i += 1;
            }
        }
    }

    DiffResult {
        lines,
        diff_count,
        diff_positions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_equal_files() {
        let result = compute_diff("hello\nworld\n", "hello\nworld\n");
        assert_eq!(result.diff_count, 0);
        assert_eq!(result.lines.len(), 2);
    }

    #[test]
    fn test_added_line() {
        let result = compute_diff("hello\n", "hello\nworld\n");
        assert_eq!(result.diff_count, 1);
        assert_eq!(result.lines[1].status, LineStatus::Added);
    }

    #[test]
    fn test_removed_line() {
        let result = compute_diff("hello\nworld\n", "hello\n");
        assert_eq!(result.diff_count, 1);
        assert_eq!(result.lines[1].status, LineStatus::Removed);
    }

    #[test]
    fn test_modified_line() {
        let result = compute_diff("hello\n", "hallo\n");
        assert_eq!(result.diff_count, 1);
        assert_eq!(result.lines[0].status, LineStatus::Modified);
    }
}
