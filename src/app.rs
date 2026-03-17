use std::fs;
use std::path::PathBuf;

use slint::{Model, ModelRc, SharedString, VecModel};

use crate::diff::engine::{compute_diff_with_options, DiffOptions};
use crate::diff::folder::compare_folders;
use crate::encoding::{decode_file, encode_text};
use crate::models::diff_line::LineStatus;
use crate::models::folder_item::FileCompareStatus;
use crate::{DiffLineData, FolderItemData, MainWindow};

pub struct AppState {
    pub left_path: Option<PathBuf>,
    pub right_path: Option<PathBuf>,
    pub diff_positions: Vec<usize>,
    pub current_diff: i32,
    pub left_lines: Vec<String>,
    pub right_lines: Vec<String>,
    pub has_unsaved_changes: bool,
    // Folder comparison
    pub left_folder: Option<PathBuf>,
    pub right_folder: Option<PathBuf>,
    pub folder_items: Vec<crate::models::folder_item::FolderItem>,
    // Encoding
    pub left_encoding: String,
    pub right_encoding: String,
    // Diff options
    pub diff_options: DiffOptions,
    // Search
    pub search_matches: Vec<usize>,
    pub current_search_match: i32,
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
            left_folder: None,
            right_folder: None,
            folder_items: Vec::new(),
            left_encoding: "UTF-8".to_string(),
            right_encoding: "UTF-8".to_string(),
            diff_options: DiffOptions::default(),
            search_matches: Vec::new(),
            current_search_match: -1,
        }
    }
}

pub fn open_file_dialog(title: &str) -> Option<PathBuf> {
    rfd::FileDialog::new().set_title(title).pick_file()
}

pub fn open_folder_dialog(title: &str) -> Option<PathBuf> {
    rfd::FileDialog::new().set_title(title).pick_folder()
}

pub fn run_diff(window: &MainWindow, state: &mut AppState) {
    let (left_path, right_path) = match (&state.left_path, &state.right_path) {
        (Some(l), Some(r)) => (l.clone(), r.clone()),
        _ => return,
    };

    let left_bytes = fs::read(&left_path).unwrap_or_default();
    let right_bytes = fs::read(&right_path).unwrap_or_default();

    let (left_text, left_enc) = decode_file(&left_bytes);
    let (right_text, right_enc) = decode_file(&right_bytes);

    state.left_encoding = left_enc.to_string();
    state.right_encoding = right_enc.to_string();

    recompute_diff_from_text(window, state, &left_text, &right_text);

    // Update encoding display in status
    let enc_info = format!(" [{}  |  {}]", state.left_encoding, state.right_encoding);
    let current = window.get_status_text().to_string();
    window.set_status_text(SharedString::from(current + &enc_info));
}

pub fn recompute_diff_from_text(
    window: &MainWindow,
    state: &mut AppState,
    left_text: &str,
    right_text: &str,
) {
    let result = compute_diff_with_options(left_text, right_text, &state.diff_options);

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
        .map(|(i, line)| {
            let status: i32 = match line.status {
                LineStatus::Equal => 0,
                LineStatus::Added => 1,
                LineStatus::Removed => 2,
                LineStatus::Modified => 3,
            };
            let diff_index = result
                .diff_positions
                .iter()
                .position(|&pos| pos == i)
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

pub fn copy_to_right(window: &MainWindow, state: &mut AppState, diff_index: i32) {
    if diff_index < 0 || diff_index as usize >= state.diff_positions.len() {
        return;
    }

    let model = window.get_diff_lines();
    let vec_model = match model.as_any().downcast_ref::<VecModel<DiffLineData>>() {
        Some(m) => m,
        None => return,
    };

    let right_text = rebuild_right_after_copy_from_left(vec_model);
    let left_text = rebuild_left(vec_model);

    state.has_unsaved_changes = true;
    window.set_has_unsaved_changes(true);

    recompute_diff_from_text(window, state, &left_text, &right_text);

    if !state.diff_positions.is_empty() {
        let new_idx = (diff_index as usize).min(state.diff_positions.len() - 1);
        update_current_diff(window, state, new_idx as i32);
    }
}

pub fn copy_to_left(window: &MainWindow, state: &mut AppState, diff_index: i32) {
    if diff_index < 0 || diff_index as usize >= state.diff_positions.len() {
        return;
    }

    let model = window.get_diff_lines();
    let vec_model = match model.as_any().downcast_ref::<VecModel<DiffLineData>>() {
        Some(m) => m,
        None => return,
    };

    let left_text = rebuild_left_after_copy_from_right(vec_model);
    let right_text = rebuild_right(vec_model);

    state.has_unsaved_changes = true;
    window.set_has_unsaved_changes(true);

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
        if row.status == 2 {
            continue;
        }
        lines.push(row.right_text.to_string());
    }
    lines.join("\n") + "\n"
}

fn rebuild_right_after_copy_from_left(vec_model: &VecModel<DiffLineData>) -> String {
    let mut lines = Vec::new();
    for i in 0..vec_model.row_count() {
        let row = vec_model.row_data(i).unwrap();
        if row.is_current_diff {
            match row.status {
                2 => continue,
                1 => continue,
                3 => lines.push(row.left_text.to_string()),
                _ => lines.push(row.right_text.to_string()),
            }
        } else if row.status == 2 {
            continue;
        } else {
            lines.push(row.right_text.to_string());
        }
    }
    lines.join("\n") + "\n"
}

fn rebuild_left_after_copy_from_right(vec_model: &VecModel<DiffLineData>) -> String {
    let mut lines = Vec::new();
    for i in 0..vec_model.row_count() {
        let row = vec_model.row_data(i).unwrap();
        if row.is_current_diff {
            match row.status {
                1 => lines.push(row.right_text.to_string()),
                2 => continue,
                3 => lines.push(row.right_text.to_string()),
                _ => lines.push(row.left_text.to_string()),
            }
        } else if row.status == 1 {
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

    let (text, path, encoding) = if save_left {
        (
            rebuild_left(vec_model),
            &state.left_path,
            &state.left_encoding,
        )
    } else {
        (
            rebuild_right(vec_model),
            &state.right_path,
            &state.right_encoding,
        )
    };

    if let Some(path) = path {
        let bytes = encode_text(&text, encoding);
        if let Err(e) = fs::write(path, &bytes) {
            window.set_status_text(SharedString::from(format!("Error saving: {}", e)));
            return;
        }
        let side = if save_left { "Left" } else { "Right" };
        window.set_status_text(SharedString::from(format!(
            "{} file saved: {} ({})",
            side,
            path.to_string_lossy(),
            encoding
        )));
    }
}

pub fn toggle_ignore_whitespace(window: &MainWindow, state: &mut AppState) {
    state.diff_options.ignore_whitespace = !state.diff_options.ignore_whitespace;
    window.set_ignore_whitespace(state.diff_options.ignore_whitespace);
    rerun_diff(window, state);
}

pub fn toggle_ignore_case(window: &MainWindow, state: &mut AppState) {
    state.diff_options.ignore_case = !state.diff_options.ignore_case;
    window.set_ignore_case(state.diff_options.ignore_case);
    rerun_diff(window, state);
}

fn rerun_diff(window: &MainWindow, state: &mut AppState) {
    if state.left_path.is_some() && state.right_path.is_some() {
        run_diff(window, state);
    }
}

pub fn search_text(window: &MainWindow, state: &mut AppState, query: &str) {
    state.search_matches.clear();
    state.current_search_match = -1;

    if query.is_empty() {
        window.set_search_match_count(0);
        window.set_status_text(SharedString::from("Search cleared"));
        return;
    }

    let query_lower = query.to_lowercase();
    let model = window.get_diff_lines();
    if let Some(vec_model) = model.as_any().downcast_ref::<VecModel<DiffLineData>>() {
        for i in 0..vec_model.row_count() {
            let row = vec_model.row_data(i).unwrap();
            if row.left_text.to_string().to_lowercase().contains(&query_lower)
                || row.right_text.to_string().to_lowercase().contains(&query_lower)
            {
                state.search_matches.push(i);
            }
        }
    }

    let count = state.search_matches.len();
    window.set_search_match_count(count as i32);

    if count > 0 {
        state.current_search_match = 0;
        window.set_status_text(SharedString::from(format!(
            "Found {} matches for \"{}\"",
            count, query
        )));
    } else {
        window.set_status_text(SharedString::from(format!(
            "No matches found for \"{}\"",
            query
        )));
    }
}

pub fn replace_text(window: &MainWindow, state: &mut AppState, search: &str, replacement: &str) {
    if search.is_empty() || state.search_matches.is_empty() || state.current_search_match < 0 {
        return;
    }

    let model = window.get_diff_lines();
    let vec_model = match model.as_any().downcast_ref::<VecModel<DiffLineData>>() {
        Some(m) => m,
        None => return,
    };

    let match_idx = state.search_matches[state.current_search_match as usize];
    let mut row = vec_model.row_data(match_idx).unwrap();

    let search_lower = search.to_lowercase();
    // Replace in both sides
    let left = row.left_text.to_string();
    let right = row.right_text.to_string();
    row.left_text = SharedString::from(case_insensitive_replace(&left, &search_lower, search, replacement));
    row.right_text = SharedString::from(case_insensitive_replace(&right, &search_lower, search, replacement));
    vec_model.set_row_data(match_idx, row);

    state.has_unsaved_changes = true;
    window.set_has_unsaved_changes(true);

    // Re-search to update matches
    search_text(window, state, search);
}

pub fn replace_all_text(window: &MainWindow, state: &mut AppState, search: &str, replacement: &str) {
    if search.is_empty() || state.search_matches.is_empty() {
        return;
    }

    let model = window.get_diff_lines();
    let vec_model = match model.as_any().downcast_ref::<VecModel<DiffLineData>>() {
        Some(m) => m,
        None => return,
    };

    let search_lower = search.to_lowercase();
    for &match_idx in &state.search_matches {
        let mut row = vec_model.row_data(match_idx).unwrap();
        let left = row.left_text.to_string();
        let right = row.right_text.to_string();
        row.left_text = SharedString::from(case_insensitive_replace(&left, &search_lower, search, replacement));
        row.right_text = SharedString::from(case_insensitive_replace(&right, &search_lower, search, replacement));
        vec_model.set_row_data(match_idx, row);
    }

    let count = state.search_matches.len();
    state.has_unsaved_changes = true;
    window.set_has_unsaved_changes(true);

    // Re-search (should find 0 matches now)
    search_text(window, state, search);
    window.set_status_text(SharedString::from(format!(
        "Replaced {} occurrences",
        count
    )));
}

fn case_insensitive_replace(text: &str, search_lower: &str, _search: &str, replacement: &str) -> String {
    let text_lower = text.to_lowercase();
    let mut result = String::new();
    let mut last = 0;
    for (idx, _) in text_lower.match_indices(search_lower) {
        result.push_str(&text[last..idx]);
        result.push_str(replacement);
        last = idx + search_lower.len();
    }
    result.push_str(&text[last..]);
    result
}

pub fn start_compare(window: &MainWindow, state: &mut AppState, left: &str, right: &str, is_folder: bool) {
    let left_path = PathBuf::from(left);
    let right_path = PathBuf::from(right);

    if is_folder {
        state.left_folder = Some(left_path);
        state.right_folder = Some(right_path);
        run_folder_compare(window, state);
    } else {
        state.left_path = Some(left_path);
        state.right_path = Some(right_path);
        window.set_view_mode(0);
        run_diff(window, state);
    }
}

pub fn discard_and_proceed(window: &MainWindow, state: &mut AppState) {
    state.has_unsaved_changes = false;
    window.set_has_unsaved_changes(false);

    let action = window.get_pending_action();
    window.set_pending_action(0);

    match action {
        1 => {
            // Open left file
            if let Some(path) = open_file_dialog("Select left file") {
                state.left_path = Some(path.clone());
                window.set_open_left_path_input(SharedString::from(path.to_string_lossy().to_string()));
                window.set_view_mode(0);
                run_diff(window, state);
            }
        }
        2 => {
            // Open right file
            if let Some(path) = open_file_dialog("Select right file") {
                state.right_path = Some(path.clone());
                window.set_open_right_path_input(SharedString::from(path.to_string_lossy().to_string()));
                window.set_view_mode(0);
                run_diff(window, state);
            }
        }
        3 => {
            // Open left folder
            if let Some(path) = open_folder_dialog("Select left folder") {
                state.left_folder = Some(path.clone());
                window.set_open_left_path_input(SharedString::from(path.to_string_lossy().to_string()));
                run_folder_compare(window, state);
            }
        }
        4 => {
            // Open right folder
            if let Some(path) = open_folder_dialog("Select right folder") {
                state.right_folder = Some(path.clone());
                window.set_open_right_path_input(SharedString::from(path.to_string_lossy().to_string()));
                run_folder_compare(window, state);
            }
        }
        5 => {
            // New compare (go to open dialog)
            window.set_view_mode(2);
        }
        _ => {}
    }
}

pub fn navigate_search(window: &MainWindow, state: &mut AppState, forward: bool) {
    if state.search_matches.is_empty() {
        return;
    }

    let new_index = if forward {
        if state.current_search_match < state.search_matches.len() as i32 - 1 {
            state.current_search_match + 1
        } else {
            0
        }
    } else if state.current_search_match > 0 {
        state.current_search_match - 1
    } else {
        state.search_matches.len() as i32 - 1
    };

    state.current_search_match = new_index;
    window.set_status_text(SharedString::from(format!(
        "Match {} of {}",
        new_index + 1,
        state.search_matches.len()
    )));
}

// --- Folder comparison ---

pub fn run_folder_compare(window: &MainWindow, state: &mut AppState) {
    let (left_folder, right_folder) = match (&state.left_folder, &state.right_folder) {
        (Some(l), Some(r)) => (l.clone(), r.clone()),
        _ => return,
    };

    let items = compare_folders(&left_folder, &right_folder);

    let folder_item_data: Vec<FolderItemData> = items
        .iter()
        .map(|item| {
            let status: i32 = match item.status {
                FileCompareStatus::Identical => 0,
                FileCompareStatus::Different => 1,
                FileCompareStatus::LeftOnly => 2,
                FileCompareStatus::RightOnly => 3,
            };
            FolderItemData {
                relative_path: SharedString::from(&item.relative_path),
                is_directory: item.is_directory,
                status,
                left_size: item
                    .left_size
                    .map(|s| SharedString::from(format_size(s)))
                    .unwrap_or_default(),
                right_size: item
                    .right_size
                    .map(|s| SharedString::from(format_size(s)))
                    .unwrap_or_default(),
            }
        })
        .collect();

    let different_count = items
        .iter()
        .filter(|i| i.status != FileCompareStatus::Identical)
        .count();

    state.folder_items = items;
    window.set_folder_items(ModelRc::new(VecModel::from(folder_item_data)));
    window.set_view_mode(1);
    window.set_status_text(SharedString::from(format!(
        "{} items, {} differences",
        state.folder_items.len(),
        different_count
    )));
}

pub fn open_folder_item(window: &MainWindow, state: &mut AppState, index: i32) {
    if index < 0 || index as usize >= state.folder_items.len() {
        return;
    }

    let item = &state.folder_items[index as usize];

    if item.is_directory {
        return;
    }

    if let (Some(left), Some(right)) = (&item.left_path, &item.right_path) {
        state.left_path = Some(left.clone());
        state.right_path = Some(right.clone());
        window.set_view_mode(0);
        run_diff(window, state);
    }
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
