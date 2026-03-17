mod app;
mod diff;
mod encoding;
mod highlight;
mod models;
mod export;
mod settings;

slint::include_modules!();

use std::cell::RefCell;
use std::rc::Rc;

use app::{
    add_tab, apply_options, close_tab, copy_current_line_text, copy_to_left, copy_to_right,
    discard_and_proceed, export_html_report, navigate_diff, navigate_search, open_file_dialog,
    open_folder_dialog, open_folder_item, redo, replace_all_text, replace_text,
    run_folder_compare, save_file, search_text, select_diff, start_compare, switch_tab,
    toggle_ignore_case, toggle_ignore_whitespace, undo, AppState,
};
use slint::SharedString;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let window = MainWindow::new().unwrap();
    let state = Rc::new(RefCell::new(AppState::new()));
    let settings = Rc::new(RefCell::new(settings::AppSettings::load()));

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
        let mut app = state.borrow_mut();
        app.current_tab_mut().diff_options.ignore_whitespace = s.ignore_whitespace;
        app.current_tab_mut().diff_options.ignore_case = s.ignore_case;
    }

    // Initialize tab list
    app::sync_tab_list(&window, &state.borrow());

    // Handle CLI arguments: winxmerge <left> <right>
    if args.len() >= 3 {
        let left = std::path::PathBuf::from(&args[1]);
        let right = std::path::PathBuf::from(&args[2]);
        let mut s = state.borrow_mut();
        let tab = s.current_tab_mut();
        tab.left_path = Some(left);
        tab.right_path = Some(right);
        tab.view_mode = 0;
        drop(tab);
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

    // --- File operations ---
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_open_left_file(move || {
            let window = window_weak.unwrap();
            let is_folder = window.get_open_is_folder_mode();
            if is_folder {
                if let Some(path) = open_folder_dialog("Select left folder") {
                    window.set_open_left_path_input(SharedString::from(
                        path.to_string_lossy().to_string(),
                    ));
                    state.borrow_mut().current_tab_mut().left_folder = Some(path);
                }
            } else {
                if let Some(path) = open_file_dialog("Select left file") {
                    window.set_open_left_path_input(SharedString::from(
                        path.to_string_lossy().to_string(),
                    ));
                    state.borrow_mut().current_tab_mut().left_path = Some(path);
                }
            }
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_open_right_file(move || {
            let window = window_weak.unwrap();
            let is_folder = window.get_open_is_folder_mode();
            if is_folder {
                if let Some(path) = open_folder_dialog("Select right folder") {
                    window.set_open_right_path_input(SharedString::from(
                        path.to_string_lossy().to_string(),
                    ));
                    state.borrow_mut().current_tab_mut().right_folder = Some(path);
                }
            } else {
                if let Some(path) = open_file_dialog("Select right file") {
                    window.set_open_right_path_input(SharedString::from(
                        path.to_string_lossy().to_string(),
                    ));
                    state.borrow_mut().current_tab_mut().right_path = Some(path);
                }
            }
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_open_folder_left(move || {
            if let Some(path) = open_folder_dialog("Select left folder") {
                let window = window_weak.unwrap();
                window.set_open_left_path_input(SharedString::from(
                    path.to_string_lossy().to_string(),
                ));
                state.borrow_mut().current_tab_mut().left_folder = Some(path);
            }
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_open_folder_right(move || {
            if let Some(path) = open_folder_dialog("Select right folder") {
                let window = window_weak.unwrap();
                window.set_open_right_path_input(SharedString::from(
                    path.to_string_lossy().to_string(),
                ));
                state.borrow_mut().current_tab_mut().right_folder = Some(path);
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
            start_compare(&window, &mut state.borrow_mut(), &left, &right, is_folder);
            let mut s = settings.borrow_mut();
            s.add_recent(&left, &right, is_folder);
            s.save();
        });
    }

    // Discard unsaved and proceed
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_discard_and_proceed(move || {
            let window = window_weak.unwrap();
            discard_and_proceed(&window, &mut state.borrow_mut());
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

    window.run().unwrap();
}
