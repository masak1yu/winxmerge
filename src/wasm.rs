use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::diff::engine::{DiffOptions, compute_diff_with_options};
use crate::models::diff_line::LineStatus;

fn build_diff_line_data(result: &crate::models::diff_line::DiffResult) -> Vec<crate::DiffLineData> {
    result
        .lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
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

            crate::DiffLineData {
                left_line_no: SharedString::from(
                    line.left_line_no.map(|n| n.to_string()).unwrap_or_default(),
                ),
                right_line_no: SharedString::from(
                    line.right_line_no
                        .map(|n| n.to_string())
                        .unwrap_or_default(),
                ),
                left_text: SharedString::from(&line.left_text),
                right_text: SharedString::from(&line.right_text),
                status,
                is_current_diff: false,
                diff_index,
                left_highlight: -1,
                right_highlight: -1,
                left_word_diff: SharedString::from(left_word_diff),
                right_word_diff: SharedString::from(right_word_diff),
                is_search_match: false,
                is_selected: false,
            }
        })
        .collect()
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

pub fn run() {
    let window = crate::WasmApp::new().unwrap();

    let window_weak = window.as_weak();
    window.on_compare(move || {
        let window = window_weak.unwrap();
        let left = window.get_left_text().to_string();
        let right = window.get_right_text().to_string();

        let options = DiffOptions::default();
        let result = compute_diff_with_options(&left, &right, &options);

        let diff_count = result.diff_count;
        let lines = build_diff_line_data(&result);

        window.set_diff_lines(ModelRc::new(VecModel::from(lines)));
        window.set_diff_count(diff_count as i32);
    });

    window.run().unwrap();
}
