use super::*;

pub fn run_diff(window: &MainWindow, state: &mut AppState) {
    let tab = state.current_tab();
    let (left_path, right_path) = match (&tab.left_path, &tab.right_path) {
        (Some(l), Some(r)) => (l.clone(), r.clone()),
        _ => return,
    };

    let left_bytes = match read_file_or_report(window, &left_path) {
        Some(b) => b,
        None => return,
    };
    let right_bytes = match read_file_or_report(window, &right_path) {
        Some(b) => b,
        None => return,
    };

    // ZIP archive comparison
    if (is_zip_bytes(&left_bytes) || is_zip_path(&left_path))
        && (is_zip_bytes(&right_bytes) || is_zip_path(&right_path))
    {
        run_zip_compare(
            window,
            state,
            &left_bytes,
            &right_bytes,
            &left_path,
            &right_path,
        );
        return;
    }

    // Excel comparison
    if is_excel_path(&left_path) && is_excel_path(&right_path) {
        run_excel_compare(
            window,
            state,
            &left_bytes,
            &right_bytes,
            &left_path,
            &right_path,
        );
        return;
    }

    // CSV/TSV comparison
    if is_csv_path(&left_path) && is_csv_path(&right_path) {
        run_csv_compare(
            window,
            state,
            &left_bytes,
            &right_bytes,
            &left_path,
            &right_path,
        );
        return;
    }

    // Image comparison (must be before binary detection; image files contain non-text bytes)
    if is_image_path(&left_path) && is_image_path(&right_path) {
        run_image_compare(
            window,
            state,
            &left_bytes,
            &right_bytes,
            &left_path,
            &right_path,
        );
        return;
    }

    // Binary file detection
    if is_binary(&left_bytes) || is_binary(&right_bytes) {
        let msg = format!(
            "Binary files: Left {} bytes, Right {} bytes — {}",
            left_bytes.len(),
            right_bytes.len(),
            if left_bytes == right_bytes {
                "identical"
            } else {
                "different"
            }
        );
        window.set_diff_lines(ModelRc::new(VecModel::from(Vec::<DiffLineData>::new())));
        window.set_left_lines(ModelRc::new(VecModel::from(Vec::<PaneLineData>::new())));
        window.set_right_lines(ModelRc::new(VecModel::from(Vec::<PaneLineData>::new())));
        window.set_diff_count(0);
        window.set_current_diff_index(-1);
        window.set_status_text(SharedString::from(msg));
        sync_tab_list(window, state);
        return;
    }

    let (left_text, left_enc) = decode_file(&left_bytes);
    let (right_text, right_enc) = decode_file(&right_bytes);

    let tab = state.current_tab_mut();
    tab.left_encoding = left_enc.to_string();
    tab.right_encoding = right_enc.to_string();
    tab.left_eol_type = detect_eol(&left_bytes).to_string();
    tab.right_eol_type = detect_eol(&right_bytes).to_string();
    // Store modification times for auto-rescan
    tab.left_mtime = fs::metadata(&left_path)
        .ok()
        .and_then(|m| m.modified().ok());
    tab.right_mtime = fs::metadata(&right_path)
        .ok()
        .and_then(|m| m.modified().ok());

    // Generate title from filenames
    let left_name = path_file_name(&left_path);
    let right_name = path_file_name(&right_path);
    tab.title = format!("{} ↔ {}", left_name, right_name);

    // Compute syntax highlights (if enabled)
    let left_path_str = left_path.to_string_lossy().to_string();
    let right_path_str = right_path.to_string_lossy().to_string();
    let syntax_enabled = window.get_opt_syntax_highlighting();
    let left_highlights = if syntax_enabled {
        highlight_lines(&left_text, &left_path_str)
    } else {
        Vec::new()
    };
    let right_highlights = if syntax_enabled {
        highlight_lines(&right_text, &right_path_str)
    } else {
        Vec::new()
    };

    recompute_diff_from_text_with_highlights(
        window,
        state,
        &left_text,
        &right_text,
        &left_highlights,
        &right_highlights,
    );

    let tab = state.current_tab();
    let left_type = detect_file_type(&left_path_str);
    let right_type = detect_file_type(&right_path_str);
    let enc_info = format!(
        " [{}  |  {}] ({} / {})",
        tab.left_encoding, tab.right_encoding, left_type, right_type
    );
    let current = window.get_status_text().to_string();
    window.set_status_text(SharedString::from(current + &enc_info));
    window.set_left_encoding_display(SharedString::from(&tab.left_encoding));
    window.set_right_encoding_display(SharedString::from(&tab.right_encoding));
    window.set_left_eol_type(SharedString::from(&tab.left_eol_type));
    window.set_right_eol_type(SharedString::from(&tab.right_eol_type));

    sync_tab_list(window, state);
}

pub fn recompute_diff_from_text(
    window: &MainWindow,
    state: &mut AppState,
    left_text: &str,
    right_text: &str,
) {
    let empty_hl: Vec<i32> = Vec::new();
    recompute_diff_from_text_with_highlights(
        window, state, left_text, right_text, &empty_hl, &empty_hl,
    );
}

pub fn recompute_diff_from_text_with_highlights(
    window: &MainWindow,
    state: &mut AppState,
    left_text: &str,
    right_text: &str,
    left_highlights: &[i32],
    right_highlights: &[i32],
) {
    let tab_width = window.get_opt_tab_width().max(1) as usize;
    let line_count = left_text.lines().count().max(right_text.lines().count());

    // For large files, offload to a background thread to avoid freezing the UI
    if line_count > ASYNC_DIFF_THRESHOLD {
        let tab = state.current_tab_mut();
        tab.is_computing = true;
        let options = tab.diff_options.clone();
        let left_text_owned = left_text.to_string();
        let right_text_owned = right_text.to_string();
        let left_highlights_owned = left_highlights.to_vec();
        let right_highlights_owned = right_highlights.to_vec();
        let left_path = tab.left_path.clone();
        let right_path = tab.right_path.clone();
        let left_encoding = tab.left_encoding.clone();
        let right_encoding = tab.right_encoding.clone();
        let tab_index = state.active_tab;
        let pending_slot = state.pending_diff.clone();

        std::thread::spawn(move || {
            let diff_result =
                compute_diff_with_options(&left_text_owned, &right_text_owned, &options);
            let mut slot = pending_slot.lock().expect("diff pending mutex poisoned");
            *slot = Some(PendingDiffResult {
                tab_index,
                diff_result,
                left_text: left_text_owned,
                right_text: right_text_owned,
                left_highlights: left_highlights_owned,
                right_highlights: right_highlights_owned,
                tab_width,
                left_path,
                right_path,
                left_encoding,
                right_encoding,
            });
        });

        window.set_status_text(SharedString::from(format!(
            "Computing diff... ({} lines)",
            line_count
        )));
        window.set_diff_lines(ModelRc::new(VecModel::from(Vec::<DiffLineData>::new())));
        window.set_left_lines(ModelRc::new(VecModel::from(Vec::<PaneLineData>::new())));
        window.set_right_lines(ModelRc::new(VecModel::from(Vec::<PaneLineData>::new())));
        window.set_diff_count(0);
        window.set_current_diff_index(-1);
        return;
    }

    let tab = state.current_tab_mut();
    let result = compute_diff_with_options(left_text, right_text, &tab.diff_options);
    apply_diff_result(
        window,
        state,
        result,
        left_text,
        right_text,
        left_highlights,
        right_highlights,
        tab_width,
    );
}

/// Apply a computed `DiffResult` to the window and current tab state.
pub(super) fn apply_diff_result(
    window: &MainWindow,
    state: &mut AppState,
    result: DiffResult,
    left_text: &str,
    right_text: &str,
    left_highlights: &[i32],
    right_highlights: &[i32],
    tab_width: usize,
) {
    let tab = state.current_tab_mut();
    tab.is_computing = false;
    tab.editing_dirty = false;

    tab.left_lines = left_text.lines().map(String::from).collect();
    tab.right_lines = right_text.lines().map(String::from).collect();
    let current_diff = if result.diff_positions.is_empty() {
        -1
    } else {
        0
    };

    let diff_line_data =
        build_diff_line_data(&result, left_highlights, right_highlights, tab_width);
    let stats = compute_diff_stats(&result);

    // Build per-pane independent buffers
    let (left_buf, right_buf) =
        build_pane_buffers_2way(&result, left_highlights, right_highlights, tab_width);
    window.set_left_lines(ModelRc::from(left_buf.model.clone()));
    window.set_right_lines(ModelRc::from(right_buf.model.clone()));
    tab.left_buffer = Some(left_buf);
    tab.right_buffer = Some(right_buf);
    tab.middle_buffer = None;

    tab.diff_positions = result.diff_positions;
    tab.current_diff = current_diff;
    let model = ModelRc::new(VecModel::from(diff_line_data.clone()));
    tab.diff_line_data = diff_line_data;
    window.set_diff_lines(model);
    window.set_diff_count(result.diff_count as i32);
    window.set_current_diff_index(tab.current_diff);
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

    let tab = state.current_tab_mut();
    tab.diff_stats = stats.clone();
    sync_diff_stats(window, &stats);

    let status = if result.diff_count == 0 {
        "Files are identical".to_string()
    } else if tab.current_diff >= 0 {
        format!("Difference 1 of {} [{}]", result.diff_count, stats)
    } else {
        format!("{} differences found [{}]", result.diff_count, stats)
    };
    window.set_status_text(SharedString::from(status));

    // Update detail pane
    let model = window.get_diff_lines();
    let tab = state.current_tab();
    update_detail_pane(window, &model, tab.current_diff, tab);
}

/// Build `DiffLineData` vec from a `DiffResult`, applying highlights and tab expansion.
fn build_diff_line_data(
    result: &DiffResult,
    left_highlights: &[i32],
    right_highlights: &[i32],
    tab_width: usize,
) -> Vec<DiffLineData> {
    // Build per-line diff block index: all lines in the same contiguous block share one index
    let mut line_block_idx: Vec<i32> = vec![-1; result.lines.len()];
    let mut current_block = -1i32;
    let mut was_in_diff = false;
    for (i, line) in result.lines.iter().enumerate() {
        if line.status != LineStatus::Equal {
            if !was_in_diff {
                current_block += 1;
                was_in_diff = true;
            }
            line_block_idx[i] = current_block;
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
            let diff_index = line_block_idx[i];

            let left_hl = line
                .left_line_no
                .and_then(|n| left_highlights.get((n - 1) as usize).copied())
                .unwrap_or(-1);
            let right_hl = line
                .right_line_no
                .and_then(|n| right_highlights.get((n - 1) as usize).copied())
                .unwrap_or(-1);

            let left_word_diff = build_word_diff_string(&line.left_word_segments);
            let right_word_diff = build_word_diff_string(&line.right_word_segments);

            DiffLineData {
                left_line_no: line
                    .left_line_no
                    .map(|n| SharedString::from(n.to_string()))
                    .unwrap_or_default(),
                right_line_no: line
                    .right_line_no
                    .map(|n| SharedString::from(n.to_string()))
                    .unwrap_or_default(),
                left_text: SharedString::from(expand_tabs(&line.left_text, tab_width)),
                right_text: SharedString::from(expand_tabs(&line.right_text, tab_width)),
                status,
                is_current_diff: false,
                diff_index,
                left_highlight: left_hl,
                right_highlight: right_hl,
                left_word_diff: SharedString::from(left_word_diff),
                right_word_diff: SharedString::from(right_word_diff),
                is_search_match: false,
                is_selected: false,
            }
        })
        .collect()
}

/// Compute diff statistics string (e.g. "+3 -1 ~2") from a DiffResult.
fn compute_diff_stats(result: &DiffResult) -> String {
    let mut added = 0u32;
    let mut removed = 0u32;
    let mut modified = 0u32;
    for line in &result.lines {
        match line.status {
            LineStatus::Added => added += 1,
            LineStatus::Removed => removed += 1,
            LineStatus::Modified => modified += 1,
            _ => {}
        }
    }
    format!("+{} -{} ~{}", added, removed, modified)
}

/// Poll the shared result slot and apply if a background diff has finished.
/// Call this from a periodic timer on the main thread.
pub fn apply_pending_diff_if_ready(window: &MainWindow, state: &mut AppState) {
    let pending = {
        let mut slot = state
            .pending_diff
            .lock()
            .expect("diff pending mutex poisoned");
        slot.take()
    };
    let Some(p) = pending else { return };

    // Only apply if it matches the currently active tab
    if p.tab_index != state.active_tab {
        return;
    }

    // Update tab paths/encoding from the pending result (already set when spawning, but keep in sync)
    {
        let tab = state.current_tab_mut();
        tab.left_path = p.left_path;
        tab.right_path = p.right_path;
        tab.left_encoding = p.left_encoding;
        tab.right_encoding = p.right_encoding;
    }

    apply_diff_result(
        window,
        state,
        p.diff_result,
        &p.left_text,
        &p.right_text,
        &p.left_highlights,
        &p.right_highlights,
        p.tab_width,
    );
}

pub(super) fn rebuild_left(vec_model: &VecModel<DiffLineData>) -> String {
    let data: Vec<DiffLineData> = (0..vec_model.row_count())
        .filter_map(|i| vec_model.row_data(i))
        .collect();
    let text = rebuild_left_from_data(&data);
    if text.is_empty() {
        "\n".to_string()
    } else {
        text
    }
}

pub(super) fn rebuild_right(vec_model: &VecModel<DiffLineData>) -> String {
    let data: Vec<DiffLineData> = (0..vec_model.row_count())
        .filter_map(|i| vec_model.row_data(i))
        .collect();
    let text = rebuild_right_from_data(&data);
    if text.is_empty() {
        "\n".to_string()
    } else {
        text
    }
}

/// Rebuild text for a specific pane (0=left, 1=base, 2=right) from the 3-way VecModel.
/// Skips ghost lines (empty line_no) since they don't represent real content.
pub(super) fn rebuild_three_way_text(vec_model: &VecModel<ThreeWayLineData>, pane: i32) -> String {
    let mut lines = Vec::new();
    for i in 0..vec_model.row_count() {
        let Some(row) = vec_model.row_data(i) else {
            continue;
        };
        let (line_no, text) = match pane {
            0 => (&row.left_line_no, &row.left_text),
            1 => (&row.base_line_no, &row.base_text),
            _ => (&row.right_line_no, &row.right_text),
        };
        // Only include lines that have a real line number (skip ghost lines)
        if !line_no.is_empty() {
            lines.push(text.to_string());
        }
    }
    if lines.is_empty() {
        String::new()
    } else {
        lines.join("\n") + "\n"
    }
}

pub(super) fn rebuild_right_after_copy_from_left(
    vec_model: &VecModel<DiffLineData>,
    target_diff_index: i32,
) -> String {
    let mut lines = Vec::new();
    for i in 0..vec_model.row_count() {
        let Some(row) = vec_model.row_data(i) else {
            continue;
        };
        if row.diff_index == target_diff_index {
            match row.status {
                STATUS_REMOVED => lines.push(row.left_text.to_string()),
                STATUS_ADDED => continue,
                STATUS_MODIFIED => lines.push(row.left_text.to_string()),
                _ => lines.push(row.right_text.to_string()),
            }
        } else if row.status == STATUS_REMOVED {
            continue;
        } else {
            lines.push(row.right_text.to_string());
        }
    }
    lines.join("\n") + "\n"
}

pub(super) fn rebuild_left_after_copy_from_right(
    vec_model: &VecModel<DiffLineData>,
    target_diff_index: i32,
) -> String {
    let mut lines = Vec::new();
    for i in 0..vec_model.row_count() {
        let Some(row) = vec_model.row_data(i) else {
            continue;
        };
        if row.diff_index == target_diff_index {
            match row.status {
                STATUS_ADDED => lines.push(row.right_text.to_string()),
                STATUS_REMOVED => continue,
                STATUS_MODIFIED => lines.push(row.right_text.to_string()),
                _ => lines.push(row.left_text.to_string()),
            }
        } else if row.status == STATUS_ADDED {
            continue;
        } else {
            lines.push(row.left_text.to_string());
        }
    }
    lines.join("\n") + "\n"
}

pub fn start_compare(
    window: &MainWindow,
    state: &mut AppState,
    left: &str,
    right: &str,
    is_folder: bool,
) {
    let left_path = PathBuf::from(left);
    let right_path = PathBuf::from(right);

    if is_folder {
        {
            let tab = state.current_tab_mut();
            tab.left_folder = Some(left_path);
            tab.right_folder = Some(right_path);
        }
        run_folder_compare(window, state);
    } else {
        {
            let tab = state.current_tab_mut();
            tab.left_path = Some(left_path);
            tab.right_path = Some(right_path);
            tab.view_mode = ViewMode::FileDiff;
        }
        window.set_view_mode(ViewMode::FileDiff.as_i32());
        run_diff(window, state);
    }
}

// --- Auto-rescan: check if files changed on disk ---

pub fn check_files_changed(state: &AppState) -> bool {
    let tab = state.current_tab();
    if !tab.view_mode.is_text_mode() {
        return false;
    }
    // Skip if user is editing inline (don't reload from disk while editing)
    if tab.editing_dirty || tab.has_unsaved_changes {
        return false;
    }
    // External file changes (mtime check)
    if let Some(left_path) = &tab.left_path {
        if let Some(old_mtime) = &tab.left_mtime {
            if let Ok(meta) = fs::metadata(left_path) {
                if let Ok(new_mtime) = meta.modified() {
                    if new_mtime != *old_mtime {
                        return true;
                    }
                }
            }
        }
    }
    if let Some(right_path) = &tab.right_path {
        if let Some(old_mtime) = &tab.right_mtime {
            if let Ok(meta) = fs::metadata(right_path) {
                if let Ok(new_mtime) = meta.modified() {
                    if new_mtime != *old_mtime {
                        return true;
                    }
                }
            }
        }
    }
    false
}

pub fn rescan(window: &MainWindow, state: &mut AppState) {
    let tab = state.current_tab();
    if tab.view_mode == ViewMode::FileDiff
        && tab.left_path.is_some()
        && tab.right_path.is_some()
        && !tab.editing_dirty
        && !tab.has_unsaved_changes
    {
        run_diff(window, state);
        window.set_status_text(SharedString::from("Files rescanned"));
    } else if tab.view_mode == ViewMode::FileDiff {
        // Editing in progress, unsaved changes, or blank document: rebuild from VecModel (authoritative source)
        let model = window.get_diff_lines();
        if let Some(vec_model) = model.as_any().downcast_ref::<VecModel<DiffLineData>>() {
            let left_text = rebuild_left(vec_model);
            let right_text = rebuild_right(vec_model);
            recompute_diff_from_text(window, state, &left_text, &right_text);
            window.set_status_text(SharedString::from("Compared"));
        }
    } else if tab.view_mode == ViewMode::ThreeWayText
        && tab.base_path.is_some()
        && tab.left_path.is_some()
        && tab.right_path.is_some()
        && !tab.editing_dirty
        && !tab.has_unsaved_changes
    {
        run_three_way_diff(window, state);
        window.set_status_text(SharedString::from("Files rescanned"));
    } else if tab.view_mode == ViewMode::ThreeWayText {
        // 3-way editing in progress or blank: rebuild from VecModel (authoritative source)
        let model = window.get_three_way_lines();
        if let Some(vec_model) = model.as_any().downcast_ref::<VecModel<ThreeWayLineData>>() {
            let left_text = rebuild_three_way_text(vec_model, 0);
            let base_text = rebuild_three_way_text(vec_model, 1);
            let right_text = rebuild_three_way_text(vec_model, 2);
            recompute_three_way_from_text(window, state, &base_text, &left_text, &right_text);
        }
        window.set_status_text(SharedString::from("Compared"));
    } else if tab.view_mode == ViewMode::FolderCompare {
        run_folder_compare(window, state);
        window.set_status_text(SharedString::from("Folders rescanned"));
    } else if (tab.view_mode == ViewMode::CsvCompare || tab.view_mode == ViewMode::ExcelCompare)
        && tab.left_path.is_some()
        && tab.right_path.is_some()
        && !tab.editing_dirty
        && !tab.has_unsaved_changes
    {
        // No edits — reload from disk
        let Some(left_path) = tab.left_path.clone() else {
            return;
        };
        let Some(right_path) = tab.right_path.clone() else {
            return;
        };
        let left_bytes = match read_file_or_report(window, &left_path) {
            Some(b) => b,
            None => return,
        };
        let right_bytes = match read_file_or_report(window, &right_path) {
            Some(b) => b,
            None => return,
        };
        if tab.view_mode == ViewMode::CsvCompare {
            run_csv_compare(
                window,
                state,
                &left_bytes,
                &right_bytes,
                &left_path,
                &right_path,
            );
        } else {
            run_excel_compare(
                window,
                state,
                &left_bytes,
                &right_bytes,
                &left_path,
                &right_path,
            );
        }
        window.set_status_text(SharedString::from("Files rescanned"));
    } else if (tab.view_mode == ViewMode::CsvCompare || tab.view_mode == ViewMode::ExcelCompare)
        && (tab.editing_dirty || tab.has_unsaved_changes)
    {
        // Editing in progress: rebuild CSV from VecModel (authoritative source) and re-compare
        let delimiter = tab.csv_delimiter;
        let left_csv = rebuild_table_csv(&tab.table_rows, 0, delimiter);
        let right_csv = rebuild_table_csv(&tab.table_rows, 2, delimiter);
        recompute_table_from_csv(window, state, &left_csv, &right_csv);
        window.set_status_text(SharedString::from("Compared"));
    } else if tab.view_mode == ViewMode::CsvThreeWay
        && tab.base_path.is_some()
        && tab.left_path.is_some()
        && tab.right_path.is_some()
        && !tab.editing_dirty
        && !tab.has_unsaved_changes
    {
        let Some(base_path) = tab.base_path.clone() else {
            return;
        };
        let Some(left_path) = tab.left_path.clone() else {
            return;
        };
        let Some(right_path) = tab.right_path.clone() else {
            return;
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
        window.set_status_text(SharedString::from("Files rescanned"));
    } else if tab.view_mode == ViewMode::CsvThreeWay
        && (tab.editing_dirty || tab.has_unsaved_changes)
    {
        // 3-way table editing: rebuild from VecModel and re-compare
        let delimiter = tab.csv_delimiter;
        let base_csv = rebuild_table_csv(&tab.table_rows, 1, delimiter);
        let left_csv = rebuild_table_csv(&tab.table_rows, 0, delimiter);
        let right_csv = rebuild_table_csv(&tab.table_rows, 2, delimiter);
        recompute_table_from_csv_3way(window, state, &base_csv, &left_csv, &right_csv);
        window.set_status_text(SharedString::from("Compared"));
    }
}

pub fn compare_clipboard_as_left(window: &MainWindow, state: &mut AppState) {
    let text = match arboard::Clipboard::new().and_then(|mut c| c.get_text()) {
        Ok(t) => t,
        Err(_) => {
            window.set_status_text(SharedString::from("Clipboard is empty or unavailable"));
            return;
        }
    };
    {
        let tab = state.current_tab_mut();
        tab.left_path = None;
        tab.view_mode = ViewMode::FileDiff;
    }
    window.set_view_mode(ViewMode::FileDiff.as_i32());
    window.set_left_path(SharedString::from("(Clipboard)"));
    window.set_left_encoding_display(SharedString::from("UTF-8"));
    window.set_left_eol_type(SharedString::from(""));

    let tab = state.current_tab();
    let right_text = tab.right_lines.join("\n");
    let right_text = if right_text.is_empty() {
        if let Some(rp) = tab.right_path.clone() {
            match read_file_or_report(window, &rp) {
                Some(bytes) => {
                    let (t, _) = crate::encoding::decode_file(&bytes);
                    t
                }
                None => return,
            }
        } else {
            String::new()
        }
    } else {
        right_text
    };

    recompute_diff_from_text(window, state, &text, &right_text);
    state.current_tab_mut().left_lines = text.lines().map(String::from).collect();
    state.current_tab_mut().title = "(Clipboard) ↔ right".to_string();
    sync_tab_list(window, state);
}

pub fn compare_clipboard_as_right(window: &MainWindow, state: &mut AppState) {
    let text = match arboard::Clipboard::new().and_then(|mut c| c.get_text()) {
        Ok(t) => t,
        Err(_) => {
            window.set_status_text(SharedString::from("Clipboard is empty or unavailable"));
            return;
        }
    };
    {
        let tab = state.current_tab_mut();
        tab.right_path = None;
        tab.view_mode = ViewMode::FileDiff;
    }
    window.set_view_mode(ViewMode::FileDiff.as_i32());
    window.set_right_path(SharedString::from("(Clipboard)"));
    window.set_right_encoding_display(SharedString::from("UTF-8"));
    window.set_right_eol_type(SharedString::from(""));

    let tab = state.current_tab();
    let left_text = tab.left_lines.join("\n");
    let left_text = if left_text.is_empty() {
        if let Some(lp) = tab.left_path.clone() {
            match read_file_or_report(window, &lp) {
                Some(bytes) => {
                    let (t, _) = crate::encoding::decode_file(&bytes);
                    t
                }
                None => return,
            }
        } else {
            String::new()
        }
    } else {
        left_text
    };

    recompute_diff_from_text(window, state, &left_text, &text);
    state.current_tab_mut().right_lines = text.lines().map(String::from).collect();
    state.current_tab_mut().title = "left ↔ (Clipboard)".to_string();
    sync_tab_list(window, state);
}
