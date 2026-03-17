use std::fs;
use std::path::PathBuf;

use slint::{Model, ModelRc, SharedString, VecModel};

use crate::diff::engine::compute_diff;
use crate::models::diff_line::LineStatus;
use crate::{DiffLineData, MainWindow};

pub struct AppState {
    pub left_path: Option<PathBuf>,
    pub right_path: Option<PathBuf>,
    pub diff_positions: Vec<usize>,
    pub current_diff: i32,
    pub left_lines: Vec<String>,
    pub right_lines: Vec<String>,
    pub has_unsaved_changes: bool,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            left_path: None,
            right_path: None,
            diff_positions: Vec::new(),
            current_diff: -1,
            left_lines: Vec::new(),
            right_lines: Vec::new(),
            has_unsaved_changes: false,
        }
    }
}

pub fn open_file_dialog(title: &str) -> Option<PathBuf> {
    rfd::FileDialog::new().set_title(title).pick_file()
}

pub fn run_diff(window: &MainWindow, state: &mut AppState) {
    let (left_path, right_path) = match (&state.left_path, &state.right_path) {
        (Some(l), Some(r)) => (l, r),
        _ => return,
    };

    let left_text = fs::read_to_string(left_path).unwrap_or_default();
    let right_text = fs::read_to_string(right_path).unwrap_or_default();

    recompute_diff_from_text(window, state, &left_text, &right_text);
}

pub fn recompute_diff_from_text(
    window: &MainWindow,
    state: &mut AppState,
    left_text: &str,
    right_text: &str,
) {
    let result = compute_diff(left_text, right_text);

    state.left_lines = left_text.lines().map(String::from).collect();
    state.right_lines = right_text.lines().map(String::from).collect();
    state.diff_positions = result.diff_positions.clone();
    state.current_diff = if result.diff_positions.is_empty() {
        -1
    } else {
        0
    };

    let diff_line_data: Vec<DiffLineData> = result
        .lines
        .iter()
        .enumerate()
        .map(|(_i, line)| {
            let status: i32 = match line.status {
                LineStatus::Equal => 0,
                LineStatus::Added => 1,
                LineStatus::Removed => 2,
                LineStatus::Modified => 3,
            };
            let diff_index = result
                .diff_positions
                .iter()
                .position(|&pos| pos == _i)
                .map(|idx| idx as i32)
                .unwrap_or(-1);
            DiffLineData {
                left_line_no: line
                    .left_line_no
                    .map(|n| SharedString::from(n.to_string()))
                    .unwrap_or_default(),
                right_line_no: line
                    .right_line_no
                    .map(|n| SharedString::from(n.to_string()))
                    .unwrap_or_default(),
                left_text: SharedString::from(&line.left_text),
                right_text: SharedString::from(&line.right_text),
                status,
                is_current_diff: diff_index == 0 && !result.diff_positions.is_empty(),
                diff_index,
            }
        })
        .collect();

    let model = ModelRc::new(VecModel::from(diff_line_data));
    window.set_diff_lines(model);
    window.set_diff_count(result.diff_count as i32);
    window.set_current_diff_index(state.current_diff);
    window.set_left_path(SharedString::from(
        state
            .left_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default(),
    ));
    window.set_right_path(SharedString::from(
        state
            .right_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default(),
    ));

    let status = if result.diff_count == 0 {
        "Files are identical".to_string()
    } else if state.current_diff >= 0 {
        format!(
            "Difference 1 of {} ({} total)",
            result.diff_count, result.diff_count
        )
    } else {
        format!("{} differences found", result.diff_count)
    };
    window.set_status_text(SharedString::from(status));
}

pub fn navigate_diff(window: &MainWindow, state: &mut AppState, forward: bool) {
    if state.diff_positions.is_empty() {
        return;
    }

    let new_index = if forward {
        if state.current_diff < state.diff_positions.len() as i32 - 1 {
            state.current_diff + 1
        } else {
            0
        }
    } else if state.current_diff > 0 {
        state.current_diff - 1
    } else {
        state.diff_positions.len() as i32 - 1
    };

    update_current_diff(window, state, new_index);
}

fn update_current_diff(window: &MainWindow, state: &mut AppState, new_index: i32) {
    state.current_diff = new_index;
    window.set_current_diff_index(new_index);

    let model = window.get_diff_lines();
    if let Some(vec_model) = model.as_any().downcast_ref::<VecModel<DiffLineData>>() {
        let current_pos = state.diff_positions[new_index as usize];
        for i in 0..vec_model.row_count() {
            let mut row = vec_model.row_data(i).unwrap();
            let should_highlight = i == current_pos;
            if row.is_current_diff != should_highlight {
                row.is_current_diff = should_highlight;
                vec_model.set_row_data(i, row);
            }
        }
    }

    window.set_status_text(SharedString::from(format!(
        "Difference {} of {}",
        new_index + 1,
        state.diff_positions.len()
    )));
}

/// Copy left side to right side for the given diff index
pub fn copy_to_right(window: &MainWindow, state: &mut AppState, diff_index: i32) {
    if diff_index < 0 || diff_index as usize >= state.diff_positions.len() {
        return;
    }

    let model = window.get_diff_lines();
    let vec_model = match model.as_any().downcast_ref::<VecModel<DiffLineData>>() {
        Some(m) => m,
        None => return,
    };

    let row_idx = state.diff_positions[diff_index as usize];
    let row = match vec_model.row_data(row_idx) {
        Some(r) => r,
        None => return,
    };

    // Reconstruct right file with the left side's content for this diff
    let right_text = rebuild_right_after_copy_from_left(vec_model);
    let left_text = rebuild_left(vec_model);

    state.has_unsaved_changes = true;
    window.set_has_unsaved_changes(true);

    // Preserve the current diff navigation position
    let _ = row;
    recompute_diff_from_text(window, state, &left_text, &right_text);

    // Try to stay near the same diff position
    if !state.diff_positions.is_empty() {
        let new_idx = (diff_index as usize).min(state.diff_positions.len() - 1);
        update_current_diff(window, state, new_idx as i32);
    }
}

/// Copy right side to left side for the given diff index
pub fn copy_to_left(window: &MainWindow, state: &mut AppState, diff_index: i32) {
    if diff_index < 0 || diff_index as usize >= state.diff_positions.len() {
        return;
    }

    let model = window.get_diff_lines();
    let vec_model = match model.as_any().downcast_ref::<VecModel<DiffLineData>>() {
        Some(m) => m,
        None => return,
    };

    let row_idx = state.diff_positions[diff_index as usize];
    let row = match vec_model.row_data(row_idx) {
        Some(r) => r,
        None => return,
    };

    let left_text = rebuild_left_after_copy_from_right(vec_model);
    let right_text = rebuild_right(vec_model);

    state.has_unsaved_changes = true;
    window.set_has_unsaved_changes(true);

    let _ = row;
    recompute_diff_from_text(window, state, &left_text, &right_text);

    if !state.diff_positions.is_empty() {
        let new_idx = (diff_index as usize).min(state.diff_positions.len() - 1);
        update_current_diff(window, state, new_idx as i32);
    }
}

fn rebuild_left(vec_model: &VecModel<DiffLineData>) -> String {
    let mut lines = Vec::new();
    for i in 0..vec_model.row_count() {
        let row = vec_model.row_data(i).unwrap();
        // Skip Added lines (only exist on right side)
        if row.status == 1 {
            continue;
        }
        lines.push(row.left_text.to_string());
    }
    lines.join("\n") + "\n"
}

fn rebuild_right(vec_model: &VecModel<DiffLineData>) -> String {
    let mut lines = Vec::new();
    for i in 0..vec_model.row_count() {
        let row = vec_model.row_data(i).unwrap();
        // Skip Removed lines (only exist on left side)
        if row.status == 2 {
            continue;
        }
        lines.push(row.right_text.to_string());
    }
    lines.join("\n") + "\n"
}

/// Rebuild right text, but for the current diff (first one with is_current_diff), use left text
fn rebuild_right_after_copy_from_left(vec_model: &VecModel<DiffLineData>) -> String {
    let mut lines = Vec::new();
    for i in 0..vec_model.row_count() {
        let row = vec_model.row_data(i).unwrap();
        if row.is_current_diff {
            // Copy from left to right
            match row.status {
                2 => {
                    // Removed: skip (don't add to right)
                    continue;
                }
                1 => {
                    // Added: replace with left (which is empty, so skip)
                    continue;
                }
                3 => {
                    // Modified: use left text
                    lines.push(row.left_text.to_string());
                }
                _ => {
                    lines.push(row.right_text.to_string());
                }
            }
        } else if row.status == 2 {
            // Removed line (not current diff): skip on right side
            continue;
        } else {
            lines.push(row.right_text.to_string());
        }
    }
    lines.join("\n") + "\n"
}

/// Rebuild left text, but for the current diff, use right text
fn rebuild_left_after_copy_from_right(vec_model: &VecModel<DiffLineData>) -> String {
    let mut lines = Vec::new();
    for i in 0..vec_model.row_count() {
        let row = vec_model.row_data(i).unwrap();
        if row.is_current_diff {
            match row.status {
                1 => {
                    // Added: copy to left
                    lines.push(row.right_text.to_string());
                }
                2 => {
                    // Removed: skip (remove from left too)
                    continue;
                }
                3 => {
                    // Modified: use right text
                    lines.push(row.right_text.to_string());
                }
                _ => {
                    lines.push(row.left_text.to_string());
                }
            }
        } else if row.status == 1 {
            // Added line (not current diff): skip on left side
            continue;
        } else {
            lines.push(row.left_text.to_string());
        }
    }
    lines.join("\n") + "\n"
}

pub fn save_file(window: &MainWindow, state: &mut AppState, save_left: bool) {
    let model = window.get_diff_lines();
    let vec_model = match model.as_any().downcast_ref::<VecModel<DiffLineData>>() {
        Some(m) => m,
        None => return,
    };

    let (text, path) = if save_left {
        (rebuild_left(vec_model), &state.left_path)
    } else {
        (rebuild_right(vec_model), &state.right_path)
    };

    if let Some(path) = path {
        if let Err(e) = fs::write(path, &text) {
            window.set_status_text(SharedString::from(format!("Error saving: {}", e)));
            return;
        }
        let side = if save_left { "Left" } else { "Right" };
        window.set_status_text(SharedString::from(format!(
            "{} file saved: {}",
            side,
            path.to_string_lossy()
        )));
    }
}
