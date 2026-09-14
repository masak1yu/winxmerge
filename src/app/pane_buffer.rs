use std::rc::Rc;

use slint::{Model, SharedString, VecModel};

use super::STATUS_EQUAL;
use super::diff_navigation::build_word_diff_string;
use super::helpers::expand_tabs;
use crate::PaneLineData;
use crate::diff::three_way::{ThreeWayResult, ThreeWayStatus};
use crate::models::diff_line::{DiffResult, LineStatus};

/// A single pane's complete buffer, including ghost rows for alignment.
///
/// The `model` (Rc<VecModel<PaneLineData>>) is the **sole source of truth** for
/// this pane's content.  Ghost rows (`is_ghost == true`) exist only for vertical
/// alignment with sibling panes; they carry no real text.
pub struct PaneBuffer {
    /// The VecModel that backs the Slint ListView for this pane.
    pub model: Rc<VecModel<PaneLineData>>,
    /// Visual row index → real line index.  `None` for ghost rows.
    pub row_to_line: Vec<Option<usize>>,
    /// Real line index → visual row index (1-to-1 for real lines).
    pub line_to_row: Vec<usize>,
    /// Sorted indices of ghost rows in the model.
    pub ghost_rows: Vec<usize>,
}

// ---------------------------------------------------------------------------
// 2-way buffer construction
// ---------------------------------------------------------------------------

/// Build two `PaneBuffer`s from a 2-way `DiffResult`.
///
/// Both buffers have identical `row_count()` — ghost rows pad the shorter side.
pub fn build_pane_buffers_2way(
    result: &DiffResult,
    left_highlights: &[i32],
    right_highlights: &[i32],
    tab_width: usize,
) -> (PaneBuffer, PaneBuffer) {
    // Pre-compute per-line diff block index (same logic as build_diff_line_data)
    let mut line_block_idx: Vec<i32> = vec![-1; result.lines.len()];
    let mut current_block = -1i32;
    let mut was_in_diff = false;
    for (i, line) in result.lines.iter().enumerate() {
        if line.status != LineStatus::Equal {
            if !was_in_diff {
                current_block += 1;
                was_in_diff = true;
            }
            line_block_idx[i] = current_block;
        } else {
            was_in_diff = false;
        }
    }

    let mut left_rows: Vec<PaneLineData> = Vec::with_capacity(result.lines.len());
    let mut right_rows: Vec<PaneLineData> = Vec::with_capacity(result.lines.len());
    let mut left_row_to_line: Vec<Option<usize>> = Vec::with_capacity(result.lines.len());
    let mut right_row_to_line: Vec<Option<usize>> = Vec::with_capacity(result.lines.len());
    let mut left_ghost_rows: Vec<usize> = Vec::new();
    let mut right_ghost_rows: Vec<usize> = Vec::new();
    let mut left_line_to_row: Vec<usize> = Vec::new();
    let mut right_line_to_row: Vec<usize> = Vec::new();

    for (i, line) in result.lines.iter().enumerate() {
        let diff_index = line_block_idx[i];
        let row_idx = left_rows.len(); // same for both since they grow in lockstep

        let left_hl = line
            .left_line_no
            .and_then(|n| left_highlights.get((n - 1) as usize).copied())
            .unwrap_or(-1);
        let right_hl = line
            .right_line_no
            .and_then(|n| right_highlights.get((n - 1) as usize).copied())
            .unwrap_or(-1);

        let left_word_diff = build_word_diff_string(&line.left_word_segments);
        let right_word_diff = build_word_diff_string(&line.right_word_segments);

        match line.status {
            LineStatus::Equal => {
                // Both sides are real lines
                let left_ln = line.left_line_no.unwrap();
                let right_ln = line.right_line_no.unwrap();
                left_rows.push(PaneLineData {
                    line_no: SharedString::from(left_ln.to_string()),
                    text: SharedString::from(expand_tabs(&line.left_text, tab_width)),
                    is_ghost: false,
                    status: STATUS_EQUAL,
                    diff_index,
                    word_diff: SharedString::default(),
                    is_current_diff: false,
                    is_search_match: false,
                    is_selected: false,
                    highlight: left_hl,
                });
                right_rows.push(PaneLineData {
                    line_no: SharedString::from(right_ln.to_string()),
                    text: SharedString::from(expand_tabs(&line.right_text, tab_width)),
                    is_ghost: false,
                    status: STATUS_EQUAL,
                    diff_index,
                    word_diff: SharedString::default(),
                    is_current_diff: false,
                    is_search_match: false,
                    is_selected: false,
                    highlight: right_hl,
                });
                left_row_to_line.push(Some(left_line_to_row.len()));
                left_line_to_row.push(row_idx);
                right_row_to_line.push(Some(right_line_to_row.len()));
                right_line_to_row.push(row_idx);
            }
            LineStatus::Added | LineStatus::Moved if line.left_line_no.is_none() => {
                // Right-only (or the "added" half of a moved pair): left gets a ghost row
                let right_ln = line.right_line_no.unwrap();
                let status = line.status.as_i32();
                left_rows.push(PaneLineData {
                    line_no: SharedString::default(),
                    text: SharedString::default(),
                    is_ghost: true,
                    status: STATUS_EQUAL, // ghost shown as neutral in this pane
                    diff_index,
                    word_diff: SharedString::default(),
                    is_current_diff: false,
                    is_search_match: false,
                    is_selected: false,
                    highlight: -1,
                });
                right_rows.push(PaneLineData {
                    line_no: SharedString::from(right_ln.to_string()),
                    text: SharedString::from(expand_tabs(&line.right_text, tab_width)),
                    is_ghost: false,
                    status,
                    diff_index,
                    word_diff: SharedString::default(),
                    is_current_diff: false,
                    is_search_match: false,
                    is_selected: false,
                    highlight: right_hl,
                });
                left_row_to_line.push(None);
                left_ghost_rows.push(row_idx);
                right_row_to_line.push(Some(right_line_to_row.len()));
                right_line_to_row.push(row_idx);
            }
            LineStatus::Removed | LineStatus::Moved if line.right_line_no.is_none() => {
                // Left-only (or the "removed" half of a moved pair): right gets a ghost row
                let left_ln = line.left_line_no.unwrap();
                let status = line.status.as_i32();
                left_rows.push(PaneLineData {
                    line_no: SharedString::from(left_ln.to_string()),
                    text: SharedString::from(expand_tabs(&line.left_text, tab_width)),
                    is_ghost: false,
                    status,
                    diff_index,
                    word_diff: SharedString::default(),
                    is_current_diff: false,
                    is_search_match: false,
                    is_selected: false,
                    highlight: left_hl,
                });
                right_rows.push(PaneLineData {
                    line_no: SharedString::default(),
                    text: SharedString::default(),
                    is_ghost: true,
                    status: STATUS_EQUAL,
                    diff_index,
                    word_diff: SharedString::default(),
                    is_current_diff: false,
                    is_search_match: false,
                    is_selected: false,
                    highlight: -1,
                });
                left_row_to_line.push(Some(left_line_to_row.len()));
                left_line_to_row.push(row_idx);
                right_row_to_line.push(None);
                right_ghost_rows.push(row_idx);
            }
            LineStatus::Modified | LineStatus::Moved => {
                // Both sides are real lines with different content
                let left_ln = line.left_line_no.unwrap();
                let right_ln = line.right_line_no.unwrap();
                let status = line.status.as_i32();
                left_rows.push(PaneLineData {
                    line_no: SharedString::from(left_ln.to_string()),
                    text: SharedString::from(expand_tabs(&line.left_text, tab_width)),
                    is_ghost: false,
                    status,
                    diff_index,
                    word_diff: SharedString::from(left_word_diff),
                    is_current_diff: false,
                    is_search_match: false,
                    is_selected: false,
                    highlight: left_hl,
                });
                right_rows.push(PaneLineData {
                    line_no: SharedString::from(right_ln.to_string()),
                    text: SharedString::from(expand_tabs(&line.right_text, tab_width)),
                    is_ghost: false,
                    status,
                    diff_index,
                    word_diff: SharedString::from(right_word_diff),
                    is_current_diff: false,
                    is_search_match: false,
                    is_selected: false,
                    highlight: right_hl,
                });
                left_row_to_line.push(Some(left_line_to_row.len()));
                left_line_to_row.push(row_idx);
                right_row_to_line.push(Some(right_line_to_row.len()));
                right_line_to_row.push(row_idx);
            }
            // Unreachable per diff engine invariants (Added always has left_line_no
            // == None, Removed always has right_line_no == None), but the guarded
            // arms above don't count toward exhaustiveness, so rustc needs this.
            LineStatus::Added | LineStatus::Removed => {
                unreachable!(
                    "Added must have left_line_no == None and Removed must have right_line_no == None"
                )
            }
        }
    }

    debug_assert_eq!(
        left_rows.len(),
        right_rows.len(),
        "pane buffers must have equal row count"
    );

    let left_model = Rc::new(VecModel::from(left_rows));
    let right_model = Rc::new(VecModel::from(right_rows));

    (
        PaneBuffer {
            model: left_model,
            row_to_line: left_row_to_line,
            line_to_row: left_line_to_row,
            ghost_rows: left_ghost_rows,
        },
        PaneBuffer {
            model: right_model,
            row_to_line: right_row_to_line,
            line_to_row: right_line_to_row,
            ghost_rows: right_ghost_rows,
        },
    )
}

// ---------------------------------------------------------------------------
// 3-way buffer construction
// ---------------------------------------------------------------------------

/// Build three `PaneBuffer`s from a 3-way diff result.
///
/// Returns `(left, middle, right)`.  All three buffers have identical `row_count()`.
/// The middle pane corresponds to the diff engine's "base" data; the caller
/// decides which logical role (base, theirs, etc.) the middle pane represents.
pub fn build_pane_buffers_3way(result: &ThreeWayResult) -> (PaneBuffer, PaneBuffer, PaneBuffer) {
    let mut left_rows: Vec<PaneLineData> = Vec::with_capacity(result.lines.len());
    let mut middle_rows: Vec<PaneLineData> = Vec::with_capacity(result.lines.len());
    let mut right_rows: Vec<PaneLineData> = Vec::with_capacity(result.lines.len());

    let mut left_row_to_line: Vec<Option<usize>> = Vec::with_capacity(result.lines.len());
    let mut middle_row_to_line: Vec<Option<usize>> = Vec::with_capacity(result.lines.len());
    let mut right_row_to_line: Vec<Option<usize>> = Vec::with_capacity(result.lines.len());

    let mut left_ghost_rows: Vec<usize> = Vec::new();
    let mut middle_ghost_rows: Vec<usize> = Vec::new();
    let mut right_ghost_rows: Vec<usize> = Vec::new();

    let mut left_line_to_row: Vec<usize> = Vec::new();
    let mut middle_line_to_row: Vec<usize> = Vec::new();
    let mut right_line_to_row: Vec<usize> = Vec::new();

    // Pre-compute per-line conflict block index
    let mut line_conflict_idx: Vec<i32> = vec![-1; result.lines.len()];
    let mut current_block = -1i32;
    let mut was_in_diff = false;
    for (i, line) in result.lines.iter().enumerate() {
        if line.status != ThreeWayStatus::Equal {
            if !was_in_diff {
                current_block += 1;
                was_in_diff = true;
            }
            line_conflict_idx[i] = current_block;
        } else {
            was_in_diff = false;
        }
    }

    for (i, line) in result.lines.iter().enumerate() {
        let row_idx = left_rows.len();
        let status = line.status.as_i32();
        let conflict_index = line_conflict_idx[i];

        // Left pane
        if let Some(ln) = line.left_line_no {
            left_rows.push(PaneLineData {
                line_no: SharedString::from(ln.to_string()),
                text: SharedString::from(&line.left_text),
                is_ghost: false,
                status,
                diff_index: conflict_index,
                word_diff: SharedString::default(),
                is_current_diff: false,
                is_search_match: false,
                is_selected: false,
                highlight: -1,
            });
            left_row_to_line.push(Some(left_line_to_row.len()));
            left_line_to_row.push(row_idx);
        } else {
            left_rows.push(ghost_pane_line(conflict_index));
            left_row_to_line.push(None);
            left_ghost_rows.push(row_idx);
        }

        // Middle pane (maps to diff engine's "base" data)
        if let Some(ln) = line.base_line_no {
            middle_rows.push(PaneLineData {
                line_no: SharedString::from(ln.to_string()),
                text: SharedString::from(&line.base_text),
                is_ghost: false,
                status,
                diff_index: conflict_index,
                word_diff: SharedString::default(),
                is_current_diff: false,
                is_search_match: false,
                is_selected: false,
                highlight: -1,
            });
            middle_row_to_line.push(Some(middle_line_to_row.len()));
            middle_line_to_row.push(row_idx);
        } else {
            middle_rows.push(ghost_pane_line(conflict_index));
            middle_row_to_line.push(None);
            middle_ghost_rows.push(row_idx);
        }

        // Right pane
        if let Some(ln) = line.right_line_no {
            right_rows.push(PaneLineData {
                line_no: SharedString::from(ln.to_string()),
                text: SharedString::from(&line.right_text),
                is_ghost: false,
                status,
                diff_index: conflict_index,
                word_diff: SharedString::default(),
                is_current_diff: false,
                is_search_match: false,
                is_selected: false,
                highlight: -1,
            });
            right_row_to_line.push(Some(right_line_to_row.len()));
            right_line_to_row.push(row_idx);
        } else {
            right_rows.push(ghost_pane_line(conflict_index));
            right_row_to_line.push(None);
            right_ghost_rows.push(row_idx);
        }
    }

    debug_assert_eq!(
        left_rows.len(),
        middle_rows.len(),
        "left/middle row count mismatch"
    );
    debug_assert_eq!(
        left_rows.len(),
        right_rows.len(),
        "left/right row count mismatch"
    );

    (
        PaneBuffer {
            model: Rc::new(VecModel::from(left_rows)),
            row_to_line: left_row_to_line,
            line_to_row: left_line_to_row,
            ghost_rows: left_ghost_rows,
        },
        PaneBuffer {
            model: Rc::new(VecModel::from(middle_rows)),
            row_to_line: middle_row_to_line,
            line_to_row: middle_line_to_row,
            ghost_rows: middle_ghost_rows,
        },
        PaneBuffer {
            model: Rc::new(VecModel::from(right_rows)),
            row_to_line: right_row_to_line,
            line_to_row: right_line_to_row,
            ghost_rows: right_ghost_rows,
        },
    )
}

/// Create a ghost (alignment padding) PaneLineData row.
fn ghost_pane_line(diff_index: i32) -> PaneLineData {
    PaneLineData {
        line_no: SharedString::default(),
        text: SharedString::default(),
        is_ghost: true,
        status: STATUS_EQUAL,
        diff_index,
        word_diff: SharedString::default(),
        is_current_diff: false,
        is_search_match: false,
        is_selected: false,
        highlight: -1,
    }
}

// ---------------------------------------------------------------------------
// Buffer utilities
// ---------------------------------------------------------------------------

/// Extract real (non-ghost) line texts from a PaneBuffer, joined with newlines.
/// Returns the empty string only when the buffer has zero real lines.
pub fn extract_real_lines(buffer: &PaneBuffer) -> String {
    let model = &buffer.model;
    let mut lines: Vec<String> = Vec::new();
    for i in 0..model.row_count() {
        if let Some(row) = model.row_data(i) {
            if !row.is_ghost {
                lines.push(row.text.to_string());
            }
        }
    }
    if lines.is_empty() {
        String::new()
    } else {
        lines.join("\n") + "\n"
    }
}

/// Sync a single PaneBuffer row's text content.
/// No-op if buffer is None or the row doesn't exist.
pub fn sync_pane_row_text(buffer: &Option<PaneBuffer>, row_idx: usize, new_text: &str) {
    if let Some(buf) = buffer {
        if let Some(mut row) = buf.model.row_data(row_idx) {
            row.text = SharedString::from(new_text);
            buf.model.set_row_data(row_idx, row);
        }
    }
}

/// Turn the ghost row at `idx` into a real (saved) line. Returns false when
/// the row is missing or already real.
pub fn materialize_ghost(buffer: &mut PaneBuffer, idx: usize) -> bool {
    let Some(mut row) = buffer.model.row_data(idx) else {
        return false;
    };
    if !row.is_ghost {
        return false;
    }
    row.is_ghost = false;
    buffer.model.set_row_data(idx, row);
    renumber_pane_buffer(buffer);
    true
}

/// What deleting the empty row at some index should do to its own pane and
/// the aligned row in the sibling pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineDeletion {
    /// The sibling row is also a ghost: drop the row from both panes.
    RemoveBoth,
    /// The sibling row is a real line: only turn the target row into a
    /// ghost, so the sibling's real line survives.
    GhostTarget,
}

/// Decide what deleting the row at `idx` in `target` should do, given the
/// aligned row in `other`. Returns `None` when nothing should happen: the
/// row is out of range, already a ghost, not empty, or the pane's last real
/// line (deleting it would leave the pane un-typeable, see
/// `three_way_delete_line`'s equivalent guard).
pub fn plan_line_deletion(
    target: &PaneBuffer,
    other: &PaneBuffer,
    idx: usize,
) -> Option<LineDeletion> {
    let target_row = target.model.row_data(idx)?;
    if target_row.is_ghost || !target_row.text.is_empty() {
        return None;
    }
    if target.line_to_row.len() <= 1 {
        return None;
    }
    let other_row = other.model.row_data(idx)?;
    Some(if other_row.is_ghost {
        LineDeletion::RemoveBoth
    } else {
        LineDeletion::GhostTarget
    })
}

/// Apply a `LineDeletion` decided by `plan_line_deletion` and renumber both
/// panes afterward.
pub fn apply_line_deletion(
    target: &mut PaneBuffer,
    other: &mut PaneBuffer,
    idx: usize,
    deletion: LineDeletion,
) {
    match deletion {
        LineDeletion::RemoveBoth => {
            target.model.remove(idx);
            other.model.remove(idx);
        }
        LineDeletion::GhostTarget => {
            // Other pane keeps its real line untouched; only the target row
            // is retired to a ghost so the row-index alignment survives.
            let diff_index = target
                .model
                .row_data(idx)
                .map(|r| r.diff_index)
                .unwrap_or(-1);
            target.model.set_row_data(idx, ghost_pane_line(diff_index));
        }
    }
    renumber_pane_buffer(target);
    renumber_pane_buffer(other);
}

/// Renumber real lines in a PaneBuffer after insert/delete operations.
/// Rebuilds `row_to_line`, `line_to_row`, and `ghost_rows` from the model.
pub fn renumber_pane_buffer(buffer: &mut PaneBuffer) {
    let count = buffer.model.row_count();
    buffer.row_to_line.clear();
    buffer.row_to_line.reserve(count);
    buffer.line_to_row.clear();
    buffer.ghost_rows.clear();

    let mut real_line = 0usize;
    for i in 0..count {
        if let Some(mut row) = buffer.model.row_data(i) {
            if row.is_ghost {
                buffer.row_to_line.push(None);
                buffer.ghost_rows.push(i);
            } else {
                buffer.row_to_line.push(Some(real_line));
                buffer.line_to_row.push(i);
                // Update line number display
                let new_no = SharedString::from((real_line + 1).to_string());
                if row.line_no != new_no {
                    row.line_no = new_no;
                    buffer.model.set_row_data(i, row);
                }
                real_line += 1;
            }
        }
    }
}

/// Estimate the pixel width needed to display the longest line across all given models.
/// Used to set `viewport-width` on diff pane ListViews for horizontal scrolling.
pub fn max_content_width_px(models: &[Rc<VecModel<PaneLineData>>], font_size: i32) -> i32 {
    let max_len = models
        .iter()
        .flat_map(|m| {
            let count = m.row_count();
            let m = m.clone();
            (0..count).map(move |i| m.row_data(i).map(|d| d.text.len()).unwrap_or(0))
        })
        .max()
        .unwrap_or(0);
    if max_len == 0 {
        return 0;
    }
    // Approximate monospace char width: font_size * 0.62 + line-number column + padding
    ((max_len as f32) * (font_size as f32) * 0.62) as i32 + 100
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::engine::{DiffOptions, compute_diff_with_options};
    use crate::models::diff_line::DiffLine;

    // What: with `ignore_blank_lines` enabled, a blank line present in the
    // original left-side file vanishes from `extract_real_lines`' reconstructed
    // text after the full `compute_diff_with_options` → `build_pane_buffers_2way`
    // → `extract_real_lines` round trip. `normalize_text` drops the blank line
    // (and its position from the line-number map) before diffing, so it never
    // reaches `DiffResult.lines`, `PaneBuffer.model`, or the saved file — this
    // pins the round-trip data-loss behavior fixed provisionally by issue #63.
    #[test]
    fn ignore_blank_lines_drops_blank_line_after_full_roundtrip() {
        let opts = DiffOptions {
            ignore_blank_lines: true,
            ..Default::default()
        };
        let left = "hello\n\nworld\n";
        let right = "hello\nworld\n";
        let result = compute_diff_with_options(left, right, &opts);
        let (left_buf, right_buf) = build_pane_buffers_2way(&result, &[], &[], 4);

        let left_out = extract_real_lines(&left_buf);
        let right_out = extract_real_lines(&right_buf);

        assert_eq!(
            left_out, "hello\nworld\n",
            "blank line silently dropped from the reconstructed left text"
        );
        assert_eq!(right_out, "hello\nworld\n");
        assert_ne!(
            left_out, left,
            "round-tripped text no longer matches the original file contents"
        );
    }

    // What: a line matching an active line filter vanishes from
    // `extract_real_lines`' reconstructed text after the same round trip —
    // same data-loss mechanism as `ignore_blank_lines`, just triggered by
    // `line_filters` instead of the blank-line check in `normalize_text`.
    #[test]
    fn line_filter_drops_filtered_line_after_full_roundtrip() {
        let opts = DiffOptions {
            line_filters: vec!["^#".to_string()],
            ..Default::default()
        };
        let left = "# note\nkeep\n";
        let right = "keep\n";
        let result = compute_diff_with_options(left, right, &opts);
        let (left_buf, right_buf) = build_pane_buffers_2way(&result, &[], &[], 4);

        let left_out = extract_real_lines(&left_buf);
        let right_out = extract_real_lines(&right_buf);

        assert_eq!(
            left_out, "keep\n",
            "filtered line silently dropped from the reconstructed left text"
        );
        assert_eq!(right_out, "keep\n");
        assert_ne!(
            left_out, left,
            "round-tripped text no longer matches the original file contents"
        );
    }

    // What: with `detect_moved_lines` enabled, a block that moved position
    // between left and right is reported by the diff engine as a `Moved` pair
    // where one half keeps `left_line_no == None` (the former Added line) and
    // the other keeps `right_line_no == None` (the former Removed line).
    // `build_pane_buffers_2way` must place a ghost row on the side missing a
    // line number instead of unwrapping it, must not panic, must preserve the
    // real text on both sides through `extract_real_lines`, must render the
    // moved half as status 4, and must keep every ghost row's text empty.
    #[test]
    fn detect_moved_lines_produces_ghost_rows_without_panicking() {
        let opts = DiffOptions {
            detect_moved_lines: true,
            ..Default::default()
        };
        let left = "alpha\nbravo\ncharlie\ndelta\necho\nfoxtrot\ngolf\nhotel\n";
        let right = "alpha\nbravo\nfoxtrot\ngolf\ncharlie\ndelta\necho\nhotel\n";
        let result = compute_diff_with_options(left, right, &opts);

        assert!(
            result.lines.iter().any(|l| l.status == LineStatus::Moved),
            "test input must produce at least one Moved line for this test to be meaningful"
        );

        let (left_buf, right_buf) = build_pane_buffers_2way(&result, &[], &[], 4);

        assert_eq!(
            extract_real_lines(&left_buf),
            left,
            "left buffer must reconstruct the original left text"
        );
        assert_eq!(
            extract_real_lines(&right_buf),
            right,
            "right buffer must reconstruct the original right text"
        );

        let mut found_moved_status = false;
        for buf in [&left_buf, &right_buf] {
            for i in 0..buf.model.row_count() {
                let row = buf.model.row_data(i).unwrap();
                if row.is_ghost {
                    assert_eq!(
                        row.text.as_str(),
                        "",
                        "ghost rows must never carry the copied counterpart text"
                    );
                } else if row.status == 4 {
                    found_moved_status = true;
                }
            }
        }
        assert!(
            found_moved_status,
            "at least one non-ghost row must show the Moved status (4)"
        );
    }

    // Helper for the delete_line planning tests: builds a DiffLine without
    // repeating all six fields at every call site.
    fn diff_line(
        left_line_no: Option<u32>,
        right_line_no: Option<u32>,
        left_text: &str,
        right_text: &str,
        status: LineStatus,
    ) -> DiffLine {
        DiffLine {
            left_line_no,
            right_line_no,
            left_text: left_text.to_string(),
            right_text: right_text.to_string(),
            status,
            left_word_segments: vec![],
            right_word_segments: vec![],
        }
    }

    // What: a pane holding exactly one real line refuses to delete it, even
    // when that line's text is empty — mirrors `three_way_delete_line`'s
    // last-line guard so the pane never becomes un-typeable.
    #[test]
    fn plan_line_deletion_refuses_last_real_line() {
        let result = DiffResult {
            lines: vec![diff_line(Some(1), Some(1), "", "", LineStatus::Equal)],
            diff_count: 0,
            diff_positions: vec![],
        };
        let (left_buf, right_buf) = build_pane_buffers_2way(&result, &[], &[], 4);

        assert_eq!(plan_line_deletion(&left_buf, &right_buf, 0), None);
        // Buffers must be untouched since no deletion was planned.
        assert_eq!(extract_real_lines(&left_buf), "\n");
        assert_eq!(extract_real_lines(&right_buf), "\n");
    }

    // What: a ghost target row is refused outright, regardless of the
    // counterpart or of how many real lines remain in the pane.
    #[test]
    fn plan_line_deletion_refuses_ghost_target() {
        let result = DiffResult {
            lines: vec![
                diff_line(Some(1), Some(1), "keep1", "keep1", LineStatus::Equal),
                // Added: left has no line here, so build_pane_buffers_2way
                // places a ghost row on the left.
                diff_line(None, Some(2), "", "added", LineStatus::Added),
                diff_line(Some(2), Some(3), "keep2", "keep2", LineStatus::Equal),
            ],
            diff_count: 1,
            diff_positions: vec![1],
        };
        let (left_buf, right_buf) = build_pane_buffers_2way(&result, &[], &[], 4);
        assert!(left_buf.model.row_data(1).unwrap().is_ghost);

        assert_eq!(plan_line_deletion(&left_buf, &right_buf, 1), None);
    }

    // What: deleting an empty row whose counterpart on the other side is a
    // real (non-ghost) line must NOT remove that counterpart. Only the
    // target row is turned into a ghost; the other pane's `extract_real_lines`
    // is byte-for-byte unchanged, and the target loses exactly that one line.
    // Under the old (pre-fix) `delete_line`, which removed the same row index
    // from both PaneBuffers unconditionally, this would have deleted "b" from
    // the right pane too — this test would fail against that behavior.
    #[test]
    fn plan_line_deletion_ghosts_target_when_other_side_is_real() {
        let result = DiffResult {
            lines: vec![
                diff_line(Some(1), Some(1), "keep1", "keep1", LineStatus::Equal),
                diff_line(Some(2), Some(2), "", "b", LineStatus::Modified),
                diff_line(Some(3), Some(3), "keep2", "keep2", LineStatus::Equal),
            ],
            diff_count: 1,
            diff_positions: vec![1],
        };
        let (mut left_buf, mut right_buf) = build_pane_buffers_2way(&result, &[], &[], 4);
        assert!(!right_buf.model.row_data(1).unwrap().is_ghost);
        assert_eq!(
            extract_real_lines(&left_buf),
            "keep1\n\nkeep2\n",
            "sanity: target row starts out real with empty text"
        );
        let right_before = extract_real_lines(&right_buf);

        let deletion = plan_line_deletion(&left_buf, &right_buf, 1);
        assert_eq!(deletion, Some(LineDeletion::GhostTarget));
        apply_line_deletion(&mut left_buf, &mut right_buf, 1, deletion.unwrap());

        assert_eq!(
            extract_real_lines(&right_buf),
            right_before,
            "other side must be untouched when its row is a real line"
        );
        let target_row = left_buf.model.row_data(1).unwrap();
        assert!(target_row.is_ghost, "target row must become a ghost");
        assert_eq!(
            extract_real_lines(&left_buf),
            "keep1\nkeep2\n",
            "target must lose exactly the deleted line"
        );
    }

    // What: deleting an empty row whose counterpart on the other side is
    // already a ghost removes the row from both panes (the pre-existing,
    // still-correct behavior for aligned ghost padding).
    #[test]
    fn plan_line_deletion_removes_both_when_other_side_is_ghost() {
        let result = DiffResult {
            lines: vec![
                diff_line(Some(1), Some(1), "keep1", "keep1", LineStatus::Equal),
                // Removed: right has no line here, so build_pane_buffers_2way
                // places a ghost row on the right; left's text is empty.
                diff_line(Some(2), None, "", "", LineStatus::Removed),
                diff_line(Some(3), Some(2), "keep2", "keep2", LineStatus::Equal),
            ],
            diff_count: 1,
            diff_positions: vec![1],
        };
        let (mut left_buf, mut right_buf) = build_pane_buffers_2way(&result, &[], &[], 4);
        assert!(right_buf.model.row_data(1).unwrap().is_ghost);
        let left_before = left_buf.model.row_count();
        let right_before = right_buf.model.row_count();

        let deletion = plan_line_deletion(&left_buf, &right_buf, 1);
        assert_eq!(deletion, Some(LineDeletion::RemoveBoth));
        apply_line_deletion(&mut left_buf, &mut right_buf, 1, deletion.unwrap());

        assert_eq!(left_buf.model.row_count(), left_before - 1);
        assert_eq!(right_buf.model.row_count(), right_before - 1);
    }

    // What: typing into a ghost row (F7) promotes it to a real line via
    // `materialize_ghost`, after which syncing text into it makes the text
    // show up in `extract_real_lines` and the row count `line_to_row`
    // tracks. A second `materialize_ghost` on the now-real row, and one on
    // a row that was always real, both report failure and change nothing.
    #[test]
    fn materialize_ghost_turns_ghost_row_into_saved_line() {
        let result = DiffResult {
            lines: vec![
                diff_line(Some(1), Some(1), "keep1", "keep1", LineStatus::Equal),
                // Added: left has no line here, so build_pane_buffers_2way
                // places a ghost row on the left.
                diff_line(None, Some(2), "", "added", LineStatus::Added),
                diff_line(Some(2), Some(3), "keep2", "keep2", LineStatus::Equal),
            ],
            diff_count: 1,
            diff_positions: vec![1],
        };
        let (mut left_buf, _right_buf) = build_pane_buffers_2way(&result, &[], &[], 4);
        assert!(left_buf.model.row_data(1).unwrap().is_ghost);

        assert!(materialize_ghost(&mut left_buf, 1));

        // sync_pane_row_text takes &Option<PaneBuffer>; wrap temporarily.
        let wrapped = Some(left_buf);
        sync_pane_row_text(&wrapped, 1, "typed");
        let mut left_buf = wrapped.unwrap();

        assert_eq!(extract_real_lines(&left_buf), "keep1\ntyped\nkeep2\n");
        assert_eq!(left_buf.line_to_row.len(), 3);

        assert!(
            !materialize_ghost(&mut left_buf, 1),
            "row is already real; second call must be a no-op"
        );
        assert!(
            !materialize_ghost(&mut left_buf, 0),
            "row 0 was never a ghost"
        );
    }
}
