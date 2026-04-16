use std::cell::{Cell, RefCell};
use std::rc::Rc;

use js_sys::Promise;
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

use crate::diff::engine::{DiffOptions, compute_diff_with_options};
use crate::models::diff_line::LineStatus;

// Clipboard shim defined in index.html — avoids web_sys_unstable_apis which
// breaks softbuffer when that cfg flag is active.
#[wasm_bindgen]
extern "C" {
    fn winxReadClipboard() -> Promise;
}

fn compute_stats(result: &crate::models::diff_line::DiffResult) -> (i32, i32, i32) {
    let mut added = 0i32;
    let mut removed = 0i32;
    let mut modified = 0i32;
    for line in &result.lines {
        match line.status {
            LineStatus::Added => added += 1,
            LineStatus::Removed => removed += 1,
            LineStatus::Modified => modified += 1,
            _ => {}
        }
    }
    (added, removed, modified)
}

/// Read text from the clipboard and call callback with the text.
/// Uses a JS shim (winxReadClipboard) defined in index.html to avoid
/// depending on web_sys_unstable_apis.
fn paste_from_clipboard(callback: impl Fn(String) + 'static) {
    let promise: Promise = winxReadClipboard();
    wasm_bindgen_futures::spawn_local(async move {
        if let Ok(text_js) = wasm_bindgen_futures::JsFuture::from(promise).await {
            if let Some(text) = text_js.as_string() {
                callback(text);
            }
        }
    });
}

/// Build per-pane PaneLineData from a DiffResult.
fn build_pane_line_data(
    result: &crate::models::diff_line::DiffResult,
) -> (Vec<crate::PaneLineData>, Vec<crate::PaneLineData>) {
    let mut left_lines = Vec::with_capacity(result.lines.len());
    let mut right_lines = Vec::with_capacity(result.lines.len());
    for (i, line) in result.lines.iter().enumerate() {
        let status = match line.status {
            LineStatus::Equal => 0,
            LineStatus::Added => 1,
            LineStatus::Removed => 2,
            LineStatus::Modified => 3,
            LineStatus::Moved => 4,
        };

        let diff_index = if result.diff_positions.contains(&i) {
            result
                .diff_positions
                .iter()
                .position(|&p| p == i)
                .map(|p| p as i32)
                .unwrap_or(-1)
        } else {
            -1
        };

        let left_word_diff = encode_word_diff(&line.left_word_segments);
        let right_word_diff = encode_word_diff(&line.right_word_segments);

        left_lines.push(crate::PaneLineData {
            line_no: SharedString::from(
                line.left_line_no.map(|n| n.to_string()).unwrap_or_default(),
            ),
            text: SharedString::from(&line.left_text),
            is_ghost: line.left_line_no.is_none(),
            status,
            diff_index,
            word_diff: SharedString::from(left_word_diff),
            is_current_diff: false,
            is_search_match: false,
            is_selected: false,
            highlight: -1,
        });

        right_lines.push(crate::PaneLineData {
            line_no: SharedString::from(
                line.right_line_no
                    .map(|n| n.to_string())
                    .unwrap_or_default(),
            ),
            text: SharedString::from(&line.right_text),
            is_ghost: line.right_line_no.is_none(),
            status,
            diff_index,
            word_diff: SharedString::from(right_word_diff),
            is_current_diff: false,
            is_search_match: false,
            is_selected: false,
            highlight: -1,
        });
    }
    (left_lines, right_lines)
}

fn encode_word_diff(segments: &[crate::models::diff_line::WordDiffSegment]) -> String {
    if segments.is_empty() {
        return String::new();
    }
    segments
        .iter()
        .map(|s| {
            if s.changed {
                format!("\x01{}\x02", s.text)
            } else {
                s.text.clone()
            }
        })
        .collect()
}

/// Update `is_current_diff` flags and scroll to the diff at `diff_idx`.
fn navigate_to_diff(window: &crate::WasmApp, diff_positions: &[usize], diff_idx: i32) {
    // Update is_current_diff on both pane models
    for model_rc in [window.get_left_lines(), window.get_right_lines()] {
        if let Some(vec_model) = model_rc
            .as_any()
            .downcast_ref::<VecModel<crate::PaneLineData>>()
        {
            for i in 0..vec_model.row_count() {
                if let Some(mut row) = vec_model.row_data(i) {
                    let should = row.diff_index == diff_idx;
                    if row.is_current_diff != should {
                        row.is_current_diff = should;
                        vec_model.set_row_data(i, row);
                    }
                }
            }
        }
    }
    window.set_current_diff_index(diff_idx);

    // Scroll to the line: row height = (font_size + 2)px (matches DiffView ListView row height)
    if diff_idx >= 0 && (diff_idx as usize) < diff_positions.len() {
        let line_idx = diff_positions[diff_idx as usize];
        let row_height = (window.get_opt_font_size() as f32 + 2.0) as f32;
        window.set_diff_scroll_y(line_idx as f32 * row_height);
    }
}

/// Open a browser file picker and pass the selected file's text content to `callback`.
fn open_file_picker(callback: impl Fn(String, String) + 'static) {
    let document = match web_sys::window().and_then(|w| w.document()) {
        Some(d) => d,
        None => return,
    };

    let input: web_sys::HtmlInputElement = match document
        .create_element("input")
        .ok()
        .and_then(|e| e.dyn_into().ok())
    {
        Some(el) => el,
        None => return,
    };

    input.set_type("file");
    // Accept common text-based file types
    input.set_attribute("accept", ".txt,.rs,.js,.ts,.jsx,.tsx,.py,.go,.java,.c,.cpp,.h,.cs,.rb,.swift,.kt,.md,.json,.yaml,.yml,.toml,.xml,.html,.htm,.css,.scss,.sh,.sql,.csv").ok();

    let input_clone = input.clone();
    let closure = Closure::once(move |_: web_sys::Event| {
        let files = match input_clone.files() {
            Some(f) => f,
            None => return,
        };
        let file = match files.get(0) {
            Some(f) => f,
            None => return,
        };

        let name = file.name();
        // Blob::text() returns Promise<string> (available in all modern browsers)
        let text_promise = file.text();

        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(text_js) = wasm_bindgen_futures::JsFuture::from(text_promise).await {
                if let Some(text) = text_js.as_string() {
                    callback(text, name);
                }
            }
        });
    });

    input
        .add_event_listener_with_callback("change", closure.as_ref().unchecked_ref())
        .ok();
    closure.forget();
    input.click();
}

fn viewport_size() -> (u32, u32) {
    web_sys::window()
        .map(|w| {
            let width = w
                .inner_width()
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(800.0) as u32;
            let height = w
                .inner_height()
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(600.0) as u32;
            (width, height)
        })
        .unwrap_or((800, 600))
}

pub fn run() {
    let window = crate::WasmApp::new().unwrap();

    // Keep size in sync when the browser window is resized.
    {
        let window_weak = window.as_weak();
        let closure = Closure::<dyn FnMut()>::new(move || {
            if let Some(w) = window_weak.upgrade() {
                let (width, height) = viewport_size();
                w.window()
                    .set_size(slint::WindowSize::Logical(slint::LogicalSize::new(
                        width as f32,
                        height as f32,
                    )));
            }
        });
        web_sys::window()
            .unwrap()
            .add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref())
            .ok();
        closure.forget();
    }

    // Shared state: diff block start-line positions and current diff index
    let diff_positions: Rc<RefCell<Vec<usize>>> = Rc::new(RefCell::new(Vec::new()));
    let current_diff_idx: Rc<Cell<i32>> = Rc::new(Cell::new(-1));

    // --- Compare ---
    {
        let window_weak = window.as_weak();
        let diff_positions = Rc::clone(&diff_positions);
        let current_diff_idx = Rc::clone(&current_diff_idx);

        window.on_compare(move || {
            let window = window_weak.unwrap();
            let left = window.get_left_text().to_string();
            let right = window.get_right_text().to_string();

            let options = DiffOptions::default();
            let result = compute_diff_with_options(&left, &right, &options);

            let diff_count = result.diff_count;
            let (added, removed, modified) = compute_stats(&result);

            *diff_positions.borrow_mut() = result.diff_positions.clone();
            current_diff_idx.set(-1);

            let (left_lines, right_lines) = build_pane_line_data(&result);
            window.set_left_lines(ModelRc::new(VecModel::from(left_lines)));
            window.set_right_lines(ModelRc::new(VecModel::from(right_lines)));
            window.set_diff_count(diff_count as i32);
            window.set_current_diff_index(-1);
            window.set_diff_scroll_y(0.0);
            window.set_stat_added(added);
            window.set_stat_removed(removed);
            window.set_stat_modified(modified);
        });
    }

    // --- Next diff ---
    {
        let window_weak = window.as_weak();
        let diff_positions = Rc::clone(&diff_positions);
        let current_diff_idx = Rc::clone(&current_diff_idx);

        window.on_next_diff(move || {
            let window = window_weak.unwrap();
            let positions = diff_positions.borrow();
            if positions.is_empty() {
                return;
            }
            let new_idx = (current_diff_idx.get() + 1).min(positions.len() as i32 - 1);
            current_diff_idx.set(new_idx);
            navigate_to_diff(&window, &positions, new_idx);
        });
    }

    // --- Prev diff ---
    {
        let window_weak = window.as_weak();
        let diff_positions = Rc::clone(&diff_positions);
        let current_diff_idx = Rc::clone(&current_diff_idx);

        window.on_prev_diff(move || {
            let window = window_weak.unwrap();
            let positions = diff_positions.borrow();
            if positions.is_empty() {
                return;
            }
            let new_idx = (current_diff_idx.get() - 1).max(0);
            current_diff_idx.set(new_idx);
            navigate_to_diff(&window, &positions, new_idx);
        });
    }

    // --- Upload left ---
    {
        let window_weak = window.as_weak();

        window.on_upload_left(move || {
            let window_weak = window_weak.clone();
            open_file_picker(move |text, name| {
                if let Some(w) = window_weak.upgrade() {
                    w.set_left_text(SharedString::from(text));
                    w.set_left_title(SharedString::from(name));
                }
            });
        });
    }

    // --- Upload right ---
    {
        let window_weak = window.as_weak();

        window.on_upload_right(move || {
            let window_weak = window_weak.clone();
            open_file_picker(move |text, name| {
                if let Some(w) = window_weak.upgrade() {
                    w.set_right_text(SharedString::from(text));
                    w.set_right_title(SharedString::from(name));
                }
            });
        });
    }

    // --- Paste left ---
    {
        let window_weak = window.as_weak();

        window.on_paste_left(move || {
            let window_weak = window_weak.clone();
            paste_from_clipboard(move |text| {
                if let Some(w) = window_weak.upgrade() {
                    w.set_left_text(SharedString::from(text));
                }
            });
        });
    }

    // --- Paste right ---
    {
        let window_weak = window.as_weak();

        window.on_paste_right(move || {
            let window_weak = window_weak.clone();
            paste_from_clipboard(move |text| {
                if let Some(w) = window_weak.upgrade() {
                    w.set_right_text(SharedString::from(text));
                }
            });
        });
    }

    window.run().unwrap();
}
