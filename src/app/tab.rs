use super::*;

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
    // Check for unsaved changes — show confirm dialog if dirty
    if state.tabs[idx].has_unsaved_changes {
        window.set_tab_close_target_index(index);
        window.set_show_tab_close_confirm(true);
        return;
    }
    force_close_tab(window, state, index);
}

/// Close tab without checking unsaved changes (used after dialog confirm).
pub fn force_close_tab(window: &MainWindow, state: &mut AppState, index: i32) {
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

pub(super) fn save_current_tab_from_window(window: &MainWindow, state: &mut AppState) {
    let tab = state.current_tab_mut();
    // PaneBuffers are Rc<VecModel> owned by TabState — no snapshot needed.
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

pub(super) fn restore_tab(window: &MainWindow, state: &AppState) {
    let tab = state.current_tab();

    window.set_view_mode(tab.view_mode.as_i32());
    window.set_active_tab_index(state.active_tab as i32);

    restore_tab_common(window, tab);
    restore_tab_diff_options(window, tab);

    // Update detail pane
    update_detail_pane(window, tab.current_diff, tab);

    // Restore folder data
    let folder_model = ModelRc::new(VecModel::from(tab.folder_item_data.clone()));
    window.set_folder_items(folder_model);

    match tab.view_mode {
        ViewMode::FolderCompare => {
            window.set_folder_summary_text(SharedString::from(&tab.folder_summary));
        }
        ViewMode::ImageCompare => restore_tab_image(window, tab),
        ViewMode::Blank => {
            window.set_status_text(SharedString::from(""));
        }
        _ => {}
    }

    if tab.view_mode.is_table_mode() {
        restore_tab_table(window, tab);
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

fn restore_tab_common(window: &MainWindow, tab: &TabState) {
    // Legacy diff-lines: set empty model (unused by ListViews, kept for Slint property compat)
    window.set_diff_lines(ModelRc::new(VecModel::from(Vec::<DiffLineData>::new())));
    // Restore per-pane models (authoritative source)
    if let Some(ref buf) = tab.left_buffer {
        window.set_left_lines(ModelRc::from(buf.model.clone()));
    } else {
        window.set_left_lines(ModelRc::new(VecModel::from(Vec::<PaneLineData>::new())));
    }
    if let Some(ref buf) = tab.middle_buffer {
        window.set_middle_lines(ModelRc::from(buf.model.clone()));
    } else {
        window.set_middle_lines(ModelRc::new(VecModel::from(Vec::<PaneLineData>::new())));
    }
    if let Some(ref buf) = tab.right_buffer {
        window.set_right_lines(ModelRc::from(buf.model.clone()));
    } else {
        window.set_right_lines(ModelRc::new(VecModel::from(Vec::<PaneLineData>::new())));
    }
    window.set_diff_count(tab.diff_positions.len() as i32);
    window.set_current_diff_index(tab.current_diff);
    window.set_has_unsaved_changes(tab.has_unsaved_changes);
    if tab.editing_dirty {
        window.set_status_text(SharedString::from("Editing — press F5 to compare"));
    }

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
}

fn restore_tab_diff_options(window: &MainWindow, tab: &TabState) {
    window.set_ignore_whitespace(tab.diff_options.ignore_whitespace);
    window.set_ignore_case(tab.diff_options.ignore_case);
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
}

fn restore_tab_image(window: &MainWindow, tab: &TabState) {
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

fn restore_tab_table(window: &MainWindow, tab: &TabState) {
    window.set_table_rows(ModelRc::new(VecModel::from(tab.table_rows.clone())));
    window.set_table_columns(ModelRc::new(VecModel::from(tab.table_columns.clone())));
    window.set_table_content_width_px(tab.table_content_width_px);
    window.set_diff_count(tab.diff_positions.len() as i32);
    window.set_current_diff_index(tab.current_diff);
    let highlight_row = if tab.current_diff >= 0 {
        tab.diff_positions
            .get(tab.current_diff as usize)
            .map(|&p| p as i32)
            .unwrap_or(-1)
    } else {
        -1
    };
    window.set_table_current_highlight_row(highlight_row);

    if tab.view_mode == ViewMode::ExcelCompare {
        let sheet_model: ModelRc<SharedString> = ModelRc::new(VecModel::from(
            std::iter::once(SharedString::from(""))
                .chain(
                    tab.excel_sheet_names
                        .iter()
                        .map(|s| SharedString::from(s.as_str())),
                )
                .collect::<Vec<_>>(),
        ));
        window.set_excel_sheet_names(sheet_model);
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
