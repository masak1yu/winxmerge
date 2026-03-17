use std::time::Duration;

use similar::{Algorithm, ChangeTag, TextDiff};

use crate::models::diff_line::{DiffLine, DiffResult, LineStatus};

/// Maximum number of lines before switching to a faster algorithm
const LARGE_FILE_THRESHOLD: usize = 10_000;
/// Timeout for diff computation
const DIFF_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Default)]
pub struct DiffOptions {
    pub ignore_whitespace: bool,
    pub ignore_case: bool,
}

pub fn compute_diff_with_options(
    left_text: &str,
    right_text: &str,
    options: &DiffOptions,
) -> DiffResult {
    let left_normalized = normalize_text(left_text, options);
    let right_normalized = normalize_text(right_text, options);

    // Use original lines for display, normalized for comparison
    let left_orig_lines: Vec<&str> = left_text.lines().collect();
    let right_orig_lines: Vec<&str> = right_text.lines().collect();

    // Use faster algorithm for large files
    let line_count = left_orig_lines.len().max(right_orig_lines.len());
    let algorithm = if line_count > LARGE_FILE_THRESHOLD {
        Algorithm::Patience
    } else {
        Algorithm::Myers
    };

    let diff = TextDiff::configure()
        .algorithm(algorithm)
        .timeout(DIFF_TIMEOUT)
        .diff_lines(&left_normalized, &right_normalized);
    let mut lines = Vec::new();
    let mut diff_positions = Vec::new();
    let mut left_line_no: u32 = 0;
    let mut right_line_no: u32 = 0;

    let changes: Vec<_> = diff.iter_all_changes().collect();
    let mut i = 0;

    while i < changes.len() {
        let change = &changes[i];
        match change.tag() {
            ChangeTag::Equal => {
                let left_display = left_orig_lines
                    .get(left_line_no as usize)
                    .unwrap_or(&"")
                    .to_string();
                let right_display = right_orig_lines
                    .get(right_line_no as usize)
                    .unwrap_or(&"")
                    .to_string();
                left_line_no += 1;
                right_line_no += 1;
                lines.push(DiffLine {
                    left_line_no: Some(left_line_no),
                    right_line_no: Some(right_line_no),
                    left_text: left_display,
                    right_text: right_display,
                    status: LineStatus::Equal,
                });
                i += 1;
            }
            ChangeTag::Delete => {
                if i + 1 < changes.len() && changes[i + 1].tag() == ChangeTag::Insert {
                    let left_display = left_orig_lines
                        .get(left_line_no as usize)
                        .unwrap_or(&"")
                        .to_string();
                    let right_display = right_orig_lines
                        .get(right_line_no as usize)
                        .unwrap_or(&"")
                        .to_string();
                    left_line_no += 1;
                    right_line_no += 1;
                    lines.push(DiffLine {
                        left_line_no: Some(left_line_no),
                        right_line_no: Some(right_line_no),
                        left_text: left_display,
                        right_text: right_display,
                        status: LineStatus::Modified,
                    });
                    i += 2;
                } else {
                    let left_display = left_orig_lines
                        .get(left_line_no as usize)
                        .unwrap_or(&"")
                        .to_string();
                    left_line_no += 1;
                    lines.push(DiffLine {
                        left_line_no: Some(left_line_no),
                        right_line_no: None,
                        left_text: left_display,
                        right_text: String::new(),
                        status: LineStatus::Removed,
                    });
                    i += 1;
                }
            }
            ChangeTag::Insert => {
                let right_display = right_orig_lines
                    .get(right_line_no as usize)
                    .unwrap_or(&"")
                    .to_string();
                right_line_no += 1;
                lines.push(DiffLine {
                    left_line_no: None,
                    right_line_no: Some(right_line_no),
                    left_text: String::new(),
                    right_text: right_display,
                    status: LineStatus::Added,
                });
                i += 1;
            }
        }
    }

    // Detect moved lines: a Removed line whose text appears as an Added line elsewhere
    detect_moved_lines(&mut lines);

    // Rebuild diff_positions and diff_count after move detection
    diff_positions.clear();
    let mut diff_count: u32 = 0;
    for (idx, line) in lines.iter().enumerate() {
        if line.status != LineStatus::Equal {
            diff_positions.push(idx);
            diff_count += 1;
        }
    }

    DiffResult {
        lines,
        diff_count,
        diff_positions,
    }
}

fn detect_moved_lines(lines: &mut [DiffLine]) {
    use std::collections::HashMap;

    // Collect all Added lines' text -> indices
    let mut added_texts: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, line) in lines.iter().enumerate() {
        if line.status == LineStatus::Added {
            let text = line.right_text.trim().to_string();
            if !text.is_empty() {
                added_texts.entry(text).or_default().push(i);
            }
        }
    }

    // Check each Removed line against Added lines
    for i in 0..lines.len() {
        if lines[i].status != LineStatus::Removed {
            continue;
        }
        let text = lines[i].left_text.trim().to_string();
        if text.is_empty() {
            continue;
        }
        if let Some(added_indices) = added_texts.get_mut(&text) {
            if let Some(added_idx) = added_indices.pop() {
                lines[i].status = LineStatus::Moved;
                lines[added_idx].status = LineStatus::Moved;
                // Copy text across for display
                lines[i].right_text = lines[added_idx].right_text.clone();
                lines[added_idx].left_text = lines[i].left_text.clone();
            }
        }
    }
}

fn normalize_text(text: &str, options: &DiffOptions) -> String {
    let mut result = String::with_capacity(text.len());
    for line in text.lines() {
        let mut l = line.to_string();
        if options.ignore_whitespace {
            // Collapse all whitespace to single space and trim
            l = l.split_whitespace().collect::<Vec<&str>>().join(" ");
        }
        if options.ignore_case {
            l = l.to_lowercase();
        }
        result.push_str(&l);
        result.push('\n');
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_equal_files() {
        let result = compute_diff_with_options("hello\nworld\n", "hello\nworld\n", &DiffOptions::default());
        assert_eq!(result.diff_count, 0);
        assert_eq!(result.lines.len(), 2);
    }

    #[test]
    fn test_added_line() {
        let result = compute_diff_with_options("hello\n", "hello\nworld\n", &DiffOptions::default());
        assert_eq!(result.diff_count, 1);
        assert_eq!(result.lines[1].status, LineStatus::Added);
    }

    #[test]
    fn test_removed_line() {
        let result = compute_diff_with_options("hello\nworld\n", "hello\n", &DiffOptions::default());
        assert_eq!(result.diff_count, 1);
        assert_eq!(result.lines[1].status, LineStatus::Removed);
    }

    #[test]
    fn test_modified_line() {
        let result = compute_diff_with_options("hello\n", "hallo\n", &DiffOptions::default());
        assert_eq!(result.diff_count, 1);
        assert_eq!(result.lines[0].status, LineStatus::Modified);
    }

    #[test]
    fn test_ignore_whitespace() {
        let opts = DiffOptions {
            ignore_whitespace: true,
            ignore_case: false,
        };
        let result =
            compute_diff_with_options("hello   world\n", "hello world\n", &opts);
        assert_eq!(result.diff_count, 0);
    }

    #[test]
    fn test_ignore_case() {
        let opts = DiffOptions {
            ignore_whitespace: false,
            ignore_case: true,
        };
        let result = compute_diff_with_options("Hello\n", "hello\n", &opts);
        assert_eq!(result.diff_count, 0);
    }

    #[test]
    fn test_moved_line() {
        let left = "aaa\nbbb\nccc\n";
        let right = "ccc\nbbb\naaa\n";
        let result = compute_diff_with_options(left, right, &DiffOptions::default());
        // aaa and ccc are moved (swapped positions), bbb stays
        let moved_count = result.lines.iter().filter(|l| l.status == LineStatus::Moved).count();
        assert!(moved_count > 0, "Should detect moved lines");
    }

    #[test]
    fn test_ignore_both() {
        let opts = DiffOptions {
            ignore_whitespace: true,
            ignore_case: true,
        };
        let result =
            compute_diff_with_options("Hello   World\n", "hello world\n", &opts);
        assert_eq!(result.diff_count, 0);
    }
}
