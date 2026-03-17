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
    #[serde(default)]
    pub ignore_whitespace: bool,
    #[serde(default)]
    pub ignore_case: bool,
    #[serde(default)]
    pub show_toolbar: bool,
    #[serde(default)]
    pub recent_files: Vec<RecentEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentEntry {
    pub left_path: String,
    pub right_path: String,
    pub is_folder: bool,
}

fn default_width() -> f32 {
    1200.0
}
fn default_height() -> f32 {
    800.0
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            window_width: default_width(),
            window_height: default_height(),
            ignore_whitespace: false,
            ignore_case: false,
            show_toolbar: true,
            recent_files: Vec::new(),
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
        // Remove duplicate if exists
        self.recent_files
            .retain(|r| !(r.left_path == left && r.right_path == right));

        // Add to front
        self.recent_files.insert(
            0,
            RecentEntry {
                left_path: left.to_string(),
                right_path: right.to_string(),
                is_folder,
            },
        );

        // Trim
        self.recent_files.truncate(MAX_RECENT_FILES);
    }
}
