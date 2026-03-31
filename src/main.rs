#[cfg(not(target_arch = "wasm32"))]
mod app;
#[cfg(not(target_arch = "wasm32"))]
mod archive;
#[cfg(not(target_arch = "wasm32"))]
mod csv;
mod diff;
#[cfg(not(target_arch = "wasm32"))]
mod encoding;
#[cfg(not(target_arch = "wasm32"))]
mod excel;
#[cfg(not(target_arch = "wasm32"))]
mod export;
mod highlight;
#[cfg(not(target_arch = "wasm32"))]
mod image_compare;
mod models;
#[cfg(not(target_arch = "wasm32"))]
mod settings;

#[cfg(target_arch = "wasm32")]
mod wasm;

slint::include_modules!();

#[cfg(not(target_arch = "wasm32"))]
use std::cell::{Cell, RefCell};
#[cfg(not(target_arch = "wasm32"))]
use std::rc::Rc;

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
use app::{
    AppState, add_tab, apply_options, apply_pending_diff_if_ready, check_files_changed, close_tab,
    compare_clipboard_as_left, compare_clipboard_as_right, copy_all_diffs_to_left,
    copy_all_diffs_to_right, copy_all_text, copy_current_line_text, copy_left_and_next,
    copy_right_and_next, copy_selection_to_left, copy_selection_to_right, copy_to_left,
    copy_to_right, discard_and_proceed, edit_line, export_all_comments, export_csv_report,
    export_folder_html_report, export_html_report, export_patch, export_xlsx_report, first_diff,
    folder_copy_to_left, folder_copy_to_right, folder_delete_item, goto_line, last_diff,
    navigate_bookmark, navigate_conflict, navigate_diff, navigate_diff_by_status, navigate_search,
    open_file_dialog, open_folder_dialog, open_folder_item, open_in_editor,
    paste_clipboard_path_left, paste_clipboard_path_right, preview_folder_item, print_diff, redo,
    reorder_tab, replace_all_text, replace_text, rescan, resolve_conflict_use_left,
    resolve_conflict_use_right, run_diff, run_folder_compare, run_plugin, save_file, search_text,
    select_diff, set_diff_comment, set_diff_filter, set_row_selection, sort_folder, start_compare,
    start_three_way_compare, switch_tab, toggle_bookmark, toggle_ignore_case,
    toggle_ignore_whitespace, undo,
};
#[cfg(not(target_arch = "wasm32"))]
use slint::{ModelRc, SharedString, VecModel};

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let args: Vec<String> = std::env::args().collect();

    // --clear-history: wipe session + recent files and exit
    if args.iter().any(|a| a == "--clear-history") {
        let mut s = settings::AppSettings::load();
        s.session = Vec::new();
        s.recent_files = Vec::new();
        s.save();
        println!("[winxmerge] comparison history cleared");
        return;
    }

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
        window.set_opt_folder_max_depth(s.folder_max_depth as i32);
        window.set_opt_folder_min_size(s.folder_min_size as i32);
        window.set_opt_folder_max_size(s.folder_max_size as i32);
        window.set_opt_folder_modified_after(SharedString::from(&s.folder_modified_after));
        window.set_opt_folder_modified_before(SharedString::from(&s.folder_modified_before));
        window.set_show_location_pane(s.show_location_pane);
        window.set_show_word_diff(s.show_word_diff);
        window.set_show_detail_pane(s.show_detail_pane);
        window.set_opt_diff_only(s.diff_only);
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
    } else {
        // No CLI args: restore previous session
        let session = settings.borrow().session.clone();
        if !session.is_empty() {
            let mut first = true;
            for entry in &session {
                if entry.left_path.is_empty() || entry.right_path.is_empty() {
                    continue;
                }
                // Skip entries where files/folders no longer exist on disk
                if !std::path::Path::new(&entry.left_path).exists()
                    || !std::path::Path::new(&entry.right_path).exists()
                {
                    continue;
                }
                if !first {
                    add_tab(&window, &mut state.borrow_mut());
                }
                first = false;
                let mut s = state.borrow_mut();
                if !entry.base_path.is_empty() {
                    start_three_way_compare(
                        &window,
                        &mut s,
                        &entry.base_path,
                        &entry.left_path,
                        &entry.right_path,
                    );
                } else {
                    let left_p = std::path::PathBuf::from(&entry.left_path);
                    let is_folder = left_p.is_dir();
                    start_compare(
                        &window,
                        &mut s,
                        &entry.left_path,
                        &entry.right_path,
                        is_folder,
                    );
                }
                // Restore extended session fields
                {
                    let tab = s.current_tab_mut();
                    if !entry.left_encoding.is_empty() {
                        tab.left_encoding = entry.left_encoding.clone();
                    }
                    if !entry.right_encoding.is_empty() {
                        tab.right_encoding = entry.right_encoding.clone();
                    }
                    if !entry.left_eol.is_empty() {
                        tab.left_eol_type = entry.left_eol.clone();
                    }
                    if !entry.right_eol.is_empty() {
                        tab.right_eol_type = entry.right_eol.clone();
                    }
                    // tab_width is a UI setting, not per-tab state
                    tab.diff_status_filter = entry.diff_status_filter;
                    for sc in &entry.diff_comments {
                        tab.diff_comments.insert(sc.block_index, sc.text.clone());
                    }
                }
                app::sync_tab_list(&window, &s);
            }
        }
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

    // Status-filtered diff navigation
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_next_diff_status(move |status| {
            let window = window_weak.unwrap();
            navigate_diff_by_status(&window, &mut state.borrow_mut(), true, status);
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_prev_diff_status(move |status| {
            let window = window_weak.unwrap();
            navigate_diff_by_status(&window, &mut state.borrow_mut(), false, status);
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

    // Folder sort
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_folder_sort(move |col| {
            let window = window_weak.unwrap();
            sort_folder(&window, &mut state.borrow_mut(), col);
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

    // Export All Comments CSV
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_export_comments_csv(move || {
            let window = window_weak.unwrap();
            export_all_comments(&window, &state.borrow(), false);
        });
    }

    // Export All Comments JSON
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_export_comments_json(move || {
            let window = window_weak.unwrap();
            export_all_comments(&window, &state.borrow(), true);
        });
    }

    // Export Excel
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_export_xlsx(move || {
            let window = window_weak.unwrap();
            export_xlsx_report(&window, &state.borrow());
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

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_export_csv(move || {
            let window = window_weak.unwrap();
            export_csv_report(&window, &state.borrow(), false);
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_export_tsv(move || {
            let window = window_weak.unwrap();
            export_csv_report(&window, &state.borrow(), true);
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_export_folder_html(move || {
            let window = window_weak.unwrap();
            export_folder_html_report(&window, &state.borrow());
        });
    }

    // Row selection (multi-line)
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_row_shift_clicked(move |idx| {
            let window = window_weak.unwrap();
            set_row_selection(&window, &mut state.borrow_mut(), idx, true);
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_copy_selection_right(move || {
            let window = window_weak.unwrap();
            copy_selection_to_right(&window, &mut state.borrow_mut());
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_copy_selection_left(move || {
            let window = window_weak.unwrap();
            copy_selection_to_left(&window, &mut state.borrow_mut());
        });
    }

    // Clipboard comparison
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_compare_clipboard_left(move || {
            let window = window_weak.unwrap();
            compare_clipboard_as_left(&window, &mut state.borrow_mut());
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_compare_clipboard_right(move || {
            let window = window_weak.unwrap();
            compare_clipboard_as_right(&window, &mut state.borrow_mut());
        });
    }

    // Tab reorder
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_tab_move_left(move |idx| {
            let window = window_weak.unwrap();
            let from = idx as usize;
            if from > 0 {
                reorder_tab(&window, &mut state.borrow_mut(), from, from - 1);
            }
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_tab_move_right(move |idx| {
            let window = window_weak.unwrap();
            let from = idx as usize;
            let to = from + 1;
            reorder_tab(&window, &mut state.borrow_mut(), from, to);
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
        window.on_run_plugin(move |command| {
            let window = window_weak.unwrap();
            run_plugin(&window, &state.borrow(), command.as_str());
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

    // Background diff result polling timer (100ms interval)
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        let timer = slint::Timer::default();
        timer.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_millis(100),
            move || {
                if let Some(window) = window_weak.upgrade() {
                    apply_pending_diff_if_ready(&window, &mut state.borrow_mut());
                }
            },
        );
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

    // Diff block comments
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_diff_comment_changed(move |comment| {
            let window = window_weak.unwrap();
            set_diff_comment(&window, &mut state.borrow_mut(), comment.to_string());
        });
    }

    // Diff status filter
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_diff_filter_changed(move |filter| {
            let window = window_weak.unwrap();
            set_diff_filter(&window, &mut state.borrow_mut(), filter);
        });
    }

    // Clipboard paste path
    {
        let window_weak = window.as_weak();
        window.on_paste_left_path(move || {
            paste_clipboard_path_left(&window_weak.unwrap());
        });
    }

    {
        let window_weak = window.as_weak();
        window.on_paste_right_path(move || {
            paste_clipboard_path_right(&window_weak.unwrap());
        });
    }

    // Folder item preview
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_folder_item_preview(move |idx| {
            let window = window_weak.unwrap();
            preview_folder_item(&window, &state.borrow(), idx);
        });
    }

    // Keyboard shortcuts dialog close
    {
        let window_weak = window.as_weak();
        window.on_shortcuts_dialog_close(move || {
            window_weak.unwrap().set_show_shortcuts_dialog(false);
        });
    }

    // Save window size on close
    {
        let settings = settings.clone();
        let state = state.clone();
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
            s.diff_only = window.get_opt_diff_only();
            s.folder_max_depth = window.get_opt_folder_max_depth().max(0) as usize;
            s.folder_min_size = window.get_opt_folder_min_size().max(0) as u64;
            s.folder_max_size = window.get_opt_folder_max_size().max(0) as u64;
            s.folder_modified_after = window.get_opt_folder_modified_after().to_string();
            s.folder_modified_before = window.get_opt_folder_modified_before().to_string();
            // Save session (open tabs)
            let app = state.borrow();
            s.session = app
                .tabs
                .iter()
                .filter(|tab| tab.view_mode != 2) // skip open-dialog tabs
                .filter_map(|tab| {
                    let left = match tab.view_mode {
                        1 => tab
                            .left_folder
                            .as_ref()
                            .map(|p| p.to_string_lossy().to_string()),
                        _ => tab
                            .left_path
                            .as_ref()
                            .map(|p| p.to_string_lossy().to_string()),
                    }?;
                    let right = match tab.view_mode {
                        1 => tab
                            .right_folder
                            .as_ref()
                            .map(|p| p.to_string_lossy().to_string()),
                        _ => tab
                            .right_path
                            .as_ref()
                            .map(|p| p.to_string_lossy().to_string()),
                    }?;
                    let base = tab
                        .base_path
                        .as_ref()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let diff_comments: Vec<settings::SessionComment> = tab
                        .diff_comments
                        .iter()
                        .map(|(&block_index, text)| settings::SessionComment {
                            block_index,
                            text: text.clone(),
                        })
                        .collect();
                    Some(settings::SessionEntry {
                        left_path: left,
                        right_path: right,
                        base_path: base,
                        left_encoding: tab.left_encoding.clone(),
                        right_encoding: tab.right_encoding.clone(),
                        left_eol: tab.left_eol_type.clone(),
                        right_eol: tab.right_eol_type.clone(),
                        tab_width: window.get_opt_tab_width(),
                        diff_only: false,
                        diff_status_filter: tab.diff_status_filter,
                        diff_comments,
                    })
                })
                .collect();
            s.save();
            slint::CloseRequestResponse::HideWindow
        });
    }

    window.run().unwrap();
}

#[cfg(not(target_arch = "wasm32"))]
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
