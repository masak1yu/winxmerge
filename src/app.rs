use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};

use crate::archive::{compare_zip_archives, is_zip_bytes, is_zip_path};
use crate::csv::{compare_csv, is_csv_path};
use crate::diff::engine::{DiffOptions, compute_diff_with_options};
use crate::diff::folder::{FolderCompareOptions, compare_folders_with_options};
use crate::diff::three_way::{ThreeWayStatus, compute_three_way_diff};
use crate::encoding::{decode_file, detect_eol, encode_text, is_binary};
use crate::excel::{compare_excel, is_excel_path};
use crate::highlight::{detect_file_type, highlight_lines};
use crate::image_compare::{compare_images, is_image_path};
use crate::models::diff_line::DiffResult;
use crate::models::diff_line::LineStatus;
use crate::models::folder_item::FileCompareStatus;
use crate::settings::AppSettings;
use crate::{
    DetailLineData, DiffLineData, ExcelCellData, FolderItemData, MainWindow, PluginEntryData,
    TabData, ThreeWayLineData, WordSegment,
};

/// Line count threshold above which diff is computed on a background thread
const ASYNC_DIFF_THRESHOLD: usize = 30_000;

/// Result produced by the background diff thread, applied on the main thread
pub struct PendingDiffResult {
    pub tab_index: usize,
    pub diff_result: DiffResult,
    pub left_text: String,
    pub right_text: String,
    pub left_highlights: Vec<i32>,
    pub right_highlights: Vec<i32>,
    pub tab_width: usize,
    pub left_path: Option<PathBuf>,
    pub right_path: Option<PathBuf>,
    pub left_encoding: String,
    pub right_encoding: String,
}

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
    pub base_path: Option<PathBuf>,
    pub three_way_conflict_positions: Vec<usize>,
    pub current_conflict: i32,
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
    pub left_eol_type: String,
    pub right_eol_type: String,
    pub diff_options: DiffOptions,
    pub search_matches: Vec<usize>,
    pub current_search_match: i32,
    /// 0=file diff, 1=folder compare, 2=open dialog
    pub view_mode: i32,
    pub diff_line_data: Vec<DiffLineData>,
    pub folder_item_data: Vec<FolderItemData>,
    pub title: String,
    pub bookmarks: Vec<usize>,
    pub current_bookmark: i32,
    /// File modification times for auto-rescan
    pub left_mtime: Option<SystemTime>,
    pub right_mtime: Option<SystemTime>,
    /// Pre-computed diff stats string "+A -R ~M"
    pub diff_stats: String,
    /// True while a background diff computation is in progress for this tab
    pub is_computing: bool,
    /// Multi-line selection start/end row indices (-1 = no selection)
    pub selection_start: i32,
    pub selection_end: i32,
    /// Folder comparison summary text
    pub folder_summary: String,
    /// Excel comparison cell diffs
    pub excel_cells: Vec<ExcelCellData>,
    /// Excel sheet names for the selector
    pub excel_sheet_names: Vec<String>,
    /// Image comparison cached images and stats (view_mode == 5)
    pub left_image: Option<slint::Image>,
    pub right_image: Option<slint::Image>,
    pub diff_image: Option<slint::Image>,
    pub overlay_image: Option<slint::Image>,
    pub image_stats: String,
    /// Image dimensions for zoom support
    pub image_left_w: i32,
    pub image_left_h: i32,
    pub image_right_w: i32,
    pub image_right_h: i32,
    /// Diff block comments: diff_index -> comment string
    pub diff_comments: HashMap<usize, String>,
    /// Status filter for diff view (0=All, 1=Added, 2=Removed, 3=Modified, 4=Moved)
    pub diff_status_filter: i32,
    /// Folder sort column: -1=none, 0=Name, 1=Status, 2=LeftSize, 3=RightSize, 4=LeftModified, 5=RightModified
    pub folder_sort_column: i32,
    pub folder_sort_ascending: bool,
}

impl TabState {
    pub fn new() -> Self {
        Self {
            left_path: None,
            right_path: None,
            base_path: None,
            three_way_conflict_positions: Vec::new(),
            current_conflict: -1,
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
            left_eol_type: String::new(),
            right_eol_type: String::new(),
            diff_options: DiffOptions::default(),
            search_matches: Vec::new(),
            current_search_match: -1,
            view_mode: 2,
            diff_line_data: Vec::new(),
            folder_item_data: Vec::new(),
            title: "New".to_string(),
            bookmarks: Vec::new(),
            current_bookmark: -1,
            left_mtime: None,
            right_mtime: None,
            diff_stats: String::new(),
            is_computing: false,
            selection_start: -1,
            selection_end: -1,
            folder_summary: String::new(),
            excel_cells: Vec::new(),
            excel_sheet_names: Vec::new(),
            left_image: None,
            right_image: None,
            diff_image: None,
            overlay_image: None,
            image_stats: String::new(),
            image_left_w: 0,
            image_left_h: 0,
            image_right_w: 0,
            image_right_h: 0,
            diff_comments: HashMap::new(),
            diff_status_filter: 0,
            folder_sort_column: -1,
            folder_sort_ascending: true,
        }
    }
}

/// Application state (manages tabs)
pub struct AppState {
    pub tabs: Vec<TabState>,
    pub active_tab: usize,
    pub folder_exclude_patterns: Vec<String>,
    /// Shared slot for background-computed diff results
    pub pending_diff: Arc<Mutex<Option<PendingDiffResult>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            tabs: vec![TabState::new()],
            active_tab: 0,
            folder_exclude_patterns: Vec::new(),
            pending_diff: Arc::new(Mutex::new(None)),
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
    // Save per-tab diff options from window state
    tab.diff_options.ignore_whitespace = window.get_ignore_whitespace();
    tab.diff_options.ignore_case = window.get_ignore_case();
    tab.diff_options.ignore_blank_lines = window.get_opt_ignore_blank_lines();
    tab.diff_options.ignore_eol = window.get_opt_ignore_eol();
    tab.diff_options.detect_moved_lines = window.get_opt_detect_moved_lines();
    let lf = window.get_opt_line_filters().to_string();
    tab.diff_options.line_filters = lf
        .split('|')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let sub_pat = window.get_opt_substitution_patterns().to_string();
    let sub_rep = window.get_opt_substitution_replacements().to_string();
    let pats: Vec<&str> = sub_pat.split('|').collect();
    let reps: Vec<&str> = sub_rep.split('|').collect();
    tab.diff_options.substitution_filters = pats
        .iter()
        .zip(reps.iter())
        .filter(|(p, _)| !p.trim().is_empty())
        .map(|(p, r)| (p.trim().to_string(), r.trim().to_string()))
        .collect();
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

    // Restore per-tab diff options
    window.set_opt_ignore_blank_lines(tab.diff_options.ignore_blank_lines);
    window.set_opt_ignore_eol(tab.diff_options.ignore_eol);
    window.set_opt_detect_moved_lines(tab.diff_options.detect_moved_lines);
    let filters_str = tab.diff_options.line_filters.join("|");
    window.set_opt_line_filters(SharedString::from(filters_str));
    let sub_pats: Vec<&str> = tab
        .diff_options
        .substitution_filters
        .iter()
        .map(|(p, _)| p.as_str())
        .collect();
    let sub_reps: Vec<&str> = tab
        .diff_options
        .substitution_filters
        .iter()
        .map(|(_, r)| r.as_str())
        .collect();
    window.set_opt_substitution_patterns(SharedString::from(sub_pats.join("|")));
    window.set_opt_substitution_replacements(SharedString::from(sub_reps.join("|")));

    sync_diff_stats(window, &tab.diff_stats.clone());
    window.set_left_encoding_display(SharedString::from(&tab.left_encoding));
    window.set_right_encoding_display(SharedString::from(&tab.right_encoding));
    window.set_left_eol_type(SharedString::from(&tab.left_eol_type));
    window.set_right_eol_type(SharedString::from(&tab.right_eol_type));

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

    // Update detail pane
    let model = window.get_diff_lines();
    update_detail_pane(window, &model, tab.current_diff, tab);

    // Restore folder data
    let folder_model = ModelRc::new(VecModel::from(tab.folder_item_data.clone()));
    window.set_folder_items(folder_model);

    if tab.view_mode == 1 {
        window.set_folder_summary_text(SharedString::from(&tab.folder_summary));
    }

    if tab.view_mode == 5 {
        if let Some(img) = tab.left_image.clone() {
            window.set_left_image(img);
        }
        if let Some(img) = tab.right_image.clone() {
            window.set_right_image(img);
        }
        if let Some(img) = tab.diff_image.clone() {
            window.set_diff_image(img);
        }
        window.set_image_stats(SharedString::from(&tab.image_stats));
        window.set_image_left_width(tab.image_left_w);
        window.set_image_left_height(tab.image_left_h);
        window.set_image_right_width(tab.image_right_w);
        window.set_image_right_height(tab.image_right_h);
    }

    if tab.view_mode == 4 {
        let sheet_model: ModelRc<SharedString> = ModelRc::new(VecModel::from(
            std::iter::once(SharedString::from(""))
                .chain(
                    tab.excel_sheet_names
                        .iter()
                        .map(|s| SharedString::from(s.as_str())),
                )
                .collect::<Vec<_>>(),
        ));
        window.set_excel_cells(ModelRc::new(VecModel::from(tab.excel_cells.clone())));
        window.set_excel_sheet_names(sheet_model);
    }

    if tab.view_mode == 6 {
        let sheet_model: ModelRc<SharedString> =
            ModelRc::new(VecModel::from(vec![SharedString::from("")]));
        window.set_excel_cells(ModelRc::new(VecModel::from(tab.excel_cells.clone())));
        window.set_excel_sheet_names(sheet_model);
        window.set_excel_active_sheet(SharedString::from(""));
    }

    if tab.view_mode == 2 {
        window.set_status_text(SharedString::from("Select files or folders to compare"));
        // Clear path inputs for a fresh open dialog state
        window.set_open_left_path_input(SharedString::from(
            tab.left_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default(),
        ));
        window.set_open_right_path_input(SharedString::from(
            tab.right_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default(),
        ));
    }

    // Restore diff comment for current diff block
    let comment = tab
        .diff_comments
        .get(&(tab.current_diff.max(0) as usize))
        .cloned()
        .unwrap_or_default();
    window.set_current_diff_comment(SharedString::from(comment));

    // Restore status filter
    window.set_diff_status_filter(tab.diff_status_filter);
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

    // ZIP archive comparison
    if (is_zip_bytes(&left_bytes) || is_zip_path(&left_path))
        && (is_zip_bytes(&right_bytes) || is_zip_path(&right_path))
    {
        run_zip_compare(
            window,
            state,
            &left_bytes,
            &right_bytes,
            &left_path,
            &right_path,
        );
        return;
    }

    // Excel comparison
    if is_excel_path(&left_path) && is_excel_path(&right_path) {
        run_excel_compare(
            window,
            state,
            &left_bytes,
            &right_bytes,
            &left_path,
            &right_path,
        );
        return;
    }

    // CSV/TSV comparison
    if is_csv_path(&left_path) && is_csv_path(&right_path) {
        run_csv_compare(
            window,
            state,
            &left_bytes,
            &right_bytes,
            &left_path,
            &right_path,
        );
        return;
    }

    // Image comparison (must be before binary detection; image files contain non-text bytes)
    if is_image_path(&left_path) && is_image_path(&right_path) {
        run_image_compare(
            window,
            state,
            &left_bytes,
            &right_bytes,
            &left_path,
            &right_path,
        );
        return;
    }

    // Binary file detection
    if is_binary(&left_bytes) || is_binary(&right_bytes) {
        let msg = format!(
            "Binary files: Left {} bytes, Right {} bytes — {}",
            left_bytes.len(),
            right_bytes.len(),
            if left_bytes == right_bytes {
                "identical"
            } else {
                "different"
            }
        );
        window.set_diff_lines(ModelRc::new(VecModel::from(Vec::<DiffLineData>::new())));
        window.set_diff_count(0);
        window.set_current_diff_index(-1);
        window.set_status_text(SharedString::from(msg));
        sync_tab_list(window, state);
        return;
    }

    let (left_text, left_enc) = decode_file(&left_bytes);
    let (right_text, right_enc) = decode_file(&right_bytes);

    let tab = state.current_tab_mut();
    tab.left_encoding = left_enc.to_string();
    tab.right_encoding = right_enc.to_string();
    tab.left_eol_type = detect_eol(&left_bytes).to_string();
    tab.right_eol_type = detect_eol(&right_bytes).to_string();
    // Store modification times for auto-rescan
    tab.left_mtime = fs::metadata(&left_path)
        .ok()
        .and_then(|m| m.modified().ok());
    tab.right_mtime = fs::metadata(&right_path)
        .ok()
        .and_then(|m| m.modified().ok());

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

    // Compute syntax highlights (if enabled)
    let left_path_str = left_path.to_string_lossy().to_string();
    let right_path_str = right_path.to_string_lossy().to_string();
    let syntax_enabled = window.get_opt_syntax_highlighting();
    let left_highlights = if syntax_enabled {
        highlight_lines(&left_text, &left_path_str)
    } else {
        Vec::new()
    };
    let right_highlights = if syntax_enabled {
        highlight_lines(&right_text, &right_path_str)
    } else {
        Vec::new()
    };

    recompute_diff_from_text_with_highlights(
        window,
        state,
        &left_text,
        &right_text,
        &left_highlights,
        &right_highlights,
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
    window.set_left_encoding_display(SharedString::from(&tab.left_encoding));
    window.set_right_encoding_display(SharedString::from(&tab.right_encoding));
    window.set_left_eol_type(SharedString::from(&tab.left_eol_type));
    window.set_right_eol_type(SharedString::from(&tab.right_eol_type));

    sync_tab_list(window, state);
}

pub fn recompute_diff_from_text(
    window: &MainWindow,
    state: &mut AppState,
    left_text: &str,
    right_text: &str,
) {
    let empty_hl: Vec<i32> = Vec::new();
    recompute_diff_from_text_with_highlights(
        window, state, left_text, right_text, &empty_hl, &empty_hl,
    );
}

pub fn recompute_diff_from_text_with_highlights(
    window: &MainWindow,
    state: &mut AppState,
    left_text: &str,
    right_text: &str,
    left_highlights: &[i32],
    right_highlights: &[i32],
) {
    let tab_width = window.get_opt_tab_width().max(1) as usize;
    let line_count = left_text.lines().count().max(right_text.lines().count());

    // For large files, offload to a background thread to avoid freezing the UI
    if line_count > ASYNC_DIFF_THRESHOLD {
        let tab = state.current_tab_mut();
        tab.is_computing = true;
        let options = tab.diff_options.clone();
        let left_text_owned = left_text.to_string();
        let right_text_owned = right_text.to_string();
        let left_highlights_owned = left_highlights.to_vec();
        let right_highlights_owned = right_highlights.to_vec();
        let left_path = tab.left_path.clone();
        let right_path = tab.right_path.clone();
        let left_encoding = tab.left_encoding.clone();
        let right_encoding = tab.right_encoding.clone();
        let tab_index = state.active_tab;
        let pending_slot = state.pending_diff.clone();

        std::thread::spawn(move || {
            let diff_result =
                compute_diff_with_options(&left_text_owned, &right_text_owned, &options);
            let mut slot = pending_slot.lock().unwrap();
            *slot = Some(PendingDiffResult {
                tab_index,
                diff_result,
                left_text: left_text_owned,
                right_text: right_text_owned,
                left_highlights: left_highlights_owned,
                right_highlights: right_highlights_owned,
                tab_width,
                left_path,
                right_path,
                left_encoding,
                right_encoding,
            });
        });

        window.set_status_text(SharedString::from(format!(
            "Computing diff... ({} lines)",
            line_count
        )));
        window.set_diff_lines(ModelRc::new(VecModel::from(Vec::<DiffLineData>::new())));
        window.set_diff_count(0);
        window.set_current_diff_index(-1);
        return;
    }

    let tab = state.current_tab_mut();
    let result = compute_diff_with_options(left_text, right_text, &tab.diff_options);
    apply_diff_result(
        window,
        state,
        result,
        left_text,
        right_text,
        left_highlights,
        right_highlights,
        tab_width,
    );
}

/// Apply a computed `DiffResult` to the window and current tab state.
fn apply_diff_result(
    window: &MainWindow,
    state: &mut AppState,
    result: DiffResult,
    left_text: &str,
    right_text: &str,
    left_highlights: &[i32],
    right_highlights: &[i32],
    tab_width: usize,
) {
    let tab = state.current_tab_mut();
    tab.is_computing = false;

    tab.left_lines = left_text.lines().map(String::from).collect();
    tab.right_lines = right_text.lines().map(String::from).collect();
    tab.diff_positions = result.diff_positions.clone();
    tab.current_diff = if result.diff_positions.is_empty() {
        -1
    } else {
        0
    };

    // Build per-line diff block index: all lines in the same contiguous block share one index
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
            let diff_index = line_block_idx[i];

            // Map line numbers to highlight indices
            let left_hl = line
                .left_line_no
                .and_then(|n| left_highlights.get((n - 1) as usize).copied())
                .unwrap_or(-1);
            let right_hl = line
                .right_line_no
                .and_then(|n| right_highlights.get((n - 1) as usize).copied())
                .unwrap_or(-1);

            // Build word-diff segment strings for the detail pane.
            // Format: \x01-separated list of ALL segments (changed and unchanged),
            // where even indices (0,2,4,...) = unchanged, odd indices (1,3,5,...) = changed.
            // If the first segment is changed, an empty unchanged prefix is prepended.
            let left_word_diff = build_word_diff_string(&line.left_word_segments);
            let right_word_diff = build_word_diff_string(&line.right_word_segments);

            DiffLineData {
                left_line_no: line
                    .left_line_no
                    .map(|n| SharedString::from(n.to_string()))
                    .unwrap_or_default(),
                right_line_no: line
                    .right_line_no
                    .map(|n| SharedString::from(n.to_string()))
                    .unwrap_or_default(),
                left_text: SharedString::from(expand_tabs(&line.left_text, tab_width)),
                right_text: SharedString::from(expand_tabs(&line.right_text, tab_width)),
                status,
                is_current_diff: false,
                diff_index,
                left_highlight: left_hl,
                right_highlight: right_hl,
                left_word_diff: SharedString::from(left_word_diff),
                right_word_diff: SharedString::from(right_word_diff),
                is_search_match: false,
                is_selected: false,
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

    // Compute diff statistics once and cache them
    let mut added = 0u32;
    let mut removed = 0u32;
    let mut modified = 0u32;
    for line in &result.lines {
        match line.status {
            LineStatus::Added => added += 1,
            LineStatus::Removed => removed += 1,
            LineStatus::Modified => modified += 1,
            _ => {}
        }
    }
    let tab = state.current_tab_mut();
    tab.diff_stats = format!("+{} -{} ~{}", added, removed, modified);
    let stats = tab.diff_stats.clone();
    sync_diff_stats(window, &stats);

    let status = if result.diff_count == 0 {
        "Files are identical".to_string()
    } else if tab.current_diff >= 0 {
        format!("Difference 1 of {} [{}]", result.diff_count, stats)
    } else {
        format!("{} differences found [{}]", result.diff_count, stats)
    };
    window.set_status_text(SharedString::from(status));

    // Update detail pane
    let model = window.get_diff_lines();
    let tab = state.current_tab();
    update_detail_pane(window, &model, tab.current_diff, tab);
}

/// Poll the shared result slot and apply if a background diff has finished.
/// Call this from a periodic timer on the main thread.
pub fn apply_pending_diff_if_ready(window: &MainWindow, state: &mut AppState) {
    let pending = {
        let mut slot = state.pending_diff.lock().unwrap();
        slot.take()
    };
    let Some(p) = pending else { return };

    // Only apply if it matches the currently active tab
    if p.tab_index != state.active_tab {
        return;
    }

    // Update tab paths/encoding from the pending result (already set when spawning, but keep in sync)
    {
        let tab = state.current_tab_mut();
        tab.left_path = p.left_path;
        tab.right_path = p.right_path;
        tab.left_encoding = p.left_encoding;
        tab.right_encoding = p.right_encoding;
    }

    apply_diff_result(
        window,
        state,
        p.diff_result,
        &p.left_text,
        &p.right_text,
        &p.left_highlights,
        &p.right_highlights,
        p.tab_width,
    );
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

/// Navigate to the next/prev diff of a specific status type.
/// status_filter: 1=Added, 2=Removed, 3=Modified, 4=Moved, 0=all (same as navigate_diff)
pub fn navigate_diff_by_status(
    window: &MainWindow,
    state: &mut AppState,
    forward: bool,
    status_filter: i32,
) {
    if status_filter == 0 {
        navigate_diff(window, state, forward);
        return;
    }
    let tab = state.current_tab();
    if tab.diff_positions.is_empty() {
        return;
    }
    let n = tab.diff_positions.len() as i32;
    let start = tab.current_diff;
    let candidates: Vec<i32> = if forward {
        let mut v: Vec<i32> = ((start + 1)..n).collect();
        v.extend(0..=start);
        v
    } else {
        let mut v: Vec<i32> = (0..start).rev().collect();
        v.extend((start..n).rev());
        v
    };
    let diff_line_data = tab.diff_line_data.clone();
    let diff_positions = tab.diff_positions.clone();
    for idx in candidates {
        let pos = diff_positions[idx as usize];
        if let Some(line) = diff_line_data.get(pos) {
            if line.status == status_filter {
                update_current_diff(window, state, idx);
                return;
            }
        }
    }
    let label = match status_filter {
        1 => "Added",
        2 => "Removed",
        3 => "Modified",
        4 => "Moved",
        _ => "status",
    };
    window.set_status_text(SharedString::from(format!("No more {} diffs", label)));
}

pub fn first_diff(window: &MainWindow, state: &mut AppState) {
    let tab = state.current_tab();
    if tab.diff_positions.is_empty() {
        return;
    }
    update_current_diff(window, state, 0);
}

pub fn last_diff(window: &MainWindow, state: &mut AppState) {
    let tab = state.current_tab();
    if tab.diff_positions.is_empty() {
        return;
    }
    let last = tab.diff_positions.len() as i32 - 1;
    update_current_diff(window, state, last);
}

pub fn goto_line(window: &MainWindow, state: &AppState, line_number: i32) {
    if line_number <= 0 {
        return;
    }
    let tab = state.current_tab();
    // Find the diff line index that corresponds to this line number
    for (idx, data) in tab.diff_line_data.iter().enumerate() {
        let left_no: i32 = data.left_line_no.parse().unwrap_or(0);
        let right_no: i32 = data.right_line_no.parse().unwrap_or(0);
        if left_no == line_number || right_no == line_number {
            // Scroll to this row
            window.invoke_scroll_diff_to_row(idx as i32);
            window.set_status_text(SharedString::from(format!("Line {}", line_number)));
            // If this line is part of a diff, select it
            if data.diff_index >= 0 {
                let model = window.get_diff_lines();
                if let Some(vec_model) = model.as_any().downcast_ref::<VecModel<DiffLineData>>() {
                    for i in 0..vec_model.row_count() {
                        let mut row = vec_model.row_data(i).unwrap();
                        let should_highlight = i == idx;
                        if row.is_current_diff != should_highlight {
                            row.is_current_diff = should_highlight;
                            vec_model.set_row_data(i, row);
                        }
                    }
                }
            }
            return;
        }
    }
    window.set_status_text(SharedString::from(format!(
        "Line {} not found",
        line_number
    )));
}

pub fn toggle_bookmark(state: &mut AppState, line_index: i32) {
    let tab = state.current_tab_mut();
    let idx = if line_index >= 0 {
        // Use the diff position for this diff index
        if let Some(&pos) = tab.diff_positions.get(line_index as usize) {
            pos
        } else {
            return;
        }
    } else {
        return;
    };
    if let Some(pos) = tab.bookmarks.iter().position(|&b| b == idx) {
        tab.bookmarks.remove(pos);
    } else {
        tab.bookmarks.push(idx);
        tab.bookmarks.sort();
    }
}

pub fn navigate_bookmark(window: &MainWindow, state: &mut AppState, forward: bool) {
    let (new_index, bookmark_pos, diff_idx_opt, total) = {
        let tab = state.current_tab_mut();
        if tab.bookmarks.is_empty() {
            window.set_status_text(SharedString::from("No bookmarks"));
            return;
        }

        let new_index = if forward {
            if tab.current_bookmark < tab.bookmarks.len() as i32 - 1 {
                tab.current_bookmark + 1
            } else {
                0
            }
        } else if tab.current_bookmark > 0 {
            tab.current_bookmark - 1
        } else {
            tab.bookmarks.len() as i32 - 1
        };

        tab.current_bookmark = new_index;
        let bookmark_pos = tab.bookmarks[new_index as usize];
        let diff_idx_opt = tab.diff_positions.iter().position(|&p| p == bookmark_pos);
        let total = tab.bookmarks.len();
        (new_index, bookmark_pos, diff_idx_opt, total)
    };

    if let Some(diff_idx) = diff_idx_opt {
        update_current_diff(window, state, diff_idx as i32);
    } else {
        window.set_status_text(SharedString::from(format!(
            "Bookmark {} of {} (line {})",
            new_index + 1,
            total,
            bookmark_pos
        )));
    }
}

fn update_current_diff(window: &MainWindow, state: &mut AppState, new_index: i32) {
    let tab = state.current_tab_mut();
    tab.current_diff = new_index;
    let current_pos = tab.diff_positions[new_index as usize];
    let total = tab.diff_positions.len();
    let stats = tab.diff_stats.clone();
    let comment = tab
        .diff_comments
        .get(&(new_index as usize))
        .cloned()
        .unwrap_or_default();

    // Update current diff index (Slint side handles highlighting reactively)
    window.set_current_diff_index(new_index);
    // Scroll to the diff position
    window.invoke_scroll_diff_to_row(current_pos as i32);

    window.set_status_text(SharedString::from(format!(
        "Difference {} of {} [{}]",
        new_index + 1,
        total,
        stats
    )));

    window.set_current_diff_comment(SharedString::from(comment));

    // Update diff detail pane
    let model = window.get_diff_lines();
    let tab = state.current_tab();
    update_detail_pane(window, &model, new_index, tab);
}

/// Build a \x01-separated segment string from word diff segments.
/// Even indices = unchanged, odd indices = changed.
/// If the first segment is changed, an empty unchanged prefix is prepended.
fn build_word_diff_string(segments: &[crate::models::diff_line::WordDiffSegment]) -> String {
    if segments.is_empty() {
        return String::new();
    }
    let mut parts: Vec<&str> = Vec::with_capacity(segments.len() + 1);
    if segments[0].changed {
        parts.push(""); // empty unchanged prefix so odd indices = changed
    }
    for seg in segments {
        parts.push(&seg.text);
    }
    parts.join("\x01")
}

fn parse_word_diff_segments(text: &str, word_diff: &str) -> ModelRc<WordSegment> {
    if word_diff.is_empty() {
        return ModelRc::new(VecModel::from(vec![WordSegment {
            text: SharedString::from(text),
            is_changed: false,
        }]));
    }
    let parts: Vec<&str> = word_diff.split('\x01').collect();
    let segments: Vec<WordSegment> = parts
        .iter()
        .enumerate()
        .map(|(i, part)| WordSegment {
            text: SharedString::from(*part),
            is_changed: i % 2 == 1,
        })
        .collect();
    ModelRc::new(VecModel::from(segments))
}

fn update_detail_pane(
    window: &MainWindow,
    model: &ModelRc<DiffLineData>,
    diff_index: i32,
    _tab: &TabState,
) {
    if diff_index < 0 {
        window.set_detail_has_left(false);
        window.set_detail_has_right(false);
        window.set_detail_left_lines(ModelRc::new(VecModel::from(Vec::<DetailLineData>::new())));
        window.set_detail_right_lines(ModelRc::new(VecModel::from(Vec::<DetailLineData>::new())));
        return;
    }

    let mut left_lines: Vec<DetailLineData> = Vec::new();
    let mut right_lines: Vec<DetailLineData> = Vec::new();

    let count = model.row_count();
    for i in 0..count {
        let dl = model.row_data(i).unwrap();
        if dl.diff_index != diff_index {
            continue;
        }
        let status = dl.status; // 1=added, 2=removed, 3=modified, 4=moved

        // Left side: removed(2), modified(3), moved(4)
        if status == 2 || status == 3 || status == 4 {
            let segments =
                parse_word_diff_segments(&dl.left_text.to_string(), &dl.left_word_diff.to_string());
            left_lines.push(DetailLineData {
                segments,
                is_current: true,
                status,
            });
        }

        // Right side: added(1), modified(3), moved(4)
        if status == 1 || status == 3 || status == 4 {
            let segments = parse_word_diff_segments(
                &dl.right_text.to_string(),
                &dl.right_word_diff.to_string(),
            );
            right_lines.push(DetailLineData {
                segments,
                is_current: true,
                status,
            });
        }
    }

    let has_left = !left_lines.is_empty();
    let has_right = !right_lines.is_empty();
    window.set_detail_left_lines(ModelRc::new(VecModel::from(left_lines)));
    window.set_detail_right_lines(ModelRc::new(VecModel::from(right_lines)));
    window.set_detail_has_left(has_left);
    window.set_detail_has_right(has_right);
    window.set_detail_left_scroll_y(0.0);
    window.set_detail_right_scroll_y(0.0);
}

fn push_undo_snapshot(state: &mut AppState, vec_model: &VecModel<DiffLineData>) {
    let left_text = rebuild_left(vec_model);
    let right_text = rebuild_right(vec_model);
    let tab = state.current_tab_mut();
    tab.undo_stack.push(TextSnapshot {
        left_text,
        right_text,
    });
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

pub fn copy_right_and_next(window: &MainWindow, state: &mut AppState) {
    let diff_index = state.current_tab().current_diff;
    copy_to_right(window, state, diff_index);
    navigate_diff(window, state, true);
}

pub fn copy_left_and_next(window: &MainWindow, state: &mut AppState) {
    let diff_index = state.current_tab().current_diff;
    copy_to_left(window, state, diff_index);
    navigate_diff(window, state, true);
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

fn expand_tabs(text: &str, tab_width: usize) -> String {
    if !text.contains('\t') {
        return text.to_string();
    }
    let spaces = " ".repeat(tab_width);
    text.replace('\t', &spaces)
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

pub fn copy_all_diffs_to_right(window: &MainWindow, state: &mut AppState) {
    let tab = state.current_tab();
    if tab.diff_positions.is_empty() {
        return;
    }

    let model = window.get_diff_lines();
    let vec_model = match model.as_any().downcast_ref::<VecModel<DiffLineData>>() {
        Some(m) => m,
        None => return,
    };

    push_undo_snapshot(state, vec_model);

    // Copy all left to right: right becomes identical to left
    let left_text = rebuild_left(vec_model);

    state.current_tab_mut().has_unsaved_changes = true;
    window.set_has_unsaved_changes(true);

    recompute_diff_from_text(window, state, &left_text, &left_text);

    window.set_can_undo(true);
    window.set_can_redo(false);
    sync_tab_list(window, state);
}

pub fn copy_all_diffs_to_left(window: &MainWindow, state: &mut AppState) {
    let tab = state.current_tab();
    if tab.diff_positions.is_empty() {
        return;
    }

    let model = window.get_diff_lines();
    let vec_model = match model.as_any().downcast_ref::<VecModel<DiffLineData>>() {
        Some(m) => m,
        None => return,
    };

    push_undo_snapshot(state, vec_model);

    // Left becomes right for all diffs
    let right_text = rebuild_right(vec_model);

    state.current_tab_mut().has_unsaved_changes = true;
    window.set_has_unsaved_changes(true);

    recompute_diff_from_text(window, state, &right_text, &right_text);

    window.set_can_undo(true);
    window.set_can_redo(false);
    sync_tab_list(window, state);
}

pub fn copy_all_text(window: &MainWindow, state: &AppState, is_left: bool) {
    let tab = state.current_tab();
    let text = if is_left {
        tab.left_lines.join("\n")
    } else {
        tab.right_lines.join("\n")
    };

    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        let _ = clipboard.set_text(&text);
        let side = if is_left { "Left" } else { "Right" };
        window.set_status_text(SharedString::from(format!(
            "{} file text copied to clipboard",
            side
        )));
    }
}

pub fn edit_line(
    window: &MainWindow,
    state: &mut AppState,
    line_index: i32,
    new_text: &str,
    is_left: bool,
) {
    let model = window.get_diff_lines();
    let vec_model = match model.as_any().downcast_ref::<VecModel<DiffLineData>>() {
        Some(m) => m,
        None => return,
    };

    if line_index < 0 || line_index as usize >= vec_model.row_count() {
        return;
    }

    // Push undo snapshot before edit
    push_undo_snapshot(state, vec_model);

    let mut row = vec_model.row_data(line_index as usize).unwrap();
    if is_left {
        row.left_text = SharedString::from(new_text);
    } else {
        row.right_text = SharedString::from(new_text);
    }
    vec_model.set_row_data(line_index as usize, row);

    // Update the internal line data
    let tab = state.current_tab_mut();
    // Find the actual line number from the diff model to update left_lines/right_lines
    let data = &vec_model.row_data(line_index as usize).unwrap();
    if is_left {
        if let Ok(line_no) = data.left_line_no.parse::<usize>() {
            if line_no > 0 && line_no <= tab.left_lines.len() {
                tab.left_lines[line_no - 1] = new_text.to_string();
            }
        }
    } else {
        if let Ok(line_no) = data.right_line_no.parse::<usize>() {
            if line_no > 0 && line_no <= tab.right_lines.len() {
                tab.right_lines[line_no - 1] = new_text.to_string();
            }
        }
    }

    tab.has_unsaved_changes = true;
    window.set_has_unsaved_changes(true);
    window.set_can_undo(true);
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

pub fn set_row_selection(window: &MainWindow, state: &mut AppState, row_idx: i32, extend: bool) {
    let model = window.get_diff_lines();
    let vec_model = match model.as_any().downcast_ref::<VecModel<DiffLineData>>() {
        Some(m) => m,
        None => return,
    };

    let tab = state.current_tab_mut();
    if !extend || tab.selection_start < 0 {
        tab.selection_start = row_idx;
    }
    tab.selection_end = row_idx;

    let sel_min = tab.selection_start.min(tab.selection_end) as usize;
    let sel_max = tab.selection_start.max(tab.selection_end) as usize;

    for i in 0..vec_model.row_count() {
        let mut row = vec_model.row_data(i).unwrap();
        let selected = i >= sel_min && i <= sel_max;
        if row.is_selected != selected {
            row.is_selected = selected;
            vec_model.set_row_data(i, row);
        }
    }
}

pub fn copy_selection_to_right(window: &MainWindow, state: &mut AppState) {
    let tab = state.current_tab();
    if tab.selection_start < 0 {
        return;
    }
    let sel_min = tab.selection_start.min(tab.selection_end) as usize;
    let sel_max = tab.selection_start.max(tab.selection_end) as usize;

    let model = window.get_diff_lines();
    let vec_model = match model.as_any().downcast_ref::<VecModel<DiffLineData>>() {
        Some(m) => m,
        None => return,
    };

    let count = sel_max.min(vec_model.row_count().saturating_sub(1)) + 1 - sel_min;
    for i in sel_min..=sel_max.min(vec_model.row_count().saturating_sub(1)) {
        let mut row = vec_model.row_data(i).unwrap();
        if row.status != 0 {
            row.right_text = row.left_text.clone();
            row.status = 0;
            vec_model.set_row_data(i, row);
        }
    }
    state.current_tab_mut().has_unsaved_changes = true;
    window.set_has_unsaved_changes(true);
    window.set_status_text(SharedString::from(format!(
        "Copied {} lines to right",
        count
    )));
}

pub fn copy_selection_to_left(window: &MainWindow, state: &mut AppState) {
    let tab = state.current_tab();
    if tab.selection_start < 0 {
        return;
    }
    let sel_min = tab.selection_start.min(tab.selection_end) as usize;
    let sel_max = tab.selection_start.max(tab.selection_end) as usize;

    let model = window.get_diff_lines();
    let vec_model = match model.as_any().downcast_ref::<VecModel<DiffLineData>>() {
        Some(m) => m,
        None => return,
    };

    let count = sel_max.min(vec_model.row_count().saturating_sub(1)) + 1 - sel_min;
    for i in sel_min..=sel_max.min(vec_model.row_count().saturating_sub(1)) {
        let mut row = vec_model.row_data(i).unwrap();
        if row.status != 0 {
            row.left_text = row.right_text.clone();
            row.status = 0;
            vec_model.set_row_data(i, row);
        }
    }
    state.current_tab_mut().has_unsaved_changes = true;
    window.set_has_unsaved_changes(true);
    window.set_status_text(SharedString::from(format!(
        "Copied {} lines to left",
        count
    )));
}

pub fn export_html_report(window: &MainWindow, state: &AppState) {
    let tab = state.current_tab();

    // Rebuild DiffResult from current state
    let left_text = tab.left_lines.join("\n") + "\n";
    let right_text = tab.right_lines.join("\n") + "\n";
    let result = compute_diff_with_options(&left_text, &right_text, &tab.diff_options);

    let left_title = tab
        .left_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "Left".to_string());
    let right_title = tab
        .right_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "Right".to_string());

    let html = crate::export::export_html(&result, &left_title, &right_title, &tab.diff_comments);

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

pub fn export_xlsx_report(window: &MainWindow, state: &AppState) {
    let tab = state.current_tab();

    let left_text = tab.left_lines.join("\n") + "\n";
    let right_text = tab.right_lines.join("\n") + "\n";
    let result = compute_diff_with_options(&left_text, &right_text, &tab.diff_options);

    let left_title = tab
        .left_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "Left".to_string());
    let right_title = tab
        .right_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "Right".to_string());

    match crate::export::export_xlsx(&result, &left_title, &right_title, &tab.diff_comments) {
        Ok(bytes) => {
            if let Some(path) = rfd::FileDialog::new()
                .set_title("Export Excel Report")
                .set_file_name("diff-report.xlsx")
                .add_filter("Excel", &["xlsx"])
                .save_file()
            {
                match fs::write(&path, &bytes) {
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
        Err(e) => {
            window.set_status_text(SharedString::from(format!("Excel export error: {}", e)));
        }
    }
}

/// Collect all non-empty comments from every tab
fn collect_all_comments(state: &AppState) -> Vec<crate::export::CommentEntry> {
    let mut entries = Vec::new();
    for tab in &state.tabs {
        if tab.diff_comments.is_empty() {
            continue;
        }
        let left_file = tab
            .left_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let right_file = tab
            .right_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let tab_title = tab.title.clone();
        let mut indices: Vec<usize> = tab.diff_comments.keys().copied().collect();
        indices.sort_unstable();
        for idx in indices {
            let comment = &tab.diff_comments[&idx];
            if comment.is_empty() {
                continue;
            }
            entries.push(crate::export::CommentEntry {
                tab_title: tab_title.clone(),
                left_file: left_file.clone(),
                right_file: right_file.clone(),
                diff_block: idx,
                comment: comment.clone(),
            });
        }
    }
    entries
}

pub fn export_all_comments(window: &MainWindow, state: &AppState, use_json: bool) {
    let entries = collect_all_comments(state);
    if entries.is_empty() {
        window.set_status_text(SharedString::from("No comments to export"));
        return;
    }

    let (content, ext, filter_name) = if use_json {
        (
            crate::export::export_all_comments_json(&entries),
            "json",
            "JSON",
        )
    } else {
        (
            crate::export::export_all_comments_csv(&entries),
            "csv",
            "CSV",
        )
    };

    let filename = if use_json {
        "diff-comments.json"
    } else {
        "diff-comments.csv"
    };
    if let Some(path) = rfd::FileDialog::new()
        .set_title("Export All Comments")
        .set_file_name(filename)
        .add_filter(filter_name, &[ext])
        .save_file()
    {
        match fs::write(&path, &content) {
            Ok(_) => {
                window.set_status_text(SharedString::from(format!(
                    "Exported {} comment(s) to {}",
                    entries.len(),
                    path.to_string_lossy()
                )));
            }
            Err(e) => {
                window.set_status_text(SharedString::from(format!("Export error: {}", e)));
            }
        }
    }
}

pub fn print_diff(window: &MainWindow, state: &AppState) {
    let tab = state.current_tab();
    let left_text = tab.left_lines.join("\n") + "\n";
    let right_text = tab.right_lines.join("\n") + "\n";
    let result = compute_diff_with_options(&left_text, &right_text, &tab.diff_options);

    let left_title = tab
        .left_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "Left".to_string());
    let right_title = tab
        .right_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "Right".to_string());

    let html = crate::export::export_html_for_print(
        &result,
        &left_title,
        &right_title,
        &tab.diff_comments,
    );

    // Write to a temp file and open in default browser
    let tmp = std::env::temp_dir().join("winxmerge-print.html");
    match fs::write(&tmp, &html) {
        Ok(_) => {
            if let Err(e) = open::that_detached(&tmp) {
                window.set_status_text(SharedString::from(format!("Print error: {}", e)));
            } else {
                window.set_status_text(SharedString::from("Opened in browser for printing"));
            }
        }
        Err(e) => {
            window.set_status_text(SharedString::from(format!("Print error: {}", e)));
        }
    }
}

pub fn export_patch(window: &MainWindow, state: &AppState) {
    let tab = state.current_tab();

    let left_text = tab.left_lines.join("\n") + "\n";
    let right_text = tab.right_lines.join("\n") + "\n";
    let result = compute_diff_with_options(&left_text, &right_text, &tab.diff_options);

    let left_title = tab
        .left_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "Left".to_string());
    let right_title = tab
        .right_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "Right".to_string());

    let patch = crate::export::export_unified_diff(&result, &left_title, &right_title);

    if let Some(path) = rfd::FileDialog::new()
        .set_title("Export Patch (Unified Diff)")
        .set_file_name("diff.patch")
        .add_filter("Patch", &["patch", "diff"])
        .save_file()
    {
        match fs::write(&path, &patch) {
            Ok(_) => {
                window.set_status_text(SharedString::from(format!(
                    "Exported patch to {}",
                    path.to_string_lossy()
                )));
            }
            Err(e) => {
                window.set_status_text(SharedString::from(format!("Export error: {}", e)));
            }
        }
    }
}

pub fn export_csv_report(window: &MainWindow, state: &AppState, use_tab: bool) {
    let tab = state.current_tab();
    let left_text = tab.left_lines.join("\n") + if tab.left_lines.is_empty() { "" } else { "\n" };
    let right_text =
        tab.right_lines.join("\n") + if tab.right_lines.is_empty() { "" } else { "\n" };
    let result = compute_diff_with_options(&left_text, &right_text, &tab.diff_options);
    let sep = if use_tab { '\t' } else { ',' };
    let ext = if use_tab { "tsv" } else { "csv" };
    let left_title = tab
        .left_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "Left".to_string());
    let right_title = tab
        .right_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "Right".to_string());
    let content = crate::export::export_csv(&result, &left_title, &right_title, sep);
    if let Some(path) = rfd::FileDialog::new()
        .set_title("Export CSV")
        .set_file_name(&format!("diff-report.{}", ext))
        .add_filter(ext.to_uppercase().as_str(), &[ext])
        .save_file()
    {
        match fs::write(&path, content.as_bytes()) {
            Ok(_) => window.set_status_text(SharedString::from(format!(
                "Exported to {}",
                path.to_string_lossy()
            ))),
            Err(e) => window.set_status_text(SharedString::from(format!("Export error: {}", e))),
        }
    }
}

pub fn export_folder_html_report(window: &MainWindow, state: &AppState) {
    let tab = state.current_tab();
    if tab.folder_items.is_empty() {
        return;
    }
    let left_title = tab
        .left_folder
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "Left".to_string());
    let right_title = tab
        .right_folder
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "Right".to_string());
    let html = crate::export::export_folder_html(&tab.folder_items, &left_title, &right_title);
    if let Some(path) = rfd::FileDialog::new()
        .set_title("Export Folder Report")
        .set_file_name("folder-report.html")
        .add_filter("HTML", &["html"])
        .save_file()
    {
        match fs::write(&path, html.as_bytes()) {
            Ok(_) => window.set_status_text(SharedString::from(format!(
                "Folder report exported to {}",
                path.to_string_lossy()
            ))),
            Err(e) => window.set_status_text(SharedString::from(format!("Export error: {}", e))),
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

pub fn apply_options(window: &MainWindow, state: &mut AppState, settings: &mut AppSettings) {
    // Read options from window
    settings.ignore_whitespace = window.get_ignore_whitespace();
    settings.ignore_case = window.get_ignore_case();
    settings.ignore_blank_lines = window.get_opt_ignore_blank_lines();
    settings.ignore_eol = window.get_opt_ignore_eol();
    settings.detect_moved_lines = window.get_opt_detect_moved_lines();
    settings.show_line_numbers = window.get_opt_show_line_numbers();
    settings.word_wrap = window.get_opt_word_wrap();
    settings.syntax_highlighting = window.get_opt_syntax_highlighting();
    settings.enable_context_menu = window.get_opt_enable_context_menu();
    settings.font_size = window.get_opt_font_size() as f32;
    settings.tab_width = window.get_opt_tab_width();
    settings.theme = if window.get_opt_theme() == 1 {
        "dark".to_string()
    } else {
        "light".to_string()
    };
    settings.language = if window.get_opt_language() == 1 {
        "ja".to_string()
    } else {
        "en".to_string()
    };
    let lang_code = if settings.language == "ja" { "ja" } else { "" };
    if let Err(e) = slint::select_bundled_translation(lang_code) {
        eprintln!("Translation error: {}", e);
    }

    settings.auto_rescan = window.get_opt_auto_rescan();

    // Read folder exclude patterns
    let folder_exclude_str = window.get_opt_folder_exclude_patterns().to_string();
    settings.folder_exclude_patterns = folder_exclude_str.clone();
    state.folder_exclude_patterns = folder_exclude_str
        .split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    settings.folder_max_depth = window.get_opt_folder_max_depth().max(0) as usize;
    settings.folder_min_size = window.get_opt_folder_min_size() as u64;
    settings.folder_max_size = window.get_opt_folder_max_size() as u64;
    settings.folder_modified_after = window.get_opt_folder_modified_after().to_string();
    settings.folder_modified_before = window.get_opt_folder_modified_before().to_string();

    // Read filter settings
    let line_filters_str = window.get_opt_line_filters().to_string();
    settings.line_filters = line_filters_str
        .split('|')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let sub_patterns_str = window.get_opt_substitution_patterns().to_string();
    let sub_replacements_str = window.get_opt_substitution_replacements().to_string();
    let patterns: Vec<&str> = sub_patterns_str.split('|').collect();
    let replacements: Vec<&str> = sub_replacements_str.split('|').collect();
    settings.substitution_filters = patterns
        .iter()
        .zip(replacements.iter())
        .filter(|(p, _)| !p.trim().is_empty())
        .map(|(p, r)| crate::settings::SubstitutionFilter {
            pattern: p.trim().to_string(),
            replacement: r.trim().to_string(),
        })
        .collect();

    // Parse plugin list from UI
    let plugin_list_str = window.get_plugin_list().to_string();
    settings.plugins = plugin_list_str
        .split('|')
        .filter(|s| !s.trim().is_empty())
        .filter_map(|entry| {
            let mut parts = entry.splitn(2, ':');
            let name = parts.next()?.trim().to_string();
            let command = parts.next()?.trim().to_string();
            if name.is_empty() || command.is_empty() {
                None
            } else {
                Some(crate::settings::PluginEntry { name, command })
            }
        })
        .collect();

    settings.save();

    // Rebuild plugin model for dynamic menu
    let plugin_entries: Vec<PluginEntryData> = settings
        .plugins
        .iter()
        .map(|p| PluginEntryData {
            name: SharedString::from(&p.name),
            command: SharedString::from(&p.command),
        })
        .collect();
    window.set_plugins(ModelRc::new(VecModel::from(plugin_entries)));

    // Apply diff options to current tab and re-run
    let tab = state.current_tab_mut();
    tab.diff_options.ignore_whitespace = settings.ignore_whitespace;
    tab.diff_options.ignore_case = settings.ignore_case;
    tab.diff_options.ignore_blank_lines = settings.ignore_blank_lines;
    tab.diff_options.ignore_eol = settings.ignore_eol;
    tab.diff_options.detect_moved_lines = settings.detect_moved_lines;
    tab.diff_options.line_filters = settings.line_filters.clone();
    tab.diff_options.substitution_filters = settings
        .substitution_filters
        .iter()
        .map(|f| (f.pattern.clone(), f.replacement.clone()))
        .collect();

    if tab.left_path.is_some() && tab.right_path.is_some() {
        run_diff(window, state);
    }

    window.set_status_text(SharedString::from("Options applied"));
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
        // Clear any existing search highlights
        let model = window.get_diff_lines();
        if let Some(vec_model) = model.as_any().downcast_ref::<VecModel<DiffLineData>>() {
            for i in 0..vec_model.row_count() {
                let mut row = vec_model.row_data(i).unwrap();
                if row.is_search_match {
                    row.is_search_match = false;
                    vec_model.set_row_data(i, row);
                }
            }
        }
        window.set_search_match_count(0);
        window.set_status_text(SharedString::from("Search cleared"));
        return;
    }

    let query_lower = query.to_lowercase();
    let model = window.get_diff_lines();
    if let Some(vec_model) = model.as_any().downcast_ref::<VecModel<DiffLineData>>() {
        for i in 0..vec_model.row_count() {
            let mut row = vec_model.row_data(i).unwrap();
            let matched = row
                .left_text
                .to_string()
                .to_lowercase()
                .contains(&query_lower)
                || row
                    .right_text
                    .to_string()
                    .to_lowercase()
                    .contains(&query_lower);
            if matched {
                tab.search_matches.push(i);
            }
            if row.is_search_match != matched {
                row.is_search_match = matched;
                vec_model.set_row_data(i, row);
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
    row.right_text =
        SharedString::from(case_insensitive_replace(&right, &search_lower, replacement));
    vec_model.set_row_data(match_idx, row);

    state.current_tab_mut().has_unsaved_changes = true;
    window.set_has_unsaved_changes(true);

    search_text(window, state, search);
}

pub fn replace_all_text(
    window: &MainWindow,
    state: &mut AppState,
    search: &str,
    replacement: &str,
) {
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
        row.left_text =
            SharedString::from(case_insensitive_replace(&left, &search_lower, replacement));
        row.right_text =
            SharedString::from(case_insensitive_replace(&right, &search_lower, replacement));
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

pub fn start_three_way_compare(
    window: &MainWindow,
    state: &mut AppState,
    base: &str,
    left: &str,
    right: &str,
) {
    {
        let tab = state.current_tab_mut();
        tab.base_path = Some(PathBuf::from(base));
        tab.left_path = Some(PathBuf::from(left));
        tab.right_path = Some(PathBuf::from(right));
        tab.view_mode = 3;
    }
    run_three_way_diff(window, state);
}

pub fn start_compare(
    window: &MainWindow,
    state: &mut AppState,
    left: &str,
    right: &str,
    is_folder: bool,
) {
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

pub fn discard_and_proceed(
    window: &MainWindow,
    state: &mut AppState,
    show_picker: impl Fn(i32, &str),
) {
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
                window.set_open_left_path_input(SharedString::from(
                    path.to_string_lossy().to_string(),
                ));
                window.set_view_mode(0);
                run_diff(window, state);
            } else {
                show_picker(11, "Select left file");
            }
        }
        2 => {
            if let Some(path) = open_file_dialog("Select right file") {
                {
                    let tab = state.current_tab_mut();
                    tab.right_path = Some(path.clone());
                    tab.view_mode = 0;
                }
                window.set_open_right_path_input(SharedString::from(
                    path.to_string_lossy().to_string(),
                ));
                window.set_view_mode(0);
                run_diff(window, state);
            } else {
                show_picker(12, "Select right file");
            }
        }
        3 => {
            if let Some(path) = open_folder_dialog("Select left folder") {
                state.current_tab_mut().left_folder = Some(path.clone());
                window.set_open_left_path_input(SharedString::from(
                    path.to_string_lossy().to_string(),
                ));
                run_folder_compare(window, state);
            } else {
                show_picker(13, "Select left folder");
            }
        }
        4 => {
            if let Some(path) = open_folder_dialog("Select right folder") {
                state.current_tab_mut().right_folder = Some(path.clone());
                window.set_open_right_path_input(SharedString::from(
                    path.to_string_lossy().to_string(),
                ));
                run_folder_compare(window, state);
            } else {
                show_picker(14, "Select right folder");
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

    let folder_max_depth = window.get_opt_folder_max_depth().max(0) as usize;
    let options = FolderCompareOptions {
        exclude_patterns: state.folder_exclude_patterns.clone(),
        max_depth: folder_max_depth,
        min_size: window.get_opt_folder_min_size() as u64,
        max_size: window.get_opt_folder_max_size() as u64,
        modified_after: window.get_opt_folder_modified_after().to_string(),
        modified_before: window.get_opt_folder_modified_before().to_string(),
        ..Default::default()
    };
    let items = compare_folders_with_options(&left_folder, &right_folder, &options);

    let folder_item_data: Vec<FolderItemData> = items
        .iter()
        .map(|item| {
            let status: i32 = match item.status {
                FileCompareStatus::Identical => 0,
                FileCompareStatus::Different => 1,
                FileCompareStatus::LeftOnly => 2,
                FileCompareStatus::RightOnly => 3,
            };
            // Compute tree depth from path separators
            let depth = item
                .relative_path
                .chars()
                .filter(|&c| c == '/' || c == '\\')
                .count() as i32;
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
                left_modified: item
                    .left_modified
                    .as_ref()
                    .map(|s| SharedString::from(s.as_str()))
                    .unwrap_or_default(),
                right_modified: item
                    .right_modified
                    .as_ref()
                    .map(|s| SharedString::from(s.as_str()))
                    .unwrap_or_default(),
                depth,
            }
        })
        .collect();

    let identical = items
        .iter()
        .filter(|i| i.status == FileCompareStatus::Identical)
        .count();
    let different = items
        .iter()
        .filter(|i| i.status == FileCompareStatus::Different)
        .count();
    let left_only = items
        .iter()
        .filter(|i| i.status == FileCompareStatus::LeftOnly)
        .count();
    let right_only = items
        .iter()
        .filter(|i| i.status == FileCompareStatus::RightOnly)
        .count();
    let total = items.len();
    let summary = format!(
        "Identical: {} | Different: {} | Left only: {} | Right only: {} | Total: {}",
        identical, different, left_only, right_only, total
    );

    let left_name = left_folder
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let right_name = right_folder
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let tab = state.current_tab_mut();
    tab.folder_items = items;
    tab.folder_item_data = folder_item_data.clone();
    tab.view_mode = 1;
    tab.title = format!("{} ↔ {}", left_name, right_name);
    tab.folder_summary = summary.clone();

    window.set_folder_items(ModelRc::new(VecModel::from(folder_item_data)));
    window.set_view_mode(1);
    window.set_folder_summary_text(SharedString::from(summary.clone()));
    window.set_status_text(SharedString::from(format!(
        "Folder: {} ↔ {} — {}",
        left_name, right_name, summary
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

// --- 3-way merge ---

pub fn run_three_way_diff(window: &MainWindow, state: &mut AppState) {
    let tab = state.current_tab();
    let (base_path, left_path, right_path) = match (&tab.base_path, &tab.left_path, &tab.right_path)
    {
        (Some(b), Some(l), Some(r)) => (b.clone(), l.clone(), r.clone()),
        _ => return,
    };

    let base_bytes = fs::read(&base_path).unwrap_or_default();
    let left_bytes = fs::read(&left_path).unwrap_or_default();
    let right_bytes = fs::read(&right_path).unwrap_or_default();

    let (base_text, _) = decode_file(&base_bytes);
    let (left_text, _) = decode_file(&left_bytes);
    let (right_text, _) = decode_file(&right_bytes);

    let result = compute_three_way_diff(&base_text, &left_text, &right_text);

    let line_data: Vec<ThreeWayLineData> = result
        .lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let status: i32 = match line.status {
                ThreeWayStatus::Equal => 0,
                ThreeWayStatus::LeftChanged => 1,
                ThreeWayStatus::RightChanged => 2,
                ThreeWayStatus::BothChanged => 3,
                ThreeWayStatus::Conflict => 4,
            };
            let conflict_index = result
                .conflict_positions
                .iter()
                .position(|&pos| pos == i)
                .map(|idx| idx as i32)
                .unwrap_or(-1);
            ThreeWayLineData {
                base_line_no: line
                    .base_line_no
                    .map(|n| SharedString::from(n.to_string()))
                    .unwrap_or_default(),
                left_line_no: line
                    .left_line_no
                    .map(|n| SharedString::from(n.to_string()))
                    .unwrap_or_default(),
                right_line_no: line
                    .right_line_no
                    .map(|n| SharedString::from(n.to_string()))
                    .unwrap_or_default(),
                base_text: SharedString::from(&line.base_text),
                left_text: SharedString::from(&line.left_text),
                right_text: SharedString::from(&line.right_text),
                status,
                is_current: conflict_index == 0 && !result.conflict_positions.is_empty(),
                conflict_index,
            }
        })
        .collect();

    let left_name = left_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let right_name = right_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let tab = state.current_tab_mut();
    tab.three_way_conflict_positions = result.conflict_positions.clone();
    tab.current_conflict = if result.conflict_positions.is_empty() {
        -1
    } else {
        0
    };
    tab.view_mode = 3;
    tab.title = format!("{} ↔ {} (3-way)", left_name, right_name);

    window.set_three_way_lines(ModelRc::new(VecModel::from(line_data)));
    window.set_conflict_count(result.conflict_count as i32);
    window.set_current_conflict_index(tab.current_conflict);
    window.set_view_mode(3);
    window.set_left_path(SharedString::from(left_path.to_string_lossy().to_string()));
    window.set_right_path(SharedString::from(right_path.to_string_lossy().to_string()));
    window.set_base_path(SharedString::from(base_path.to_string_lossy().to_string()));
    window.set_status_text(SharedString::from(format!(
        "{} conflicts found",
        result.conflict_count
    )));

    sync_tab_list(window, state);
}

pub fn navigate_conflict(window: &MainWindow, state: &mut AppState, forward: bool) {
    let tab = state.current_tab_mut();
    if tab.three_way_conflict_positions.is_empty() {
        return;
    }

    let new_index = if forward {
        if tab.current_conflict < tab.three_way_conflict_positions.len() as i32 - 1 {
            tab.current_conflict + 1
        } else {
            0
        }
    } else if tab.current_conflict > 0 {
        tab.current_conflict - 1
    } else {
        tab.three_way_conflict_positions.len() as i32 - 1
    };

    tab.current_conflict = new_index;
    let total = tab.three_way_conflict_positions.len();
    let current_pos = tab.three_way_conflict_positions[new_index as usize];

    window.set_current_conflict_index(new_index);

    let model = window.get_three_way_lines();
    if let Some(vec_model) = model.as_any().downcast_ref::<VecModel<ThreeWayLineData>>() {
        for i in 0..vec_model.row_count() {
            let mut row = vec_model.row_data(i).unwrap();
            let should = i == current_pos;
            if row.is_current != should {
                row.is_current = should;
                vec_model.set_row_data(i, row);
            }
        }
    }

    window.set_status_text(SharedString::from(format!(
        "Conflict {} of {}",
        new_index + 1,
        total
    )));
}

pub fn resolve_conflict_use_left(window: &MainWindow, state: &mut AppState, conflict_index: i32) {
    let tab = state.current_tab();
    if conflict_index < 0 || conflict_index as usize >= tab.three_way_conflict_positions.len() {
        return;
    }

    let pos = tab.three_way_conflict_positions[conflict_index as usize];
    let model = window.get_three_way_lines();
    if let Some(vec_model) = model.as_any().downcast_ref::<VecModel<ThreeWayLineData>>() {
        if let Some(mut row) = vec_model.row_data(pos) {
            row.right_text = row.left_text.clone();
            row.status = 3; // BothChanged (resolved)
            row.conflict_index = -1;
            vec_model.set_row_data(pos, row);
        }
    }

    // Remove from conflict positions
    let tab = state.current_tab_mut();
    tab.three_way_conflict_positions.retain(|&p| p != pos);
    window.set_conflict_count(tab.three_way_conflict_positions.len() as i32);

    if tab.three_way_conflict_positions.is_empty() {
        tab.current_conflict = -1;
        window.set_status_text(SharedString::from("All conflicts resolved"));
    } else {
        let new_idx = (conflict_index as usize).min(tab.three_way_conflict_positions.len() - 1);
        tab.current_conflict = new_idx as i32;
        window.set_status_text(SharedString::from(format!(
            "{} conflicts remaining",
            tab.three_way_conflict_positions.len()
        )));
    }
}

pub fn resolve_conflict_use_right(window: &MainWindow, state: &mut AppState, conflict_index: i32) {
    let tab = state.current_tab();
    if conflict_index < 0 || conflict_index as usize >= tab.three_way_conflict_positions.len() {
        return;
    }

    let pos = tab.three_way_conflict_positions[conflict_index as usize];
    let model = window.get_three_way_lines();
    if let Some(vec_model) = model.as_any().downcast_ref::<VecModel<ThreeWayLineData>>() {
        if let Some(mut row) = vec_model.row_data(pos) {
            row.left_text = row.right_text.clone();
            row.status = 3; // BothChanged (resolved)
            row.conflict_index = -1;
            vec_model.set_row_data(pos, row);
        }
    }

    let tab = state.current_tab_mut();
    tab.three_way_conflict_positions.retain(|&p| p != pos);
    window.set_conflict_count(tab.three_way_conflict_positions.len() as i32);

    if tab.three_way_conflict_positions.is_empty() {
        tab.current_conflict = -1;
        window.set_status_text(SharedString::from("All conflicts resolved"));
    } else {
        let new_idx = (conflict_index as usize).min(tab.three_way_conflict_positions.len() - 1);
        tab.current_conflict = new_idx as i32;
        window.set_status_text(SharedString::from(format!(
            "{} conflicts remaining",
            tab.three_way_conflict_positions.len()
        )));
    }
}

// --- Folder file operations ---

pub fn folder_copy_to_right(window: &MainWindow, state: &mut AppState, index: i32) {
    let tab = state.current_tab();
    if index < 0 || index as usize >= tab.folder_items.len() {
        return;
    }
    let item = &tab.folder_items[index as usize];
    if let (Some(src), Some(right_folder)) = (&item.left_path, &tab.right_folder) {
        let dest = right_folder.join(&item.relative_path);
        if let Some(parent) = dest.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if item.is_directory {
            copy_dir_recursive(src, &dest);
        } else {
            let _ = fs::copy(src, &dest);
        }
        window.set_status_text(SharedString::from(format!(
            "Copied '{}' to right",
            item.relative_path
        )));
        run_folder_compare(window, state);
    }
}

pub fn folder_copy_to_left(window: &MainWindow, state: &mut AppState, index: i32) {
    let tab = state.current_tab();
    if index < 0 || index as usize >= tab.folder_items.len() {
        return;
    }
    let item = &tab.folder_items[index as usize];
    if let (Some(src), Some(left_folder)) = (&item.right_path, &tab.left_folder) {
        let dest = left_folder.join(&item.relative_path);
        if let Some(parent) = dest.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if item.is_directory {
            copy_dir_recursive(src, &dest);
        } else {
            let _ = fs::copy(src, &dest);
        }
        window.set_status_text(SharedString::from(format!(
            "Copied '{}' to left",
            item.relative_path
        )));
        run_folder_compare(window, state);
    }
}

pub fn folder_delete_item(window: &MainWindow, state: &mut AppState, index: i32) {
    let tab = state.current_tab();
    if index < 0 || index as usize >= tab.folder_items.len() {
        return;
    }
    let item = &tab.folder_items[index as usize];
    if let Some(left) = &item.left_path {
        if left.exists() {
            if item.is_directory {
                let _ = fs::remove_dir_all(left);
            } else {
                let _ = fs::remove_file(left);
            }
        }
    }
    if let Some(right) = &item.right_path {
        if right.exists() {
            if item.is_directory {
                let _ = fs::remove_dir_all(right);
            } else {
                let _ = fs::remove_file(right);
            }
        }
    }
    window.set_status_text(SharedString::from(format!(
        "Deleted '{}'",
        item.relative_path
    )));
    run_folder_compare(window, state);
}

fn copy_dir_recursive(src: &std::path::Path, dest: &std::path::Path) {
    let _ = fs::create_dir_all(dest);
    if let Ok(entries) = fs::read_dir(src) {
        for entry in entries.flatten() {
            let src_path = entry.path();
            let dest_path = dest.join(entry.file_name());
            if src_path.is_dir() {
                copy_dir_recursive(&src_path, &dest_path);
            } else {
                let _ = fs::copy(&src_path, &dest_path);
            }
        }
    }
}

// --- Open in external editor ---

pub fn open_in_editor(window: &MainWindow, state: &AppState, is_left: bool, editor_cmd: &str) {
    let tab = state.current_tab();
    let path = if is_left {
        &tab.left_path
    } else {
        &tab.right_path
    };
    if let Some(path) = path {
        if editor_cmd.is_empty() {
            // Use system default
            match open::that_detached(path) {
                Ok(_) => {
                    window.set_status_text(SharedString::from(format!(
                        "Opened {} in default editor",
                        path.to_string_lossy()
                    )));
                }
                Err(e) => {
                    window.set_status_text(SharedString::from(format!(
                        "Failed to open editor: {}",
                        e
                    )));
                }
            }
        } else {
            // Use custom editor command
            let result = std::process::Command::new(editor_cmd).arg(path).spawn();
            match result {
                Ok(_) => {
                    window.set_status_text(SharedString::from(format!(
                        "Opened {} in {}",
                        path.to_string_lossy(),
                        editor_cmd
                    )));
                }
                Err(e) => {
                    window.set_status_text(SharedString::from(format!(
                        "Failed to open editor '{}': {}",
                        editor_cmd, e
                    )));
                }
            }
        }
    }
}

// --- Plugin execution ---

pub fn run_plugin(window: &MainWindow, state: &AppState, command: &str) {
    let tab = state.current_tab();
    let left = tab
        .left_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let right = tab
        .right_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let cmd = command.replace("{LEFT}", &left).replace("{RIGHT}", &right);

    window.set_status_text(SharedString::from(format!("Running plugin...")));
    let window_weak = window.as_weak();
    let cmd_clone = cmd.clone();
    std::thread::spawn(move || {
        #[cfg(target_os = "windows")]
        let result = std::process::Command::new("cmd")
            .args(["/C", &cmd_clone])
            .output();
        #[cfg(not(target_os = "windows"))]
        let result = std::process::Command::new("sh")
            .args(["-c", &cmd_clone])
            .output();

        let (title, text, status_msg) = match result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let combined = match (stdout.trim().is_empty(), stderr.trim().is_empty()) {
                    (false, false) => format!("{}\n---\n{}", stdout.trim(), stderr.trim()),
                    (false, true) => stdout.trim().to_string(),
                    (true, false) => stderr.trim().to_string(),
                    (true, true) => "(no output)".to_string(),
                };
                let exit = output.status.code().unwrap_or(-1);
                (
                    format!("Plugin: {} (exit {})", cmd_clone, exit),
                    combined,
                    format!("Plugin finished: exit {}", exit),
                )
            }
            Err(e) => (
                "Plugin error".to_string(),
                e.to_string(),
                "Plugin failed".to_string(),
            ),
        };
        let _ = window_weak.upgrade_in_event_loop(move |w: MainWindow| {
            w.set_plugin_output_title(SharedString::from(title));
            w.set_plugin_output_text(SharedString::from(text));
            w.set_show_plugin_output(true);
            w.set_status_text(SharedString::from(status_msg));
        });
    });
}

// --- Auto-rescan: check if files changed on disk ---

pub fn check_files_changed(state: &AppState) -> bool {
    let tab = state.current_tab();
    if tab.view_mode != 0 {
        return false;
    }
    if let Some(left_path) = &tab.left_path {
        if let Some(old_mtime) = &tab.left_mtime {
            if let Ok(meta) = fs::metadata(left_path) {
                if let Ok(new_mtime) = meta.modified() {
                    if new_mtime != *old_mtime {
                        return true;
                    }
                }
            }
        }
    }
    if let Some(right_path) = &tab.right_path {
        if let Some(old_mtime) = &tab.right_mtime {
            if let Ok(meta) = fs::metadata(right_path) {
                if let Ok(new_mtime) = meta.modified() {
                    if new_mtime != *old_mtime {
                        return true;
                    }
                }
            }
        }
    }
    false
}

pub fn rescan(window: &MainWindow, state: &mut AppState) {
    let tab = state.current_tab();
    if tab.view_mode == 0 && tab.left_path.is_some() && tab.right_path.is_some() {
        run_diff(window, state);
        window.set_status_text(SharedString::from("Files rescanned"));
    } else if tab.view_mode == 1 {
        run_folder_compare(window, state);
        window.set_status_text(SharedString::from("Folders rescanned"));
    }
}

pub fn compare_clipboard_as_left(window: &MainWindow, state: &mut AppState) {
    let text = match arboard::Clipboard::new().and_then(|mut c| c.get_text()) {
        Ok(t) => t,
        Err(_) => {
            window.set_status_text(SharedString::from("Clipboard is empty or unavailable"));
            return;
        }
    };
    {
        let tab = state.current_tab_mut();
        tab.left_path = None;
        tab.view_mode = 0;
    }
    window.set_view_mode(0);
    window.set_left_path(SharedString::from("(Clipboard)"));
    window.set_left_encoding_display(SharedString::from("UTF-8"));
    window.set_left_eol_type(SharedString::from(""));

    let tab = state.current_tab();
    let right_text = tab.right_lines.join("\n");
    let right_text = if right_text.is_empty() {
        if let Some(rp) = tab.right_path.clone() {
            let bytes = fs::read(&rp).unwrap_or_default();
            let (t, _) = crate::encoding::decode_file(&bytes);
            t
        } else {
            String::new()
        }
    } else {
        right_text
    };

    recompute_diff_from_text(window, state, &text, &right_text);
    state.current_tab_mut().left_lines = text.lines().map(String::from).collect();
    state.current_tab_mut().title = "(Clipboard) ↔ right".to_string();
    sync_tab_list(window, state);
}

pub fn compare_clipboard_as_right(window: &MainWindow, state: &mut AppState) {
    let text = match arboard::Clipboard::new().and_then(|mut c| c.get_text()) {
        Ok(t) => t,
        Err(_) => {
            window.set_status_text(SharedString::from("Clipboard is empty or unavailable"));
            return;
        }
    };
    {
        let tab = state.current_tab_mut();
        tab.right_path = None;
        tab.view_mode = 0;
    }
    window.set_view_mode(0);
    window.set_right_path(SharedString::from("(Clipboard)"));
    window.set_right_encoding_display(SharedString::from("UTF-8"));
    window.set_right_eol_type(SharedString::from(""));

    let tab = state.current_tab();
    let left_text = tab.left_lines.join("\n");
    let left_text = if left_text.is_empty() {
        if let Some(lp) = tab.left_path.clone() {
            let bytes = fs::read(&lp).unwrap_or_default();
            let (t, _) = crate::encoding::decode_file(&bytes);
            t
        } else {
            String::new()
        }
    } else {
        left_text
    };

    recompute_diff_from_text(window, state, &left_text, &text);
    state.current_tab_mut().right_lines = text.lines().map(String::from).collect();
    state.current_tab_mut().title = "left ↔ (Clipboard)".to_string();
    sync_tab_list(window, state);
}

pub fn reorder_tab(window: &MainWindow, state: &mut AppState, from: usize, to: usize) {
    if from >= state.tabs.len() || to >= state.tabs.len() || from == to {
        return;
    }
    let tab = state.tabs.remove(from);
    state.tabs.insert(to, tab);
    // Update active_tab index
    if state.active_tab == from {
        state.active_tab = to;
    } else if from < state.active_tab && to >= state.active_tab {
        state.active_tab -= 1;
    } else if from > state.active_tab && to <= state.active_tab {
        state.active_tab += 1;
    }
    sync_tab_list(window, state);
}

fn run_zip_compare(
    window: &MainWindow,
    state: &mut AppState,
    left_bytes: &[u8],
    right_bytes: &[u8],
    left_path: &std::path::Path,
    right_path: &std::path::Path,
) {
    let left_str = left_path.to_string_lossy();
    let right_str = right_path.to_string_lossy();
    let items = compare_zip_archives(left_bytes, right_bytes, &left_str, &right_str);

    let folder_item_data: Vec<FolderItemData> = items
        .iter()
        .map(|item| {
            let status: i32 = match item.status {
                crate::models::folder_item::FileCompareStatus::Identical => 0,
                crate::models::folder_item::FileCompareStatus::Different => 1,
                crate::models::folder_item::FileCompareStatus::LeftOnly => 2,
                crate::models::folder_item::FileCompareStatus::RightOnly => 3,
            };
            let depth = item.relative_path.chars().filter(|&c| c == '/').count() as i32;
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
                left_modified: item
                    .left_modified
                    .as_ref()
                    .map(|s| SharedString::from(s.as_str()))
                    .unwrap_or_default(),
                right_modified: item
                    .right_modified
                    .as_ref()
                    .map(|s| SharedString::from(s.as_str()))
                    .unwrap_or_default(),
                depth,
            }
        })
        .collect();

    let identical = items
        .iter()
        .filter(|i| i.status == crate::models::folder_item::FileCompareStatus::Identical)
        .count();
    let different = items
        .iter()
        .filter(|i| i.status == crate::models::folder_item::FileCompareStatus::Different)
        .count();
    let left_only = items
        .iter()
        .filter(|i| i.status == crate::models::folder_item::FileCompareStatus::LeftOnly)
        .count();
    let right_only = items
        .iter()
        .filter(|i| i.status == crate::models::folder_item::FileCompareStatus::RightOnly)
        .count();

    let left_name = left_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let right_name = right_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let summary = format!(
        "Identical: {} | Different: {} | Left only: {} | Right only: {} | Total: {}",
        identical,
        different,
        left_only,
        right_only,
        items.len()
    );

    let tab = state.current_tab_mut();
    tab.folder_items = items;
    tab.folder_item_data = folder_item_data.clone();
    tab.view_mode = 1;
    tab.title = format!("{} ↔ {}", left_name, right_name);
    tab.folder_summary = summary.clone();

    let model = ModelRc::new(VecModel::from(folder_item_data));
    window.set_folder_items(model);
    window.set_view_mode(1);
    window.set_folder_summary_text(SharedString::from(&summary));
    window.set_status_text(SharedString::from(format!(
        "[ZIP] {} ↔ {} — {}",
        left_name, right_name, summary
    )));
    window.set_left_path(SharedString::from(left_path.to_string_lossy().to_string()));
    window.set_right_path(SharedString::from(right_path.to_string_lossy().to_string()));
    sync_tab_list(window, state);
}

fn run_excel_compare(
    window: &MainWindow,
    state: &mut AppState,
    left_bytes: &[u8],
    right_bytes: &[u8],
    left_path: &std::path::Path,
    right_path: &std::path::Path,
) {
    let diffs = compare_excel(left_bytes, right_bytes);

    // Collect unique sheet names (preserve order via BTreeSet for determinism)
    let mut sheet_set: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for d in &diffs {
        sheet_set.insert(d.sheet.clone());
    }
    let sheet_names: Vec<String> = sheet_set.into_iter().collect();

    let cell_data: Vec<ExcelCellData> = diffs
        .iter()
        .map(|d| ExcelCellData {
            sheet_name: SharedString::from(&d.sheet),
            row: d.row as i32,
            col_name: SharedString::from(&d.col_name),
            left_value: SharedString::from(&d.left_value),
            right_value: SharedString::from(&d.right_value),
            status: d.status,
        })
        .collect();

    let left_name = left_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let right_name = right_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let diff_count = diffs.len();

    let tab = state.current_tab_mut();
    tab.excel_cells = cell_data.clone();
    tab.excel_sheet_names = sheet_names.clone();
    tab.view_mode = 4;
    tab.title = format!("{} ↔ {}", left_name, right_name);

    // Prepend empty string so user can select "All sheets"
    let sheet_model: ModelRc<SharedString> = ModelRc::new(VecModel::from(
        std::iter::once(SharedString::from(""))
            .chain(sheet_names.iter().map(|s| SharedString::from(s.as_str())))
            .collect::<Vec<_>>(),
    ));

    window.set_excel_cells(ModelRc::new(VecModel::from(cell_data)));
    window.set_excel_sheet_names(sheet_model);
    window.set_excel_active_sheet(SharedString::from(""));
    window.set_view_mode(4);
    window.set_left_path(SharedString::from(left_path.to_string_lossy().to_string()));
    window.set_right_path(SharedString::from(right_path.to_string_lossy().to_string()));
    window.set_status_text(SharedString::from(format!(
        "[Excel] {} ↔ {} — {} cells changed",
        left_name, right_name, diff_count
    )));
    sync_tab_list(window, state);
}

fn run_csv_compare(
    window: &MainWindow,
    state: &mut AppState,
    left_bytes: &[u8],
    right_bytes: &[u8],
    left_path: &std::path::Path,
    right_path: &std::path::Path,
) {
    use crate::encoding::decode_file;

    let (left_text, _) = decode_file(left_bytes);
    let (right_text, _) = decode_file(right_bytes);

    let diffs = compare_csv(&left_text, &right_text);

    let cell_data: Vec<ExcelCellData> = diffs
        .iter()
        .map(|d| ExcelCellData {
            sheet_name: SharedString::from(""),
            row: d.row as i32,
            col_name: SharedString::from(&d.col_name),
            left_value: SharedString::from(&d.left_value),
            right_value: SharedString::from(&d.right_value),
            status: d.status,
        })
        .collect();

    let left_name = left_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let right_name = right_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let diff_count = diffs.len();

    let tab = state.current_tab_mut();
    tab.excel_cells = cell_data.clone();
    tab.excel_sheet_names = Vec::new();
    tab.view_mode = 6;
    tab.title = format!("{} ↔ {}", left_name, right_name);

    let sheet_model: ModelRc<SharedString> =
        ModelRc::new(VecModel::from(vec![SharedString::from("")]));

    window.set_excel_cells(ModelRc::new(VecModel::from(cell_data)));
    window.set_excel_sheet_names(sheet_model);
    window.set_excel_active_sheet(SharedString::from(""));
    window.set_view_mode(6);
    window.set_left_path(SharedString::from(left_path.to_string_lossy().to_string()));
    window.set_right_path(SharedString::from(right_path.to_string_lossy().to_string()));
    window.set_status_text(SharedString::from(format!(
        "[CSV] {} ↔ {} — {} cells changed",
        left_name, right_name, diff_count
    )));
    sync_tab_list(window, state);
}

fn rgba_to_slint_image(rgba: &[u8], width: u32, height: u32) -> slint::Image {
    let mut pixel_buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(width, height);
    pixel_buffer.make_mut_bytes().copy_from_slice(rgba);
    slint::Image::from_rgba8(pixel_buffer)
}

fn run_image_compare(
    window: &MainWindow,
    state: &mut AppState,
    left_bytes: &[u8],
    right_bytes: &[u8],
    left_path: &std::path::Path,
    right_path: &std::path::Path,
) {
    let left_name = left_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let right_name = right_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    match compare_images(left_bytes, right_bytes) {
        Err(e) => {
            window.set_status_text(SharedString::from(format!("Image error: {e}")));
            sync_tab_list(window, state);
        }
        Ok(result) => {
            let diff_pct = if result.total_pixels > 0 {
                result.diff_pixels as f64 / result.total_pixels as f64 * 100.0
            } else {
                0.0
            };
            let stats = format!(
                "Left: {}×{}  Right: {}×{}  Changed: {} / {} px ({:.2}%)",
                result.left_width,
                result.left_height,
                result.right_width,
                result.right_height,
                result.diff_pixels,
                result.total_pixels,
                diff_pct,
            );

            let left_img =
                rgba_to_slint_image(&result.left_rgba, result.left_width, result.left_height);
            let right_img =
                rgba_to_slint_image(&result.right_rgba, result.right_width, result.right_height);
            let diff_img =
                rgba_to_slint_image(&result.diff_rgba, result.diff_width, result.diff_height);
            let overlay_img =
                rgba_to_slint_image(&result.overlay_rgba, result.diff_width, result.diff_height);

            let tab = state.current_tab_mut();
            tab.view_mode = 5;
            tab.title = format!("{} ↔ {}", left_name, right_name);
            tab.image_stats = stats.clone();
            tab.left_image = Some(left_img.clone());
            tab.right_image = Some(right_img.clone());
            tab.diff_image = Some(diff_img.clone());
            tab.overlay_image = Some(overlay_img.clone());
            tab.image_left_w = result.left_width as i32;
            tab.image_left_h = result.left_height as i32;
            tab.image_right_w = result.right_width as i32;
            tab.image_right_h = result.right_height as i32;

            window.set_view_mode(5);
            window.set_left_image(left_img);
            window.set_right_image(right_img);
            window.set_diff_image(diff_img);
            window.set_overlay_image(overlay_img);
            window.set_image_stats(SharedString::from(stats.clone()));
            window.set_image_left_width(result.left_width as i32);
            window.set_image_left_height(result.left_height as i32);
            window.set_image_right_width(result.right_width as i32);
            window.set_image_right_height(result.right_height as i32);
            window.set_left_path(SharedString::from(left_path.to_string_lossy().to_string()));
            window.set_right_path(SharedString::from(right_path.to_string_lossy().to_string()));
            window.set_status_text(SharedString::from(format!(
                "[Image] {} ↔ {} — {}",
                left_name, right_name, stats
            )));
            sync_tab_list(window, state);
        }
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

// --- Feature: Folder sort ---

/// Helper: convert folder_items to folder_item_data
fn folder_items_to_data(items: &[crate::models::folder_item::FolderItem]) -> Vec<FolderItemData> {
    use crate::models::folder_item::FileCompareStatus;
    items
        .iter()
        .map(|item| {
            let status: i32 = match item.status {
                FileCompareStatus::Identical => 0,
                FileCompareStatus::Different => 1,
                FileCompareStatus::LeftOnly => 2,
                FileCompareStatus::RightOnly => 3,
            };
            let depth = item
                .relative_path
                .chars()
                .filter(|&c| c == '/' || c == '\\')
                .count() as i32;
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
                left_modified: item
                    .left_modified
                    .as_ref()
                    .map(|s| SharedString::from(s.as_str()))
                    .unwrap_or_default(),
                right_modified: item
                    .right_modified
                    .as_ref()
                    .map(|s| SharedString::from(s.as_str()))
                    .unwrap_or_default(),
                depth,
            }
        })
        .collect()
}

pub fn sort_folder(window: &MainWindow, state: &mut AppState, column: i32) {
    use crate::models::folder_item::FileCompareStatus;

    let tab = state.current_tab_mut();
    if tab.view_mode != 1 || tab.folder_items.is_empty() {
        return;
    }

    // Toggle direction if same column, else reset to ascending
    if tab.folder_sort_column == column {
        tab.folder_sort_ascending = !tab.folder_sort_ascending;
    } else {
        tab.folder_sort_column = column;
        tab.folder_sort_ascending = true;
    }

    let ascending = tab.folder_sort_ascending;

    tab.folder_items.sort_by(|a, b| {
        let ord = match column {
            0 => a
                .relative_path
                .to_lowercase()
                .cmp(&b.relative_path.to_lowercase()),
            1 => {
                let status_ord = |s: &FileCompareStatus| match s {
                    FileCompareStatus::Identical => 0,
                    FileCompareStatus::Different => 1,
                    FileCompareStatus::LeftOnly => 2,
                    FileCompareStatus::RightOnly => 3,
                };
                status_ord(&a.status).cmp(&status_ord(&b.status))
            }
            2 => a.left_size.unwrap_or(0).cmp(&b.left_size.unwrap_or(0)),
            3 => a.right_size.unwrap_or(0).cmp(&b.right_size.unwrap_or(0)),
            4 => a
                .left_modified
                .as_deref()
                .unwrap_or("")
                .cmp(b.left_modified.as_deref().unwrap_or("")),
            5 => a
                .right_modified
                .as_deref()
                .unwrap_or("")
                .cmp(b.right_modified.as_deref().unwrap_or("")),
            _ => std::cmp::Ordering::Equal,
        };
        if ascending { ord } else { ord.reverse() }
    });

    let data = folder_items_to_data(&tab.folder_items);
    tab.folder_item_data = data.clone();
    let sort_col = tab.folder_sort_column;
    let sort_asc = tab.folder_sort_ascending;

    window.set_folder_items(ModelRc::new(VecModel::from(data)));
    window.set_folder_selected_index(-1);
    window.set_folder_sort_column(sort_col);
    window.set_folder_sort_ascending(sort_asc);
}

// --- Feature: Diff block comments ---

pub fn set_diff_comment(_window: &MainWindow, state: &mut AppState, comment: String) {
    let tab = state.current_tab_mut();
    if tab.current_diff < 0 {
        return;
    }
    let idx = tab.current_diff as usize;
    if comment.is_empty() {
        tab.diff_comments.remove(&idx);
    } else {
        tab.diff_comments.insert(idx, comment);
    }
}

// --- Feature: Diff status filter ---

pub fn set_diff_filter(window: &MainWindow, state: &mut AppState, filter: i32) {
    state.current_tab_mut().diff_status_filter = filter;
    window.set_diff_status_filter(filter);
}

// --- Feature: Clipboard paste path ---

pub fn paste_clipboard_path_left(window: &MainWindow) {
    if let Ok(mut cb) = arboard::Clipboard::new() {
        if let Ok(text) = cb.get_text() {
            let path = text.trim().trim_start_matches("file://");
            window.set_open_left_path_input(SharedString::from(path));
        }
    }
}

pub fn paste_clipboard_path_right(window: &MainWindow) {
    if let Ok(mut cb) = arboard::Clipboard::new() {
        if let Ok(text) = cb.get_text() {
            let path = text.trim().trim_start_matches("file://");
            window.set_open_right_path_input(SharedString::from(path));
        }
    }
}

// --- Feature: Folder item preview ---

pub fn preview_folder_item(window: &MainWindow, state: &AppState, idx: i32) {
    let tab = state.current_tab();
    if idx < 0 || idx as usize >= tab.folder_items.len() {
        return;
    }
    let item = &tab.folder_items[idx as usize];
    let name = item
        .relative_path
        .split('/')
        .last()
        .unwrap_or(&item.relative_path)
        .to_string();

    if item.is_directory {
        window.set_folder_preview_name(SharedString::from(name));
        window.set_folder_preview_left(SharedString::from("(directory)"));
        window.set_folder_preview_right(SharedString::from("(directory)"));
        return;
    }

    let (left_folder, right_folder) = match (&tab.left_folder, &tab.right_folder) {
        (Some(l), Some(r)) => (l.clone(), r.clone()),
        _ => return,
    };

    let left_path = left_folder.join(&item.relative_path);
    let right_path = right_folder.join(&item.relative_path);

    window.set_folder_preview_name(SharedString::from(name));
    window.set_folder_preview_left(SharedString::from(load_text_preview(&left_path)));
    window.set_folder_preview_right(SharedString::from(load_text_preview(&right_path)));
}

fn load_text_preview(path: &std::path::Path) -> String {
    if !path.exists() {
        return "(file not found)".to_string();
    }
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) => return format!("(read error: {e})"),
    };
    if is_binary(&bytes) {
        return format!("[binary  {} bytes]", bytes.len());
    }
    let (text, _) = decode_file(&bytes);
    text.lines().take(20).collect::<Vec<_>>().join("\n")
}

/// Parse diff stats string "+A -R ~M" into (added, removed, modified)
pub fn parse_diff_stats(s: &str) -> (i32, i32, i32) {
    let mut added = 0i32;
    let mut removed = 0i32;
    let mut modified = 0i32;
    for token in s.split_whitespace() {
        if let Some(n) = token.strip_prefix('+') {
            added = n.parse().unwrap_or(0);
        } else if let Some(n) = token.strip_prefix('-') {
            removed = n.parse().unwrap_or(0);
        } else if let Some(n) = token.strip_prefix('~') {
            modified = n.parse().unwrap_or(0);
        }
    }
    (added, removed, modified)
}

/// Set diff stats text and also update individual stat properties on the window
pub fn sync_diff_stats(window: &MainWindow, stats: &str) {
    window.set_diff_stats_text(SharedString::from(stats));
    let (added, removed, modified) = parse_diff_stats(stats);
    window.set_diff_stats_added(added);
    window.set_diff_stats_removed(removed);
    window.set_diff_stats_modified(modified);
}
