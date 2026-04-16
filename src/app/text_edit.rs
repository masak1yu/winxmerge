use super::*;

/// Renumber left/right line numbers in the diff model starting from `from_row`.
/// Counts existing line numbers in rows before `from_row`, then sequentially
/// renumbers all rows from `from_row` onward.
fn renumber_diff_lines(vec_model: &VecModel<DiffLineData>, from_row: usize) {
    let mut left_counter = 0usize;
    let mut right_counter = 0usize;
    for i in 0..from_row {
        if let Some(r) = vec_model.row_data(i) {
            if !r.left_line_no.is_empty() {
                left_counter += 1;
            }
            if !r.right_line_no.is_empty() {
                right_counter += 1;
            }
        }
    }
    for i in from_row..vec_model.row_count() {
        if let Some(mut r) = vec_model.row_data(i) {
            let mut changed = false;
            if !r.left_line_no.is_empty() {
                left_counter += 1;
                let new_no = SharedString::from(left_counter.to_string());
                if r.left_line_no != new_no {
                    r.left_line_no = new_no;
                    changed = true;
                }
            }
            if !r.right_line_no.is_empty() {
                right_counter += 1;
                let new_no = SharedString::from(right_counter.to_string());
                if r.right_line_no != new_no {
                    r.right_line_no = new_no;
                    changed = true;
                }
            }
            if changed {
                vec_model.set_row_data(i, r);
            }
        }
    }
}

pub(super) fn push_undo_snapshot(state: &mut AppState, vec_model: &VecModel<DiffLineData>) {
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
    let vm = state.current_tab().view_mode;
    if vm.is_table_mode() {
        table_undo(window, state);
        return;
    }

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

    let Some(snapshot) = tab.undo_stack.pop() else {
        return;
    };
    recompute_diff_from_text(window, state, &snapshot.left_text, &snapshot.right_text);

    let tab = state.current_tab();
    window.set_can_undo(!tab.undo_stack.is_empty());
    window.set_can_redo(!tab.redo_stack.is_empty());
    window.set_status_text(SharedString::from("Undo"));
}

pub fn redo(window: &MainWindow, state: &mut AppState) {
    let vm = state.current_tab().view_mode;
    if vm.is_table_mode() {
        table_redo(window, state);
        return;
    }

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

    let Some(snapshot) = tab.redo_stack.pop() else {
        return;
    };
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

    let_diff_vec_model!(model, vec_model, window);

    push_undo_snapshot(state, vec_model);

    let right_text = rebuild_right_after_copy_from_left(vec_model, diff_index);
    let left_text = rebuild_left(vec_model);

    mark_dirty(window, state);

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

    let_diff_vec_model!(model, vec_model, window);

    push_undo_snapshot(state, vec_model);

    let left_text = rebuild_left_after_copy_from_right(vec_model, diff_index);
    let right_text = rebuild_right(vec_model);

    mark_dirty(window, state);

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

pub fn copy_all_diffs_to_right(window: &MainWindow, state: &mut AppState) {
    let tab = state.current_tab();
    if tab.diff_positions.is_empty() {
        return;
    }

    let_diff_vec_model!(model, vec_model, window);

    push_undo_snapshot(state, vec_model);

    // Copy all left to right: right becomes identical to left
    let left_text = rebuild_left(vec_model);

    mark_dirty(window, state);

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

    let_diff_vec_model!(model, vec_model, window);

    push_undo_snapshot(state, vec_model);

    // Left becomes right for all diffs
    let right_text = rebuild_right(vec_model);

    mark_dirty(window, state);

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

pub fn insert_line_after(
    window: &MainWindow,
    state: &mut AppState,
    line_index: i32,
    is_left: bool,
) {
    let_diff_vec_model!(model, vec_model, window);
    if line_index < 0 || line_index as usize >= vec_model.row_count() {
        return;
    }

    push_undo_snapshot(state, vec_model);

    let insert_at = (line_index + 1) as usize;

    // Determine line_no for the new row from the current row
    let current_row = match vec_model.row_data(line_index as usize) {
        Some(r) => r,
        None => return,
    };
    let line_no_str = if is_left {
        &current_row.left_line_no
    } else {
        &current_row.right_line_no
    };
    let line_no = line_no_str.parse::<usize>().unwrap_or(1);

    // Insert empty line into left_lines or right_lines
    {
        let tab = state.current_tab_mut();
        if is_left {
            let pos = line_no.min(tab.left_lines.len());
            tab.left_lines.insert(pos, String::new());
        } else {
            let pos = line_no.min(tab.right_lines.len());
            tab.right_lines.insert(pos, String::new());
        }
    }

    // Insert a ghost row into the diff model
    // Left insert: status=2 (Removed) → left has content, right is ghost
    // Right insert: status=1 (Added) → right has content, left is ghost
    let new_row = DiffLineData {
        left_line_no: if is_left {
            SharedString::from("?")
        } else {
            SharedString::from("")
        },
        right_line_no: if is_left {
            SharedString::from("")
        } else {
            SharedString::from("?")
        },
        left_text: SharedString::from(""),
        right_text: SharedString::from(""),
        status: if is_left {
            STATUS_REMOVED
        } else {
            STATUS_ADDED
        },
        is_current_diff: false,
        diff_index: -1,
        left_highlight: 0,
        right_highlight: 0,
        left_word_diff: SharedString::from(""),
        right_word_diff: SharedString::from(""),
        is_search_match: false,
        is_selected: false,
    };
    vec_model.insert(insert_at, new_row);

    renumber_diff_lines(vec_model, insert_at);

    mark_dirty_editing(window, state);
    window.set_can_undo(true);

    // Move focus to the newly inserted row
    if is_left {
        window.set_diff_edit_focus_row(insert_at as i32);
    } else {
        window.set_diff_edit_focus_right_row(insert_at as i32);
    }
}

/// Delete a row (Backspace at start of line). Removes from the shared diff model
/// and syncs left_lines / right_lines.
pub fn delete_line(window: &MainWindow, state: &mut AppState, line_index: i32, is_left: bool) {
    if line_index < 0 {
        return;
    }

    let_diff_vec_model!(model, vec_model, window);
    let idx = line_index as usize;
    if idx >= vec_model.row_count() {
        return;
    }
    let row = match vec_model.row_data(idx) {
        Some(r) => r,
        None => return,
    };
    let can_delete = if is_left {
        row.left_text.is_empty()
    } else {
        row.right_text.is_empty()
    };
    if can_delete {
        push_undo_snapshot(state, vec_model);

        // Sync left_lines / right_lines: remove the corresponding real line
        let line_no_str = if is_left {
            &row.left_line_no
        } else {
            &row.right_line_no
        };
        if let Ok(line_no) = line_no_str.parse::<usize>() {
            let tab = state.current_tab_mut();
            if is_left {
                if line_no > 0 && line_no <= tab.left_lines.len() {
                    tab.left_lines.remove(line_no - 1);
                }
            } else {
                if line_no > 0 && line_no <= tab.right_lines.len() {
                    tab.right_lines.remove(line_no - 1);
                }
            }
        }

        vec_model.remove(idx);

        renumber_diff_lines(vec_model, idx);

        mark_dirty_editing(window, state);
        window.set_can_undo(true);
    }
    // Find the nearest previous editable (non-ghost) row
    let mut prev = if line_index > 0 { line_index - 1 } else { 0 };
    while prev > 0 {
        if let Some(r) = vec_model.row_data(prev as usize) {
            let has_line_no = if is_left {
                !r.left_line_no.is_empty()
            } else {
                !r.right_line_no.is_empty()
            };
            if has_line_no {
                break;
            }
        }
        prev -= 1;
    }
    if is_left {
        window.set_diff_edit_focus_row(prev);
    } else {
        window.set_diff_edit_focus_right_row(prev);
    }
}

pub fn edit_line(
    window: &MainWindow,
    state: &mut AppState,
    line_index: i32,
    new_text: &str,
    is_left: bool,
) {
    // Always operate on the shared diff model
    let_diff_vec_model!(model, vec_model, window);

    if line_index < 0 || line_index as usize >= vec_model.row_count() {
        return;
    }

    push_undo_snapshot(state, vec_model);

    let mut row = match vec_model.row_data(line_index as usize) {
        Some(r) => r,
        None => return,
    };
    if is_left {
        row.left_text = SharedString::from(new_text);
    } else {
        row.right_text = SharedString::from(new_text);
    }
    vec_model.set_row_data(line_index as usize, row);

    let tab = state.current_tab_mut();
    let Some(data) = vec_model.row_data(line_index as usize) else {
        return;
    };
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

    mark_dirty_editing(window, state);
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
    let_diff_vec_model!(model, vec_model, window);

    let tab = state.current_tab_mut();
    if !extend || tab.selection_start < 0 {
        tab.selection_start = row_idx;
    }
    tab.selection_end = row_idx;

    let sel_min = tab.selection_start.min(tab.selection_end) as usize;
    let sel_max = tab.selection_start.max(tab.selection_end) as usize;

    for i in 0..vec_model.row_count() {
        if let Some(mut row) = vec_model.row_data(i) {
            let selected = i >= sel_min && i <= sel_max;
            if row.is_selected != selected {
                row.is_selected = selected;
                vec_model.set_row_data(i, row);
            }
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

    let_diff_vec_model!(model, vec_model, window);

    let count = sel_max.min(vec_model.row_count().saturating_sub(1)) + 1 - sel_min;
    for i in sel_min..=sel_max.min(vec_model.row_count().saturating_sub(1)) {
        if let Some(mut row) = vec_model.row_data(i) {
            if row.status != STATUS_EQUAL {
                row.right_text = row.left_text.clone();
                row.status = STATUS_EQUAL;
                vec_model.set_row_data(i, row);
            }
        }
    }
    mark_dirty(window, state);
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

    let_diff_vec_model!(model, vec_model, window);

    let count = sel_max.min(vec_model.row_count().saturating_sub(1)) + 1 - sel_min;
    for i in sel_min..=sel_max.min(vec_model.row_count().saturating_sub(1)) {
        if let Some(mut row) = vec_model.row_data(i) {
            if row.status != STATUS_EQUAL {
                row.left_text = row.right_text.clone();
                row.status = STATUS_EQUAL;
                vec_model.set_row_data(i, row);
            }
        }
    }
    mark_dirty(window, state);
    window.set_status_text(SharedString::from(format!(
        "Copied {} lines to left",
        count
    )));
}

pub fn new_blank_text(window: &MainWindow, state: &mut AppState) {
    // Reuse current tab if it is blank; otherwise open a new tab
    let current_is_blank = {
        let tab = state.current_tab();
        tab.view_mode == ViewMode::Blank && tab.left_path.is_none() && tab.right_path.is_none()
    };
    if !current_is_blank {
        add_tab(window, state);
    }

    {
        let tab = state.current_tab_mut();
        tab.view_mode = ViewMode::FileDiff;
        tab.left_path = None;
        tab.right_path = None;
        tab.title = "Untitled".to_string();
        tab.diff_line_data = vec![DiffLineData {
            left_line_no: SharedString::from("1"),
            right_line_no: SharedString::from("1"),
            left_text: SharedString::from(""),
            right_text: SharedString::from(""),
            status: STATUS_EQUAL,
            is_current_diff: false,
            diff_index: -1,
            left_highlight: -1,
            right_highlight: -1,
            left_word_diff: SharedString::from(""),
            right_word_diff: SharedString::from(""),
            is_search_match: false,
            is_selected: false,
        }];
        tab.left_lines = vec![String::new()];
        tab.right_lines = vec![String::new()];
        tab.diff_positions.clear();
        tab.diff_stats = String::new();
        tab.has_unsaved_changes = false;
        tab.editing_dirty = false;
        tab.left_encoding = "UTF-8".to_string();
        tab.right_encoding = "UTF-8".to_string();
        tab.left_eol_type = "LF".to_string();
        tab.right_eol_type = "LF".to_string();
    }

    window.set_view_mode(ViewMode::FileDiff.as_i32());
    let model = ModelRc::new(VecModel::from(state.current_tab().diff_line_data.clone()));
    window.set_diff_lines(model);
    window.set_diff_count(0);
    window.set_current_diff_index(-1);
    window.set_left_path(SharedString::from(""));
    window.set_right_path(SharedString::from(""));
    window.set_has_unsaved_changes(false);
    window.set_left_encoding_display(SharedString::from("UTF-8"));
    window.set_right_encoding_display(SharedString::from("UTF-8"));
    window.set_left_eol_type(SharedString::from("LF"));
    window.set_right_eol_type(SharedString::from("LF"));
    window.set_status_text(SharedString::from("New blank document"));
    sync_tab_list(window, state);
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
                let path_str = path.to_string_lossy().to_string();
                {
                    let tab = state.current_tab_mut();
                    tab.left_path = Some(path);
                    tab.view_mode = ViewMode::FileDiff;
                }
                window.set_open_left_path_input(SharedString::from(&path_str));
                window.set_left_path(SharedString::from(&path_str));
                window.set_view_mode(ViewMode::FileDiff.as_i32());
                run_diff(window, state);
            } else if !has_native_file_dialog() {
                show_picker(11, "Select left file");
            }
        }
        2 => {
            if let Some(path) = open_file_dialog("Select right file") {
                let path_str = path.to_string_lossy().to_string();
                {
                    let tab = state.current_tab_mut();
                    tab.right_path = Some(path);
                    tab.view_mode = ViewMode::FileDiff;
                }
                window.set_open_right_path_input(SharedString::from(&path_str));
                window.set_right_path(SharedString::from(&path_str));
                window.set_view_mode(ViewMode::FileDiff.as_i32());
                run_diff(window, state);
            } else if !has_native_file_dialog() {
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
            } else if !has_native_file_dialog() {
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
            } else if !has_native_file_dialog() {
                show_picker(14, "Select right folder");
            }
        }
        5 => {
            // New compare: show open dialog as modal
            window.set_show_open_dialog(true);
        }
        _ => {}
    }
}
