use std::rc::Rc;

use super::*;

pub(super) fn push_undo_snapshot(state: &mut AppState) {
    let tab = state.current_tab();
    let left_text = tab
        .left_buffer
        .as_ref()
        .map(|b| extract_real_lines(b))
        .unwrap_or_else(|| "\n".to_string());
    let right_text = tab
        .right_buffer
        .as_ref()
        .map(|b| extract_real_lines(b))
        .unwrap_or_else(|| "\n".to_string());
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

    // Save current state to redo stack (derive from PaneBuffers)
    let current_left = tab
        .left_buffer
        .as_ref()
        .map(|b| extract_real_lines(b))
        .unwrap_or_else(|| "\n".to_string());
    let current_right = tab
        .right_buffer
        .as_ref()
        .map(|b| extract_real_lines(b))
        .unwrap_or_else(|| "\n".to_string());
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

    // Save current state to undo stack (derive from PaneBuffers)
    let current_left = tab
        .left_buffer
        .as_ref()
        .map(|b| extract_real_lines(b))
        .unwrap_or_else(|| "\n".to_string());
    let current_right = tab
        .right_buffer
        .as_ref()
        .map(|b| extract_real_lines(b))
        .unwrap_or_else(|| "\n".to_string());
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

    push_undo_snapshot(state);

    let tab = state.current_tab();
    let left_text = tab
        .left_buffer
        .as_ref()
        .map(|b| extract_real_lines(b))
        .unwrap_or_else(|| "\n".to_string());
    let right_text = rebuild_right_after_copy_from_left_buffers(tab, diff_index);

    mark_dirty(window, state);

    recompute_diff_from_text(window, state, &left_text, &right_text);

    // Update diff index without scrolling — copy-only should not move the view
    let tab = state.current_tab_mut();
    if !tab.diff_positions.is_empty() {
        let new_idx = (diff_index as usize).min(tab.diff_positions.len() - 1) as i32;
        tab.current_diff = new_idx;
        window.set_current_diff_index(new_idx);
        window.set_status_text(SharedString::from(format!(
            "Difference {} of {} [{}]",
            new_idx + 1,
            tab.diff_positions.len(),
            tab.diff_stats
        )));
    }
    window.set_can_undo(true);
    window.set_can_redo(false);
    sync_tab_list(window, state);
}

pub fn copy_right_and_next(window: &MainWindow, state: &mut AppState) {
    let diff_index = state.current_tab().current_diff;
    copy_to_right(window, state, diff_index);
    // After copy, current_diff already points to the next diff (shifted),
    // so use update_current_diff to scroll to it
    let tab = state.current_tab();
    if !tab.diff_positions.is_empty() {
        let idx = tab.current_diff.min(tab.diff_positions.len() as i32 - 1);
        update_current_diff(window, state, idx);
    }
}

pub fn copy_left_and_next(window: &MainWindow, state: &mut AppState) {
    let diff_index = state.current_tab().current_diff;
    copy_to_left(window, state, diff_index);
    let tab = state.current_tab();
    if !tab.diff_positions.is_empty() {
        let idx = tab.current_diff.min(tab.diff_positions.len() as i32 - 1);
        update_current_diff(window, state, idx);
    }
}

pub fn copy_to_left(window: &MainWindow, state: &mut AppState, diff_index: i32) {
    let tab = state.current_tab();
    if diff_index < 0 || diff_index as usize >= tab.diff_positions.len() {
        return;
    }

    push_undo_snapshot(state);

    let tab = state.current_tab();
    let right_text = tab
        .right_buffer
        .as_ref()
        .map(|b| extract_real_lines(b))
        .unwrap_or_else(|| "\n".to_string());
    let left_text = rebuild_left_after_copy_from_right_buffers(tab, diff_index);

    mark_dirty(window, state);

    recompute_diff_from_text(window, state, &left_text, &right_text);

    // Update diff index without scrolling
    let tab = state.current_tab_mut();
    if !tab.diff_positions.is_empty() {
        let new_idx = (diff_index as usize).min(tab.diff_positions.len() - 1) as i32;
        tab.current_diff = new_idx;
        window.set_current_diff_index(new_idx);
        window.set_status_text(SharedString::from(format!(
            "Difference {} of {} [{}]",
            new_idx + 1,
            tab.diff_positions.len(),
            tab.diff_stats
        )));
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

    push_undo_snapshot(state);

    // Copy all left to right: right becomes identical to left
    let left_text = state
        .current_tab()
        .left_buffer
        .as_ref()
        .map(|b| extract_real_lines(b))
        .unwrap_or_else(|| "\n".to_string());

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

    push_undo_snapshot(state);

    // Left becomes right for all diffs
    let right_text = state
        .current_tab()
        .right_buffer
        .as_ref()
        .map(|b| extract_real_lines(b))
        .unwrap_or_else(|| "\n".to_string());

    mark_dirty(window, state);

    recompute_diff_from_text(window, state, &right_text, &right_text);

    window.set_can_undo(true);
    window.set_can_redo(false);
    sync_tab_list(window, state);
}

pub fn copy_all_text(window: &MainWindow, state: &AppState, is_left: bool) {
    let tab = state.current_tab();
    let buf = if is_left {
        &tab.left_buffer
    } else {
        &tab.right_buffer
    };
    let text = buf
        .as_ref()
        .map(|b| extract_real_lines(b))
        .unwrap_or_default();
    // extract_real_lines adds trailing newline; trim for clipboard
    let text = text.trim_end_matches('\n').to_string();

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
    {
        let tab = state.current_tab();
        let buf = if is_left {
            &tab.left_buffer
        } else {
            &tab.right_buffer
        };
        let Some(b) = buf else { return };
        if line_index < 0 || line_index as usize >= b.model.row_count() {
            return;
        }
    }

    push_undo_snapshot(state);

    let insert_at = (line_index + 1) as usize;

    // Insert real row in target pane, ghost row in other pane
    let status = if is_left {
        STATUS_REMOVED
    } else {
        STATUS_ADDED
    };
    let real_row = PaneLineData {
        line_no: SharedString::from("?"),
        text: SharedString::from(""),
        is_ghost: false,
        status,
        diff_index: -1,
        word_diff: SharedString::from(""),
        is_current_diff: false,
        is_search_match: false,
        is_selected: false,
        highlight: -1,
    };
    let ghost_row = PaneLineData {
        line_no: SharedString::from(""),
        text: SharedString::from(""),
        is_ghost: true,
        status: STATUS_EQUAL,
        diff_index: -1,
        word_diff: SharedString::from(""),
        is_current_diff: false,
        is_search_match: false,
        is_selected: false,
        highlight: -1,
    };

    {
        let tab = state.current_tab();
        if is_left {
            tab.left_buffer
                .as_ref()
                .unwrap()
                .model
                .insert(insert_at, real_row);
            tab.right_buffer
                .as_ref()
                .unwrap()
                .model
                .insert(insert_at, ghost_row);
        } else {
            tab.left_buffer
                .as_ref()
                .unwrap()
                .model
                .insert(insert_at, ghost_row);
            tab.right_buffer
                .as_ref()
                .unwrap()
                .model
                .insert(insert_at, real_row);
        }
    }

    // Renumber line numbers in both PaneBuffers
    let tab = state.current_tab_mut();
    if let Some(lb) = &mut tab.left_buffer {
        renumber_pane_buffer(lb);
    }
    if let Some(rb) = &mut tab.right_buffer {
        renumber_pane_buffer(rb);
    }

    mark_dirty_editing(window, state);
    window.set_can_undo(true);

    // Move focus to the newly inserted row
    if is_left {
        window.set_diff_edit_focus_row(insert_at as i32);
    } else {
        window.set_diff_edit_focus_right_row(insert_at as i32);
    }
}

/// Delete a row (Backspace at start of line). Removes from both PaneBuffers.
pub fn delete_line(window: &MainWindow, state: &mut AppState, line_index: i32, is_left: bool) {
    if line_index < 0 {
        return;
    }
    let idx = line_index as usize;

    let can_delete = {
        let tab = state.current_tab();
        let buf = if is_left {
            &tab.left_buffer
        } else {
            &tab.right_buffer
        };
        let Some(b) = buf else { return };
        if idx >= b.model.row_count() {
            return;
        }
        let Some(row) = b.model.row_data(idx) else {
            return;
        };
        row.text.is_empty()
    };

    if can_delete {
        push_undo_snapshot(state);

        // Remove row from both PaneBuffers (aligned)
        {
            let tab = state.current_tab();
            if let Some(lb) = &tab.left_buffer {
                lb.model.remove(idx);
            }
            if let Some(rb) = &tab.right_buffer {
                rb.model.remove(idx);
            }
        }

        // Renumber line numbers in both PaneBuffers
        let tab = state.current_tab_mut();
        if let Some(lb) = &mut tab.left_buffer {
            renumber_pane_buffer(lb);
        }
        if let Some(rb) = &mut tab.right_buffer {
            renumber_pane_buffer(rb);
        }

        mark_dirty_editing(window, state);
        window.set_can_undo(true);
    }

    // Find the nearest previous editable (non-ghost) row
    let mut prev = if line_index > 0 { line_index - 1 } else { 0 };
    {
        let tab = state.current_tab();
        let buf = if is_left {
            &tab.left_buffer
        } else {
            &tab.right_buffer
        };
        if let Some(b) = buf {
            while prev > 0 {
                if let Some(r) = b.model.row_data(prev as usize) {
                    if !r.is_ghost {
                        break;
                    }
                }
                prev -= 1;
            }
        }
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
    {
        let tab = state.current_tab();
        let buf = if is_left {
            &tab.left_buffer
        } else {
            &tab.right_buffer
        };
        let Some(b) = buf else { return };
        if line_index < 0 || line_index as usize >= b.model.row_count() {
            return;
        }
    }

    push_undo_snapshot(state);

    // Update PaneBuffer row text (authoritative source)
    {
        let tab = state.current_tab();
        let buf = if is_left {
            &tab.left_buffer
        } else {
            &tab.right_buffer
        };
        sync_pane_row_text(buf, line_index as usize, new_text);
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
    let buf = if is_left {
        &tab.left_buffer
    } else {
        &tab.right_buffer
    };
    if let Some(b) = buf {
        if let Some(row) = b.model.row_data(pos) {
            let text = row.text.to_string();
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

pub fn set_row_selection(_window: &MainWindow, state: &mut AppState, row_idx: i32, extend: bool) {
    let tab = state.current_tab_mut();
    if !extend || tab.selection_start < 0 {
        tab.selection_start = row_idx;
    }
    tab.selection_end = row_idx;

    let sel_min = tab.selection_start.min(tab.selection_end) as usize;
    let sel_max = tab.selection_start.max(tab.selection_end) as usize;

    // Set is_selected on PaneBuffers directly
    let tab = state.current_tab();
    for buf_opt in [&tab.left_buffer, &tab.right_buffer] {
        if let Some(buf) = buf_opt {
            for i in 0..buf.model.row_count() {
                if let Some(mut row) = buf.model.row_data(i) {
                    let selected = i >= sel_min && i <= sel_max;
                    if row.is_selected != selected {
                        row.is_selected = selected;
                        buf.model.set_row_data(i, row);
                    }
                }
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

    let (Some(lb), Some(rb)) = (&tab.left_buffer, &tab.right_buffer) else {
        return;
    };
    let row_count = lb.model.row_count();
    let end = sel_max.min(row_count.saturating_sub(1));
    let count = end + 1 - sel_min;

    for i in sel_min..=end {
        if let (Some(lr), Some(mut rr)) = (lb.model.row_data(i), rb.model.row_data(i)) {
            if !lr.is_ghost && !rr.is_ghost && lr.status != STATUS_EQUAL {
                rr.text = lr.text.clone();
                rr.status = STATUS_EQUAL;
                rb.model.set_row_data(i, rr);
                // Also update left status
                let mut lr2 = lb.model.row_data(i).unwrap();
                lr2.status = STATUS_EQUAL;
                lb.model.set_row_data(i, lr2);
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

    let (Some(lb), Some(rb)) = (&tab.left_buffer, &tab.right_buffer) else {
        return;
    };
    let row_count = lb.model.row_count();
    let end = sel_max.min(row_count.saturating_sub(1));
    let count = end + 1 - sel_min;

    for i in sel_min..=end {
        if let (Some(mut lr), Some(rr)) = (lb.model.row_data(i), rb.model.row_data(i)) {
            if !lr.is_ghost && !rr.is_ghost && rr.status != STATUS_EQUAL {
                lr.text = rr.text.clone();
                lr.status = STATUS_EQUAL;
                lb.model.set_row_data(i, lr);
                // Also update right status
                let mut rr2 = rb.model.row_data(i).unwrap();
                rr2.status = STATUS_EQUAL;
                rb.model.set_row_data(i, rr2);
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
        tab.diff_positions.clear();
        tab.diff_stats = String::new();
        tab.has_unsaved_changes = false;
        tab.editing_dirty = false;
        tab.left_encoding = "UTF-8".to_string();
        tab.right_encoding = "UTF-8".to_string();
        tab.left_eol_type = "LF".to_string();
        tab.right_eol_type = "LF".to_string();
    }

    // Build per-pane buffers for blank document
    {
        let blank_line = PaneLineData {
            line_no: SharedString::from("1"),
            text: SharedString::from(""),
            is_ghost: false,
            status: STATUS_EQUAL,
            diff_index: -1,
            word_diff: SharedString::from(""),
            is_current_diff: false,
            is_search_match: false,
            is_selected: false,
            highlight: -1,
        };
        let left_model = Rc::new(VecModel::from(vec![blank_line.clone()]));
        let right_model = Rc::new(VecModel::from(vec![blank_line]));
        window.set_left_lines(ModelRc::from(left_model.clone()));
        window.set_right_lines(ModelRc::from(right_model.clone()));
        let tab = state.current_tab_mut();
        tab.left_buffer = Some(PaneBuffer {
            model: left_model,
            row_to_line: vec![Some(0)],
            line_to_row: vec![0],
            ghost_rows: vec![],
        });
        tab.right_buffer = Some(PaneBuffer {
            model: right_model,
            row_to_line: vec![Some(0)],
            line_to_row: vec![0],
            ghost_rows: vec![],
        });
        tab.middle_buffer = None;
    }

    window.set_view_mode(ViewMode::FileDiff.as_i32());
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

/// Rebuild what right text would be after copying left→right for one diff block.
/// Reads from PaneBuffers.
fn rebuild_right_after_copy_from_left_buffers(tab: &TabState, target_diff_index: i32) -> String {
    let (Some(lb), Some(rb)) = (&tab.left_buffer, &tab.right_buffer) else {
        return "\n".to_string();
    };
    let mut lines = Vec::new();
    for i in 0..lb.model.row_count() {
        let (Some(lr), Some(rr)) = (lb.model.row_data(i), rb.model.row_data(i)) else {
            continue;
        };
        let in_target = lr.diff_index == target_diff_index || rr.diff_index == target_diff_index;
        if in_target {
            // Copy left→right: take non-ghost left text, skip Added rows (left=ghost)
            if !lr.is_ghost {
                lines.push(lr.text.to_string());
            }
        } else {
            // Keep right side
            if !rr.is_ghost {
                lines.push(rr.text.to_string());
            }
        }
    }
    if lines.is_empty() {
        "\n".to_string()
    } else {
        lines.join("\n") + "\n"
    }
}

/// Rebuild what left text would be after copying right→left for one diff block.
/// Reads from PaneBuffers.
fn rebuild_left_after_copy_from_right_buffers(tab: &TabState, target_diff_index: i32) -> String {
    let (Some(lb), Some(rb)) = (&tab.left_buffer, &tab.right_buffer) else {
        return "\n".to_string();
    };
    let mut lines = Vec::new();
    for i in 0..lb.model.row_count() {
        let (Some(lr), Some(rr)) = (lb.model.row_data(i), rb.model.row_data(i)) else {
            continue;
        };
        let in_target = lr.diff_index == target_diff_index || rr.diff_index == target_diff_index;
        if in_target {
            // Copy right→left: take non-ghost right text, skip Removed rows (right=ghost)
            if !rr.is_ghost {
                lines.push(rr.text.to_string());
            }
        } else {
            // Keep left side
            if !lr.is_ghost {
                lines.push(lr.text.to_string());
            }
        }
    }
    if lines.is_empty() {
        "\n".to_string()
    } else {
        lines.join("\n") + "\n"
    }
}
