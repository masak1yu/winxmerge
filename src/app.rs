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
}

impl AppState {
    pub fn new() -> Self {
        Self {
            left_path: None,
            right_path: None,
            diff_positions: Vec::new(),
            current_diff: -1,
        }
    }
}

pub fn open_file_dialog(title: &str) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title(title)
        .pick_file()
}

pub fn run_diff(window: &MainWindow, state: &mut AppState) {
    let (left_path, right_path) = match (&state.left_path, &state.right_path) {
        (Some(l), Some(r)) => (l, r),
        _ => return,
    };

    let left_text = fs::read_to_string(left_path).unwrap_or_default();
    let right_text = fs::read_to_string(right_path).unwrap_or_default();

    let result = compute_diff(&left_text, &right_text);

    state.diff_positions = result.diff_positions.clone();
    state.current_diff = -1;

    let diff_line_data: Vec<DiffLineData> = result
        .lines
        .iter()
        .map(|line| {
            let status: i32 = match line.status {
                LineStatus::Equal => 0,
                LineStatus::Added => 1,
                LineStatus::Removed => 2,
                LineStatus::Modified => 3,
            };
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
                is_current_diff: false,
            }
        })
        .collect();

    let model = ModelRc::new(VecModel::from(diff_line_data));
    window.set_diff_lines(model);
    window.set_diff_count(result.diff_count as i32);
    window.set_current_diff_index(-1);
    window.set_left_path(SharedString::from(
        left_path.to_string_lossy().to_string(),
    ));
    window.set_right_path(SharedString::from(
        right_path.to_string_lossy().to_string(),
    ));
    window.set_status_text(SharedString::from(format!(
        "{} differences found",
        result.diff_count
    )));
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

    state.current_diff = new_index;
    window.set_current_diff_index(new_index);

    // Update is_current_diff flags
    let model = window.get_diff_lines();
    if let Some(vec_model) = model.as_any().downcast_ref::<VecModel<DiffLineData>>() {
        let current_pos = state.diff_positions[new_index as usize];
        for i in 0..vec_model.row_count() {
            let mut row = vec_model.row_data(i).unwrap();
            row.is_current_diff = i == current_pos;
            vec_model.set_row_data(i, row);
        }
    }

    window.set_status_text(SharedString::from(format!(
        "Difference {} of {}",
        new_index + 1,
        state.diff_positions.len()
    )));
}
