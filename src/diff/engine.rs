use std::time::Duration;

use regex::Regex;
use similar::{Algorithm, ChangeTag, TextDiff};

use crate::models::diff_line::{DiffLine, DiffResult, LineStatus, WordDiffSegment};

/// Maximum number of lines before switching to a faster algorithm
const LARGE_FILE_THRESHOLD: usize = 10_000;
/// Timeout for diff computation
const DIFF_TIMEOUT: Duration = Duration::from_secs(5);
/// Skip per-pair word diff above this total line count to avoid O(N²) slowdown
const WORD_DIFF_LINE_LIMIT: usize = 20_000;

#[derive(Debug, Clone)]
pub struct DiffOptions {
    pub ignore_whitespace: bool,
    /// Upgrade `ignore_whitespace` from "ignore whitespace change" to "ignore all
    /// whitespace" (WinMerge's /ignorews:2).  No effect on its own.
    // ponytail: two bools instead of a three-state enum — the toolbar toggle, the
    // settings file and the Slint property all stay plain bools that way.
    pub ignore_whitespace_all: bool,
    pub ignore_case: bool,
    pub ignore_blank_lines: bool,
    pub ignore_eol: bool,
    pub detect_moved_lines: bool,
    /// Line filter: regex patterns to exclude matching lines from comparison
    pub line_filters: Vec<String>,
    /// Substitution filters: (pattern, replacement) pairs applied before comparison
    pub substitution_filters: Vec<(String, String)>,
}

impl Default for DiffOptions {
    fn default() -> Self {
        Self {
            ignore_whitespace: false,
            ignore_whitespace_all: false,
            ignore_case: false,
            ignore_blank_lines: false,
            ignore_eol: false,
            detect_moved_lines: true,
            line_filters: Vec::new(),
            substitution_filters: Vec::new(),
        }
    }
}

pub fn compute_diff_with_options(
    left_text: &str,
    right_text: &str,
    options: &DiffOptions,
) -> DiffResult {
    let (left_normalized, left_map) = normalize_text(left_text, options);
    let (right_normalized, right_map) = normalize_text(right_text, options);

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
        match changes[i].tag() {
            ChangeTag::Equal => {
                let (left_display, left_no) = orig_line(&left_orig_lines, &left_map, left_line_no);
                let (right_display, right_no) =
                    orig_line(&right_orig_lines, &right_map, right_line_no);
                left_line_no += 1;
                right_line_no += 1;
                lines.push(DiffLine {
                    left_line_no: Some(left_no),
                    right_line_no: Some(right_no),
                    left_text: left_display,
                    right_text: right_display,
                    status: LineStatus::Equal,
                    left_word_segments: Vec::new(),
                    right_word_segments: Vec::new(),
                });
                i += 1;
            }
            ChangeTag::Delete | ChangeTag::Insert => {
                // Collect all consecutive non-Equal changes as one diff block
                let mut del_indices: Vec<u32> = Vec::new();
                let mut ins_indices: Vec<u32> = Vec::new();
                while i < changes.len() && changes[i].tag() != ChangeTag::Equal {
                    match changes[i].tag() {
                        ChangeTag::Delete => {
                            del_indices.push(left_line_no);
                            left_line_no += 1;
                        }
                        ChangeTag::Insert => {
                            ins_indices.push(right_line_no);
                            right_line_no += 1;
                        }
                        _ => unreachable!(),
                    }
                    i += 1;
                }

                // Pair up deletions and insertions as Modified lines (word-diff computed per pair)
                let word_diff_enabled = line_count <= WORD_DIFF_LINE_LIMIT;
                let n_pairs = del_indices.len().min(ins_indices.len());
                for j in 0..n_pairs {
                    let (left_display, left_no) =
                        orig_line(&left_orig_lines, &left_map, del_indices[j]);
                    let (right_display, right_no) =
                        orig_line(&right_orig_lines, &right_map, ins_indices[j]);
                    let (left_segs, right_segs) = if word_diff_enabled {
                        compute_word_diff(&left_display, &right_display)
                    } else {
                        (Vec::new(), Vec::new())
                    };
                    lines.push(DiffLine {
                        left_line_no: Some(left_no),
                        right_line_no: Some(right_no),
                        left_text: left_display,
                        right_text: right_display,
                        status: LineStatus::Modified,
                        left_word_segments: left_segs,
                        right_word_segments: right_segs,
                    });
                }
                // Extra deletions → Removed
                for j in n_pairs..del_indices.len() {
                    let (left_display, left_no) =
                        orig_line(&left_orig_lines, &left_map, del_indices[j]);
                    lines.push(DiffLine {
                        left_line_no: Some(left_no),
                        right_line_no: None,
                        left_text: left_display,
                        right_text: String::new(),
                        status: LineStatus::Removed,
                        left_word_segments: Vec::new(),
                        right_word_segments: Vec::new(),
                    });
                }
                // Extra insertions → Added
                for j in n_pairs..ins_indices.len() {
                    let (right_display, right_no) =
                        orig_line(&right_orig_lines, &right_map, ins_indices[j]);
                    lines.push(DiffLine {
                        left_line_no: None,
                        right_line_no: Some(right_no),
                        left_text: String::new(),
                        right_text: right_display,
                        status: LineStatus::Added,
                        left_word_segments: Vec::new(),
                        right_word_segments: Vec::new(),
                    });
                }
            }
        }
    }

    // Detect moved lines (if enabled)
    if options.detect_moved_lines {
        detect_moved_lines(&mut lines);
    }

    // Rebuild diff_positions (one entry per contiguous diff block) and diff_count
    diff_positions.clear();
    let mut diff_count: u32 = 0;
    let mut in_diff_block = false;
    for (idx, line) in lines.iter().enumerate() {
        if line.status != LineStatus::Equal {
            if !in_diff_block {
                diff_positions.push(idx);
                diff_count += 1;
                in_diff_block = true;
            }
        } else {
            in_diff_block = false;
        }
    }

    DiffResult {
        lines,
        diff_count,
        diff_positions,
    }
}

/// Compute word-level diff between two strings, returning segments for left and right.
fn compute_word_diff(left: &str, right: &str) -> (Vec<WordDiffSegment>, Vec<WordDiffSegment>) {
    let diff = TextDiff::configure()
        .algorithm(Algorithm::Myers)
        .diff_chars(left, right);

    let mut left_segs: Vec<WordDiffSegment> = Vec::new();
    let mut right_segs: Vec<WordDiffSegment> = Vec::new();

    for change in diff.iter_all_changes() {
        let text = change.value().to_string();
        match change.tag() {
            ChangeTag::Equal => {
                push_segment(&mut left_segs, &text, false);
                push_segment(&mut right_segs, &text, false);
            }
            ChangeTag::Delete => {
                push_segment(&mut left_segs, &text, true);
            }
            ChangeTag::Insert => {
                push_segment(&mut right_segs, &text, true);
            }
        }
    }

    (left_segs, right_segs)
}

fn push_segment(segs: &mut Vec<WordDiffSegment>, text: &str, changed: bool) {
    if let Some(last) = segs.last_mut() {
        if last.changed == changed {
            last.text.push_str(text);
            return;
        }
    }
    segs.push(WordDiffSegment {
        text: text.to_string(),
        changed,
    });
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

fn compile_line_filters(patterns: &[String]) -> Vec<Regex> {
    patterns
        .iter()
        .filter_map(|p| {
            if p.trim().is_empty() {
                None
            } else {
                Regex::new(p).ok()
            }
        })
        .collect()
}

fn compile_substitution_filters(filters: &[(String, String)]) -> Vec<(Regex, String)> {
    filters
        .iter()
        .filter_map(|(p, r)| {
            if p.trim().is_empty() {
                None
            } else {
                Regex::new(p).ok().map(|re| (re, r.clone()))
            }
        })
        .collect()
}

/// Normalize `text` for comparison.
///
/// Returns the normalized text plus a map from each normalized line index to the
/// 0-based line index it came from in `text`.  `ignore_blank_lines` and the line
/// filters drop lines, so the two numbering schemes diverge and every lookup into
/// the original text has to go through this map — see `orig_line`.
fn normalize_text(text: &str, options: &DiffOptions) -> (String, Vec<usize>) {
    let line_filters = compile_line_filters(&options.line_filters);
    let sub_filters = compile_substitution_filters(&options.substitution_filters);

    let mut result = String::with_capacity(text.len());
    let mut map: Vec<usize> = Vec::new();
    for (orig_idx, line) in text.lines().enumerate() {
        let mut l = if options.ignore_eol {
            line.trim_end_matches(['\r', '\n']).to_string()
        } else {
            line.to_string()
        };
        if options.ignore_blank_lines && l.trim().is_empty() {
            continue;
        }
        // Line filter: skip lines matching any filter pattern
        if line_filters.iter().any(|re| re.is_match(&l)) {
            continue;
        }
        // Substitution filters: apply regex replacements
        for (re, replacement) in &sub_filters {
            l = re.replace_all(&l, replacement.as_str()).to_string();
        }
        if options.ignore_whitespace && options.ignore_whitespace_all {
            l.retain(|c| !c.is_whitespace());
        } else if options.ignore_whitespace {
            l = l.split_whitespace().collect::<Vec<&str>>().join(" ");
        }
        if options.ignore_case {
            l = l.to_lowercase();
        }
        result.push_str(&l);
        result.push('\n');
        map.push(orig_idx);
    }
    (result, map)
}

/// Resolve a normalized line index back to (original text, original 1-based line number).
fn orig_line(orig_lines: &[&str], map: &[usize], norm_idx: u32) -> (String, u32) {
    match map.get(norm_idx as usize) {
        Some(&i) => (orig_lines.get(i).unwrap_or(&"").to_string(), i as u32 + 1),
        None => (String::new(), norm_idx + 1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_equal_files() {
        let result =
            compute_diff_with_options("hello\nworld\n", "hello\nworld\n", &DiffOptions::default());
        assert_eq!(result.diff_count, 0);
        assert_eq!(result.lines.len(), 2);
    }

    #[test]
    fn test_added_line() {
        let result =
            compute_diff_with_options("hello\n", "hello\nworld\n", &DiffOptions::default());
        assert_eq!(result.diff_count, 1);
        assert_eq!(result.lines[1].status, LineStatus::Added);
    }

    #[test]
    fn test_removed_line() {
        let result =
            compute_diff_with_options("hello\nworld\n", "hello\n", &DiffOptions::default());
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
            ..Default::default()
        };
        let result = compute_diff_with_options("hello   world\n", "hello world\n", &opts);
        assert_eq!(result.diff_count, 0);
    }

    #[test]
    fn test_ignore_whitespace_change_only() {
        let opts = DiffOptions {
            ignore_whitespace: true,
            ..Default::default()
        };
        // Leading/trailing whitespace and run-length differences collapse.
        let result = compute_diff_with_options("  a  b\n", "a b\n", &opts);
        assert_eq!(result.diff_count, 0);
        // But removing the inner whitespace entirely is still a difference.
        let result = compute_diff_with_options("a b\n", "ab\n", &opts);
        assert_eq!(result.diff_count, 1);
    }

    #[test]
    fn test_ignore_whitespace_all() {
        let opts = DiffOptions {
            ignore_whitespace: true,
            ignore_whitespace_all: true,
            ..Default::default()
        };
        let result = compute_diff_with_options("a b\n", "ab\n", &opts);
        assert_eq!(result.diff_count, 0);
    }

    #[test]
    fn test_ignore_case() {
        let opts = DiffOptions {
            ignore_case: true,
            ..Default::default()
        };
        let result = compute_diff_with_options("Hello\n", "hello\n", &opts);
        assert_eq!(result.diff_count, 0);
    }

    #[test]
    fn test_moved_line() {
        let left = "aaa\nbbb\nccc\n";
        let right = "ccc\nbbb\naaa\n";
        let result = compute_diff_with_options(left, right, &DiffOptions::default());
        let moved_count = result
            .lines
            .iter()
            .filter(|l| l.status == LineStatus::Moved)
            .count();
        assert!(moved_count > 0, "Should detect moved lines");
    }

    #[test]
    fn test_ignore_blank_lines() {
        let opts = DiffOptions {
            ignore_blank_lines: true,
            ..Default::default()
        };
        let result = compute_diff_with_options("hello\n\nworld\n", "hello\nworld\n", &opts);
        assert_eq!(result.diff_count, 0);
    }

    #[test]
    fn test_ignore_both() {
        let opts = DiffOptions {
            ignore_whitespace: true,
            ignore_case: true,
            ..Default::default()
        };
        let result = compute_diff_with_options("Hello   World\n", "hello world\n", &opts);
        assert_eq!(result.diff_count, 0);
    }

    #[test]
    fn test_line_filter() {
        let opts = DiffOptions {
            line_filters: vec![r"^//.*".to_string()],
            ..Default::default()
        };
        let left = "code\n// comment v1\n";
        let right = "code\n// comment v2\n";
        let result = compute_diff_with_options(left, right, &opts);
        assert_eq!(result.diff_count, 0, "Comment lines should be filtered out");
    }

    #[test]
    fn test_substitution_filter() {
        let opts = DiffOptions {
            substitution_filters: vec![(r"\d{4}-\d{2}-\d{2}".to_string(), "DATE".to_string())],
            ..Default::default()
        };
        let left = "created: 2024-01-01\n";
        let right = "created: 2025-06-15\n";
        let result = compute_diff_with_options(left, right, &opts);
        assert_eq!(result.diff_count, 0, "Date differences should be ignored");
    }

    #[test]
    fn test_block_grouping_multi_delete_insert() {
        // 3 lines deleted, 3 lines inserted → 1 diff block with 3 Modified lines
        let left = "a\nb\nc\nd\n";
        let right = "x\ny\nz\nd\n";
        let result = compute_diff_with_options(left, right, &DiffOptions::default());
        assert_eq!(result.diff_positions.len(), 1, "Should be 1 diff block");
        assert_eq!(result.diff_count, 1);
        let modified_count = result
            .lines
            .iter()
            .filter(|l| l.status == LineStatus::Modified)
            .count();
        assert_eq!(modified_count, 3, "3 pairs should be Modified");
    }

    #[test]
    fn test_block_grouping_unequal_sides() {
        // 2 lines deleted, 4 lines inserted → 1 block: 2 Modified + 2 Added
        let left = "a\nb\nc\n";
        let right = "x\ny\np\nq\nc\n";
        let result = compute_diff_with_options(left, right, &DiffOptions::default());
        assert_eq!(result.diff_positions.len(), 1, "Should be 1 diff block");
        let modified = result
            .lines
            .iter()
            .filter(|l| l.status == LineStatus::Modified)
            .count();
        let added = result
            .lines
            .iter()
            .filter(|l| l.status == LineStatus::Added)
            .count();
        assert_eq!(modified, 2);
        assert_eq!(added, 2);
    }

    #[test]
    fn test_two_separate_blocks() {
        // Two separate diff hunks → 2 diff blocks
        let left = "a\nb\nc\nd\ne\n";
        let right = "X\nb\nc\nd\nY\n";
        let result = compute_diff_with_options(left, right, &DiffOptions::default());
        assert_eq!(result.diff_positions.len(), 2, "Should be 2 diff blocks");
        assert_eq!(result.diff_count, 2);
    }

    #[test]
    fn test_line_filter_empty_pattern() {
        let opts = DiffOptions {
            line_filters: vec!["".to_string(), "  ".to_string()],
            ..Default::default()
        };
        let result = compute_diff_with_options("hello\n", "world\n", &opts);
        assert_eq!(result.diff_count, 1, "Empty filters should be ignored");
    }

    #[test]
    fn test_substitution_filter_invalid_regex() {
        let opts = DiffOptions {
            substitution_filters: vec![("[invalid".to_string(), "x".to_string())],
            ..Default::default()
        };
        let result = compute_diff_with_options("hello\n", "hello\n", &opts);
        assert_eq!(result.diff_count, 0, "Invalid regex should be skipped");
    }

    #[test]
    fn test_ignore_blank_lines_keeps_original_text_and_line_numbers() {
        let opts = DiffOptions {
            ignore_blank_lines: true,
            ..Default::default()
        };
        // The blank line is dropped from the comparison, so the left side's
        // normalized line 2 is the original line 3.
        let result = compute_diff_with_options("hello\n\nworld\n", "hello\nworld\n", &opts);
        assert_eq!(result.diff_count, 0);
        assert_eq!(result.lines.len(), 2);
        assert_eq!(result.lines[1].left_text, "world");
        assert_eq!(result.lines[1].right_text, "world");
        assert_eq!(result.lines[1].left_line_no, Some(3));
        assert_eq!(result.lines[1].right_line_no, Some(2));
    }

    #[test]
    fn test_line_filter_keeps_original_text_and_line_numbers() {
        let opts = DiffOptions {
            line_filters: vec!["^#".to_string()],
            ..Default::default()
        };
        let result = compute_diff_with_options("# note\nkeep\n", "keep\n", &opts);
        assert_eq!(result.diff_count, 0);
        assert_eq!(result.lines.len(), 1);
        assert_eq!(result.lines[0].left_text, "keep");
        assert_eq!(result.lines[0].left_line_no, Some(2));
        assert_eq!(result.lines[0].right_line_no, Some(1));
    }

    #[test]
    fn test_ignore_blank_lines_with_a_real_difference() {
        let opts = DiffOptions {
            ignore_blank_lines: true,
            ..Default::default()
        };
        let result = compute_diff_with_options("a\n\nb\n", "a\nc\n", &opts);
        assert_eq!(result.diff_count, 1);
        let modified = result
            .lines
            .iter()
            .find(|l| l.status == LineStatus::Modified)
            .expect("expected a modified line");
        assert_eq!(modified.left_text, "b");
        assert_eq!(modified.left_line_no, Some(3));
        assert_eq!(modified.right_text, "c");
        assert_eq!(modified.right_line_no, Some(2));
    }
}
