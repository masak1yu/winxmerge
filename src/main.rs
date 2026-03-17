mod app;
mod diff;
mod models;

slint::include_modules!();

use std::cell::RefCell;
use std::rc::Rc;

use app::{copy_to_left, copy_to_right, navigate_diff, open_file_dialog, run_diff, save_file, AppState};

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
                state.borrow_mut().left_path = Some(path);
                run_diff(&window, &mut state.borrow_mut());
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
                state.borrow_mut().right_path = Some(path);
                run_diff(&window, &mut state.borrow_mut());
            }
        });
    }

    // Next diff
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_next_diff(move || {
            let window = window_weak.unwrap();
            navigate_diff(&window, &mut state.borrow_mut(), true);
        });
    }

    // Prev diff
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_prev_diff(move || {
            let window = window_weak.unwrap();
            navigate_diff(&window, &mut state.borrow_mut(), false);
        });
    }

    // Copy to right
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_copy_to_right(move |diff_index| {
            let window = window_weak.unwrap();
            copy_to_right(&window, &mut state.borrow_mut(), diff_index);
        });
    }

    // Copy to left
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_copy_to_left(move |diff_index| {
            let window = window_weak.unwrap();
            copy_to_left(&window, &mut state.borrow_mut(), diff_index);
        });
    }

    // Save left
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_save_left(move || {
            let window = window_weak.unwrap();
            save_file(&window, &mut state.borrow_mut(), true);
        });
    }

    // Save right
    {
        let window_weak = window.as_weak();
        let state = state.clone();
        window.on_save_right(move || {
            let window = window_weak.unwrap();
            save_file(&window, &mut state.borrow_mut(), false);
        });
    }

    window.run().unwrap();
}
