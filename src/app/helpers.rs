use super::*;

// --- Shared helpers ---

/// Downcast the window's diff_lines model to `&VecModel<DiffLineData>`.
/// Usage: `let_diff_vec_model!(model, vec_model, window);`
/// Expands to: let model = window.get_diff_lines(); let Some(vec_model) = ... else { return; };
macro_rules! let_diff_vec_model {
    ($model:ident, $vec:ident, $window:expr) => {
        let $model = $window.get_diff_lines();
        let Some($vec) = $model.as_any().downcast_ref::<VecModel<DiffLineData>>() else {
            return;
        };
    };
}
pub(super) use let_diff_vec_model;

/// Mark the current tab as having unsaved changes and sync to the window.
pub(super) fn mark_dirty(window: &MainWindow, state: &mut AppState) {
    state.current_tab_mut().has_unsaved_changes = true;
    window.set_has_unsaved_changes(true);
}

// --- Native dialog helpers ---

/// Returns true if the platform supports native file dialogs.
/// On macOS and Windows, rfd always has a working backend.
/// On Linux without a display server or GTK, it may fail silently.
pub fn has_native_file_dialog() -> bool {
    cfg!(target_os = "macos") || cfg!(target_os = "windows")
}

pub fn open_file_dialog(title: &str) -> Option<PathBuf> {
    rfd::FileDialog::new().set_title(title).pick_file()
}

pub fn open_folder_dialog(title: &str) -> Option<PathBuf> {
    rfd::FileDialog::new().set_title(title).pick_folder()
}

// --- File I/O helpers ---

/// Read a file, showing an error on the status bar if it fails.
pub(super) fn read_file_or_report(window: &MainWindow, path: &std::path::Path) -> Option<Vec<u8>> {
    match fs::read(path) {
        Ok(bytes) => Some(bytes),
        Err(e) => {
            window.set_status_text(SharedString::from(format!(
                "Error reading {}: {}",
                path.display(),
                e
            )));
            None
        }
    }
}

// --- Text helpers ---

pub(super) fn expand_tabs(text: &str, tab_width: usize) -> String {
    if !text.contains('\t') {
        return text.to_string();
    }
    let spaces = " ".repeat(tab_width);
    text.replace('\t', &spaces)
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

pub fn paste_clipboard_path_base(window: &MainWindow) {
    if let Ok(mut cb) = arboard::Clipboard::new() {
        if let Ok(text) = cb.get_text() {
            let path = text.trim().trim_start_matches("file://");
            window.set_open_base_path_input(SharedString::from(path));
        }
    }
}

// --- Diff stats ---

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
