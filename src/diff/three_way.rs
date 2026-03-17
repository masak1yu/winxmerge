use similar::{ChangeTag, TextDiff};

#[derive(Debug, Clone, PartialEq)]
pub enum ThreeWayStatus {
    Equal,        // Same in all three
    LeftChanged,  // Only left changed from base
    RightChanged, // Only right changed from base
    BothChanged,  // Both changed same way (auto-merge)
    Conflict,     // Both changed differently
}

#[derive(Debug, Clone)]
pub struct ThreeWayLine {
    pub base_text: String,
    pub left_text: String,
    pub right_text: String,
    pub status: ThreeWayStatus,
    pub base_line_no: Option<u32>,
    pub left_line_no: Option<u32>,
    pub right_line_no: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct ThreeWayResult {
    pub lines: Vec<ThreeWayLine>,
    pub conflict_count: u32,
    pub conflict_positions: Vec<usize>,
}

/// Compute a 3-way diff given base, left, and right texts.
/// Uses base↔left and base↔right diffs to detect conflicts.
pub fn compute_three_way_diff(
    base_text: &str,
    left_text: &str,
    right_text: &str,
) -> ThreeWayResult {
    let base_lines: Vec<&str> = base_text.lines().collect();
    let left_lines: Vec<&str> = left_text.lines().collect();
    let right_lines: Vec<&str> = right_text.lines().collect();

    // Compute base→left and base→right changes
    let left_diff = TextDiff::from_lines(base_text, left_text);
    let right_diff = TextDiff::from_lines(base_text, right_text);

    let left_changes = collect_line_changes(&left_diff);
    let right_changes = collect_line_changes(&right_diff);

    let mut result_lines = Vec::new();
    let mut conflict_positions = Vec::new();
    let mut conflict_count = 0u32;

    let mut base_idx = 0u32;
    let mut left_idx = 0u32;
    let mut right_idx = 0u32;

    let max_base = base_lines.len() as u32;

    // Walk through base lines and detect changes
    let mut li = 0; // index into left_changes
    let mut ri = 0; // index into right_changes

    while base_idx < max_base || li < left_changes.len() as u32 || ri < right_changes.len() as u32 {
        let left_change = left_changes.get(li as usize);
        let right_change = right_changes.get(ri as usize);

        // Check if current base line has changes
        let left_at_base = left_change
            .map(|c| c.base_line == base_idx)
            .unwrap_or(false);
        let right_at_base = right_change
            .map(|c| c.base_line == base_idx)
            .unwrap_or(false);

        if left_at_base && right_at_base {
            let lc = left_change.unwrap();
            let rc = right_change.unwrap();

            if lc.new_text == rc.new_text {
                // Both changed the same way - auto-merge
                left_idx += 1;
                right_idx += 1;
                base_idx += 1;
                result_lines.push(ThreeWayLine {
                    base_text: base_lines
                        .get(lc.base_line as usize)
                        .unwrap_or(&"")
                        .to_string(),
                    left_text: lc.new_text.clone(),
                    right_text: rc.new_text.clone(),
                    status: ThreeWayStatus::BothChanged,
                    base_line_no: Some(base_idx),
                    left_line_no: Some(left_idx),
                    right_line_no: Some(right_idx),
                });
            } else {
                // Conflict
                conflict_positions.push(result_lines.len());
                conflict_count += 1;
                left_idx += 1;
                right_idx += 1;
                base_idx += 1;
                result_lines.push(ThreeWayLine {
                    base_text: base_lines
                        .get(lc.base_line as usize)
                        .unwrap_or(&"")
                        .to_string(),
                    left_text: lc.new_text.clone(),
                    right_text: rc.new_text.clone(),
                    status: ThreeWayStatus::Conflict,
                    base_line_no: Some(base_idx),
                    left_line_no: Some(left_idx),
                    right_line_no: Some(right_idx),
                });
            }
            li += 1;
            ri += 1;
        } else if left_at_base {
            let lc = left_change.unwrap();
            left_idx += 1;
            right_idx += 1;
            base_idx += 1;
            result_lines.push(ThreeWayLine {
                base_text: base_lines
                    .get(lc.base_line as usize)
                    .unwrap_or(&"")
                    .to_string(),
                left_text: lc.new_text.clone(),
                right_text: right_lines
                    .get(right_idx as usize - 1)
                    .unwrap_or(&"")
                    .to_string(),
                status: ThreeWayStatus::LeftChanged,
                base_line_no: Some(base_idx),
                left_line_no: Some(left_idx),
                right_line_no: Some(right_idx),
            });
            li += 1;
        } else if right_at_base {
            let rc = right_change.unwrap();
            left_idx += 1;
            right_idx += 1;
            base_idx += 1;
            result_lines.push(ThreeWayLine {
                base_text: base_lines
                    .get(rc.base_line as usize)
                    .unwrap_or(&"")
                    .to_string(),
                left_text: left_lines
                    .get(left_idx as usize - 1)
                    .unwrap_or(&"")
                    .to_string(),
                right_text: rc.new_text.clone(),
                status: ThreeWayStatus::RightChanged,
                base_line_no: Some(base_idx),
                left_line_no: Some(left_idx),
                right_line_no: Some(right_idx),
            });
            ri += 1;
        } else if base_idx < max_base {
            // No changes at this base line
            left_idx += 1;
            right_idx += 1;
            base_idx += 1;
            let text = base_lines
                .get(base_idx as usize - 1)
                .unwrap_or(&"")
                .to_string();
            result_lines.push(ThreeWayLine {
                base_text: text.clone(),
                left_text: text.clone(),
                right_text: text,
                status: ThreeWayStatus::Equal,
                base_line_no: Some(base_idx),
                left_line_no: Some(left_idx),
                right_line_no: Some(right_idx),
            });
        } else {
            break;
        }
    }

    ThreeWayResult {
        lines: result_lines,
        conflict_count,
        conflict_positions,
    }
}

#[derive(Debug)]
struct LineChange {
    base_line: u32, // 0-indexed base line that was modified
    new_text: String,
}

fn collect_line_changes<'a>(diff: &TextDiff<'a, 'a, 'a, str>) -> Vec<LineChange> {
    let mut changes = Vec::new();
    let all_changes: Vec<_> = diff.iter_all_changes().collect();
    let mut base_line: u32 = 0;
    let mut i = 0;

    while i < all_changes.len() {
        let change = &all_changes[i];
        match change.tag() {
            ChangeTag::Equal => {
                base_line += 1;
                i += 1;
            }
            ChangeTag::Delete => {
                // Modified: delete followed by insert
                if i + 1 < all_changes.len() && all_changes[i + 1].tag() == ChangeTag::Insert {
                    let new_text = all_changes[i + 1]
                        .value()
                        .trim_end_matches('\n')
                        .to_string();
                    changes.push(LineChange {
                        base_line,
                        new_text,
                    });
                    base_line += 1;
                    i += 2;
                } else {
                    // Pure deletion
                    changes.push(LineChange {
                        base_line,
                        new_text: String::new(),
                    });
                    base_line += 1;
                    i += 1;
                }
            }
            ChangeTag::Insert => {
                // Pure insertion (no base line consumed)
                i += 1;
            }
        }
    }

    changes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_conflicts() {
        let base = "aaa\nbbb\nccc\n";
        let left = "AAA\nbbb\nccc\n"; // changed line 1
        let right = "aaa\nbbb\nCCC\n"; // changed line 3
        let result = compute_three_way_diff(base, left, right);
        assert_eq!(result.conflict_count, 0);
    }

    #[test]
    fn test_conflict() {
        let base = "aaa\nbbb\nccc\n";
        let left = "XXX\nbbb\nccc\n"; // changed line 1
        let right = "YYY\nbbb\nccc\n"; // changed line 1 differently
        let result = compute_three_way_diff(base, left, right);
        assert_eq!(result.conflict_count, 1);
        assert_eq!(result.lines[0].status, ThreeWayStatus::Conflict);
    }

    #[test]
    fn test_both_same_change() {
        let base = "aaa\nbbb\nccc\n";
        let left = "XXX\nbbb\nccc\n";
        let right = "XXX\nbbb\nccc\n";
        let result = compute_three_way_diff(base, left, right);
        assert_eq!(result.conflict_count, 0);
        assert_eq!(result.lines[0].status, ThreeWayStatus::BothChanged);
    }
}
