use super::*;

pub fn search_text(window: &MainWindow, state: &mut AppState, query: &str) {
    let tab = state.current_tab_mut();
    tab.search_matches.clear();
    tab.current_search_match = -1;

    // 3-way search
    if tab.view_mode == ViewMode::ThreeWayText {
        if query.is_empty() {
            let model = window.get_three_way_lines();
            if let Some(vec_model) = model.as_any().downcast_ref::<VecModel<ThreeWayLineData>>() {
                for i in 0..vec_model.row_count() {
                    if let Some(mut row) = vec_model.row_data(i) {
                        if row.is_search_match {
                            row.is_search_match = false;
                            vec_model.set_row_data(i, row);
                        }
                    }
                }
            }
            window.set_search_match_count(0);
            window.set_status_text(SharedString::from("Search cleared"));
            return;
        }

        let query_lower = query.to_lowercase();
        let model = window.get_three_way_lines();
        if let Some(vec_model) = model.as_any().downcast_ref::<VecModel<ThreeWayLineData>>() {
            for i in 0..vec_model.row_count() {
                let Some(mut row) = vec_model.row_data(i) else {
                    continue;
                };
                let matched = row
                    .left_text
                    .to_string()
                    .to_lowercase()
                    .contains(&query_lower)
                    || row
                        .base_text
                        .to_string()
                        .to_lowercase()
                        .contains(&query_lower)
                    || row
                        .right_text
                        .to_string()
                        .to_lowercase()
                        .contains(&query_lower);
                if matched {
                    tab.search_matches.push(i);
                }
                if row.is_search_match != matched {
                    row.is_search_match = matched;
                    vec_model.set_row_data(i, row);
                }
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
        return;
    }

    // 2-way search
    if query.is_empty() {
        // Clear any existing search highlights
        let model = window.get_diff_lines();
        if let Some(vec_model) = model.as_any().downcast_ref::<VecModel<DiffLineData>>() {
            for i in 0..vec_model.row_count() {
                if let Some(mut row) = vec_model.row_data(i) {
                    if row.is_search_match {
                        row.is_search_match = false;
                        vec_model.set_row_data(i, row);
                    }
                }
            }
        }
        window.set_search_match_count(0);
        window.set_status_text(SharedString::from("Search cleared"));
        return;
    }

    let query_lower = query.to_lowercase();
    let model = window.get_diff_lines();
    if let Some(vec_model) = model.as_any().downcast_ref::<VecModel<DiffLineData>>() {
        for i in 0..vec_model.row_count() {
            let Some(mut row) = vec_model.row_data(i) else {
                continue;
            };
            let matched = row
                .left_text
                .to_string()
                .to_lowercase()
                .contains(&query_lower)
                || row
                    .right_text
                    .to_string()
                    .to_lowercase()
                    .contains(&query_lower);
            if matched {
                tab.search_matches.push(i);
            }
            if row.is_search_match != matched {
                row.is_search_match = matched;
                vec_model.set_row_data(i, row);
            }
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
}

pub fn replace_text(window: &MainWindow, state: &mut AppState, search: &str, replacement: &str) {
    let tab = state.current_tab();
    if search.is_empty() || tab.search_matches.is_empty() || tab.current_search_match < 0 {
        return;
    }

    let search_lower = search.to_lowercase();

    if tab.view_mode == ViewMode::ThreeWayText {
        let model = window.get_three_way_lines();
        let vec_model = match model.as_any().downcast_ref::<VecModel<ThreeWayLineData>>() {
            Some(m) => m,
            None => return,
        };
        let match_idx = tab.search_matches[tab.current_search_match as usize];
        let Some(mut row) = vec_model.row_data(match_idx) else {
            return;
        };
        row.left_text = SharedString::from(case_insensitive_replace(
            &row.left_text.to_string(),
            &search_lower,
            replacement,
        ));
        row.base_text = SharedString::from(case_insensitive_replace(
            &row.base_text.to_string(),
            &search_lower,
            replacement,
        ));
        row.right_text = SharedString::from(case_insensitive_replace(
            &row.right_text.to_string(),
            &search_lower,
            replacement,
        ));
        vec_model.set_row_data(match_idx, row);

        mark_dirty(window, state);
        search_text(window, state, search);
        return;
    }

    let_diff_vec_model!(model, vec_model, window);

    let match_idx = tab.search_matches[tab.current_search_match as usize];
    let Some(mut row) = vec_model.row_data(match_idx) else {
        return;
    };

    let left = row.left_text.to_string();
    let right = row.right_text.to_string();
    row.left_text = SharedString::from(case_insensitive_replace(&left, &search_lower, replacement));
    row.right_text =
        SharedString::from(case_insensitive_replace(&right, &search_lower, replacement));
    vec_model.set_row_data(match_idx, row);

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
        let model = window.get_three_way_lines();
        let vec_model = match model.as_any().downcast_ref::<VecModel<ThreeWayLineData>>() {
            Some(m) => m,
            None => return,
        };
        for &match_idx in &matches {
            if let Some(mut row) = vec_model.row_data(match_idx) {
                row.left_text = SharedString::from(case_insensitive_replace(
                    &row.left_text.to_string(),
                    &search_lower,
                    replacement,
                ));
                row.base_text = SharedString::from(case_insensitive_replace(
                    &row.base_text.to_string(),
                    &search_lower,
                    replacement,
                ));
                row.right_text = SharedString::from(case_insensitive_replace(
                    &row.right_text.to_string(),
                    &search_lower,
                    replacement,
                ));
                vec_model.set_row_data(match_idx, row);
            }
        }
    } else {
        let_diff_vec_model!(model, vec_model, window);
        for &match_idx in &matches {
            if let Some(mut row) = vec_model.row_data(match_idx) {
                let left = row.left_text.to_string();
                let right = row.right_text.to_string();
                row.left_text =
                    SharedString::from(case_insensitive_replace(&left, &search_lower, replacement));
                row.right_text = SharedString::from(case_insensitive_replace(
                    &right,
                    &search_lower,
                    replacement,
                ));
                vec_model.set_row_data(match_idx, row);
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
