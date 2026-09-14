use super::*;

/// Save all tabs with unsaved changes.
/// Tabs with file paths are auto-saved. Tabs without paths prompt for a filename via dialog.
/// Sync VecModel, auto-save tabs with paths, and return a queue of
/// (tab_index, is_left, text, encoding) for pathless sides needing a Save As dialog.
pub fn collect_pending_saves(
    window: &MainWindow,
    state: &mut AppState,
) -> Vec<(usize, i32, String, String)> {
    let mut queue = Vec::new();
    let n = state.tabs.len();
    for i in 0..n {
        if !state.tabs[i].has_unsaved_changes {
            continue;
        }

        if state.tabs[i].view_mode == ViewMode::ThreeWayText {
            // 3-way tab: save left, base, right — derive from PaneBuffers
            let left_text = state.tabs[i]
                .left_buffer
                .as_ref()
                .map(|b| extract_real_lines(b))
                .unwrap_or_else(|| "\n".to_string());
            let base_text = state.tabs[i]
                .middle_buffer
                .as_ref()
                .map(|b| extract_real_lines(b))
                .unwrap_or_else(|| "\n".to_string());
            let right_text = state.tabs[i]
                .right_buffer
                .as_ref()
                .map(|b| extract_real_lines(b))
                .unwrap_or_else(|| "\n".to_string());
            let left_enc = state.tabs[i].left_encoding.clone();
            let right_enc = state.tabs[i].right_encoding.clone();
            let base_enc = "UTF-8".to_string();

            save_or_queue(
                window,
                &mut queue,
                i,
                0,
                &state.tabs[i].left_path,
                left_text,
                left_enc,
            );
            save_or_queue(
                window,
                &mut queue,
                i,
                2,
                &state.tabs[i].base_path,
                base_text,
                base_enc,
            );
            save_or_queue(
                window,
                &mut queue,
                i,
                1,
                &state.tabs[i].right_path,
                right_text,
                right_enc,
            );

            if state.tabs[i].left_path.is_some()
                && state.tabs[i].base_path.is_some()
                && state.tabs[i].right_path.is_some()
            {
                state.tabs[i].has_unsaved_changes = false;
            }
        } else {
            // 2-way tab: extract text from PaneBuffers (authoritative source)

            // ponytail: same provisional guard as save_file (see its comment for
            // the full rationale) — this "Save All" path (used on quit) never
            // calls save_file, so it needs its own copy of the check, keyed off
            // this tab's own TabState rather than state.current_tab(). The check
            // itself lives in TabState::save_blocked_by_hidden_lines, which also
            // covers the F9 case: options toggled off after an edit made while
            // they were on still leaves hidden_lines_dropped set (Gotcha 3 —
            // rescan-from-VecModel can't un-drop lines already missing from the
            // buffers). Skipping here — without touching queue or
            // has_unsaved_changes — leaves the tab's in-memory edits unsaved. If
            // this runs from on_quit_save_all (src/main.rs), the app proceeds to
            // exit anyway and those edits are lost. That's an accepted tradeoff:
            // an unsaved in-app edit is recoverable in principle (redo the edit),
            // a file silently truncated on disk is not. Stopping the quit itself
            // would mean touching src/main.rs's quit flow, out of scope here.
            if state.tabs[i].save_blocked_by_hidden_lines() {
                window.set_status_text(SharedString::from(format!(
                    "Save blocked for \"{}\": turn off \"Ignore blank lines\" and clear line filters first — saving now could permanently delete lines that were hidden from comparison (#63). If you edited while the option was on, close without saving and reopen the files instead.",
                    state.tabs[i].title
                )));
                continue;
            }

            let left_enc = state.tabs[i].left_encoding.clone();
            let left_text = state.tabs[i]
                .left_buffer
                .as_ref()
                .map(|b| extract_real_lines(b))
                .unwrap_or_else(|| "\n".to_string());
            save_or_queue(
                window,
                &mut queue,
                i,
                0,
                &state.tabs[i].left_path,
                left_text,
                left_enc,
            );

            let right_enc = state.tabs[i].right_encoding.clone();
            let right_text = state.tabs[i]
                .right_buffer
                .as_ref()
                .map(|b| extract_real_lines(b))
                .unwrap_or_else(|| "\n".to_string());
            save_or_queue(
                window,
                &mut queue,
                i,
                1,
                &state.tabs[i].right_path,
                right_text,
                right_enc,
            );

            if state.tabs[i].left_path.is_some() && state.tabs[i].right_path.is_some() {
                state.tabs[i].has_unsaved_changes = false;
            }
        }
    }
    queue
}

/// Save text to path if available, otherwise queue for Save-As dialog.
fn save_or_queue(
    window: &MainWindow,
    queue: &mut Vec<(usize, i32, String, String)>,
    tab_idx: usize,
    pane: i32,
    path: &Option<std::path::PathBuf>,
    text: String,
    encoding: String,
) {
    if let Some(path) = path {
        let bytes = encode_text(&text, &encoding);
        if let Err(e) = fs::write(path, bytes) {
            window.set_status_text(SharedString::from(format!(
                "Error saving {}: {}",
                path.display(),
                e
            )));
        }
    } else if !text.trim().is_empty() {
        queue.push((tab_idx, pane, text, encoding));
    }
}

pub fn export_html_report(window: &MainWindow, state: &AppState) {
    let tab = state.current_tab();

    // Rebuild DiffResult from PaneBuffers (authoritative source)
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
    let result = compute_diff_with_options(&left_text, &right_text, &tab.diff_options);

    let left_title = tab
        .left_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "Left".to_string());
    let right_title = tab
        .right_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "Right".to_string());

    let html = crate::export::export_html(&result, &left_title, &right_title, &tab.diff_comments);

    // Save dialog
    if let Some(path) = rfd::FileDialog::new()
        .set_title("Export HTML Report")
        .set_file_name("diff-report.html")
        .add_filter("HTML", &["html"])
        .save_file()
    {
        match fs::write(&path, &html) {
            Ok(_) => {
                window.set_status_text(SharedString::from(format!(
                    "Exported to {}",
                    path.to_string_lossy()
                )));
            }
            Err(e) => {
                window.set_status_text(SharedString::from(format!("Export error: {}", e)));
            }
        }
    }
}

pub fn export_xlsx_report(window: &MainWindow, state: &AppState) {
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
    let result = compute_diff_with_options(&left_text, &right_text, &tab.diff_options);

    let left_title = tab
        .left_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "Left".to_string());
    let right_title = tab
        .right_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "Right".to_string());

    match crate::export::export_xlsx(&result, &left_title, &right_title, &tab.diff_comments) {
        Ok(bytes) => {
            if let Some(path) = rfd::FileDialog::new()
                .set_title("Export Excel Report")
                .set_file_name("diff-report.xlsx")
                .add_filter("Excel", &["xlsx"])
                .save_file()
            {
                match fs::write(&path, &bytes) {
                    Ok(_) => {
                        window.set_status_text(SharedString::from(format!(
                            "Exported to {}",
                            path.to_string_lossy()
                        )));
                    }
                    Err(e) => {
                        window.set_status_text(SharedString::from(format!("Export error: {}", e)));
                    }
                }
            }
        }
        Err(e) => {
            window.set_status_text(SharedString::from(format!("Excel export error: {}", e)));
        }
    }
}

/// Collect all non-empty comments from every tab
fn collect_all_comments(state: &AppState) -> Vec<crate::export::CommentEntry> {
    let mut entries = Vec::new();
    for tab in &state.tabs {
        if tab.diff_comments.is_empty() {
            continue;
        }
        let left_file = tab
            .left_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let right_file = tab
            .right_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let tab_title = tab.title.clone();
        let mut indices: Vec<usize> = tab.diff_comments.keys().copied().collect();
        indices.sort_unstable();
        for idx in indices {
            let comment = &tab.diff_comments[&idx];
            if comment.is_empty() {
                continue;
            }
            entries.push(crate::export::CommentEntry {
                tab_title: tab_title.clone(),
                left_file: left_file.clone(),
                right_file: right_file.clone(),
                diff_block: idx,
                comment: comment.clone(),
            });
        }
    }
    entries
}

pub fn export_all_comments(window: &MainWindow, state: &AppState, use_json: bool) {
    let entries = collect_all_comments(state);
    if entries.is_empty() {
        window.set_status_text(SharedString::from("No comments to export"));
        return;
    }

    let (content, ext, filter_name) = if use_json {
        (
            crate::export::export_all_comments_json(&entries),
            "json",
            "JSON",
        )
    } else {
        (
            crate::export::export_all_comments_csv(&entries),
            "csv",
            "CSV",
        )
    };

    let filename = if use_json {
        "diff-comments.json"
    } else {
        "diff-comments.csv"
    };
    if let Some(path) = rfd::FileDialog::new()
        .set_title("Export All Comments")
        .set_file_name(filename)
        .add_filter(filter_name, &[ext])
        .save_file()
    {
        match fs::write(&path, &content) {
            Ok(_) => {
                window.set_status_text(SharedString::from(format!(
                    "Exported {} comment(s) to {}",
                    entries.len(),
                    path.to_string_lossy()
                )));
            }
            Err(e) => {
                window.set_status_text(SharedString::from(format!("Export error: {}", e)));
            }
        }
    }
}

pub fn print_diff(window: &MainWindow, state: &AppState) {
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
    let result = compute_diff_with_options(&left_text, &right_text, &tab.diff_options);

    let left_title = tab
        .left_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "Left".to_string());
    let right_title = tab
        .right_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "Right".to_string());

    let html = crate::export::export_html_for_print(
        &result,
        &left_title,
        &right_title,
        &tab.diff_comments,
    );

    // Write to a temp file and open in default browser
    let tmp = std::env::temp_dir().join("winxmerge-print.html");
    match fs::write(&tmp, &html) {
        Ok(_) => {
            if let Err(e) = open::that_detached(&tmp) {
                window.set_status_text(SharedString::from(format!("Print error: {}", e)));
            } else {
                window.set_status_text(SharedString::from("Opened in browser for printing"));
            }
        }
        Err(e) => {
            window.set_status_text(SharedString::from(format!("Print error: {}", e)));
        }
    }
}

pub fn export_patch(window: &MainWindow, state: &AppState) {
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
    let result = compute_diff_with_options(&left_text, &right_text, &tab.diff_options);

    let left_title = tab
        .left_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "Left".to_string());
    let right_title = tab
        .right_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "Right".to_string());

    let patch = crate::export::export_unified_diff(&result, &left_title, &right_title);

    if let Some(path) = rfd::FileDialog::new()
        .set_title("Export Patch (Unified Diff)")
        .set_file_name("diff.patch")
        .add_filter("Patch", &["patch", "diff"])
        .save_file()
    {
        match fs::write(&path, &patch) {
            Ok(_) => {
                window.set_status_text(SharedString::from(format!(
                    "Exported patch to {}",
                    path.to_string_lossy()
                )));
            }
            Err(e) => {
                window.set_status_text(SharedString::from(format!("Export error: {}", e)));
            }
        }
    }
}

pub fn export_csv_report(window: &MainWindow, state: &AppState, use_tab: bool) {
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
    let result = compute_diff_with_options(&left_text, &right_text, &tab.diff_options);
    let sep = if use_tab { '\t' } else { ',' };
    let ext = if use_tab { "tsv" } else { "csv" };
    let left_title = tab
        .left_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "Left".to_string());
    let right_title = tab
        .right_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "Right".to_string());
    let content = crate::export::export_csv(&result, &left_title, &right_title, sep);
    if let Some(path) = rfd::FileDialog::new()
        .set_title("Export CSV")
        .set_file_name(&format!("diff-report.{}", ext))
        .add_filter(ext.to_uppercase().as_str(), &[ext])
        .save_file()
    {
        match fs::write(&path, content.as_bytes()) {
            Ok(_) => window.set_status_text(SharedString::from(format!(
                "Exported to {}",
                path.to_string_lossy()
            ))),
            Err(e) => window.set_status_text(SharedString::from(format!("Export error: {}", e))),
        }
    }
}

pub fn export_folder_html_report(window: &MainWindow, state: &AppState) {
    let tab = state.current_tab();
    if tab.folder_items.is_empty() {
        return;
    }
    let left_title = tab
        .left_folder
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "Left".to_string());
    let right_title = tab
        .right_folder
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "Right".to_string());
    let html = crate::export::export_folder_html(&tab.folder_items, &left_title, &right_title);
    if let Some(path) = rfd::FileDialog::new()
        .set_title("Export Folder Report")
        .set_file_name("folder-report.html")
        .add_filter("HTML", &["html"])
        .save_file()
    {
        match fs::write(&path, html.as_bytes()) {
            Ok(_) => window.set_status_text(SharedString::from(format!(
                "Folder report exported to {}",
                path.to_string_lossy()
            ))),
            Err(e) => window.set_status_text(SharedString::from(format!("Export error: {}", e))),
        }
    }
}

pub fn save_file(window: &MainWindow, state: &mut AppState, save_left: bool) {
    let tab = state.current_tab();

    // ponytail: provisional guard for #63 — normalize_text (src/diff/engine.rs)
    // drops blank/filtered lines (and their line-number mapping) before diffing,
    // so they never reach DiffResult, PaneBuffer, or extract_real_lines. Saving
    // here would silently and irreversibly delete those lines from disk. We
    // can't tell whether this compare actually dropped any lines — DiffResult
    // (src/models/diff_line.rs) keeps only lines/diff_count/diff_positions, and
    // normalize_text's line-number map is gone by the time compute_diff_with_options
    // returns — so TabState::save_blocked_by_hidden_lines blocks on the toggle
    // alone, whether or not a line was actually lost, OR on hidden_lines_dropped
    // still being set from an earlier diff on these buffers (F9: toggling the
    // option off doesn't undo a drop already baked into the PaneBuffers — see
    // that method's doc comment). A false block just needs the toggle turned
    // off (and, for F9, a close-and-reopen) to recover; a false save is
    // unrecoverable, so we bias toward blocking. The real fix (carry dropped
    // lines through via a new LineStatus variant instead of discarding them)
    // is planned for v0.52.
    if tab.save_blocked_by_hidden_lines() {
        window.set_status_text(SharedString::from(
            "Save blocked: turn off \"Ignore blank lines\" and clear line filters first — saving now could permanently delete lines that were hidden from comparison (#63). If you edited while the option was on, close without saving and reopen the files instead.",
        ));
        return;
    }

    let (text, path, encoding) = if save_left {
        let text = tab
            .left_buffer
            .as_ref()
            .map(|b| extract_real_lines(b))
            .unwrap_or_else(|| "\n".to_string());
        (text, tab.left_path.clone(), tab.left_encoding.clone())
    } else {
        let text = tab
            .right_buffer
            .as_ref()
            .map(|b| extract_real_lines(b))
            .unwrap_or_else(|| "\n".to_string());
        (text, tab.right_path.clone(), tab.right_encoding.clone())
    };

    let path = if let Some(p) = path {
        p
    } else {
        // New document (no path yet) — show Save As dialog
        match rfd::FileDialog::new().set_title("Save As").save_file() {
            Some(p) => {
                if save_left {
                    state.current_tab_mut().left_path = Some(p.clone());
                    window.set_left_path(SharedString::from(p.to_string_lossy().to_string()));
                } else {
                    state.current_tab_mut().right_path = Some(p.clone());
                    window.set_right_path(SharedString::from(p.to_string_lossy().to_string()));
                }
                sync_tab_list(window, state);
                p
            }
            None => return,
        }
    };

    let bytes = encode_text(&text, &encoding);
    if let Err(e) = fs::write(&path, &bytes) {
        window.set_status_text(SharedString::from(format!("Error saving: {}", e)));
        return;
    }
    state.current_tab_mut().has_unsaved_changes = false;
    window.set_has_unsaved_changes(false);
    let side = if save_left { "Left" } else { "Right" };
    window.set_status_text(SharedString::from(format!(
        "{} file saved: {} ({})",
        side,
        path.to_string_lossy(),
        encoding
    )));
}

/// Save a pane of 3-way diff.  pane: 0=left, 1=base(middle), 2=right.
pub fn save_three_way_pane(window: &MainWindow, state: &mut AppState, pane: i32) {
    let tab = state.current_tab();
    let buf = match pane {
        0 => &tab.left_buffer,
        1 => &tab.middle_buffer,
        _ => &tab.right_buffer,
    };
    let text = buf
        .as_ref()
        .map(|b| extract_real_lines(b))
        .unwrap_or_else(|| "\n".to_string());
    let tab = state.current_tab();
    let (path, encoding) = match pane {
        0 => (tab.left_path.clone(), tab.left_encoding.clone()),
        1 => (tab.base_path.clone(), tab.base_encoding.clone()),
        _ => (tab.right_path.clone(), tab.right_encoding.clone()),
    };

    let path = if let Some(p) = path {
        p
    } else {
        match rfd::FileDialog::new().set_title("Save As").save_file() {
            Some(p) => {
                match pane {
                    0 => {
                        state.current_tab_mut().left_path = Some(p.clone());
                        window.set_left_path(SharedString::from(p.to_string_lossy().to_string()));
                    }
                    1 => {
                        state.current_tab_mut().base_path = Some(p.clone());
                    }
                    _ => {
                        state.current_tab_mut().right_path = Some(p.clone());
                        window.set_right_path(SharedString::from(p.to_string_lossy().to_string()));
                    }
                }
                sync_tab_list(window, state);
                p
            }
            None => return,
        }
    };

    let bytes = encode_text(&text, &encoding);
    if let Err(e) = fs::write(&path, &bytes) {
        window.set_status_text(SharedString::from(format!("Error saving: {}", e)));
        return;
    }
    state.current_tab_mut().has_unsaved_changes = false;
    window.set_has_unsaved_changes(false);
    let side = match pane {
        0 => "Left",
        1 => "Middle",
        _ => "Right",
    };
    window.set_status_text(SharedString::from(format!(
        "{} file saved: {} ({})",
        side,
        path.to_string_lossy(),
        encoding
    )));
}

/// Rebuild CSV text from table rows for a given pane (0=left, 2=right, 1=base).
pub(super) fn rebuild_table_csv(rows: &[TableRowData], pane: i32, delimiter: u8) -> String {
    let delim = delimiter as char;
    let mut output = String::new();
    for row in rows {
        let cells = match pane {
            0 => &row.left_cells,
            1 => &row.base_cells,
            _ => &row.right_cells,
        };
        let count = cells.row_count();
        for i in 0..count {
            if i > 0 {
                output.push(delim);
            }
            if let Some(cell) = cells.row_data(i) {
                let text = cell.text.to_string();
                // Quote if the field contains delimiter, quote, or newline
                if text.contains(delim)
                    || text.contains('"')
                    || text.contains('\n')
                    || text.contains('\r')
                {
                    output.push('"');
                    output.push_str(&text.replace('"', "\"\""));
                    output.push('"');
                } else {
                    output.push_str(&text);
                }
            }
        }
        output.push('\n');
    }
    output
}

/// Save table (CSV) file for a given pane. pane: 0=left, 2=right, 1=base.
pub fn save_table_file(window: &MainWindow, state: &mut AppState, pane: i32) {
    let tab = state.current_tab();
    let delimiter = tab.csv_delimiter;
    let text = rebuild_table_csv(&tab.table_rows, pane, delimiter);

    let (path, encoding) = match pane {
        0 => (tab.left_path.clone(), tab.left_encoding.clone()),
        1 => (tab.base_path.clone(), tab.base_encoding.clone()),
        _ => (tab.right_path.clone(), tab.right_encoding.clone()),
    };

    let path = if let Some(p) = path {
        p
    } else {
        match rfd::FileDialog::new()
            .set_title("Save As")
            .add_filter("CSV", &["csv"])
            .add_filter("TSV", &["tsv"])
            .add_filter("All Files", &["*"])
            .save_file()
        {
            Some(p) => {
                match pane {
                    0 => {
                        state.current_tab_mut().left_path = Some(p.clone());
                        window.set_left_path(SharedString::from(p.to_string_lossy().to_string()));
                    }
                    1 => {
                        state.current_tab_mut().base_path = Some(p.clone());
                    }
                    _ => {
                        state.current_tab_mut().right_path = Some(p.clone());
                        window.set_right_path(SharedString::from(p.to_string_lossy().to_string()));
                    }
                }
                sync_tab_list(window, state);
                p
            }
            None => return,
        }
    };

    let bytes = encode_text(&text, &encoding);
    if let Err(e) = fs::write(&path, &bytes) {
        window.set_status_text(SharedString::from(format!("Error saving: {}", e)));
        return;
    }
    state.current_tab_mut().has_unsaved_changes = false;
    window.set_has_unsaved_changes(false);
    let side = match pane {
        0 => "Left",
        1 => "Middle",
        _ => "Right",
    };
    window.set_status_text(SharedString::from(format!(
        "{} file saved: {} ({})",
        side,
        path.to_string_lossy(),
        encoding
    )));
}
