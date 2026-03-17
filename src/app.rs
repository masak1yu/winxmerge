use std::fs;
use std::path::PathBuf;

use slint::{Model, ModelRc, SharedString, VecModel};

use crate::diff::engine::{compute_diff_with_options, DiffOptions};
use crate::diff::folder::compare_folders;
use crate::encoding::{decode_file, encode_text};
use crate::highlight::{detect_file_type, highlight_lines};
use crate::models::diff_line::LineStatus;
use crate::models::folder_item::FileCompareStatus;
use crate::{DiffLineData, FolderItemData, MainWindow, TabData};

/// Snapshot for undo/redo
#[derive(Clone)]
struct TextSnapshot {
    left_text: String,
    right_text: String,
}

/// Per-tab state
pub struct TabState {
    pub left_path: Option<PathBuf>,
    pub right_path: Option<PathBuf>,
    pub diff_positions: Vec<usize>,
    pub current_diff: i32,
    pub left_lines: Vec<String>,
    pub right_lines: Vec<String>,
    pub has_unsaved_changes: bool,
    // Undo/Redo
    undo_stack: Vec<TextSnapshot>,
    redo_stack: Vec<TextSnapshot>,
    pub left_folder: Option<PathBuf>,
    pub right_folder: Option<PathBuf>,
    pub folder_items: Vec<crate::models::folder_item::FolderItem>,
    pub left_encoding: String,
    pub right_encoding: String,
    pub diff_options: DiffOptions,
    pub search_matches: Vec<usize>,
    pub current_search_match: i32,
    /// 0=file diff, 1=folder compare, 2=open dialog
    pub view_mode: i32,
    pub diff_line_data: Vec<DiffLineData>,
    pub folder_item_data: Vec<FolderItemData>,
    pub title: String,
}

impl TabState {
    pub fn new() -> Self {
        Self {
            left_path: None,
            right_path: None,
            diff_positions: Vec::new(),
            current_diff: -1,
            left_lines: Vec::new(),
            right_lines: Vec::new(),
            has_unsaved_changes: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            left_folder: None,
            right_folder: None,
            folder_items: Vec::new(),
            left_encoding: "UTF-8".to_string(),
            right_encoding: "UTF-8".to_string(),
            diff_options: DiffOptions::default(),
            search_matches: Vec::new(),
            current_search_match: -1,
            view_mode: 2,
            diff_line_data: Vec::new(),
            folder_item_data: Vec::new(),
            title: "New".to_string(),
        }
    }
}

/// Application state (manages tabs)
pub struct AppState {
    pub tabs: Vec<TabState>,
    pub active_tab: usize,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            tabs: vec![TabState::new()],
            active_tab: 0,
        }
    }

    pub fn current_tab(&self) -> &TabState {
        &self.tabs[self.active_tab]
    }

    pub fn current_tab_mut(&mut self) -> &mut TabState {
        &mut self.tabs[self.active_tab]
    }
}

pub fn open_file_dialog(title: &str) -> Option<PathBuf> {
    rfd::FileDialog::new().set_title(title).pick_file()
}

pub fn open_folder_dialog(title: &str) -> Option<PathBuf> {
    rfd::FileDialog::new().set_title(title).pick_folder()
}

// --- Tab management ---

pub fn add_tab(window: &MainWindow, state: &mut AppState) {
    state.tabs.push(TabState::new());
    let new_idx = state.tabs.len() - 1;
    switch_tab(window, state, new_idx as i32);
    sync_tab_list(window, state);
}

pub fn close_tab(window: &MainWindow, state: &mut AppState, index: i32) {
    if index < 0 || index as usize >= state.tabs.len() || state.tabs.len() <= 1 {
        return;
    }
    let idx = index as usize;
    state.tabs.remove(idx);
    if state.active_tab >= state.tabs.len() {
        state.active_tab = state.tabs.len() - 1;
    } else if state.active_tab > idx {
        state.active_tab -= 1;
    }
    restore_tab(window, state);
    sync_tab_list(window, state);
}

pub fn switch_tab(window: &MainWindow, state: &mut AppState, index: i32) {
    if index < 0 || index as usize >= state.tabs.len() {
        return;
    }
    // Save current tab's diff data from window
    save_current_tab_from_window(window, state);
    state.active_tab = index as usize;
    restore_tab(window, state);
    sync_tab_list(window, state);
}

fn save_current_tab_from_window(window: &MainWindow, state: &mut AppState) {
    let tab = state.current_tab_mut();
    // Save diff line data from the model
    let model = window.get_diff_lines();
    if let Some(vec_model) = model.as_any().downcast_ref::<VecModel<DiffLineData>>() {
        tab.diff_line_data.clear();
        for i in 0..vec_model.row_count() {
            if let Some(row) = vec_model.row_data(i) {
                tab.diff_line_data.push(row);
            }
        }
    }
}

fn restore_tab(window: &MainWindow, state: &AppState) {
    let tab = state.current_tab();

    window.set_view_mode(tab.view_mode);
    window.set_active_tab_index(state.active_tab as i32);

    // Restore diff data
    let model = ModelRc::new(VecModel::from(tab.diff_line_data.clone()));
    window.set_diff_lines(model);
    window.set_diff_count(tab.diff_positions.len() as i32);
    window.set_current_diff_index(tab.current_diff);
    window.set_has_unsaved_changes(tab.has_unsaved_changes);
    window.set_ignore_whitespace(tab.diff_options.ignore_whitespace);
    window.set_ignore_case(tab.diff_options.ignore_case);

    window.set_left_path(SharedString::from(
        tab.left_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default(),
    ));
    window.set_right_path(SharedString::from(
        tab.right_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default(),
    ));

    // Restore folder data
    let folder_model = ModelRc::new(VecModel::from(tab.folder_item_data.clone()));
    window.set_folder_items(folder_model);

    if tab.view_mode == 2 {
        window.set_status_text(SharedString::from("Select files or folders to compare"));
    }
}

pub fn sync_tab_list(window: &MainWindow, state: &AppState) {
    let tab_data: Vec<TabData> = state
        .tabs
        .iter()
        .enumerate()
        .map(|(i, tab)| TabData {
            title: SharedString::from(&tab.title),
            is_active: i == state.active_tab,
            has_unsaved: tab.has_unsaved_changes,
        })
        .collect();
    window.set_tab_list(ModelRc::new(VecModel::from(tab_data)));
    window.set_active_tab_index(state.active_tab as i32);

    // Update window title
    let tab = state.current_tab();
    let title = if tab.title == "New" {
        "WinXMerge".to_string()
    } else {
        format!("{} - WinXMerge", tab.title)
    };
    window.set_window_title(SharedString::from(title));
}

// --- Diff operations (work on current tab) ---

pub fn run_diff(window: &MainWindow, state: &mut AppState) {
    let tab = state.current_tab();
    let (left_path, right_path) = match (&tab.left_path, &tab.right_path) {
        (Some(l), Some(r)) => (l.clone(), r.clone()),
        _ => return,
    };

    let left_bytes = fs::read(&left_path).unwrap_or_default();
    let right_bytes = fs::read(&right_path).unwrap_or_default();

    let (left_text, left_enc) = decode_file(&left_bytes);
    let (right_text, right_enc) = decode_file(&right_bytes);

    let tab = state.current_tab_mut();
    tab.left_encoding = left_enc.to_string();
    tab.right_encoding = right_enc.to_string();

    // Generate title from filenames
    let left_name = left_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let right_name = right_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    tab.title = format!("{} ↔ {}", left_name, right_name);

    // Compute syntax highlights
    let left_path_str = left_path.to_string_lossy().to_string();
    let right_path_str = right_path.to_string_lossy().to_string();
    let left_highlights = highlight_lines(&left_text, &left_path_str);
    let right_highlights = highlight_lines(&right_text, &right_path_str);

    recompute_diff_from_text_with_highlights(
        window, state, &left_text, &right_text, &left_highlights, &right_highlights,
    );

    let tab = state.current_tab();
    let left_type = detect_file_type(&left_path_str);
    let right_type = detect_file_type(&right_path_str);
    let enc_info = format!(
        " [{}  |  {}] ({} / {})",
        tab.left_encoding, tab.right_encoding, left_type, right_type
    );
    let current = window.get_status_text().to_string();
    window.set_status_text(SharedString::from(current + &enc_info));

    sync_tab_list(window, state);
}

pub fn recompute_diff_from_text(
    window: &MainWindow,
    state: &mut AppState,
    left_text: &str,
    right_text: &str,
) {
    let empty_hl: Vec<i32> = Vec::new();
    recompute_diff_from_text_with_highlights(window, state, left_text, right_text, &empty_hl, &empty_hl);
}

pub fn recompute_diff_from_text_with_highlights(
    window: &MainWindow,
    state: &mut AppState,
    left_text: &str,
    right_text: &str,
    left_highlights: &[i32],
    right_highlights: &[i32],
) {
    let tab = state.current_tab_mut();
    let result = compute_diff_with_options(left_text, right_text, &tab.diff_options);

    tab.left_lines = left_text.lines().map(String::from).collect();
    tab.right_lines = right_text.lines().map(String::from).collect();
    tab.diff_positions = result.diff_positions.clone();
    tab.current_diff = if result.diff_positions.is_empty() {
        -1
    } else {
        0
    };

    // Build position→index lookup for O(1) access
    let pos_to_idx: std::collections::HashMap<usize, i32> = result
        .diff_positions
        .iter()
        .enumerate()
        .map(|(idx, &pos)| (pos, idx as i32))
        .collect();

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
                LineStatus::Moved => 4,
            };
            let diff_index = pos_to_idx.get(&i).copied().unwrap_or(-1);

            // Map line numbers to highlight indices
            let left_hl = line.left_line_no
                .and_then(|n| left_highlights.get((n - 1) as usize).copied())
                .unwrap_or(-1);
            let right_hl = line.right_line_no
                .and_then(|n| right_highlights.get((n - 1) as usize).copied())
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
                left_highlight: left_hl,
                right_highlight: right_hl,
            }
        })
        .collect();

    tab.diff_line_data = diff_line_data.clone();

    let model = ModelRc::new(VecModel::from(diff_line_data));
    window.set_diff_lines(model);
    window.set_diff_count(result.diff_count as i32);
    window.set_current_diff_index(tab.current_diff);
    window.set_left_path(SharedString::from(
        tab.left_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default(),
    ));
    window.set_right_path(SharedString::from(
        tab.right_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default(),
    ));

    let status = if result.diff_count == 0 {
        "Files are identical".to_string()
    } else if tab.current_diff >= 0 {
        format!(
            "Difference 1 of {} ({} total)",
            result.diff_count, result.diff_count
        )
    } else {
        format!("{} differences found", result.diff_count)
    };
    window.set_status_text(SharedString::from(status));
}

pub fn select_diff(window: &MainWindow, state: &mut AppState, diff_index: i32) {
    let tab = state.current_tab();
    if diff_index < 0 || diff_index as usize >= tab.diff_positions.len() {
        return;
    }
    update_current_diff(window, state, diff_index);
}

pub fn navigate_diff(window: &MainWindow, state: &mut AppState, forward: bool) {
    let tab = state.current_tab();
    if tab.diff_positions.is_empty() {
        return;
    }

    let new_index = if forward {
        if tab.current_diff < tab.diff_positions.len() as i32 - 1 {
            tab.current_diff + 1
        } else {
            0
        }
    } else if tab.current_diff > 0 {
        tab.current_diff - 1
    } else {
        tab.diff_positions.len() as i32 - 1
    };

    update_current_diff(window, state, new_index);
}

fn update_current_diff(window: &MainWindow, state: &mut AppState, new_index: i32) {
    let tab = state.current_tab_mut();
    tab.current_diff = new_index;
    window.set_current_diff_index(new_index);

    let current_pos = tab.diff_positions[new_index as usize];
    let total = tab.diff_positions.len();

    let model = window.get_diff_lines();
    if let Some(vec_model) = model.as_any().downcast_ref::<VecModel<DiffLineData>>() {
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
        total
    )));
}

fn push_undo_snapshot(state: &mut AppState, vec_model: &VecModel<DiffLineData>) {
    let left_text = rebuild_left(vec_model);
    let right_text = rebuild_right(vec_model);
    let tab = state.current_tab_mut();
    tab.undo_stack.push(TextSnapshot { left_text, right_text });
    tab.redo_stack.clear();
}

pub fn undo(window: &MainWindow, state: &mut AppState) {
    let tab = state.current_tab_mut();
    if tab.undo_stack.is_empty() {
        return;
    }

    // Save current state to redo stack
    let current_left = tab.left_lines.join("\n") + "\n";
    let current_right = tab.right_lines.join("\n") + "\n";
    tab.redo_stack.push(TextSnapshot {
        left_text: current_left,
        right_text: current_right,
    });

    let snapshot = tab.undo_stack.pop().unwrap();
    recompute_diff_from_text(window, state, &snapshot.left_text, &snapshot.right_text);

    let tab = state.current_tab();
    window.set_can_undo(!tab.undo_stack.is_empty());
    window.set_can_redo(!tab.redo_stack.is_empty());
    window.set_status_text(SharedString::from("Undo"));
}

pub fn redo(window: &MainWindow, state: &mut AppState) {
    let tab = state.current_tab_mut();
    if tab.redo_stack.is_empty() {
        return;
    }

    // Save current state to undo stack
    let current_left = tab.left_lines.join("\n") + "\n";
    let current_right = tab.right_lines.join("\n") + "\n";
    tab.undo_stack.push(TextSnapshot {
        left_text: current_left,
        right_text: current_right,
    });

    let snapshot = tab.redo_stack.pop().unwrap();
    recompute_diff_from_text(window, state, &snapshot.left_text, &snapshot.right_text);

    let tab = state.current_tab();
    window.set_can_undo(!tab.undo_stack.is_empty());
    window.set_can_redo(!tab.redo_stack.is_empty());
    window.set_status_text(SharedString::from("Redo"));
}

pub fn copy_to_right(window: &MainWindow, state: &mut AppState, diff_index: i32) {
    let tab = state.current_tab();
    if diff_index < 0 || diff_index as usize >= tab.diff_positions.len() {
        return;
    }

    let model = window.get_diff_lines();
    let vec_model = match model.as_any().downcast_ref::<VecModel<DiffLineData>>() {
        Some(m) => m,
        None => return,
    };

    push_undo_snapshot(state, vec_model);

    let right_text = rebuild_right_after_copy_from_left(vec_model);
    let left_text = rebuild_left(vec_model);

    state.current_tab_mut().has_unsaved_changes = true;
    window.set_has_unsaved_changes(true);

    recompute_diff_from_text(window, state, &left_text, &right_text);

    let tab = state.current_tab();
    if !tab.diff_positions.is_empty() {
        let new_idx = (diff_index as usize).min(tab.diff_positions.len() - 1);
        update_current_diff(window, state, new_idx as i32);
    }
    window.set_can_undo(true);
    window.set_can_redo(false);
    sync_tab_list(window, state);
}

pub fn copy_to_left(window: &MainWindow, state: &mut AppState, diff_index: i32) {
    let tab = state.current_tab();
    if diff_index < 0 || diff_index as usize >= tab.diff_positions.len() {
        return;
    }

    let model = window.get_diff_lines();
    let vec_model = match model.as_any().downcast_ref::<VecModel<DiffLineData>>() {
        Some(m) => m,
        None => return,
    };

    push_undo_snapshot(state, vec_model);

    let left_text = rebuild_left_after_copy_from_right(vec_model);
    let right_text = rebuild_right(vec_model);

    state.current_tab_mut().has_unsaved_changes = true;
    window.set_has_unsaved_changes(true);

    recompute_diff_from_text(window, state, &left_text, &right_text);

    let tab = state.current_tab();
    if !tab.diff_positions.is_empty() {
        let new_idx = (diff_index as usize).min(tab.diff_positions.len() - 1);
        update_current_diff(window, state, new_idx as i32);
    }
    window.set_can_undo(true);
    window.set_can_redo(false);
    sync_tab_list(window, state);
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

pub fn copy_current_line_text(window: &MainWindow, state: &AppState, is_left: bool) {
    let tab = state.current_tab();
    if tab.current_diff < 0 || tab.current_diff as usize >= tab.diff_positions.len() {
        return;
    }

    let pos = tab.diff_positions[tab.current_diff as usize];
    let model = window.get_diff_lines();
    if let Some(vec_model) = model.as_any().downcast_ref::<VecModel<DiffLineData>>() {
        if let Some(row) = vec_model.row_data(pos) {
            let text = if is_left {
                row.left_text.to_string()
            } else {
                row.right_text.to_string()
            };

            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(&text);
                let side = if is_left { "Left" } else { "Right" };
                window.set_status_text(SharedString::from(format!(
                    "{} text copied to clipboard",
                    side
                )));
            }
        }
    }
}

pub fn export_html_report(window: &MainWindow, state: &AppState) {
    let tab = state.current_tab();

    // Rebuild DiffResult from current state
    let left_text = tab.left_lines.join("\n") + "\n";
    let right_text = tab.right_lines.join("\n") + "\n";
    let result = compute_diff_with_options(&left_text, &right_text, &tab.diff_options);

    let left_title = tab.left_path.as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "Left".to_string());
    let right_title = tab.right_path.as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "Right".to_string());

    let html = crate::export::export_html(&result, &left_title, &right_title);

    // Save dialog
    if let Some(path) = rfd::FileDialog::new()
        .set_title("Export HTML Report")
        .set_file_name("diff-report.html")
        .add_filter("HTML", &["html"])
        .save_file()
    {
        match fs::write(&path, &html) {
            Ok(_) => {
                window.set_status_text(SharedString::from(format!(
                    "Exported to {}",
                    path.to_string_lossy()
                )));
            }
            Err(e) => {
                window.set_status_text(SharedString::from(format!("Export error: {}", e)));
            }
        }
    }
}

pub fn save_file(window: &MainWindow, state: &mut AppState, save_left: bool) {
    let model = window.get_diff_lines();
    let vec_model = match model.as_any().downcast_ref::<VecModel<DiffLineData>>() {
        Some(m) => m,
        None => return,
    };

    let tab = state.current_tab();
    let (text, path, encoding) = if save_left {
        (
            rebuild_left(vec_model),
            tab.left_path.clone(),
            tab.left_encoding.clone(),
        )
    } else {
        (
            rebuild_right(vec_model),
            tab.right_path.clone(),
            tab.right_encoding.clone(),
        )
    };

    if let Some(path) = path {
        let bytes = encode_text(&text, &encoding);
        if let Err(e) = fs::write(&path, &bytes) {
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
    let tab = state.current_tab_mut();
    tab.diff_options.ignore_whitespace = !tab.diff_options.ignore_whitespace;
    window.set_ignore_whitespace(tab.diff_options.ignore_whitespace);
    rerun_diff(window, state);
}

pub fn toggle_ignore_case(window: &MainWindow, state: &mut AppState) {
    let tab = state.current_tab_mut();
    tab.diff_options.ignore_case = !tab.diff_options.ignore_case;
    window.set_ignore_case(tab.diff_options.ignore_case);
    rerun_diff(window, state);
}

fn rerun_diff(window: &MainWindow, state: &mut AppState) {
    let tab = state.current_tab();
    if tab.left_path.is_some() && tab.right_path.is_some() {
        run_diff(window, state);
    }
}

pub fn search_text(window: &MainWindow, state: &mut AppState, query: &str) {
    let tab = state.current_tab_mut();
    tab.search_matches.clear();
    tab.current_search_match = -1;

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
                tab.search_matches.push(i);
            }
        }
    }

    let count = tab.search_matches.len();
    window.set_search_match_count(count as i32);

    if count > 0 {
        tab.current_search_match = 0;
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
    let tab = state.current_tab();
    if search.is_empty() || tab.search_matches.is_empty() || tab.current_search_match < 0 {
        return;
    }

    let model = window.get_diff_lines();
    let vec_model = match model.as_any().downcast_ref::<VecModel<DiffLineData>>() {
        Some(m) => m,
        None => return,
    };

    let match_idx = tab.search_matches[tab.current_search_match as usize];
    let mut row = vec_model.row_data(match_idx).unwrap();

    let search_lower = search.to_lowercase();
    let left = row.left_text.to_string();
    let right = row.right_text.to_string();
    row.left_text = SharedString::from(case_insensitive_replace(&left, &search_lower, replacement));
    row.right_text = SharedString::from(case_insensitive_replace(&right, &search_lower, replacement));
    vec_model.set_row_data(match_idx, row);

    state.current_tab_mut().has_unsaved_changes = true;
    window.set_has_unsaved_changes(true);

    search_text(window, state, search);
}

pub fn replace_all_text(window: &MainWindow, state: &mut AppState, search: &str, replacement: &str) {
    let tab = state.current_tab();
    if search.is_empty() || tab.search_matches.is_empty() {
        return;
    }

    let model = window.get_diff_lines();
    let vec_model = match model.as_any().downcast_ref::<VecModel<DiffLineData>>() {
        Some(m) => m,
        None => return,
    };

    let search_lower = search.to_lowercase();
    let matches = tab.search_matches.clone();
    for &match_idx in &matches {
        let mut row = vec_model.row_data(match_idx).unwrap();
        let left = row.left_text.to_string();
        let right = row.right_text.to_string();
        row.left_text = SharedString::from(case_insensitive_replace(&left, &search_lower, replacement));
        row.right_text = SharedString::from(case_insensitive_replace(&right, &search_lower, replacement));
        vec_model.set_row_data(match_idx, row);
    }

    let count = matches.len();
    state.current_tab_mut().has_unsaved_changes = true;
    window.set_has_unsaved_changes(true);

    search_text(window, state, search);
    window.set_status_text(SharedString::from(format!(
        "Replaced {} occurrences",
        count
    )));
}

fn case_insensitive_replace(text: &str, search_lower: &str, replacement: &str) -> String {
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
        {
            let tab = state.current_tab_mut();
            tab.left_folder = Some(left_path);
            tab.right_folder = Some(right_path);
        }
        run_folder_compare(window, state);
    } else {
        {
            let tab = state.current_tab_mut();
            tab.left_path = Some(left_path);
            tab.right_path = Some(right_path);
            tab.view_mode = 0;
        }
        window.set_view_mode(0);
        run_diff(window, state);
    }
}

pub fn discard_and_proceed(window: &MainWindow, state: &mut AppState) {
    state.current_tab_mut().has_unsaved_changes = false;
    window.set_has_unsaved_changes(false);

    let action = window.get_pending_action();
    window.set_pending_action(0);

    match action {
        1 => {
            if let Some(path) = open_file_dialog("Select left file") {
                {
                    let tab = state.current_tab_mut();
                    tab.left_path = Some(path.clone());
                    tab.view_mode = 0;
                }
                window.set_open_left_path_input(SharedString::from(path.to_string_lossy().to_string()));
                window.set_view_mode(0);
                run_diff(window, state);
            }
        }
        2 => {
            if let Some(path) = open_file_dialog("Select right file") {
                {
                    let tab = state.current_tab_mut();
                    tab.right_path = Some(path.clone());
                    tab.view_mode = 0;
                }
                window.set_open_right_path_input(SharedString::from(path.to_string_lossy().to_string()));
                window.set_view_mode(0);
                run_diff(window, state);
            }
        }
        3 => {
            if let Some(path) = open_folder_dialog("Select left folder") {
                state.current_tab_mut().left_folder = Some(path.clone());
                window.set_open_left_path_input(SharedString::from(path.to_string_lossy().to_string()));
                run_folder_compare(window, state);
            }
        }
        4 => {
            if let Some(path) = open_folder_dialog("Select right folder") {
                state.current_tab_mut().right_folder = Some(path.clone());
                window.set_open_right_path_input(SharedString::from(path.to_string_lossy().to_string()));
                run_folder_compare(window, state);
            }
        }
        5 => {
            // New compare (go to open dialog)
            state.current_tab_mut().view_mode = 2;
            window.set_view_mode(2);
        }
        _ => {}
    }
}

pub fn navigate_search(window: &MainWindow, state: &mut AppState, forward: bool) {
    let tab = state.current_tab_mut();
    if tab.search_matches.is_empty() {
        return;
    }

    let new_index = if forward {
        if tab.current_search_match < tab.search_matches.len() as i32 - 1 {
            tab.current_search_match + 1
        } else {
            0
        }
    } else if tab.current_search_match > 0 {
        tab.current_search_match - 1
    } else {
        tab.search_matches.len() as i32 - 1
    };

    tab.current_search_match = new_index;
    let total = tab.search_matches.len();
    window.set_status_text(SharedString::from(format!(
        "Match {} of {}",
        new_index + 1,
        total
    )));
}

// --- Folder comparison ---

pub fn run_folder_compare(window: &MainWindow, state: &mut AppState) {
    let tab = state.current_tab();
    let (left_folder, right_folder) = match (&tab.left_folder, &tab.right_folder) {
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

    let left_name = left_folder.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let right_name = right_folder.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();

    let tab = state.current_tab_mut();
    tab.folder_items = items;
    tab.folder_item_data = folder_item_data.clone();
    tab.view_mode = 1;
    tab.title = format!("{} ↔ {}", left_name, right_name);

    window.set_folder_items(ModelRc::new(VecModel::from(folder_item_data)));
    window.set_view_mode(1);
    window.set_status_text(SharedString::from(format!(
        "{} items, {} differences",
        tab.folder_items.len(),
        different_count
    )));

    sync_tab_list(window, state);
}

pub fn open_folder_item(window: &MainWindow, state: &mut AppState, index: i32) {
    let tab = state.current_tab();
    if index < 0 || index as usize >= tab.folder_items.len() {
        return;
    }

    let item = &tab.folder_items[index as usize];
    if item.is_directory {
        return;
    }

    if let (Some(left), Some(right)) = (&item.left_path, &item.right_path) {
        let left = left.clone();
        let right = right.clone();
        {
            let tab = state.current_tab_mut();
            tab.left_path = Some(left);
            tab.right_path = Some(right);
            tab.view_mode = 0;
        }
        window.set_view_mode(0);
        window.set_has_folder_context(true);
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
