use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const MAX_RECENT_FILES: usize = 10;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default = "default_width")]
    pub window_width: f32,
    #[serde(default = "default_height")]
    pub window_height: f32,

    // Compare options
    #[serde(default)]
    pub ignore_whitespace: bool,
    #[serde(default)]
    pub ignore_case: bool,
    #[serde(default)]
    pub ignore_blank_lines: bool,
    #[serde(default)]
    pub ignore_eol: bool,
    #[serde(default)]
    pub detect_moved_lines: bool,

    // View options
    #[serde(default)]
    pub show_toolbar: bool,
    #[serde(default = "default_font_size")]
    pub font_size: f32,
    #[serde(default = "default_tab_width")]
    pub tab_width: i32,
    #[serde(default = "default_true")]
    pub show_line_numbers: bool,
    #[serde(default = "default_true")]
    pub word_wrap: bool,
    #[serde(default = "default_true")]
    pub syntax_highlighting: bool,
    #[serde(default = "default_true")]
    pub enable_context_menu: bool,

    // Theme: "light", "dark"
    #[serde(default = "default_theme")]
    pub theme: String,

    // Language: "en", "ja"
    #[serde(default = "default_language")]
    pub language: String,

    // Filters
    #[serde(default)]
    pub line_filters: Vec<String>,
    #[serde(default)]
    pub substitution_filters: Vec<SubstitutionFilter>,

    // Plugins
    #[serde(default)]
    pub plugins: Vec<PluginEntry>,

    // External editor command (empty = system default)
    #[serde(default)]
    pub external_editor: String,

    // Auto-rescan: watch files for changes
    #[serde(default)]
    pub auto_rescan: bool,

    // Folder comparison: exclude patterns (e.g., "*.log", "build/", "node_modules")
    #[serde(default)]
    pub folder_exclude_patterns: String,

    // View panel visibility
    #[serde(default = "default_true")]
    pub show_location_pane: bool,
    #[serde(default = "default_true")]
    pub show_word_diff: bool,
    #[serde(default = "default_true")]
    pub show_detail_pane: bool,

    #[serde(default)]
    pub recent_files: Vec<RecentEntry>,

    // Session: tabs to restore on next launch
    #[serde(default)]
    pub session: Vec<SessionEntry>,

    // Diff-only mode: hide equal lines
    #[serde(default)]
    pub diff_only: bool,

    // Folder max recursion depth (0 = unlimited)
    #[serde(default)]
    pub folder_max_depth: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubstitutionFilter {
    pub pattern: String,
    pub replacement: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginEntry {
    pub name: String,
    pub command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentEntry {
    pub left_path: String,
    pub right_path: String,
    pub is_folder: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEntry {
    pub left_path: String,
    pub right_path: String,
    #[serde(default)]
    pub base_path: String,
}

fn default_width() -> f32 {
    1200.0
}
fn default_height() -> f32 {
    800.0
}
fn default_font_size() -> f32 {
    13.0
}
fn default_tab_width() -> i32 {
    4
}
fn default_true() -> bool {
    true
}
fn default_theme() -> String {
    "light".to_string()
}
fn default_language() -> String {
    "en".to_string()
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            window_width: default_width(),
            window_height: default_height(),
            ignore_whitespace: false,
            ignore_case: false,
            ignore_blank_lines: false,
            ignore_eol: false,
            detect_moved_lines: true,
            show_toolbar: true,
            font_size: default_font_size(),
            tab_width: default_tab_width(),
            show_line_numbers: true,
            word_wrap: true,
            syntax_highlighting: true,
            enable_context_menu: true,
            theme: default_theme(),
            language: default_language(),
            line_filters: Vec::new(),
            substitution_filters: Vec::new(),
            plugins: Vec::new(),
            external_editor: String::new(),
            auto_rescan: false,
            folder_exclude_patterns: String::new(),
            show_location_pane: true,
            show_word_diff: true,
            show_detail_pane: true,
            recent_files: Vec::new(),
            session: Vec::new(),
            diff_only: false,
            folder_max_depth: 0,
        }
    }
}

impl AppSettings {
    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("winxmerge").join("settings.json"))
    }

    pub fn load() -> Self {
        let path = match Self::config_path() {
            Some(p) => p,
            None => return Self::default(),
        };

        match fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) {
        let path = match Self::config_path() {
            Some(p) => p,
            None => return,
        };

        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(&path, json);
        }
    }

    pub fn add_recent(&mut self, left: &str, right: &str, is_folder: bool) {
        self.recent_files
            .retain(|r| !(r.left_path == left && r.right_path == right));

        self.recent_files.insert(
            0,
            RecentEntry {
                left_path: left.to_string(),
                right_path: right.to_string(),
                is_folder,
            },
        );

        self.recent_files.truncate(MAX_RECENT_FILES);
    }
}
