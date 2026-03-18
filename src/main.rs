mod app;
mod diff;
mod encoding;
mod export;
mod highlight;
mod models;
mod settings;

slint::include_modules!();

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use app::{
    AppState, add_tab, apply_options, check_files_changed, close_tab, copy_all_diffs_to_left,
    copy_all_diffs_to_right, copy_all_text, copy_current_line_text, copy_left_and_next,
    copy_right_and_next, copy_to_left, copy_to_right, discard_and_proceed, edit_line,
    export_html_report, export_patch, first_diff, folder_copy_to_left, folder_copy_to_right,
    print_diff,
    folder_delete_item, goto_line, last_diff, navigate_bookmark, navigate_conflict, navigate_diff,
    navigate_search, open_file_dialog, open_folder_dialog, open_folder_item, open_in_editor,
    print_diff, redo, replace_all_text, replace_text, rescan, resolve_conflict_use_left,
    resolve_conflict_use_right, run_diff, run_folder_compare, run_plugin, save_file, search_text,
    select_diff, start_compare, start_three_way_compare, switch_tab, toggle_bookmark,
    toggle_ignore_case, toggle_ignore_whitespace, undo,
};
use slint::{ModelRc, SharedString, VecModel};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let window = MainWindow::new().unwrap();
    let state = Rc::new(RefCell::new(AppState::new()));
    let settings = Rc::new(RefCell::new(settings::AppSettings::load()));
    // browse_ctx: tracks which Browse action is pending when path picker dialog is shown
    // 1=open-left-file, 2=open-right-file, 3=open-left-folder, 4=open-right-folder, 5=browse-base
    // 11=direct-open-left, 12=direct-open-right, 13=direct-open-left-folder, 14=direct-open-right-folder
    let browse_ctx: Rc<Cell<i32>> = Rc::new(Cell::new(0));

    // Restore window size
    {
        let s = settings.borrow();
        let size = slint::LogicalSize::new(s.window_width, s.window_height);
        window.window().set_size(size);
    }

    // Apply loaded settings
    {
        let s = settings.borrow();
        window.set_ignore_whitespace(s.ignore_whitespace);
        window.set_ignore_case(s.ignore_case);
        window.set_show_toolbar(s.show_toolbar);
        window.set_opt_ignore_blank_lines(s.ignore_blank_lines);
        window.set_opt_ignore_eol(s.ignore_eol);
        window.set_opt_detect_moved_lines(s.detect_moved_lines);
        window.set_opt_show_line_numbers(s.show_line_numbers);
        window.set_opt_word_wrap(s.word_wrap);
        window.set_opt_syntax_highlighting(s.syntax_highlighting);
        window.set_opt_font_size(s.font_size as i32);
        window.set_opt_tab_width(s.tab_width);
        window.set_opt_enable_context_menu(s.enable_context_menu);
        let theme_index = if s.theme == "dark" { 1 } else { 0 };
        window.set_opt_theme(theme_index);
        window.invoke_set_theme(theme_index);
        let lang_index = if s.language == "ja" { 1 } else { 0 };
        window.set_opt_language(lang_index);
        let lang_code = if s.language == "ja" { "ja" } else { "" };
        if let Err(e) = slint::select_bundled_translation(lang_code) {
            eprintln!("Translation init error: {}", e);
        }
        // Load filter settings into UI
        window.set_opt_line_filters(SharedString::from(s.line_filters.join("|")));
        let sub_patterns: Vec<&str> = s
            .substitution_filters
            .iter()
            .map(|f| f.pattern.as_str())
            .collect();
        let sub_replacements: Vec<&str> = s
            .substitution_filters
            .iter()
            .map(|f| f.replacement.as_str())
            .collect();
        window.set_opt_substitution_patterns(SharedString::from(sub_patterns.join("|")));
        window.set_opt_substitution_replacements(SharedString::from(sub_replacements.join("|")));
        window.set_opt_auto_rescan(s.auto_rescan);
        window.set_opt_folder_exclude_patterns(SharedString::from(&s.folder_exclude_patterns));
        window.set_show_location_pane(s.show_location_pane);
        window.set_show_word_diff(s.show_word_diff);
        window.set_show_detail_pane(s.show_detail_pane);
        // Load plugin list as pipe-separated "name:command" pairs (for options dialog)
        let plugin_str: Vec<String> = s
            .plugins
            .iter()
            .map(|p| format!("{}:{}", p.name, p.command))
            .collect();
        window.set_plugin_list(SharedString::from(plugin_str.join("|")));
        // Build plugin model for dynamic menu
        let plugin_entries: Vec<PluginEntryData> = s
            .plugins
            .iter()
            .map(|p| PluginEntryData {
                name: SharedString::from(&p.name),
                command: SharedString::from(&p.command),
            })
            .collect();
        window.set_plugins(ModelRc::new(VecModel::from(plugin_entries)));

        let mut app = state.borrow_mut();
        let tab = app.current_tab_mut();
        tab.diff_options.ignore_whitespace = s.ignore_whitespace;
        tab.diff_options.ignore_case = s.ignore_case;
        tab.diff_options.line_filters = s.line_filters.clone();
        tab.diff_options.substitution_filters = s
            .substitution_filters
            .iter()
            .map(|f| (f.pattern.clone(), f.replacement.clone()))
            .collect();
        app.folder_exclude_patterns = s
            .folder_exclude_patterns
            .split(';')
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty())
            .collect();
    }

    // Load recent entries into UI
    sync_recent_entries(&window, &settings.borrow());

    // Initialize tab list
    app::sync_tab_list(&window, &state.borrow());

    // Parse CLI flags and positional arguments
    let mut positional: Vec<String> = Vec::new();
    let mut cli_ignore_whitespace = false;
    let mut cli_ignore_case = false;
    let mut cli_ignore_blank_lines = false;
    {
        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--ignore-whitespace" | "-w" => cli_ignore_whitespace = true,
                "--ignore-case" | "-i" => cli_ignore_case = true,
                "--ignore-blank-lines" | "-B" => cli_ignore_blank_lines = true,
                _ => positional.push(args[i].clone()),
            }
            i += 1;
        }
    }

    // Apply CLI flags to initial tab's diff options
    if cli_ignore_whitespace || cli_ignore_case || cli_ignore_blank_lines {
        let mut s = state.borrow_mut();
        let tab = s.current_tab_mut();
        if cli_ignore_whitespace {
            tab.diff_options.ignore_whitespace = true;
        }
        if cli_ignore_case {
            tab.diff_options.ignore_case = true;
        }
        if cli_ignore_blank_lines {
            tab.diff_options.ignore_blank_lines = true;
        }
        // Sync UI toggles
        drop(s);
        if cli_ignore_whitespace {
            window.set_ignore_whitespace(true);
        }
        if cli_ignore_case {
            window.set_ignore_case(true);
        }
    }

    // Handle positional arguments:
    //   winxmerge <left> <right>           — 2-way diff
    //   winxmerge <base> <left> <right>    — 3-way merge
    if positional.len() >= 3 {
        // 3-way merge
        let mut s = state.borrow_mut();
        start_three_way_compare(
            &window,
            &mut s,
            &positional[0],
            &positional[1],
            &positional[2],
        );
        app::sync_tab_list(&window, &s);
    } else if positional.len() >= 2 {
        // 2-way diff
        let left = std::path::PathBuf::from(&positional[0]);
        let right = std::path::PathBuf::from(&positional[1]);
        let mut s = state.borrow_mut();
        {
            let tab = s.current_tab_mut();
            tab.left_path = Some(left);
            tab.right_path = Some(right);
            tab.view_mode = 0;
        }
        window.set_view_mode(0);
        app::run_diff(&window, &mut s);
        app::sync_tab_list(&window, &s);
    }

    // --- Tab management ---
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_new_tab(move || {
            let window = window_weak.unwrap();
            add_tab(&window, &mut state.borrow_mut());
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_close_tab(move |idx| {
            let window = window_weak.unwrap();
            close_tab(&window, &mut state.borrow_mut(), idx);
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_switch_tab(move |idx| {
            let window = window_weak.unwrap();
            switch_tab(&window, &mut state.borrow_mut(), idx);
        });
    }

    // Helper: populate file browser listing from a directory path
    fn populate_file_browser(window: &MainWindow, dir: &std::path::Path) {
        let dir_str = dir.to_string_lossy();
        let display_dir = dir_str.trim_end_matches('/').to_string();
        window.set_file_browser_current_dir(SharedString::from(display_dir));
        window.set_file_browser_selected_index(-1);

        let mut dirs: Vec<String> = Vec::new();
        let mut files: Vec<String> = Vec::new();
        if let Ok(rd) = std::fs::read_dir(dir) {
            for entry in rd.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') {
                    continue;
                }
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    dirs.push(name);
                } else {
                    files.push(name);
                }
            }
        }
        dirs.sort();
        files.sort();
        let entries: Vec<FileEntryData> = dirs
            .into_iter()
            .map(|n| FileEntryData {
                name: SharedString::from(n),
                is_dir: true,
            })
            .chain(files.into_iter().map(|n| FileEntryData {
                name: SharedString::from(n),
                is_dir: false,
            }))
            .collect();
        window.set_file_browser_entries(ModelRc::new(VecModel::from(entries)));
    }

    // Helper: show file browser as fallback when native dialog unavailable
    fn show_file_browser(
        window: &MainWindow,
        browse_ctx: &Cell<i32>,
        ctx: i32,
        current: SharedString,
        is_folder: bool,
    ) {
        browse_ctx.set(ctx);
        window.set_file_browser_ctx(ctx);
        window.set_file_browser_is_folder_mode(is_folder);
        let start = if current.is_empty() {
            dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/"))
        } else {
            let p = std::path::PathBuf::from(current.as_str());
            if p.is_dir() {
                p
            } else {
                p.parent().map(|x| x.to_path_buf()).unwrap_or_else(|| {
                    dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/"))
                })
            }
        };
        populate_file_browser(window, &start);
        window.set_file_browser_visible(true);
    }

    // --- File operations ---
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        let browse_ctx = browse_ctx.clone();
        window.on_open_left_file(move || {
            let window = window_weak.unwrap();
            let is_folder = window.get_open_is_folder_mode();
            if is_folder {
                if let Some(path) = open_folder_dialog("Select left folder") {
                    window.set_open_left_path_input(SharedString::from(
                        path.to_string_lossy().to_string(),
                    ));
                    state.borrow_mut().current_tab_mut().left_folder = Some(path);
                } else {
                    show_file_browser(
                        &window,
                        &browse_ctx,
                        3,
                        window.get_open_left_path_input(),
                        true,
                    );
                }
            } else if let Some(path) = open_file_dialog("Select left file") {
                window.set_open_left_path_input(SharedString::from(
                    path.to_string_lossy().to_string(),
                ));
                state.borrow_mut().current_tab_mut().left_path = Some(path);
            } else {
                show_file_browser(
                    &window,
                    &browse_ctx,
                    1,
                    window.get_open_left_path_input(),
                    false,
                );
            }
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        let browse_ctx = browse_ctx.clone();
        window.on_open_right_file(move || {
            let window = window_weak.unwrap();
            let is_folder = window.get_open_is_folder_mode();
            if is_folder {
                if let Some(path) = open_folder_dialog("Select right folder") {
                    window.set_open_right_path_input(SharedString::from(
                        path.to_string_lossy().to_string(),
                    ));
                    state.borrow_mut().current_tab_mut().right_folder = Some(path);
                } else {
                    show_file_browser(
                        &window,
                        &browse_ctx,
                        4,
                        window.get_open_right_path_input(),
                        true,
                    );
                }
            } else if let Some(path) = open_file_dialog("Select right file") {
                window.set_open_right_path_input(SharedString::from(
                    path.to_string_lossy().to_string(),
                ));
                state.borrow_mut().current_tab_mut().right_path = Some(path);
            } else {
                show_file_browser(
                    &window,
                    &browse_ctx,
                    2,
                    window.get_open_right_path_input(),
                    false,
                );
            }
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        let browse_ctx = browse_ctx.clone();
        window.on_open_folder_left(move || {
            let window = window_weak.unwrap();
            if let Some(path) = open_folder_dialog("Select left folder") {
                window.set_open_left_path_input(SharedString::from(
                    path.to_string_lossy().to_string(),
                ));
                state.borrow_mut().current_tab_mut().left_folder = Some(path);
            } else {
                show_file_browser(
                    &window,
                    &browse_ctx,
                    3,
                    window.get_open_left_path_input(),
                    true,
                );
            }
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        let browse_ctx = browse_ctx.clone();
        window.on_open_folder_right(move || {
            let window = window_weak.unwrap();
            if let Some(path) = open_folder_dialog("Select right folder") {
                window.set_open_right_path_input(SharedString::from(
                    path.to_string_lossy().to_string(),
                ));
                state.borrow_mut().current_tab_mut().right_folder = Some(path);
            } else {
                show_file_browser(
                    &window,
                    &browse_ctx,
                    4,
                    window.get_open_right_path_input(),
                    true,
                );
            }
        });
    }

    // Start compare (from open dialog)
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        let settings = settings.clone();
        window.on_start_compare(move |left, right, is_folder| {
            let window = window_weak.unwrap();
            let is_three_way = window.get_open_is_three_way();
            if is_three_way {
                let base = window.get_open_base_path_input().to_string();
                start_three_way_compare(&window, &mut state.borrow_mut(), &base, &left, &right);
            } else {
                start_compare(&window, &mut state.borrow_mut(), &left, &right, is_folder);
            }
            let mut s = settings.borrow_mut();
            s.add_recent(&left, &right, is_folder);
            s.save();
            sync_recent_entries(&window, &s);
        });
    }

    // Discard unsaved and proceed
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        let browse_ctx = browse_ctx.clone();
        window.on_discard_and_proceed(move || {
            let window = window_weak.unwrap();
            let ww = window.as_weak();
            let bc = browse_ctx.clone();
            discard_and_proceed(&window, &mut state.borrow_mut(), move |ctx, _title| {
                let w = ww.unwrap();
                let is_folder = ctx == 13 || ctx == 14;
                show_file_browser(&w, &bc, ctx, SharedString::from(""), is_folder);
            });
        });
    }

    // Folder item double click
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_folder_item_double_clicked(move |idx| {
            let window = window_weak.unwrap();
            open_folder_item(&window, &mut state.borrow_mut(), idx);
        });
    }

    // Back to folder view
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_back_to_folder_view(move || {
            let window = window_weak.unwrap();
            let mut s = state.borrow_mut();
            s.current_tab_mut().view_mode = 1;
            window.set_view_mode(1);
            run_folder_compare(&window, &mut s);
        });
    }

    // Navigation
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_next_diff(move || {
            let window = window_weak.unwrap();
            navigate_diff(&window, &mut state.borrow_mut(), true);
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_prev_diff(move || {
            let window = window_weak.unwrap();
            navigate_diff(&window, &mut state.borrow_mut(), false);
        });
    }

    // Merge
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_copy_to_right(move |diff_index| {
            let window = window_weak.unwrap();
            copy_to_right(&window, &mut state.borrow_mut(), diff_index);
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_copy_to_left(move |diff_index| {
            let window = window_weak.unwrap();
            copy_to_left(&window, &mut state.borrow_mut(), diff_index);
        });
    }

    // Save
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_save_left(move || {
            let window = window_weak.unwrap();
            save_file(&window, &mut state.borrow_mut(), true);
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_save_right(move || {
            let window = window_weak.unwrap();
            save_file(&window, &mut state.borrow_mut(), false);
        });
    }

    // Diff options
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        let settings = settings.clone();
        window.on_toggle_ignore_whitespace(move || {
            let window = window_weak.unwrap();
            toggle_ignore_whitespace(&window, &mut state.borrow_mut());
            let mut s = settings.borrow_mut();
            s.ignore_whitespace = window.get_ignore_whitespace();
            s.save();
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        let settings = settings.clone();
        window.on_toggle_ignore_case(move || {
            let window = window_weak.unwrap();
            toggle_ignore_case(&window, &mut state.borrow_mut());
            let mut s = settings.borrow_mut();
            s.ignore_case = window.get_ignore_case();
            s.save();
        });
    }

    // Search
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_search(move |query| {
            let window = window_weak.unwrap();
            search_text(&window, &mut state.borrow_mut(), &query);
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_search_next(move || {
            let window = window_weak.unwrap();
            navigate_search(&window, &mut state.borrow_mut(), true);
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_search_prev(move || {
            let window = window_weak.unwrap();
            navigate_search(&window, &mut state.borrow_mut(), false);
        });
    }

    // Replace
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_replace(move |search, replacement| {
            let window = window_weak.unwrap();
            replace_text(&window, &mut state.borrow_mut(), &search, &replacement);
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_replace_all(move |search, replacement| {
            let window = window_weak.unwrap();
            replace_all_text(&window, &mut state.borrow_mut(), &search, &replacement);
        });
    }

    // Select diff (from line number click)
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_select_diff(move |idx| {
            let window = window_weak.unwrap();
            select_diff(&window, &mut state.borrow_mut(), idx);
        });
    }

    // Undo/Redo
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_undo(move || {
            let window = window_weak.unwrap();
            undo(&window, &mut state.borrow_mut());
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_redo(move || {
            let window = window_weak.unwrap();
            redo(&window, &mut state.borrow_mut());
        });
    }

    // First/Last diff
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_first_diff(move || {
            let window = window_weak.unwrap();
            first_diff(&window, &mut state.borrow_mut());
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_last_diff(move || {
            let window = window_weak.unwrap();
            last_diff(&window, &mut state.borrow_mut());
        });
    }

    // Go to line
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_goto_line(move |line_no| {
            let window = window_weak.unwrap();
            goto_line(&window, &state.borrow(), line_no);
        });
    }

    // Bookmarks
    {
        let state = state.clone();
        window.on_toggle_bookmark(move |line_index| {
            toggle_bookmark(&mut state.borrow_mut(), line_index);
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_next_bookmark(move || {
            let window = window_weak.unwrap();
            navigate_bookmark(&window, &mut state.borrow_mut(), true);
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_prev_bookmark(move || {
            let window = window_weak.unwrap();
            navigate_bookmark(&window, &mut state.borrow_mut(), false);
        });
    }

    // Folder file operations
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_folder_copy_to_right(move |idx| {
            let window = window_weak.unwrap();
            folder_copy_to_right(&window, &mut state.borrow_mut(), idx);
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_folder_copy_to_left(move |idx| {
            let window = window_weak.unwrap();
            folder_copy_to_left(&window, &mut state.borrow_mut(), idx);
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_folder_delete_item(move |idx| {
            let window = window_weak.unwrap();
            folder_delete_item(&window, &mut state.borrow_mut(), idx);
        });
    }

    // Copy text to clipboard (from context menu)
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_copy_left_text(move || {
            let window = window_weak.unwrap();
            copy_current_line_text(&window, &state.borrow(), true);
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_copy_right_text(move || {
            let window = window_weak.unwrap();
            copy_current_line_text(&window, &state.borrow(), false);
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_copy_all_left(move || {
            let window = window_weak.unwrap();
            copy_all_text(&window, &state.borrow(), true);
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_copy_all_right(move || {
            let window = window_weak.unwrap();
            copy_all_text(&window, &state.borrow(), false);
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_copy_right_and_next(move || {
            let window = window_weak.unwrap();
            copy_right_and_next(&window, &mut state.borrow_mut());
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_copy_left_and_next(move || {
            let window = window_weak.unwrap();
            copy_left_and_next(&window, &mut state.borrow_mut());
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_copy_all_diffs_right(move || {
            let window = window_weak.unwrap();
            copy_all_diffs_to_right(&window, &mut state.borrow_mut());
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_copy_all_diffs_left(move || {
            let window = window_weak.unwrap();
            copy_all_diffs_to_left(&window, &mut state.borrow_mut());
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_edit_left_line(move |idx, text| {
            let window = window_weak.unwrap();
            edit_line(&window, &mut state.borrow_mut(), idx, &text, true);
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_edit_right_line(move |idx, text| {
            let window = window_weak.unwrap();
            edit_line(&window, &mut state.borrow_mut(), idx, &text, false);
        });
    }

    // Apply options
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        let settings = settings.clone();
        window.on_apply_options(move || {
            let window = window_weak.unwrap();
            apply_options(&window, &mut state.borrow_mut(), &mut settings.borrow_mut());
        });
    }

    // Export HTML
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_export_html(move || {
            let window = window_weak.unwrap();
            export_html_report(&window, &state.borrow());
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_export_patch(move || {
            let window = window_weak.unwrap();
            export_patch(&window, &state.borrow());
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_print_diff(move || {
            let window = window_weak.unwrap();
            print_diff(&window, &state.borrow());
        });
    }

    // Browse base file (for 3-way)
    {
        let window_weak = window.as_weak();
        let browse_ctx = browse_ctx.clone();
        window.on_browse_base(move || {
            let window = window_weak.unwrap();
            if let Some(path) = open_file_dialog("Select base file") {
                window.set_open_base_path_input(SharedString::from(
                    path.to_string_lossy().to_string(),
                ));
            } else {
                show_file_browser(
                    &window,
                    &browse_ctx,
                    5,
                    window.get_open_base_path_input(),
                    false,
                );
            }
        });
    }

    // File browser dialog callbacks
    {
        let window_weak = window.as_weak();
        window.on_file_browser_navigate(move |path| {
            let window = window_weak.unwrap();
            let p = std::path::PathBuf::from(path.as_str());
            if p.is_dir() {
                populate_file_browser(&window, &p);
            }
        });
    }

    {
        let window_weak = window.as_weak();
        window.on_file_browser_go_parent(move || {
            let window = window_weak.unwrap();
            let current = window.get_file_browser_current_dir().to_string();
            let p = std::path::PathBuf::from(&current);
            if let Some(parent) = p.parent() {
                populate_file_browser(&window, &parent.to_path_buf());
            }
        });
    }

    {
        let window_weak = window.as_weak();
        window.on_file_browser_cancelled(move || {
            window_weak.unwrap().set_file_browser_visible(false);
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        let browse_ctx = browse_ctx.clone();
        window.on_file_browser_confirmed(move |path| {
            let window = window_weak.unwrap();
            window.set_file_browser_visible(false);
            let path_str = path.to_string();
            if path_str.is_empty() {
                return;
            }
            // Normalize path (remove double slashes from string concat)
            let path_buf = std::path::PathBuf::from(&path_str);
            let normalized = SharedString::from(path_buf.to_string_lossy().to_string());
            let ctx = browse_ctx.get();
            browse_ctx.set(0);
            match ctx {
                1 => {
                    window.set_open_left_path_input(normalized);
                    state.borrow_mut().current_tab_mut().left_path = Some(path_buf);
                }
                2 => {
                    window.set_open_right_path_input(normalized);
                    state.borrow_mut().current_tab_mut().right_path = Some(path_buf);
                }
                3 => {
                    window.set_open_left_path_input(normalized);
                    state.borrow_mut().current_tab_mut().left_folder = Some(path_buf);
                }
                4 => {
                    window.set_open_right_path_input(normalized);
                    state.borrow_mut().current_tab_mut().right_folder = Some(path_buf);
                }
                5 => {
                    window.set_open_base_path_input(normalized);
                }
                11 => {
                    let mut s = state.borrow_mut();
                    s.current_tab_mut().left_path = Some(path_buf);
                    s.current_tab_mut().view_mode = 0;
                    drop(s);
                    window.set_view_mode(0);
                    run_diff(&window, &mut state.borrow_mut());
                }
                12 => {
                    let mut s = state.borrow_mut();
                    s.current_tab_mut().right_path = Some(path_buf);
                    s.current_tab_mut().view_mode = 0;
                    drop(s);
                    window.set_view_mode(0);
                    run_diff(&window, &mut state.borrow_mut());
                }
                13 => {
                    state.borrow_mut().current_tab_mut().left_folder = Some(path_buf);
                    run_folder_compare(&window, &mut state.borrow_mut());
                }
                14 => {
                    state.borrow_mut().current_tab_mut().right_folder = Some(path_buf);
                    run_folder_compare(&window, &mut state.borrow_mut());
                }
                _ => {}
            }
        });
    }

    // 3-way conflict navigation
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_next_conflict(move || {
            let window = window_weak.unwrap();
            navigate_conflict(&window, &mut state.borrow_mut(), true);
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_prev_conflict(move || {
            let window = window_weak.unwrap();
            navigate_conflict(&window, &mut state.borrow_mut(), false);
        });
    }

    // 3-way conflict resolution
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_use_left(move |idx| {
            let window = window_weak.unwrap();
            resolve_conflict_use_left(&window, &mut state.borrow_mut(), idx);
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_use_right(move |idx| {
            let window = window_weak.unwrap();
            resolve_conflict_use_right(&window, &mut state.borrow_mut(), idx);
        });
    }

    // Open in external editor
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        let settings = settings.clone();
        window.on_open_left_in_editor(move || {
            let window = window_weak.unwrap();
            let editor = settings.borrow().external_editor.clone();
            open_in_editor(&window, &state.borrow(), true, &editor);
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        let settings = settings.clone();
        window.on_open_right_in_editor(move || {
            let window = window_weak.unwrap();
            let editor = settings.borrow().external_editor.clone();
            open_in_editor(&window, &state.borrow(), false, &editor);
        });
    }

    // Plugin execution
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        let settings = settings.clone();
        window.on_run_plugin(move |_plugin_list| {
            let window = window_weak.unwrap();
            let s = settings.borrow();
            // Run the first plugin (for now)
            if let Some(plugin) = s.plugins.first() {
                run_plugin(&window, &state.borrow(), &plugin.command);
            }
        });
    }

    // Rescan
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_rescan(move || {
            let window = window_weak.unwrap();
            rescan(&window, &mut state.borrow_mut());
        });
    }

    // Auto-rescan timer
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_check_auto_rescan(move || {
            let window = window_weak.unwrap();
            if window.get_opt_auto_rescan() && check_files_changed(&state.borrow()) {
                rescan(&window, &mut state.borrow_mut());
            }
        });
    }

    // Set up auto-rescan timer (check every 2 seconds)
    {
        let window_weak = window.as_weak();
        let timer = slint::Timer::default();
        timer.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_secs(2),
            move || {
                if let Some(window) = window_weak.upgrade() {
                    if window.get_opt_auto_rescan() {
                        window.invoke_check_auto_rescan();
                    }
                }
            },
        );
        // Keep timer alive by leaking it (it runs for the lifetime of the app)
        std::mem::forget(timer);
    }

    // Open recent entry
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        let settings = settings.clone();
        window.on_open_recent(move |idx| {
            let window = window_weak.unwrap();
            let s = settings.borrow();
            if let Some(entry) = s.recent_files.get(idx as usize) {
                let left = entry.left_path.clone();
                let right = entry.right_path.clone();
                let is_folder = entry.is_folder;
                drop(s);
                start_compare(&window, &mut state.borrow_mut(), &left, &right, is_folder);
            }
        });
    }

    // Save window size on close
    {
        let settings = settings.clone();
        let window_weak = window.as_weak();
        window.window().on_close_requested(move || {
            let window = window_weak.unwrap();
            let size = window
                .window()
                .size()
                .to_logical(window.window().scale_factor());
            let mut s = settings.borrow_mut();
            s.window_width = size.width;
            s.window_height = size.height;
            s.show_toolbar = window.get_show_toolbar();
            s.show_location_pane = window.get_show_location_pane();
            s.show_word_diff = window.get_show_word_diff();
            s.show_detail_pane = window.get_show_detail_pane();
            s.save();
            slint::CloseRequestResponse::HideWindow
        });
    }

    window.run().unwrap();
}

fn sync_recent_entries(window: &MainWindow, settings: &settings::AppSettings) {
    let entries: Vec<RecentEntryData> = settings
        .recent_files
        .iter()
        .map(|r| RecentEntryData {
            left_path: SharedString::from(&r.left_path),
            right_path: SharedString::from(&r.right_path),
            is_folder: r.is_folder,
        })
        .collect();
    window.set_recent_entries(ModelRc::new(VecModel::from(entries)));
}
