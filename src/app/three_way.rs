use super::*;

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
        tab.view_mode = ViewMode::ThreeWayText;
    }
    run_three_way_diff(window, state);
}

pub fn new_blank_text_3way(window: &MainWindow, state: &mut AppState) {
    let current_is_blank = {
        let tab = state.current_tab();
        tab.view_mode == ViewMode::Blank && tab.left_path.is_none() && tab.right_path.is_none()
    };
    if !current_is_blank {
        add_tab(window, state);
    }

    {
        let tab = state.current_tab_mut();
        tab.view_mode = ViewMode::ThreeWayText;
        tab.left_path = None;
        tab.right_path = None;
        tab.base_path = None;
        tab.title = "Untitled (3-way)".to_string();
        tab.diff_positions.clear();
        tab.diff_stats = String::new();
        tab.has_unsaved_changes = false;
        tab.editing_dirty = false;
        tab.left_lines = vec![String::new()];
        tab.right_lines = vec![String::new()];
        tab.base_lines = vec![String::new()];
        tab.left_encoding = "UTF-8".to_string();
        tab.right_encoding = "UTF-8".to_string();
        tab.left_eol_type = "LF".to_string();
        tab.right_eol_type = "LF".to_string();
    }

    window.set_view_mode(ViewMode::ThreeWayText.as_i32());

    // Build per-pane PaneBuffers for blank 3-way
    let blank_pane_row = PaneLineData {
        line_no: SharedString::from("1"),
        text: SharedString::from(""),
        is_ghost: false,
        status: 0,
        diff_index: -1,
        word_diff: SharedString::default(),
        is_current_diff: false,
        is_search_match: false,
        is_selected: false,
        highlight: -1,
    };
    let left_model = std::rc::Rc::new(VecModel::from(vec![blank_pane_row.clone()]));
    let middle_model = std::rc::Rc::new(VecModel::from(vec![blank_pane_row.clone()]));
    let right_model = std::rc::Rc::new(VecModel::from(vec![blank_pane_row]));
    window.set_left_lines(ModelRc::from(left_model.clone()));
    window.set_middle_lines(ModelRc::from(middle_model.clone()));
    window.set_right_lines(ModelRc::from(right_model.clone()));
    {
        let tab = state.current_tab_mut();
        tab.left_buffer = Some(PaneBuffer {
            model: left_model,
            row_to_line: vec![Some(0)],
            line_to_row: vec![0],
            ghost_rows: Vec::new(),
        });
        tab.middle_buffer = Some(PaneBuffer {
            model: middle_model,
            row_to_line: vec![Some(0)],
            line_to_row: vec![0],
            ghost_rows: Vec::new(),
        });
        tab.right_buffer = Some(PaneBuffer {
            model: right_model,
            row_to_line: vec![Some(0)],
            line_to_row: vec![0],
            ghost_rows: Vec::new(),
        });
    }

    window.set_diff_count(0);
    window.set_current_diff_index(-1);
    window.set_left_path(SharedString::from(""));
    window.set_right_path(SharedString::from(""));
    window.set_base_path(SharedString::from(""));
    window.set_has_unsaved_changes(false);
    window.set_left_encoding_display(SharedString::from("UTF-8"));
    window.set_right_encoding_display(SharedString::from("UTF-8"));
    window.set_left_eol_type(SharedString::from("LF"));
    window.set_right_eol_type(SharedString::from("LF"));
    window.set_status_text(SharedString::from("New blank 3-way document"));
    sync_tab_list(window, state);
}

pub fn run_three_way_diff(window: &MainWindow, state: &mut AppState) {
    let tab = state.current_tab();
    let (base_path, left_path, right_path) = match (&tab.base_path, &tab.left_path, &tab.right_path)
    {
        (Some(b), Some(l), Some(r)) => (b.clone(), l.clone(), r.clone()),
        _ => return,
    };

    let base_bytes = match read_file_or_report(window, &base_path) {
        Some(b) => b,
        None => return,
    };
    let left_bytes = match read_file_or_report(window, &left_path) {
        Some(b) => b,
        None => return,
    };
    let right_bytes = match read_file_or_report(window, &right_path) {
        Some(b) => b,
        None => return,
    };

    // Route CSV files to 3-way table comparison
    if is_csv_path(&left_path) && is_csv_path(&right_path) && is_csv_path(&base_path) {
        run_csv_compare_3way(
            window,
            state,
            &base_bytes,
            &left_bytes,
            &right_bytes,
            &base_path,
            &left_path,
            &right_path,
        );
        return;
    }

    let (base_text, base_enc) = decode_file(&base_bytes);
    let (left_text, left_enc) = decode_file(&left_bytes);
    let (right_text, right_enc) = decode_file(&right_bytes);

    let result = compute_three_way_diff(&base_text, &left_text, &right_text);

    let left_name = path_file_name(&left_path);
    let right_name = path_file_name(&right_path);

    let tab = state.current_tab_mut();
    tab.three_way_conflict_positions = result.conflict_positions.clone();
    tab.current_conflict = if result.conflict_positions.is_empty() {
        -1
    } else {
        0
    };
    tab.view_mode = ViewMode::ThreeWayText;
    tab.left_encoding = left_enc.to_string();
    tab.right_encoding = right_enc.to_string();
    tab.base_encoding = base_enc.to_string();
    tab.title = format!("{} ↔ {} (3-way)", left_name, right_name);

    // Build per-pane PaneBuffers
    let (left_buf, middle_buf, right_buf) = build_pane_buffers_3way(&result);
    window.set_left_lines(ModelRc::from(left_buf.model.clone()));
    window.set_middle_lines(ModelRc::from(middle_buf.model.clone()));
    window.set_right_lines(ModelRc::from(right_buf.model.clone()));
    tab.left_buffer = Some(left_buf);
    tab.middle_buffer = Some(middle_buf);
    tab.right_buffer = Some(right_buf);

    window.set_conflict_count(result.conflict_positions.len() as i32);
    window.set_current_conflict_index(tab.current_conflict);
    window.set_view_mode(ViewMode::ThreeWayText.as_i32());
    window.set_left_path(SharedString::from(left_path.to_string_lossy().to_string()));
    window.set_right_path(SharedString::from(right_path.to_string_lossy().to_string()));
    window.set_base_path(SharedString::from(base_path.to_string_lossy().to_string()));
    window.set_status_text(SharedString::from(format!(
        "{} differences found",
        result.conflict_positions.len()
    )));

    update_three_way_detail_pane(window, tab.current_conflict, tab);

    sync_tab_list(window, state);
}

/// Recompute 3-way diff from in-memory text (for rescan of edited/blank 3-way docs).
pub fn recompute_three_way_from_text(
    window: &MainWindow,
    state: &mut AppState,
    base_text: &str,
    left_text: &str,
    right_text: &str,
) {
    let result = compute_three_way_diff(base_text, left_text, right_text);

    let tab = state.current_tab_mut();
    tab.three_way_conflict_positions = result.conflict_positions.clone();
    tab.current_conflict = if result.conflict_positions.is_empty() {
        -1
    } else {
        0
    };
    tab.editing_dirty = false;
    tab.left_lines = left_text.lines().map(String::from).collect();
    tab.right_lines = right_text.lines().map(String::from).collect();
    tab.base_lines = base_text.lines().map(String::from).collect();

    // Build per-pane PaneBuffers
    let (left_buf, middle_buf, right_buf) = build_pane_buffers_3way(&result);
    window.set_left_lines(ModelRc::from(left_buf.model.clone()));
    window.set_middle_lines(ModelRc::from(middle_buf.model.clone()));
    window.set_right_lines(ModelRc::from(right_buf.model.clone()));
    tab.left_buffer = Some(left_buf);
    tab.middle_buffer = Some(middle_buf);
    tab.right_buffer = Some(right_buf);

    // BUG-2: Reset stale focus state after model replacement
    window.set_three_way_edit_focus_left_row(-1);
    window.set_three_way_edit_focus_base_row(-1);
    window.set_three_way_edit_focus_right_row(-1);
    window.set_conflict_count(result.conflict_positions.len() as i32);
    window.set_current_conflict_index(tab.current_conflict);

    update_three_way_detail_pane(window, tab.current_conflict, tab);
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

    // Update is_current_diff on PaneBuffers (authoritative source)
    let tab = state.current_tab();
    for buf_opt in [&tab.left_buffer, &tab.middle_buffer, &tab.right_buffer] {
        if let Some(buf) = buf_opt {
            for i in 0..buf.model.row_count() {
                if let Some(mut row) = buf.model.row_data(i) {
                    let should = i == current_pos;
                    if row.is_current_diff != should {
                        row.is_current_diff = should;
                        buf.model.set_row_data(i, row);
                    }
                }
            }
        }
    }

    window.set_status_text(SharedString::from(format!(
        "Conflict {} of {}",
        new_index + 1,
        total
    )));

    update_three_way_detail_pane(window, new_index, state.current_tab());
}

pub(super) fn update_three_way_detail_pane(
    window: &MainWindow,
    conflict_index: i32,
    tab: &TabState,
) {
    if conflict_index < 0 {
        window.set_detail_has_left(false);
        window.set_detail_has_base(false);
        window.set_detail_has_right(false);
        window.set_detail_left_lines(ModelRc::new(VecModel::from(Vec::<DetailLineData>::new())));
        window.set_detail_base_lines(ModelRc::new(VecModel::from(Vec::<DetailLineData>::new())));
        window.set_detail_right_lines(ModelRc::new(VecModel::from(Vec::<DetailLineData>::new())));
        return;
    }

    let mut left_detail: Vec<DetailLineData> = Vec::new();
    let mut base_detail: Vec<DetailLineData> = Vec::new();
    let mut right_detail: Vec<DetailLineData> = Vec::new();

    // Use left_buffer as the representative for row count and status/diff_index
    if let (Some(lb), Some(mb), Some(rb)) =
        (&tab.left_buffer, &tab.middle_buffer, &tab.right_buffer)
    {
        let count = lb.model.row_count();
        for i in 0..count {
            let (Some(lr), Some(mr), Some(rr)) = (
                lb.model.row_data(i),
                mb.model.row_data(i),
                rb.model.row_data(i),
            ) else {
                continue;
            };
            if lr.diff_index != conflict_index {
                continue;
            }
            let status = lr.status;

            // Left side
            let seg = ModelRc::new(VecModel::from(vec![WordSegment {
                text: lr.text.clone(),
                is_changed: status == STATUS_ADDED
                    || status == STATUS_MODIFIED
                    || status == STATUS_MOVED,
            }]));
            left_detail.push(DetailLineData {
                segments: seg,
                is_current: true,
                status,
            });

            // Base (middle) side
            let seg = ModelRc::new(VecModel::from(vec![WordSegment {
                text: mr.text.clone(),
                is_changed: status == STATUS_MODIFIED || status == STATUS_MOVED,
            }]));
            base_detail.push(DetailLineData {
                segments: seg,
                is_current: true,
                status,
            });

            // Right side
            let seg = ModelRc::new(VecModel::from(vec![WordSegment {
                text: rr.text.clone(),
                is_changed: status == STATUS_REMOVED
                    || status == STATUS_MODIFIED
                    || status == STATUS_MOVED,
            }]));
            right_detail.push(DetailLineData {
                segments: seg,
                is_current: true,
                status,
            });
        }
    }

    let has_left = !left_detail.is_empty();
    let has_base = !base_detail.is_empty();
    let has_right = !right_detail.is_empty();
    window.set_detail_left_lines(ModelRc::new(VecModel::from(left_detail)));
    window.set_detail_base_lines(ModelRc::new(VecModel::from(base_detail)));
    window.set_detail_right_lines(ModelRc::new(VecModel::from(right_detail)));
    window.set_detail_has_left(has_left);
    window.set_detail_has_base(has_base);
    window.set_detail_has_right(has_right);
    window.set_detail_left_scroll_y(0.0);
    window.set_detail_base_scroll_y(0.0);
    window.set_detail_right_scroll_y(0.0);
}

/// Copy left text to base (middle) pane for the given diff index.
pub fn resolve_conflict_use_left(window: &MainWindow, state: &mut AppState, conflict_index: i32) {
    resolve_conflict_copy_to_base(window, state, conflict_index, true);
}

/// Copy right text to base (middle) pane for the given diff index.
pub fn resolve_conflict_use_right(window: &MainWindow, state: &mut AppState, conflict_index: i32) {
    resolve_conflict_copy_to_base(window, state, conflict_index, false);
}

/// Copy left or right text to base (middle) pane for the given diff index.
fn resolve_conflict_copy_to_base(
    window: &MainWindow,
    state: &mut AppState,
    conflict_index: i32,
    use_left: bool,
) {
    let tab = state.current_tab();
    if conflict_index < 0 || conflict_index as usize >= tab.three_way_conflict_positions.len() {
        return;
    }

    // Determine the block's diff_index from the conflict position
    let pos = tab.three_way_conflict_positions[conflict_index as usize];
    let block_di = tab
        .left_buffer
        .as_ref()
        .and_then(|b| b.model.row_data(pos))
        .map(|r| r.diff_index)
        .unwrap_or(-1);
    if block_di < 0 {
        return;
    }

    let (Some(lb), Some(mb), Some(rb)) = (&tab.left_buffer, &tab.middle_buffer, &tab.right_buffer)
    else {
        return;
    };

    // Status when source matches the other side: Equal(0).
    // Status when they differ: LeftChanged(1) if right was copied, RightChanged(2) if left was copied.
    let diff_status = if use_left { 2 } else { 1 };
    let count = lb.model.row_count();
    for i in 0..count {
        let (Some(lr), Some(mut mr), Some(rr)) = (
            lb.model.row_data(i),
            mb.model.row_data(i),
            rb.model.row_data(i),
        ) else {
            continue;
        };
        if lr.diff_index != block_di {
            continue;
        }

        let (src_is_ghost, src_text) = if use_left {
            (lr.is_ghost, lr.text.clone())
        } else {
            (rr.is_ghost, rr.text.clone())
        };

        if !src_is_ghost {
            mr.text = src_text;
            if mr.is_ghost {
                // Convert ghost to real line in middle pane
                mr.is_ghost = false;
                mr.line_no = SharedString::from("+");
            }
        }

        // Compute new status for all three panes
        let new_status = if lr.text == rr.text { 0 } else { diff_status };
        mr.status = new_status;
        mb.model.set_row_data(i, mr);

        // Update status on left and right panes too
        if let Some(mut lr2) = lb.model.row_data(i) {
            if lr2.status != new_status {
                lr2.status = new_status;
                lb.model.set_row_data(i, lr2);
            }
        }
        if let Some(mut rr2) = rb.model.row_data(i) {
            if rr2.status != new_status {
                rr2.status = new_status;
                rb.model.set_row_data(i, rr2);
            }
        }
    }

    let tab = state.current_tab_mut();
    tab.has_unsaved_changes = true;
    tab.editing_dirty = true;
    window.set_has_unsaved_changes(true);
    let msg = if use_left {
        "Left → Middle copied"
    } else {
        "Right → Middle copied"
    };
    window.set_status_text(SharedString::from(msg));

    // BUG-3: Clear stale focus state after copy
    window.set_three_way_edit_focus_left_row(-1);
    window.set_three_way_edit_focus_base_row(-1);
    window.set_three_way_edit_focus_right_row(-1);

    let tab = state.current_tab();
    update_three_way_detail_pane(window, conflict_index, tab);
}

/// Copy left to middle and advance to next diff block.
pub fn resolve_use_left_and_next(window: &MainWindow, state: &mut AppState) {
    let conflict_index = state.current_tab().current_conflict;
    resolve_conflict_use_left(window, state, conflict_index);
    navigate_conflict(window, state, true);
}

/// Copy right to middle and advance to next diff block.
pub fn resolve_use_right_and_next(window: &MainWindow, state: &mut AppState) {
    let conflict_index = state.current_tab().current_conflict;
    resolve_conflict_use_right(window, state, conflict_index);
    navigate_conflict(window, state, true);
}

/// Copy all left diffs to middle (resolve all with left).
pub fn resolve_all_use_left(window: &MainWindow, state: &mut AppState) {
    let tab = state.current_tab();
    let count = tab.three_way_conflict_positions.len();
    if count == 0 {
        return;
    }
    // Resolve from last to first to avoid index shifts
    for i in (0..count as i32).rev() {
        resolve_conflict_use_left(window, state, i);
    }
    window.set_status_text(SharedString::from(format!(
        "All {} blocks: Left → Middle",
        count
    )));
}

/// Copy all right diffs to middle (resolve all with right).
pub fn resolve_all_use_right(window: &MainWindow, state: &mut AppState) {
    let tab = state.current_tab();
    let count = tab.three_way_conflict_positions.len();
    if count == 0 {
        return;
    }
    for i in (0..count as i32).rev() {
        resolve_conflict_use_right(window, state, i);
    }
    window.set_status_text(SharedString::from(format!(
        "All {} blocks: Right → Middle",
        count
    )));
}

pub fn three_way_edit_line(
    window: &MainWindow,
    state: &mut AppState,
    row_index: i32,
    new_text: &str,
    pane: i32,
) {
    if row_index < 0 {
        return;
    }
    let tab = state.current_tab();
    let buf_opt = match pane {
        0 => &tab.left_buffer,
        1 => &tab.middle_buffer,
        _ => &tab.right_buffer,
    };
    if let Some(buf) = buf_opt {
        if row_index as usize >= buf.model.row_count() {
            return;
        }
    } else {
        return;
    }
    // Update PaneBuffer (authoritative source)
    sync_pane_row_text(buf_opt, row_index as usize, new_text);
    mark_dirty_editing(window, state);
}

/// Insert a blank row after `row_index` in the 3-way view. pane: 0=left, 1=base, 2=right.
pub fn three_way_insert_line_after(
    window: &MainWindow,
    state: &mut AppState,
    row_index: i32,
    pane: i32,
) {
    if row_index < 0 {
        return;
    }
    let tab = state.current_tab();
    let (Some(lb), Some(mb), Some(rb)) = (&tab.left_buffer, &tab.middle_buffer, &tab.right_buffer)
    else {
        return;
    };
    let insert_at = (row_index + 1) as usize;
    if insert_at > lb.model.row_count() {
        return;
    }

    // 1=LeftChanged, 3=BothChanged(base), 2=RightChanged
    let status = match pane {
        0 => 1,
        1 => 3,
        _ => 2,
    };

    // For the target pane: insert a real row. For the other two: insert ghost rows.
    let bufs: [&PaneBuffer; 3] = [lb, mb, rb];
    for (idx, buf) in bufs.iter().enumerate() {
        let is_target =
            (pane == 0 && idx == 0) || (pane == 1 && idx == 1) || (pane >= 2 && idx == 2);
        let new_row = PaneLineData {
            line_no: if is_target {
                SharedString::from("?")
            } else {
                SharedString::default()
            },
            text: SharedString::default(),
            is_ghost: !is_target,
            status,
            diff_index: -1,
            word_diff: SharedString::default(),
            is_current_diff: false,
            is_search_match: false,
            is_selected: false,
            highlight: -1,
        };
        buf.model.insert(insert_at, new_row);
    }

    // Renumber all three PaneBuffers and rebuild metadata
    let tab = state.current_tab_mut();
    for buf_opt in [
        &mut tab.left_buffer,
        &mut tab.middle_buffer,
        &mut tab.right_buffer,
    ] {
        if let Some(buf) = buf_opt {
            renumber_pane_buffer(buf);
        }
    }

    // Move focus to the new row in the correct pane
    match pane {
        0 => window.set_three_way_edit_focus_left_row(insert_at as i32),
        1 => window.set_three_way_edit_focus_base_row(insert_at as i32),
        _ => window.set_three_way_edit_focus_right_row(insert_at as i32),
    }
    mark_dirty_editing(window, state);
}

/// Delete the row at `row_index` in the 3-way view if the pane's cell is empty.
pub fn three_way_delete_line(window: &MainWindow, state: &mut AppState, row_index: i32, pane: i32) {
    if row_index < 0 {
        return;
    }
    let tab = state.current_tab();
    let buf_opt = match pane {
        0 => &tab.left_buffer,
        1 => &tab.middle_buffer,
        _ => &tab.right_buffer,
    };
    let Some(buf) = buf_opt else { return };
    let idx = row_index as usize;
    if idx >= buf.model.row_count() {
        return;
    }
    let Some(row) = buf.model.row_data(idx) else {
        return;
    };
    let text_empty = row.text.is_empty();

    // Count how many real lines remain for this pane (non-ghost rows)
    let real_line_count = buf.line_to_row.len();

    // Don't delete the last real line — keep at least one row so user can type
    let can_delete = text_empty && real_line_count > 1;
    if can_delete {
        // Remove row from all three PaneBuffer models in lockstep
        let tab = state.current_tab();
        for buf_opt in [&tab.left_buffer, &tab.middle_buffer, &tab.right_buffer] {
            if let Some(buf) = buf_opt {
                buf.model.remove(idx);
            }
        }
        // Renumber all three PaneBuffers and rebuild metadata
        let tab = state.current_tab_mut();
        for buf_opt in [
            &mut tab.left_buffer,
            &mut tab.middle_buffer,
            &mut tab.right_buffer,
        ] {
            if let Some(buf) = buf_opt {
                renumber_pane_buffer(buf);
            }
        }
        mark_dirty_editing(window, state);
    }
    let prev = if row_index > 0 { row_index - 1 } else { 0 };
    match pane {
        0 => window.set_three_way_edit_focus_left_row(prev),
        1 => window.set_three_way_edit_focus_base_row(prev),
        _ => window.set_three_way_edit_focus_right_row(prev),
    }
}
