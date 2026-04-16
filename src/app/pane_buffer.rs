use std::rc::Rc;

use slint::{Model, SharedString, VecModel};

use super::diff_navigation::build_word_diff_string;
use super::helpers::expand_tabs;
use super::{STATUS_ADDED, STATUS_EQUAL, STATUS_REMOVED};
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

impl PaneBuffer {
    pub fn row_count(&self) -> usize {
        self.model.row_count()
    }
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
            LineStatus::Added => {
                // Right-only: left gets a ghost row
                let right_ln = line.right_line_no.unwrap();
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
                    status: STATUS_ADDED,
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
            LineStatus::Removed => {
                // Left-only: right gets a ghost row
                let left_ln = line.left_line_no.unwrap();
                left_rows.push(PaneLineData {
                    line_no: SharedString::from(left_ln.to_string()),
                    text: SharedString::from(expand_tabs(&line.left_text, tab_width)),
                    is_ghost: false,
                    status: STATUS_REMOVED,
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
