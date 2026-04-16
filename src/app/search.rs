use super::*;

pub fn search_text(window: &MainWindow, state: &mut AppState, query: &str) {
    let tab = state.current_tab_mut();
    tab.search_matches.clear();
    tab.current_search_match = -1;

    // 3-way search — operates directly on PaneBuffers
    if tab.view_mode == ViewMode::ThreeWayText {
        if query.is_empty() {
            sync_search_match_to_3way_pane_buffers(state, false);
            window.set_search_match_count(0);
            window.set_status_text(SharedString::from("Search cleared"));
            return;
        }

        let query_lower = query.to_lowercase();
        let row_count = tab
            .left_buffer
            .as_ref()
            .map(|b| b.model.row_count())
            .unwrap_or(0);
        let tab = state.current_tab_mut();
        for i in 0..row_count {
            let left_match = tab
                .left_buffer
                .as_ref()
                .and_then(|b| b.model.row_data(i))
                .is_some_and(|r| {
                    !r.is_ghost && r.text.to_string().to_lowercase().contains(&query_lower)
                });
            let middle_match = tab
                .middle_buffer
                .as_ref()
                .and_then(|b| b.model.row_data(i))
                .is_some_and(|r| {
                    !r.is_ghost && r.text.to_string().to_lowercase().contains(&query_lower)
                });
            let right_match = tab
                .right_buffer
                .as_ref()
                .and_then(|b| b.model.row_data(i))
                .is_some_and(|r| {
                    !r.is_ghost && r.text.to_string().to_lowercase().contains(&query_lower)
                });
            if left_match || middle_match || right_match {
                tab.search_matches.push(i);
            }
        }

        let count = tab.search_matches.len();
        window.set_search_match_count(count as i32);
        if count > 0 {
            tab.current_search_match = 0;
            window.set_status_text(SharedString::from(format!(
                "Found {} matches for \"{}\"",
                count, query
            )));
        } else {
            window.set_status_text(SharedString::from(format!(
                "No matches found for \"{}\"",
                query
            )));
        }
        sync_search_match_to_3way_pane_buffers_from_matches(state);
        return;
    }

    // 2-way search — operates directly on PaneBuffers
    if query.is_empty() {
        sync_search_match_to_pane_buffers(state, false);
        window.set_search_match_count(0);
        window.set_status_text(SharedString::from("Search cleared"));
        return;
    }

    let query_lower = query.to_lowercase();
    // Search across both pane buffers
    {
        let tab_ref = state.current_tab();
        let lb = &tab_ref.left_buffer;
        let rb = &tab_ref.right_buffer;
        let row_count = lb
            .as_ref()
            .map(|b| b.model.row_count())
            .unwrap_or(0)
            .max(rb.as_ref().map(|b| b.model.row_count()).unwrap_or(0));
        // Re-borrow mutably to store matches
        let tab = state.current_tab_mut();
        for i in 0..row_count {
            let left_match = tab
                .left_buffer
                .as_ref()
                .and_then(|b| b.model.row_data(i))
                .is_some_and(|r| {
                    !r.is_ghost && r.text.to_string().to_lowercase().contains(&query_lower)
                });
            let right_match = tab
                .right_buffer
                .as_ref()
                .and_then(|b| b.model.row_data(i))
                .is_some_and(|r| {
                    !r.is_ghost && r.text.to_string().to_lowercase().contains(&query_lower)
                });
            if left_match || right_match {
                tab.search_matches.push(i);
            }
        }
    }

    let tab = state.current_tab();
    let count = tab.search_matches.len();
    window.set_search_match_count(count as i32);

    if count > 0 {
        let tab = state.current_tab_mut();
        tab.current_search_match = 0;
        window.set_status_text(SharedString::from(format!(
            "Found {} matches for \"{}\"",
            count, query
        )));
    } else {
        window.set_status_text(SharedString::from(format!(
            "No matches found for \"{}\"",
            query
        )));
    }

    // Set is_search_match on PaneBuffer rows
    sync_search_match_to_pane_buffers_from_matches(state);
}

/// Clear is_search_match on all PaneBuffer rows.
fn sync_search_match_to_pane_buffers(state: &AppState, value: bool) {
    let tab = state.current_tab();
    for buf_opt in [&tab.left_buffer, &tab.right_buffer] {
        if let Some(buf) = buf_opt {
            for i in 0..buf.model.row_count() {
                if let Some(mut row) = buf.model.row_data(i) {
                    if row.is_search_match != value {
                        row.is_search_match = value;
                        buf.model.set_row_data(i, row);
                    }
                }
            }
        }
    }
}

/// Set is_search_match on PaneBuffer rows based on tab.search_matches.
fn sync_search_match_to_pane_buffers_from_matches(state: &AppState) {
    let tab = state.current_tab();
    let matches: std::collections::HashSet<usize> = tab.search_matches.iter().copied().collect();
    for buf_opt in [&tab.left_buffer, &tab.right_buffer] {
        if let Some(buf) = buf_opt {
            for i in 0..buf.model.row_count() {
                if let Some(mut row) = buf.model.row_data(i) {
                    let matched = matches.contains(&i);
                    if row.is_search_match != matched {
                        row.is_search_match = matched;
                        buf.model.set_row_data(i, row);
                    }
                }
            }
        }
    }
}

/// Clear/set is_search_match on all 3-way PaneBuffer rows.
fn sync_search_match_to_3way_pane_buffers(state: &AppState, value: bool) {
    let tab = state.current_tab();
    for buf_opt in [&tab.left_buffer, &tab.middle_buffer, &tab.right_buffer] {
        if let Some(buf) = buf_opt {
            for i in 0..buf.model.row_count() {
                if let Some(mut row) = buf.model.row_data(i) {
                    if row.is_search_match != value {
                        row.is_search_match = value;
                        buf.model.set_row_data(i, row);
                    }
                }
            }
        }
    }
}

/// Set is_search_match on 3-way PaneBuffer rows based on tab.search_matches.
fn sync_search_match_to_3way_pane_buffers_from_matches(state: &AppState) {
    let tab = state.current_tab();
    let matches: std::collections::HashSet<usize> = tab.search_matches.iter().copied().collect();
    for buf_opt in [&tab.left_buffer, &tab.middle_buffer, &tab.right_buffer] {
        if let Some(buf) = buf_opt {
            for i in 0..buf.model.row_count() {
                if let Some(mut row) = buf.model.row_data(i) {
                    let matched = matches.contains(&i);
                    if row.is_search_match != matched {
                        row.is_search_match = matched;
                        buf.model.set_row_data(i, row);
                    }
                }
            }
        }
    }
}

pub fn replace_text(window: &MainWindow, state: &mut AppState, search: &str, replacement: &str) {
    let tab = state.current_tab();
    if search.is_empty() || tab.search_matches.is_empty() || tab.current_search_match < 0 {
        return;
    }

    let search_lower = search.to_lowercase();

    if tab.view_mode == ViewMode::ThreeWayText {
        let match_idx = tab.search_matches[tab.current_search_match as usize];
        // Replace directly in PaneBuffers (authoritative source)
        for buf_opt in [&tab.left_buffer, &tab.middle_buffer, &tab.right_buffer] {
            if let Some(buf) = buf_opt {
                if let Some(row) = buf.model.row_data(match_idx) {
                    if !row.is_ghost {
                        let new_text = case_insensitive_replace(
                            &row.text.to_string(),
                            &search_lower,
                            replacement,
                        );
                        sync_pane_row_text(buf_opt, match_idx, &new_text);
                    }
                }
            }
        }
        mark_dirty(window, state);
        search_text(window, state, search);
        return;
    }

    let match_idx = tab.search_matches[tab.current_search_match as usize];

    // Replace text in PaneBuffers (authoritative source)
    let tab = state.current_tab();
    let new_left = tab
        .left_buffer
        .as_ref()
        .and_then(|b| b.model.row_data(match_idx))
        .map(|r| case_insensitive_replace(&r.text.to_string(), &search_lower, replacement));
    let new_right = tab
        .right_buffer
        .as_ref()
        .and_then(|b| b.model.row_data(match_idx))
        .map(|r| case_insensitive_replace(&r.text.to_string(), &search_lower, replacement));
    if let Some(ref text) = new_left {
        sync_pane_row_text(&tab.left_buffer, match_idx, text);
    }
    if let Some(ref text) = new_right {
        sync_pane_row_text(&tab.right_buffer, match_idx, text);
    }

    mark_dirty(window, state);

    search_text(window, state, search);
}

pub fn replace_all_text(
    window: &MainWindow,
    state: &mut AppState,
    search: &str,
    replacement: &str,
) {
    let tab = state.current_tab();
    if search.is_empty() || tab.search_matches.is_empty() {
        return;
    }

    let search_lower = search.to_lowercase();
    let matches = tab.search_matches.clone();

    if tab.view_mode == ViewMode::ThreeWayText {
        // Replace directly in PaneBuffers (authoritative source)
        let tab = state.current_tab();
        for &match_idx in &matches {
            for buf_opt in [&tab.left_buffer, &tab.middle_buffer, &tab.right_buffer] {
                if let Some(buf) = buf_opt {
                    if let Some(row) = buf.model.row_data(match_idx) {
                        if !row.is_ghost {
                            let new_text = case_insensitive_replace(
                                &row.text.to_string(),
                                &search_lower,
                                replacement,
                            );
                            sync_pane_row_text(buf_opt, match_idx, &new_text);
                        }
                    }
                }
            }
        }
    } else {
        // 2-way: replace directly in PaneBuffers
        let tab = state.current_tab();
        for &match_idx in &matches {
            let new_left = tab
                .left_buffer
                .as_ref()
                .and_then(|b| b.model.row_data(match_idx))
                .map(|r| case_insensitive_replace(&r.text.to_string(), &search_lower, replacement));
            let new_right = tab
                .right_buffer
                .as_ref()
                .and_then(|b| b.model.row_data(match_idx))
                .map(|r| case_insensitive_replace(&r.text.to_string(), &search_lower, replacement));
            if let Some(ref text) = new_left {
                sync_pane_row_text(&tab.left_buffer, match_idx, text);
            }
            if let Some(ref text) = new_right {
                sync_pane_row_text(&tab.right_buffer, match_idx, text);
            }
        }
    }

    let count = matches.len();
    mark_dirty(window, state);

    search_text(window, state, search);
    window.set_status_text(SharedString::from(format!(
        "Replaced {} occurrences",
        count
    )));
}

fn case_insensitive_replace(text: &str, search_lower: &str, replacement: &str) -> String {
    let text_lower = text.to_lowercase();
    let mut result = String::new();
    let mut last = 0;
    for (idx, _) in text_lower.match_indices(search_lower) {
        result.push_str(&text[last..idx]);
        result.push_str(replacement);
        last = idx + search_lower.len();
    }
    result.push_str(&text[last..]);
    result
}

pub fn navigate_search(window: &MainWindow, state: &mut AppState, forward: bool) {
    let tab = state.current_tab_mut();
    if tab.search_matches.is_empty() {
        return;
    }

    let new_index = if forward {
        if tab.current_search_match < tab.search_matches.len() as i32 - 1 {
            tab.current_search_match + 1
        } else {
            0
        }
    } else if tab.current_search_match > 0 {
        tab.current_search_match - 1
    } else {
        tab.search_matches.len() as i32 - 1
    };

    tab.current_search_match = new_index;
    let total = tab.search_matches.len();
    window.set_status_text(SharedString::from(format!(
        "Match {} of {}",
        new_index + 1,
        total
    )));
}
