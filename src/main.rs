mod app;
mod diff;
mod encoding;
mod models;

slint::include_modules!();

use std::cell::RefCell;
use std::rc::Rc;

use app::{
    copy_to_left, copy_to_right, navigate_diff, navigate_search, open_file_dialog,
    open_folder_dialog, open_folder_item, run_diff, run_folder_compare, save_file, search_text,
    toggle_ignore_case, toggle_ignore_whitespace, AppState,
};

fn main() {
    let window = MainWindow::new().unwrap();
    let state = Rc::new(RefCell::new(AppState::new()));

    // Open left file
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_open_left_file(move || {
            if let Some(path) = open_file_dialog("Select left file") {
                let window = window_weak.unwrap();
                let mut s = state.borrow_mut();
                s.left_path = Some(path);
                window.set_view_mode(0);
                run_diff(&window, &mut s);
            }
        });
    }

    // Open right file
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_open_right_file(move || {
            if let Some(path) = open_file_dialog("Select right file") {
                let window = window_weak.unwrap();
                let mut s = state.borrow_mut();
                s.right_path = Some(path);
                window.set_view_mode(0);
                run_diff(&window, &mut s);
            }
        });
    }

    // Open left folder
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_open_folder_left(move || {
            if let Some(path) = open_folder_dialog("Select left folder") {
                let window = window_weak.unwrap();
                let mut s = state.borrow_mut();
                s.left_folder = Some(path);
                run_folder_compare(&window, &mut s);
            }
        });
    }

    // Open right folder
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_open_folder_right(move || {
            if let Some(path) = open_folder_dialog("Select right folder") {
                let window = window_weak.unwrap();
                let mut s = state.borrow_mut();
                s.right_folder = Some(path);
                run_folder_compare(&window, &mut s);
            }
        });
    }

    // Folder item double click
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_folder_item_double_clicked(move |idx| {
            let window = window_weak.unwrap();
            let mut s = state.borrow_mut();
            open_folder_item(&window, &mut s, idx);
        });
    }

    // Back to folder view
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_back_to_folder_view(move || {
            let window = window_weak.unwrap();
            let mut s = state.borrow_mut();
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
        window.on_toggle_ignore_whitespace(move || {
            let window = window_weak.unwrap();
            toggle_ignore_whitespace(&window, &mut state.borrow_mut());
        });
    }

    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_toggle_ignore_case(move || {
            let window = window_weak.unwrap();
            toggle_ignore_case(&window, &mut state.borrow_mut());
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

    window.run().unwrap();
}
