use super::*;

/// Bytes per hex-view row.
pub(super) const ROW_BYTES: usize = 16;

/// Byte offsets where a run of differing bytes starts, comparing `left` and
/// `right` at the same offset (no realignment for inserted/deleted bytes).
/// Past the shorter file's EOF every remaining byte counts as different.
pub(super) fn diff_block_starts(left: &[u8], right: &[u8]) -> Vec<usize> {
    let len = left.len().max(right.len());
    let mut starts = Vec::new();
    let mut in_block = false;
    for i in 0..len {
        if left.get(i) != right.get(i) {
            if !in_block {
                starts.push(i);
                in_block = true;
            }
        } else {
            in_block = false;
        }
    }
    starts
}

/// Read-only Hex view model. Rows are built on demand instead of materialized
/// up front — a 1 MiB file would otherwise need ~26 MB of HexRowData.
pub(super) struct HexRows {
    left: Vec<u8>,
    right: Vec<u8>,
}

impl HexRows {
    pub(super) fn new(left: Vec<u8>, right: Vec<u8>) -> Self {
        Self { left, right }
    }
}

impl Model for HexRows {
    type Data = HexRowData;

    fn row_count(&self) -> usize {
        self.left.len().max(self.right.len()).div_ceil(ROW_BYTES)
    }

    fn row_data(&self, row: usize) -> Option<HexRowData> {
        if row >= self.row_count() {
            return None;
        }
        let start = row * ROW_BYTES;
        let (left_cells, right_cells) = build_row_cells(&self.left, &self.right, start);
        Some(HexRowData {
            offset: SharedString::from(format!("{:08X}", start)),
            left: ModelRc::new(VecModel::from(left_cells)),
            right: ModelRc::new(VecModel::from(right_cells)),
        })
    }

    fn model_tracker(&self) -> &dyn slint::ModelTracker {
        // Byte contents never mutate in place — a recompare swaps in a brand
        // new ModelRc (see run_hex_compare) rather than editing this one.
        &()
    }
}

fn build_row_cells(left: &[u8], right: &[u8], start: usize) -> (Vec<HexCell>, Vec<HexCell>) {
    let mut left_cells = Vec::with_capacity(ROW_BYTES);
    let mut right_cells = Vec::with_capacity(ROW_BYTES);
    for i in start..start + ROW_BYTES {
        let l = left.get(i).copied();
        let r = right.get(i).copied();
        let diff = l != r;
        left_cells.push(hex_cell(l, diff));
        right_cells.push(hex_cell(r, diff));
    }
    (left_cells, right_cells)
}

fn hex_cell(byte: Option<u8>, diff: bool) -> HexCell {
    match byte {
        Some(b) => HexCell {
            hex: SharedString::from(format!("{:02X}", b)),
            ch: SharedString::from(if (0x20..0x7f).contains(&b) {
                (b as char).to_string()
            } else {
                ".".to_string()
            }),
            diff,
        },
        // Past this side's EOF — blank cell, still marked as a diff.
        None => HexCell {
            hex: SharedString::new(),
            ch: SharedString::new(),
            diff,
        },
    }
}

/// Show `left`/`right` as a byte-for-byte Hex view instead of erroring out on
/// binary files (called from run_diff's binary-detection branch).
pub(super) fn run_hex_compare(
    window: &MainWindow,
    state: &mut AppState,
    left: Vec<u8>,
    right: Vec<u8>,
) {
    let left_len = left.len();
    let right_len = right.len();
    let identical = left == right;
    let starts = diff_block_starts(&left, &right);
    let diff_count = starts.len();
    let hex_rows: ModelRc<HexRowData> = ModelRc::new(HexRows::new(left, right));

    let tab = state.current_tab_mut();
    tab.view_mode = ViewMode::HexCompare;
    let left_name = tab
        .left_path
        .as_ref()
        .map(path_file_name)
        .unwrap_or_default();
    let right_name = tab
        .right_path
        .as_ref()
        .map(path_file_name)
        .unwrap_or_default();
    tab.title = format!("{} ↔ {}", left_name, right_name);
    tab.hex_rows = hex_rows.clone();
    tab.diff_positions = starts;
    tab.current_diff = -1;
    tab.compare_identical = Some(identical);
    tab.left_buffer = None;
    tab.right_buffer = None;
    // Nothing clears the undo/redo stacks on view_mode change and Ctrl+Z calls
    // undo() unconditionally (main.slint), so a stale stack would replay a
    // previous text diff on top of the Hex tab.
    tab.undo_stack.clear();
    tab.redo_stack.clear();
    tab.has_unsaved_changes = false;
    tab.editing_dirty = false;
    tab.left_encoding = String::new();
    tab.right_encoding = String::new();
    tab.left_eol_type = String::new();
    tab.right_eol_type = String::new();
    tab.diff_stats = String::new();

    window.set_view_mode(ViewMode::HexCompare.as_i32());
    window.set_left_lines(ModelRc::new(VecModel::from(Vec::<PaneLineData>::new())));
    window.set_right_lines(ModelRc::new(VecModel::from(Vec::<PaneLineData>::new())));
    window.set_diff_count(diff_count as i32);
    window.set_current_diff_index(-1);
    window.set_hex_rows(hex_rows);
    window.set_hex_current_row(-1);
    window.set_hex_scroll_y(0.0);
    window.set_has_unsaved_changes(false);
    window.set_can_undo(false);
    window.set_can_redo(false);
    sync_diff_stats(window, "");
    window.set_left_encoding_display(SharedString::from(""));
    window.set_right_encoding_display(SharedString::from(""));
    window.set_left_eol_type(SharedString::from(""));
    window.set_right_eol_type(SharedString::from(""));

    let status = if identical {
        format!("[Hex] {} bytes / {} bytes — identical", left_len, right_len)
    } else {
        format!(
            "[Hex] {} bytes / {} bytes — {} differing block(s)",
            left_len, right_len, diff_count
        )
    };
    window.set_status_text(SharedString::from(status));

    sync_tab_list(window, state);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_block_starts_identical_files_have_no_blocks() {
        assert_eq!(diff_block_starts(b"abcdef", b"abcdef"), Vec::<usize>::new());
    }

    #[test]
    fn diff_block_starts_finds_one_run_in_the_middle() {
        assert_eq!(diff_block_starts(b"abcdef", b"abXYef"), vec![2]);
    }

    #[test]
    fn diff_block_starts_treats_eof_tail_as_one_run() {
        // "aXcd" vs "abc": byte 1 differs (X vs b), byte 2 matches (c vs c),
        // then the left file's extra byte 3 ('d') has no right counterpart.
        assert_eq!(diff_block_starts(b"aXcd", b"abc"), vec![1, 3]);
    }

    #[test]
    fn row_data_marks_bytes_past_eof_as_diff_with_blank_cell() {
        let left = vec![0u8; 17];
        let right = vec![0u8; 16];
        let rows = HexRows::new(left, right);
        assert_eq!(rows.row_count(), 2);

        let row1 = rows.row_data(1).expect("row 1 exists");
        let left_cell = row1.left.row_data(0).expect("left cell 0 exists");
        let right_cell = row1.right.row_data(0).expect("right cell 0 exists");
        assert!(left_cell.diff);
        assert!(right_cell.diff);
        assert!(!left_cell.hex.is_empty());
        assert!(right_cell.hex.is_empty());
    }
}
