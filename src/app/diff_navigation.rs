use super::*;

pub fn select_diff(window: &MainWindow, state: &mut AppState, diff_index: i32) {
    let tab = state.current_tab();
    if diff_index < 0 || diff_index as usize >= tab.diff_positions.len() {
        return;
    }
    update_current_diff(window, state, diff_index);
}

pub fn navigate_diff(window: &MainWindow, state: &mut AppState, forward: bool) {
    let tab = state.current_tab();
    if tab.diff_positions.is_empty() {
        return;
    }

    let new_index = if forward {
        if tab.current_diff < tab.diff_positions.len() as i32 - 1 {
            tab.current_diff + 1
        } else {
            0
        }
    } else if tab.current_diff > 0 {
        tab.current_diff - 1
    } else {
        tab.diff_positions.len() as i32 - 1
    };

    update_current_diff(window, state, new_index);
}

/// Navigate to the next/prev diff of a specific status type.
/// status_filter: 1=Added, 2=Removed, 3=Modified, 4=Moved, 0=all (same as navigate_diff)
pub fn navigate_diff_by_status(
    window: &MainWindow,
    state: &mut AppState,
    forward: bool,
    status_filter: i32,
) {
    if status_filter == 0 {
        navigate_diff(window, state, forward);
        return;
    }
    let tab = state.current_tab();
    if tab.diff_positions.is_empty() {
        return;
    }
    let n = tab.diff_positions.len() as i32;
    let start = tab.current_diff;
    let candidates: Vec<i32> = if forward {
        let mut v: Vec<i32> = ((start + 1)..n).collect();
        v.extend(0..=start);
        v
    } else {
        let mut v: Vec<i32> = (0..start).rev().collect();
        v.extend((start..n).rev());
        v
    };
    // Find the matching diff index before mutating state
    let found = candidates.into_iter().find(|&idx| {
        let pos = tab.diff_positions[idx as usize];
        // Check status from PaneBuffer (non-ghost side has the real status)
        if let Some(ref lb) = tab.left_buffer {
            if let Some(row) = lb.model.row_data(pos) {
                if !row.is_ghost {
                    return row.status == status_filter;
                }
            }
        }
        if let Some(ref rb) = tab.right_buffer {
            if let Some(row) = rb.model.row_data(pos) {
                if !row.is_ghost {
                    return row.status == status_filter;
                }
            }
        }
        false
    });
    if let Some(idx) = found {
        update_current_diff(window, state, idx);
        return;
    }
    let label = match status_filter {
        STATUS_ADDED => "Added",
        STATUS_REMOVED => "Removed",
        STATUS_MODIFIED => "Modified",
        STATUS_MOVED => "Moved",
        _ => "status",
    };
    window.set_status_text(SharedString::from(format!("No more {} diffs", label)));
}

pub fn first_diff(window: &MainWindow, state: &mut AppState) {
    let tab = state.current_tab();
    if tab.diff_positions.is_empty() {
        return;
    }
    update_current_diff(window, state, 0);
}

pub fn last_diff(window: &MainWindow, state: &mut AppState) {
    let tab = state.current_tab();
    if tab.diff_positions.is_empty() {
        return;
    }
    let last = tab.diff_positions.len() as i32 - 1;
    update_current_diff(window, state, last);
}

pub fn goto_line(window: &MainWindow, state: &AppState, line_number: i32) {
    if line_number <= 0 {
        return;
    }
    let tab = state.current_tab();
    // Search PaneBuffers for the matching line number
    let mut found_idx: Option<usize> = None;
    let mut found_diff_index: i32 = -1;
    // Check left buffer first, then right
    for buf_opt in [&tab.left_buffer, &tab.right_buffer] {
        if found_idx.is_some() {
            break;
        }
        if let Some(buf) = buf_opt {
            for i in 0..buf.model.row_count() {
                if let Some(row) = buf.model.row_data(i) {
                    if !row.is_ghost {
                        if let Ok(n) = row.line_no.parse::<i32>() {
                            if n == line_number {
                                found_idx = Some(i);
                                found_diff_index = row.diff_index;
                                break;
                            }
                        }
                    }
                }
            }
        }
    }
    if let Some(idx) = found_idx {
        window.invoke_scroll_diff_to_row(idx as i32);
        window.set_status_text(SharedString::from(format!("Line {}", line_number)));
        if found_diff_index >= 0 {
            // Sync is_current_diff to PaneBuffers
            for buf_opt in [&tab.left_buffer, &tab.right_buffer] {
                if let Some(buf) = buf_opt {
                    for i in 0..buf.model.row_count() {
                        if let Some(mut row) = buf.model.row_data(i) {
                            let should_highlight = i == idx;
                            if row.is_current_diff != should_highlight {
                                row.is_current_diff = should_highlight;
                                buf.model.set_row_data(i, row);
                            }
                        }
                    }
                }
            }
        }
    } else {
        window.set_status_text(SharedString::from(format!(
            "Line {} not found",
            line_number
        )));
    }
}

pub fn toggle_bookmark(state: &mut AppState, line_index: i32) {
    let tab = state.current_tab_mut();
    let idx = if line_index >= 0 {
        // Use the diff position for this diff index
        if let Some(&pos) = tab.diff_positions.get(line_index as usize) {
            pos
        } else {
            return;
        }
    } else {
        return;
    };
    if let Some(pos) = tab.bookmarks.iter().position(|&b| b == idx) {
        tab.bookmarks.remove(pos);
    } else {
        tab.bookmarks.push(idx);
        tab.bookmarks.sort();
    }
}

pub fn navigate_bookmark(window: &MainWindow, state: &mut AppState, forward: bool) {
    let (new_index, bookmark_pos, diff_idx_opt, total) = {
        let tab = state.current_tab_mut();
        if tab.bookmarks.is_empty() {
            window.set_status_text(SharedString::from("No bookmarks"));
            return;
        }

        let new_index = if forward {
            if tab.current_bookmark < tab.bookmarks.len() as i32 - 1 {
                tab.current_bookmark + 1
            } else {
                0
            }
        } else if tab.current_bookmark > 0 {
            tab.current_bookmark - 1
        } else {
            tab.bookmarks.len() as i32 - 1
        };

        tab.current_bookmark = new_index;
        let bookmark_pos = tab.bookmarks[new_index as usize];
        let diff_idx_opt = tab.diff_positions.iter().position(|&p| p == bookmark_pos);
        let total = tab.bookmarks.len();
        (new_index, bookmark_pos, diff_idx_opt, total)
    };

    if let Some(diff_idx) = diff_idx_opt {
        update_current_diff(window, state, diff_idx as i32);
    } else {
        window.set_status_text(SharedString::from(format!(
            "Bookmark {} of {} (line {})",
            new_index + 1,
            total,
            bookmark_pos
        )));
    }
}

pub(super) fn update_current_diff(window: &MainWindow, state: &mut AppState, new_index: i32) {
    let tab = state.current_tab_mut();
    tab.current_diff = new_index;
    let current_pos = tab.diff_positions[new_index as usize];
    let total = tab.diff_positions.len();
    let view_mode = tab.view_mode;

    // Update current diff index (Slint side handles highlighting reactively)
    window.set_current_diff_index(new_index);

    if view_mode.is_table_mode() {
        // Table view: scroll table grid to row and highlight
        window.invoke_scroll_table_to_row(current_pos as i32);
        window.set_table_current_highlight_row(current_pos as i32);
        // Update table detail pane
        update_table_detail_pane(window, state, current_pos);
        window.set_status_text(SharedString::from(format!(
            "Difference {} of {}",
            new_index + 1,
            total
        )));
        return;
    }

    let tab = state.current_tab();
    let stats = tab.diff_stats.clone();
    let comment = tab
        .diff_comments
        .get(&(new_index as usize))
        .cloned()
        .unwrap_or_default();

    // Scroll to the diff position
    window.invoke_scroll_diff_to_row(current_pos as i32);

    window.set_status_text(SharedString::from(format!(
        "Difference {} of {} [{}]",
        new_index + 1,
        total,
        stats
    )));

    window.set_current_diff_comment(SharedString::from(comment));

    // Update diff detail pane
    let tab = state.current_tab();
    update_detail_pane(window, new_index, tab);
}

/// Build a \x01-separated segment string from word diff segments.
/// Even indices = unchanged, odd indices = changed.
/// If the first segment is changed, an empty unchanged prefix is prepended.
pub(super) fn build_word_diff_string(
    segments: &[crate::models::diff_line::WordDiffSegment],
) -> String {
    if segments.is_empty() {
        return String::new();
    }
    let mut parts: Vec<&str> = Vec::with_capacity(segments.len() + 1);
    if segments[0].changed {
        parts.push(""); // empty unchanged prefix so odd indices = changed
    }
    for seg in segments {
        parts.push(&seg.text);
    }
    parts.join("\x01")
}

fn parse_word_diff_segments(text: &str, word_diff: &str) -> ModelRc<WordSegment> {
    if word_diff.is_empty() {
        return ModelRc::new(VecModel::from(vec![WordSegment {
            text: SharedString::from(text),
            is_changed: false,
        }]));
    }
    let parts: Vec<&str> = word_diff.split('\x01').collect();
    let segments: Vec<WordSegment> = parts
        .iter()
        .enumerate()
        .map(|(i, part)| WordSegment {
            text: SharedString::from(*part),
            is_changed: i % 2 == 1,
        })
        .collect();
    ModelRc::new(VecModel::from(segments))
}

pub(super) fn update_detail_pane(window: &MainWindow, diff_index: i32, tab: &TabState) {
    if diff_index < 0 {
        window.set_detail_has_left(false);
        window.set_detail_has_right(false);
        window.set_detail_left_lines(ModelRc::new(VecModel::from(Vec::<DetailLineData>::new())));
        window.set_detail_right_lines(ModelRc::new(VecModel::from(Vec::<DetailLineData>::new())));
        return;
    }

    let mut left_lines: Vec<DetailLineData> = Vec::new();
    let mut right_lines: Vec<DetailLineData> = Vec::new();

    let start = tab
        .diff_positions
        .get(diff_index as usize)
        .copied()
        .unwrap_or(0);

    if let (Some(lb), Some(rb)) = (&tab.left_buffer, &tab.right_buffer) {
        let count = lb.model.row_count();
        for i in start..count {
            let (Some(lr), Some(rr)) = (lb.model.row_data(i), rb.model.row_data(i)) else {
                continue;
            };
            // Determine diff_index from the non-ghost side
            let row_diff_idx = if !lr.is_ghost {
                lr.diff_index
            } else {
                rr.diff_index
            };
            if row_diff_idx != diff_index {
                if row_diff_idx > diff_index {
                    break;
                }
                continue;
            }
            let status = if !lr.is_ghost { lr.status } else { rr.status };

            // Left side: removed, modified, moved
            if (status == STATUS_REMOVED || status == STATUS_MODIFIED || status == STATUS_MOVED)
                && !lr.is_ghost
            {
                let segments =
                    parse_word_diff_segments(&lr.text.to_string(), &lr.word_diff.to_string());
                left_lines.push(DetailLineData {
                    segments,
                    is_current: true,
                    status,
                });
            }

            // Right side: added, modified, moved
            if (status == STATUS_ADDED || status == STATUS_MODIFIED || status == STATUS_MOVED)
                && !rr.is_ghost
            {
                let segments =
                    parse_word_diff_segments(&rr.text.to_string(), &rr.word_diff.to_string());
                right_lines.push(DetailLineData {
                    segments,
                    is_current: true,
                    status,
                });
            }
        }
    }

    let has_left = !left_lines.is_empty();
    let has_right = !right_lines.is_empty();
    window.set_detail_left_lines(ModelRc::new(VecModel::from(left_lines)));
    window.set_detail_right_lines(ModelRc::new(VecModel::from(right_lines)));
    window.set_detail_has_left(has_left);
    window.set_detail_has_right(has_right);
    window.set_detail_left_scroll_y(0.0);
    window.set_detail_right_scroll_y(0.0);
}
