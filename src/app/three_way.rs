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
        tab.diff_line_data.clear();
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

    let empty_row = ThreeWayLineData {
        base_line_no: SharedString::from("1"),
        left_line_no: SharedString::from("1"),
        right_line_no: SharedString::from("1"),
        base_text: SharedString::from(""),
        left_text: SharedString::from(""),
        right_text: SharedString::from(""),
        status: 0,
        is_current: false,
        conflict_index: -1,
        is_search_match: false,
    };
    window.set_view_mode(ViewMode::ThreeWayText.as_i32());
    window.set_three_way_lines(ModelRc::new(VecModel::from(vec![empty_row])));

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
    let line_data = build_three_way_line_data(&result);

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

    window.set_three_way_lines(ModelRc::new(VecModel::from(line_data)));

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

    let model = window.get_three_way_lines();
    update_three_way_detail_pane(window, &model, tab.current_conflict);

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
    let line_data = build_three_way_line_data(&result);

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

    window.set_three_way_lines(ModelRc::new(VecModel::from(line_data)));

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

    let model = window.get_three_way_lines();
    update_three_way_detail_pane(window, &model, tab.current_conflict);
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
            if let Some(mut row) = vec_model.row_data(i) {
                let should = i == current_pos;
                if row.is_current != should {
                    row.is_current = should;
                    vec_model.set_row_data(i, row);
                }
            }
        }
    }

    // Sync is_current_diff to PaneBuffers
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

    update_three_way_detail_pane(window, &model, new_index);
}

/// Build ThreeWayLineData from ThreeWayResult with block-level conflict_index.
/// All lines in the same contiguous diff block share the same conflict_index.
fn build_three_way_line_data(result: &ThreeWayResult) -> Vec<ThreeWayLineData> {
    // Assign block-level conflict_index: consecutive non-Equal lines share the same index
    let mut block_indices: Vec<i32> = vec![-1; result.lines.len()];
    let mut current_block = -1i32;
    let mut was_in_diff = false;
    for (i, line) in result.lines.iter().enumerate() {
        if line.status != ThreeWayStatus::Equal {
            if !was_in_diff {
                current_block += 1;
                was_in_diff = true;
            }
            block_indices[i] = current_block;
        } else {
            was_in_diff = false;
        }
    }

    result
        .lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let status: i32 = line.status.as_i32();
            let conflict_index = block_indices[i];
            ThreeWayLineData {
                base_line_no: line
                    .base_line_no
                    .map(|n: u32| SharedString::from(n.to_string()))
                    .unwrap_or_default(),
                left_line_no: line
                    .left_line_no
                    .map(|n: u32| SharedString::from(n.to_string()))
                    .unwrap_or_default(),
                right_line_no: line
                    .right_line_no
                    .map(|n: u32| SharedString::from(n.to_string()))
                    .unwrap_or_default(),
                base_text: SharedString::from(&line.base_text),
                left_text: SharedString::from(&line.left_text),
                right_text: SharedString::from(&line.right_text),
                status,
                is_current: conflict_index == 0 && !result.conflict_positions.is_empty(),
                conflict_index,
                is_search_match: false,
            }
        })
        .collect()
}

pub(super) fn update_three_way_detail_pane(
    window: &MainWindow,
    model: &ModelRc<ThreeWayLineData>,
    conflict_index: i32,
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

    let mut left_lines: Vec<DetailLineData> = Vec::new();
    let mut base_lines: Vec<DetailLineData> = Vec::new();
    let mut right_lines: Vec<DetailLineData> = Vec::new();

    let count = model.row_count();
    for i in 0..count {
        let Some(row) = model.row_data(i) else {
            continue;
        };
        if row.conflict_index != conflict_index {
            continue;
        }
        let status = row.status;

        // Left side (always include for 3-way)
        let seg = ModelRc::new(VecModel::from(vec![WordSegment {
            text: row.left_text.clone(),
            is_changed: status == STATUS_ADDED
                || status == STATUS_MODIFIED
                || status == STATUS_MOVED,
        }]));
        left_lines.push(DetailLineData {
            segments: seg,
            is_current: true,
            status,
        });

        // Base (middle) side (always include for 3-way)
        let seg = ModelRc::new(VecModel::from(vec![WordSegment {
            text: row.base_text.clone(),
            is_changed: status == STATUS_MODIFIED || status == STATUS_MOVED,
        }]));
        base_lines.push(DetailLineData {
            segments: seg,
            is_current: true,
            status,
        });

        // Right side (always include for 3-way)
        let seg = ModelRc::new(VecModel::from(vec![WordSegment {
            text: row.right_text.clone(),
            is_changed: status == STATUS_REMOVED
                || status == STATUS_MODIFIED
                || status == STATUS_MOVED,
        }]));
        right_lines.push(DetailLineData {
            segments: seg,
            is_current: true,
            status,
        });
    }

    let has_left = !left_lines.is_empty();
    let has_base = !base_lines.is_empty();
    let has_right = !right_lines.is_empty();
    window.set_detail_left_lines(ModelRc::new(VecModel::from(left_lines)));
    window.set_detail_base_lines(ModelRc::new(VecModel::from(base_lines)));
    window.set_detail_right_lines(ModelRc::new(VecModel::from(right_lines)));
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

    let pos = tab.three_way_conflict_positions[conflict_index as usize];
    let model = window.get_three_way_lines();
    if let Some(vec_model) = model.as_any().downcast_ref::<VecModel<ThreeWayLineData>>() {
        let block_ci = vec_model
            .row_data(pos)
            .map(|r| r.conflict_index)
            .unwrap_or(-1);
        if block_ci < 0 {
            return;
        }
        // Status when source matches the other side: Equal(0).
        // Status when they differ: LeftChanged(1) if right was copied, RightChanged(2) if left was copied.
        let diff_status = if use_left { 2 } else { 1 };
        for i in 0..vec_model.row_count() {
            if let Some(mut row) = vec_model.row_data(i) {
                if row.conflict_index == block_ci {
                    let (src_line_no, src_text) = if use_left {
                        (&row.left_line_no, row.left_text.clone())
                    } else {
                        (&row.right_line_no, row.right_text.clone())
                    };
                    if !src_line_no.is_empty() {
                        row.base_text = src_text.clone();
                        if row.base_line_no.is_empty() {
                            row.base_line_no = SharedString::from("+");
                        }
                    }
                    row.status = if row.left_text == row.right_text {
                        0
                    } else {
                        diff_status
                    };
                    vec_model.set_row_data(i, row);
                }
            }
        }
        // Rebuild tab.base_lines from VecModel (authoritative after copy)
        let tab = state.current_tab_mut();
        tab.base_lines = Vec::new();
        for i in 0..vec_model.row_count() {
            if let Some(row) = vec_model.row_data(i) {
                if !row.base_line_no.is_empty() {
                    tab.base_lines.push(row.base_text.to_string());
                }
            }
        }
        // Rebuild PaneBuffers after conflict resolution
        rebuild_3way_pane_buffers_from_model(window, state, vec_model);
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

    let model = window.get_three_way_lines();
    update_three_way_detail_pane(window, &model, conflict_index);
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
    let_three_way_vec_model!(model, vec_model, window);
    if row_index < 0 || row_index as usize >= vec_model.row_count() {
        return;
    }
    let mut row = match vec_model.row_data(row_index as usize) {
        Some(r) => r,
        None => return,
    };
    match pane {
        0 => row.left_text = SharedString::from(new_text),
        1 => row.base_text = SharedString::from(new_text),
        _ => row.right_text = SharedString::from(new_text),
    }
    vec_model.set_row_data(row_index as usize, row);

    // Sync PaneBuffer row text
    {
        let tab = state.current_tab();
        let buf = match pane {
            0 => &tab.left_buffer,
            1 => &tab.middle_buffer,
            _ => &tab.right_buffer,
        };
        sync_pane_row_text(buf, row_index as usize, new_text);
    }

    // Sync internal line arrays
    let Some(row_ref) = vec_model.row_data(row_index as usize) else {
        return;
    };
    let tab = state.current_tab_mut();
    match pane {
        0 => {
            if let Ok(n) = row_ref.left_line_no.parse::<usize>() {
                if n > 0 && n <= tab.left_lines.len() {
                    tab.left_lines[n - 1] = new_text.to_string();
                }
            }
        }
        1 => {
            if let Ok(n) = row_ref.base_line_no.parse::<usize>() {
                if n > 0 && n <= tab.base_lines.len() {
                    tab.base_lines[n - 1] = new_text.to_string();
                }
            }
        }
        _ => {
            if let Ok(n) = row_ref.right_line_no.parse::<usize>() {
                if n > 0 && n <= tab.right_lines.len() {
                    tab.right_lines[n - 1] = new_text.to_string();
                }
            }
        }
    }
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
    let_three_way_vec_model!(model, vec_model, window);
    let insert_at = (row_index + 1) as usize;
    let count = vec_model.row_count();
    if insert_at > count {
        return;
    }

    // Find the correct insertion position in the internal line array.
    // If the current row is a ghost for this pane (empty line_no), scan backwards
    // to find the last real line number for this pane.
    let parse_pane_line_no = |r: &ThreeWayLineData| -> Option<usize> {
        match pane {
            0 => r.left_line_no.parse::<usize>().ok(),
            1 => r.base_line_no.parse::<usize>().ok(),
            _ => r.right_line_no.parse::<usize>().ok(),
        }
    };
    let mut insert_pos = 0usize;
    // First try the current row
    if let Some(r) = vec_model.row_data(row_index as usize) {
        if let Some(n) = parse_pane_line_no(&r) {
            insert_pos = n;
        } else {
            // Ghost row: scan backwards for the nearest real line in this pane
            for i in (0..row_index as usize).rev() {
                if let Some(r) = vec_model.row_data(i) {
                    if let Some(n) = parse_pane_line_no(&r) {
                        insert_pos = n;
                        break;
                    }
                }
            }
        }
    }
    {
        let tab = state.current_tab_mut();
        let lines = match pane {
            0 => &mut tab.left_lines,
            1 => &mut tab.base_lines,
            _ => &mut tab.right_lines,
        };
        let pos = insert_pos.min(lines.len());
        lines.insert(pos, String::new());
    }

    // Build new row: only the edited pane gets a placeholder line number, others empty
    let new_row = ThreeWayLineData {
        left_line_no: if pane == 0 {
            SharedString::from("?")
        } else {
            SharedString::from("")
        },
        base_line_no: if pane == 1 {
            SharedString::from("?")
        } else {
            SharedString::from("")
        },
        right_line_no: if pane == 2 {
            SharedString::from("?")
        } else {
            SharedString::from("")
        },
        left_text: SharedString::from(""),
        base_text: SharedString::from(""),
        right_text: SharedString::from(""),
        // 1=LeftChanged, 3=BothChanged(base), 2=RightChanged
        status: match pane {
            0 => 1,
            1 => 3,
            _ => 2,
        },
        is_current: false,
        conflict_index: -1,
        is_search_match: false,
    };
    vec_model.insert(insert_at, new_row);
    // Renumber each pane independently
    let mut left_counter = 0usize;
    let mut base_counter = 0usize;
    let mut right_counter = 0usize;
    for i in 0..vec_model.row_count() {
        if let Some(mut r) = vec_model.row_data(i) {
            if !r.left_line_no.is_empty() {
                left_counter += 1;
                r.left_line_no = SharedString::from(left_counter.to_string());
            }
            if !r.base_line_no.is_empty() {
                base_counter += 1;
                r.base_line_no = SharedString::from(base_counter.to_string());
            }
            if !r.right_line_no.is_empty() {
                right_counter += 1;
                r.right_line_no = SharedString::from(right_counter.to_string());
            }
            vec_model.set_row_data(i, r);
        }
    }
    // Rebuild PaneBuffers after structural change
    rebuild_3way_pane_buffers_from_model(window, state, vec_model);

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
    let_three_way_vec_model!(model, vec_model, window);
    let idx = row_index as usize;
    if idx >= vec_model.row_count() {
        return;
    }
    let row = match vec_model.row_data(idx) {
        Some(r) => r,
        None => return,
    };
    let text_empty = match pane {
        0 => row.left_text.is_empty(),
        1 => row.base_text.is_empty(),
        _ => row.right_text.is_empty(),
    };
    // Count how many real lines remain for this pane (non-ghost rows)
    let real_line_count = {
        let mut count = 0usize;
        for i in 0..vec_model.row_count() {
            if let Some(r) = vec_model.row_data(i) {
                let has_line = match pane {
                    0 => !r.left_line_no.is_empty(),
                    1 => !r.base_line_no.is_empty(),
                    _ => !r.right_line_no.is_empty(),
                };
                if has_line {
                    count += 1;
                }
            }
        }
        count
    };
    // Don't delete the last real line — keep at least one row so user can type
    let can_delete = text_empty && real_line_count > 1;
    if can_delete {
        // Sync internal line arrays: remove the corresponding real line
        let line_no_str = match pane {
            0 => &row.left_line_no,
            1 => &row.base_line_no,
            _ => &row.right_line_no,
        };
        if let Ok(line_no) = line_no_str.parse::<usize>() {
            let tab = state.current_tab_mut();
            match pane {
                0 => {
                    if line_no > 0 && line_no <= tab.left_lines.len() {
                        tab.left_lines.remove(line_no - 1);
                    }
                }
                1 => {
                    if line_no > 0 && line_no <= tab.base_lines.len() {
                        tab.base_lines.remove(line_no - 1);
                    }
                }
                _ => {
                    if line_no > 0 && line_no <= tab.right_lines.len() {
                        tab.right_lines.remove(line_no - 1);
                    }
                }
            }
        }

        vec_model.remove(idx);
        // Renumber each pane independently
        let mut left_counter = 0usize;
        let mut base_counter = 0usize;
        let mut right_counter = 0usize;
        for i in 0..vec_model.row_count() {
            if let Some(mut r) = vec_model.row_data(i) {
                if !r.left_line_no.is_empty() {
                    left_counter += 1;
                    r.left_line_no = SharedString::from(left_counter.to_string());
                }
                if !r.base_line_no.is_empty() {
                    base_counter += 1;
                    r.base_line_no = SharedString::from(base_counter.to_string());
                }
                if !r.right_line_no.is_empty() {
                    right_counter += 1;
                    r.right_line_no = SharedString::from(right_counter.to_string());
                }
                vec_model.set_row_data(i, r);
            }
        }
        // Rebuild PaneBuffers after structural change
        rebuild_3way_pane_buffers_from_model(window, state, vec_model);
        mark_dirty_editing(window, state);
    }
    let prev = if row_index > 0 { row_index - 1 } else { 0 };
    match pane {
        0 => window.set_three_way_edit_focus_left_row(prev),
        1 => window.set_three_way_edit_focus_base_row(prev),
        _ => window.set_three_way_edit_focus_right_row(prev),
    }
}

/// Rebuild 3-way PaneBuffers from the shared ThreeWayLineData VecModel.
/// Used for structural changes (insert/delete) and conflict resolution.
fn rebuild_3way_pane_buffers_from_model(
    window: &MainWindow,
    state: &mut AppState,
    vec_model: &VecModel<ThreeWayLineData>,
) {
    let count = vec_model.row_count();
    let mut left_rows: Vec<PaneLineData> = Vec::with_capacity(count);
    let mut middle_rows: Vec<PaneLineData> = Vec::with_capacity(count);
    let mut right_rows: Vec<PaneLineData> = Vec::with_capacity(count);
    let mut left_rtl: Vec<Option<usize>> = Vec::with_capacity(count);
    let mut middle_rtl: Vec<Option<usize>> = Vec::with_capacity(count);
    let mut right_rtl: Vec<Option<usize>> = Vec::with_capacity(count);
    let mut left_ltr: Vec<usize> = Vec::new();
    let mut middle_ltr: Vec<usize> = Vec::new();
    let mut right_ltr: Vec<usize> = Vec::new();
    let mut left_ghosts: Vec<usize> = Vec::new();
    let mut middle_ghosts: Vec<usize> = Vec::new();
    let mut right_ghosts: Vec<usize> = Vec::new();

    for i in 0..count {
        let Some(row) = vec_model.row_data(i) else {
            continue;
        };
        let left_is_ghost = row.left_line_no.is_empty();
        let middle_is_ghost = row.base_line_no.is_empty();
        let right_is_ghost = row.right_line_no.is_empty();

        left_rows.push(PaneLineData {
            line_no: row.left_line_no.clone(),
            text: row.left_text.clone(),
            is_ghost: left_is_ghost,
            status: row.status,
            diff_index: row.conflict_index,
            word_diff: SharedString::default(),
            is_current_diff: row.is_current,
            is_search_match: row.is_search_match,
            is_selected: false,
            highlight: -1,
        });
        middle_rows.push(PaneLineData {
            line_no: row.base_line_no.clone(),
            text: row.base_text.clone(),
            is_ghost: middle_is_ghost,
            status: row.status,
            diff_index: row.conflict_index,
            word_diff: SharedString::default(),
            is_current_diff: row.is_current,
            is_search_match: row.is_search_match,
            is_selected: false,
            highlight: -1,
        });
        right_rows.push(PaneLineData {
            line_no: row.right_line_no.clone(),
            text: row.right_text.clone(),
            is_ghost: right_is_ghost,
            status: row.status,
            diff_index: row.conflict_index,
            word_diff: SharedString::default(),
            is_current_diff: row.is_current,
            is_search_match: row.is_search_match,
            is_selected: false,
            highlight: -1,
        });

        if left_is_ghost {
            left_rtl.push(None);
            left_ghosts.push(i);
        } else {
            left_rtl.push(Some(left_ltr.len()));
            left_ltr.push(i);
        }
        if middle_is_ghost {
            middle_rtl.push(None);
            middle_ghosts.push(i);
        } else {
            middle_rtl.push(Some(middle_ltr.len()));
            middle_ltr.push(i);
        }
        if right_is_ghost {
            right_rtl.push(None);
            right_ghosts.push(i);
        } else {
            right_rtl.push(Some(right_ltr.len()));
            right_ltr.push(i);
        }
    }

    let left_model = std::rc::Rc::new(VecModel::from(left_rows));
    let middle_model = std::rc::Rc::new(VecModel::from(middle_rows));
    let right_model = std::rc::Rc::new(VecModel::from(right_rows));
    window.set_left_lines(ModelRc::from(left_model.clone()));
    window.set_middle_lines(ModelRc::from(middle_model.clone()));
    window.set_right_lines(ModelRc::from(right_model.clone()));

    let tab = state.current_tab_mut();
    tab.left_buffer = Some(PaneBuffer {
        model: left_model,
        row_to_line: left_rtl,
        line_to_row: left_ltr,
        ghost_rows: left_ghosts,
    });
    tab.middle_buffer = Some(PaneBuffer {
        model: middle_model,
        row_to_line: middle_rtl,
        line_to_row: middle_ltr,
        ghost_rows: middle_ghosts,
    });
    tab.right_buffer = Some(PaneBuffer {
        model: right_model,
        row_to_line: right_rtl,
        line_to_row: right_ltr,
        ghost_rows: right_ghosts,
    });
}
