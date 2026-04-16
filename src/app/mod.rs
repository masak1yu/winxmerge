use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};

use crate::archive::{compare_zip_archives, is_zip_bytes, is_zip_path};
use crate::csv::{compare_csv_full, compare_csv_full_3way, is_csv_path};
use crate::diff::engine::{DiffOptions, compute_diff_with_options};
use crate::diff::folder::{FolderCompareOptions, compare_folders_with_options};
use crate::diff::three_way::compute_three_way_diff;
use crate::encoding::{decode_file, detect_eol, encode_text, is_binary};
use crate::excel::{compare_excel_full, is_excel_path};
use crate::highlight::{detect_file_type, highlight_lines};
use crate::image_compare::{compare_images, is_image_path};
use crate::models::diff_line::DiffResult;
use crate::models::diff_line::LineStatus;
use crate::models::folder_item::FileCompareStatus;
use crate::settings::AppSettings;
use crate::{
    DetailLineData, DiffLineData, ExcelCellData, FolderItemData, MainWindow, PaneLineData,
    PluginEntryData, TabData, TableCellData, TableColumnInfo, TableDetailCellData, TableRowData,
    WordSegment,
};

/// Line count threshold above which diff is computed on a background thread
const ASYNC_DIFF_THRESHOLD: usize = 30_000;

/// DiffLineData.status codes (matches `status: int` in diff-view.slint)
pub const STATUS_EQUAL: i32 = 0;
pub const STATUS_ADDED: i32 = 1;
pub const STATUS_REMOVED: i32 = 2;
pub const STATUS_MODIFIED: i32 = 3;
pub const STATUS_MOVED: i32 = 4;

/// View mode discriminants match `view-mode: int` in main.slint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ViewMode {
    FileDiff = 0,
    FolderCompare = 1,
    // 2 is intentionally unused / reserved
    ThreeWayText = 3,
    ExcelCompare = 4,
    ImageCompare = 5,
    CsvCompare = 6,
    Blank = 7,
    CsvThreeWay = 8,
}

impl ViewMode {
    pub fn as_i32(self) -> i32 {
        self as i32
    }
    pub fn is_table_mode(self) -> bool {
        matches!(
            self,
            ViewMode::ExcelCompare | ViewMode::CsvCompare | ViewMode::CsvThreeWay
        )
    }
    pub fn is_text_mode(self) -> bool {
        matches!(self, ViewMode::FileDiff | ViewMode::ThreeWayText)
    }
}

impl From<i32> for ViewMode {
    fn from(v: i32) -> Self {
        match v {
            0 => ViewMode::FileDiff,
            1 => ViewMode::FolderCompare,
            3 => ViewMode::ThreeWayText,
            4 => ViewMode::ExcelCompare,
            5 => ViewMode::ImageCompare,
            6 => ViewMode::CsvCompare,
            8 => ViewMode::CsvThreeWay,
            _ => ViewMode::Blank,
        }
    }
}

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
pub(crate) struct TextSnapshot {
    pub(crate) left_text: String,
    pub(crate) right_text: String,
}

/// Snapshot for table undo/redo (stores cell texts as grids)
#[derive(Clone)]
pub(crate) struct TableSnapshot {
    pub(crate) left_grid: Vec<Vec<String>>,
    pub(crate) right_grid: Vec<Vec<String>>,
    pub(crate) base_grid: Vec<Vec<String>>,
}

/// Cached table data for a single Excel sheet (for fast sheet switching)
pub struct SheetTableCache {
    pub rows: Vec<TableRowData>,
    pub columns: Vec<TableColumnInfo>,
    pub content_width_px: i32,
}

/// Per-tab state
pub struct TabState {
    pub left_path: Option<PathBuf>,
    pub right_path: Option<PathBuf>,
    pub base_path: Option<PathBuf>,
    /// Per-pane independent buffers (Phase 0+).
    /// These are populated alongside the legacy shared models during migration.
    pub left_buffer: Option<PaneBuffer>,
    pub right_buffer: Option<PaneBuffer>,
    pub middle_buffer: Option<PaneBuffer>,
    pub three_way_conflict_positions: Vec<usize>,
    pub current_conflict: i32,
    pub diff_positions: Vec<usize>,
    pub current_diff: i32,
    pub left_lines: Vec<String>,
    pub right_lines: Vec<String>,
    pub base_lines: Vec<String>,
    pub has_unsaved_changes: bool,
    // True after any inline edit; cleared on rescan/compare
    pub editing_dirty: bool,
    // Undo/Redo
    pub(crate) undo_stack: Vec<TextSnapshot>,
    pub(crate) redo_stack: Vec<TextSnapshot>,
    pub left_folder: Option<PathBuf>,
    pub right_folder: Option<PathBuf>,
    pub folder_items: Vec<crate::models::folder_item::FolderItem>,
    pub left_encoding: String,
    pub right_encoding: String,
    pub base_encoding: String,
    pub left_eol_type: String,
    pub right_eol_type: String,
    pub diff_options: DiffOptions,
    pub search_matches: Vec<usize>,
    pub current_search_match: i32,
    pub view_mode: ViewMode,
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
    /// Image comparison cached images and stats (ViewMode::ImageCompare)
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
    /// True when folder view was built from IPC-received file pairs (not real directory scan)
    pub is_virtual_folder: bool,
    /// Table grid rows for side-by-side table view (view_mode 4 and 6)
    pub table_rows: Vec<TableRowData>,
    /// Column info for table grid view
    pub table_columns: Vec<TableColumnInfo>,
    /// Total content width in pixels for table grid view
    pub table_content_width_px: i32,
    /// Cached per-sheet table data for Excel sheet switching
    pub excel_sheet_data: std::collections::BTreeMap<String, SheetTableCache>,
    /// CSV delimiter for saving (default comma)
    pub csv_delimiter: u8,
    /// Table undo/redo stacks
    pub table_undo_stack: Vec<TableSnapshot>,
    pub table_redo_stack: Vec<TableSnapshot>,
}

impl TabState {
    pub fn new() -> Self {
        Self {
            left_path: None,
            right_path: None,
            base_path: None,
            left_buffer: None,
            right_buffer: None,
            middle_buffer: None,
            three_way_conflict_positions: Vec::new(),
            current_conflict: -1,
            diff_positions: Vec::new(),
            current_diff: -1,
            left_lines: Vec::new(),
            right_lines: Vec::new(),
            base_lines: Vec::new(),
            has_unsaved_changes: false,
            editing_dirty: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            left_folder: None,
            right_folder: None,
            folder_items: Vec::new(),
            left_encoding: "UTF-8".to_string(),
            right_encoding: "UTF-8".to_string(),
            base_encoding: "UTF-8".to_string(),
            left_eol_type: String::new(),
            right_eol_type: String::new(),
            diff_options: DiffOptions::default(),
            search_matches: Vec::new(),
            current_search_match: -1,
            view_mode: ViewMode::Blank,
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
            is_virtual_folder: false,
            table_rows: Vec::new(),
            table_columns: Vec::new(),
            table_content_width_px: 0,
            excel_sheet_data: std::collections::BTreeMap::new(),
            csv_delimiter: b',',
            table_undo_stack: Vec::new(),
            table_redo_stack: Vec::new(),
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

// --- Submodules ---

mod diff_navigation;
mod diff_text;
mod folder;
mod helpers;
mod options;
pub mod pane_buffer;
mod save_export;
mod search;
mod tab;
mod table;
mod text_edit;
mod three_way;

pub use diff_navigation::*;
pub use diff_text::*;
pub use folder::*;
pub use helpers::*;
pub use options::*;
pub use pane_buffer::*;
pub use save_export::*;
pub use search::*;
pub use tab::*;
pub use table::*;
pub use text_edit::*;
pub use three_way::*;
