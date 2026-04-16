use super::*;

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
        if tab.has_unsaved_changes || tab.editing_dirty {
            // Preserve user edits: rebuild diff from current VecModel text
            rescan(window, state);
        } else {
            run_diff(window, state);
        }
    }

    window.set_status_text(SharedString::from("Options applied"));
}

pub fn toggle_ignore_whitespace(window: &MainWindow, state: &mut AppState) {
    let tab = state.current_tab_mut();
    tab.diff_options.ignore_whitespace = !tab.diff_options.ignore_whitespace;
    window.set_ignore_whitespace(tab.diff_options.ignore_whitespace);
    rerun_diff_safe(window, state);
}

pub fn toggle_ignore_case(window: &MainWindow, state: &mut AppState) {
    let tab = state.current_tab_mut();
    tab.diff_options.ignore_case = !tab.diff_options.ignore_case;
    window.set_ignore_case(tab.diff_options.ignore_case);
    rerun_diff_safe(window, state);
}

pub(super) fn rerun_diff_safe(window: &MainWindow, state: &mut AppState) {
    let tab = state.current_tab();
    if tab.left_path.is_some() && tab.right_path.is_some() {
        if tab.has_unsaved_changes || tab.editing_dirty {
            rescan(window, state);
        } else {
            run_diff(window, state);
        }
    }
}
