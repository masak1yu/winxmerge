use super::*;

const DEFAULT_COL_WIDTH_PX: i32 = 100;
const MIN_COL_WIDTH_PX: i32 = 30;

// --- Table detail pane ---

/// Update the table detail pane with cell-by-cell comparison for the given row.
pub(super) fn update_table_detail_pane(window: &MainWindow, state: &AppState, row_idx: usize) {
    let tab = state.current_tab();
    if row_idx >= tab.table_rows.len() {
        window.set_table_detail_cells(ModelRc::new(VecModel::from(
            Vec::<TableDetailCellData>::new(),
        )));
        return;
    }

    let row = &tab.table_rows[row_idx];
    let left_model = &row.left_cells;
    let right_model = &row.right_cells;
    let base_model = &row.base_cells;
    let cell_count = left_model
        .row_count()
        .max(right_model.row_count())
        .max(base_model.row_count());
    let columns = &tab.table_columns;

    let mut detail_cells = Vec::with_capacity(cell_count);
    for i in 0..cell_count {
        let col_name = columns
            .get(i)
            .map(|c| c.name.to_string())
            .unwrap_or_else(|| crate::csv::col_to_name(i));
        let left_cell = left_model.row_data(i);
        let right_cell = right_model.row_data(i);
        let base_cell = base_model.row_data(i);
        let left_value = left_cell
            .as_ref()
            .map(|c| c.text.to_string())
            .unwrap_or_default();
        let right_value = right_cell
            .as_ref()
            .map(|c| c.text.to_string())
            .unwrap_or_default();
        let base_value = base_cell
            .as_ref()
            .map(|c| c.text.to_string())
            .unwrap_or_default();
        let left_status = left_cell.as_ref().map(|c| c.status).unwrap_or(0);
        let right_status = right_cell.as_ref().map(|c| c.status).unwrap_or(0);
        let base_status = base_cell.as_ref().map(|c| c.status).unwrap_or(0);
        // Combined status for column header indicator
        let status = if left_status != 0 {
            left_status
        } else if right_status != 0 {
            right_status
        } else {
            base_status
        };
        let col_x_px = columns.get(i).map(|c| c.x_px).unwrap_or(0);
        let col_width_px = columns
            .get(i)
            .map(|c| c.width_px)
            .unwrap_or(DEFAULT_COL_WIDTH_PX);

        detail_cells.push(TableDetailCellData {
            col_name: SharedString::from(col_name),
            left_value: SharedString::from(left_value),
            base_value: SharedString::from(base_value),
            right_value: SharedString::from(right_value),
            status,
            left_status,
            right_status,
            base_status,
            col_x_px,
            col_width_px,
        });
    }

    window.set_table_detail_cells(ModelRc::new(VecModel::from(detail_cells)));
}

// --- Table cell editing ---

/// Extract cell texts from table_rows into a Vec<Vec<String>> grid.
fn extract_table_grid(rows: &[TableRowData], pane: i32) -> Vec<Vec<String>> {
    rows.iter()
        .map(|row| {
            let cells = match pane {
                0 => &row.left_cells,
                1 => &row.base_cells,
                _ => &row.right_cells,
            };
            let count = cells.row_count();
            (0..count)
                .map(|i| {
                    cells
                        .row_data(i)
                        .map(|c| c.text.to_string())
                        .unwrap_or_default()
                })
                .collect()
        })
        .collect()
}

/// Push a snapshot of the current table state onto the undo stack.
pub(super) fn push_table_undo_snapshot(state: &mut AppState) {
    let tab = state.current_tab();
    let snapshot = TableSnapshot {
        left_grid: extract_table_grid(&tab.table_rows, 0),
        right_grid: extract_table_grid(&tab.table_rows, 2),
        base_grid: extract_table_grid(&tab.table_rows, 1),
    };
    let tab = state.current_tab_mut();
    tab.table_undo_stack.push(snapshot);
    tab.table_redo_stack.clear();
}

/// Edit a single cell in the table grid view.
pub fn table_cell_edit(
    window: &MainWindow,
    state: &mut AppState,
    row_idx: i32,
    col_idx: i32,
    pane: i32,
    new_text: &str,
) {
    let tab = state.current_tab();
    let row_idx = row_idx as usize;
    let col_idx = col_idx as usize;
    if row_idx >= tab.table_rows.len() {
        return;
    }

    push_table_undo_snapshot(state);

    let tab = state.current_tab_mut();
    let row = &tab.table_rows[row_idx];
    let cells = match pane {
        0 => &row.left_cells,
        1 => &row.base_cells,
        _ => &row.right_cells,
    };
    if col_idx >= cells.row_count() {
        return;
    }
    if let Some(mut cell) = cells.row_data(col_idx) {
        cell.text = SharedString::from(new_text);
        cells.set_row_data(col_idx, cell);
    }

    let tab = state.current_tab_mut();
    tab.has_unsaved_changes = true;
    tab.editing_dirty = true;
    window.set_has_unsaved_changes(true);
    window.set_can_undo(true);
    window.set_status_text(SharedString::from("Editing \u{2014} press F5 to compare"));
}

// --- Table copy functions (row-level copy for view-mode 4/6) ---

/// Resize a table column: update column width, recalculate all cell positions, push to Slint.
pub fn resize_table_column(
    window: &MainWindow,
    state: &mut AppState,
    col_idx: i32,
    new_width: i32,
) {
    let tab = state.current_tab_mut();
    let col_idx = col_idx as usize;
    if col_idx >= tab.table_columns.len() {
        return;
    }

    let new_width = new_width.max(MIN_COL_WIDTH_PX);
    let old_width = tab.table_columns[col_idx].width_px;
    let width_delta = new_width - old_width;
    if width_delta == 0 {
        return;
    }

    // Update column model
    tab.table_columns[col_idx].width_px = new_width;
    for i in (col_idx + 1)..tab.table_columns.len() {
        tab.table_columns[i].x_px += width_delta;
    }
    tab.table_content_width_px += width_delta;

    // Update all cells' positions
    for row in &mut tab.table_rows {
        update_cell_col_positions_model(&row.left_cells, col_idx, new_width, width_delta);
        update_cell_col_positions_model(&row.right_cells, col_idx, new_width, width_delta);
        update_cell_col_positions_model(&row.base_cells, col_idx, new_width, width_delta);
    }

    // Push to Slint
    window.set_table_columns(ModelRc::new(VecModel::from(tab.table_columns.clone())));
    window.set_table_rows(ModelRc::new(VecModel::from(tab.table_rows.clone())));
    window.set_table_content_width_px(tab.table_content_width_px);

    // Refresh detail pane if a row is highlighted
    let highlight_row = window.get_table_current_highlight_row();
    if highlight_row >= 0 {
        update_table_detail_pane(window, state, highlight_row as usize);
    }
}

fn update_cell_col_positions_model(
    cells: &ModelRc<TableCellData>,
    col_idx: usize,
    new_width: i32,
    width_delta: i32,
) {
    let count = cells.row_count();
    if col_idx < count {
        if let Some(mut cell) = cells.row_data(col_idx) {
            cell.col_width_px = new_width;
            cells.set_row_data(col_idx, cell);
        }
    }
    for i in (col_idx + 1)..count {
        if let Some(mut cell) = cells.row_data(i) {
            cell.col_x_px += width_delta;
            cells.set_row_data(i, cell);
        }
    }
}

/// Helper: rebuild diff_positions from table_rows and update window state.
pub(super) fn rebuild_table_diff_positions(window: &MainWindow, state: &mut AppState) {
    let tab = state.current_tab_mut();
    let diff_positions: Vec<usize> = tab
        .table_rows
        .iter()
        .enumerate()
        .filter(|(_, r)| r.row_status != 0)
        .map(|(i, _)| i)
        .collect();
    let diff_pos_count = diff_positions.len();
    tab.diff_positions = diff_positions;
    if tab.current_diff >= diff_pos_count as i32 {
        tab.current_diff = if diff_pos_count > 0 {
            diff_pos_count as i32 - 1
        } else {
            -1
        };
    }
    window.set_diff_count(diff_pos_count as i32);
    window.set_current_diff_index(tab.current_diff);
}

/// Helper: apply table_rows to the window VecModel.
pub(super) fn apply_table_rows_to_window(window: &MainWindow, state: &AppState) {
    let tab = state.current_tab();
    window.set_table_rows(ModelRc::new(VecModel::from(tab.table_rows.clone())));
}

/// Copy left cells to right for a single diff row (table view).
pub fn table_copy_to_right(window: &MainWindow, state: &mut AppState, diff_index: i32) {
    {
        let tab = state.current_tab();
        if diff_index < 0 || diff_index as usize >= tab.diff_positions.len() {
            return;
        }
    }
    let row_idx = state.current_tab().diff_positions[diff_index as usize];
    let tab = state.current_tab_mut();
    let row = &tab.table_rows[row_idx];

    // Read left cells
    let left_model = &row.left_cells;
    let cell_count = left_model.row_count();
    let mut new_right_cells = Vec::with_capacity(cell_count);
    let mut new_left_cells = Vec::with_capacity(cell_count);
    for i in 0..cell_count {
        if let Some(lc) = left_model.row_data(i) {
            new_right_cells.push(TableCellData {
                text: lc.text.clone(),
                status: 0,
                col_x_px: lc.col_x_px,
                col_width_px: lc.col_width_px,
            });
            new_left_cells.push(TableCellData {
                text: lc.text.clone(),
                status: 0,
                col_x_px: lc.col_x_px,
                col_width_px: lc.col_width_px,
            });
        }
    }

    let base_cells_clone = tab.table_rows[row_idx].base_cells.clone();
    tab.table_rows[row_idx] = TableRowData {
        row_number: tab.table_rows[row_idx].row_number,
        left_cells: ModelRc::new(VecModel::from(new_left_cells)),
        right_cells: ModelRc::new(VecModel::from(new_right_cells)),
        base_cells: base_cells_clone,
        row_status: 0,
    };

    apply_table_rows_to_window(window, state);
    rebuild_table_diff_positions(window, state);

    let tab = state.current_tab();
    if !tab.diff_positions.is_empty() {
        let new_idx = (diff_index as usize).min(tab.diff_positions.len() - 1);
        update_current_diff(window, state, new_idx as i32);
    }
}

/// Copy right cells to left for a single diff row (table view).
pub fn table_copy_to_left(window: &MainWindow, state: &mut AppState, diff_index: i32) {
    {
        let tab = state.current_tab();
        if diff_index < 0 || diff_index as usize >= tab.diff_positions.len() {
            return;
        }
    }
    let row_idx = state.current_tab().diff_positions[diff_index as usize];
    let tab = state.current_tab_mut();
    let row = &tab.table_rows[row_idx];

    let right_model = &row.right_cells;
    let cell_count = right_model.row_count();
    let mut new_left_cells = Vec::with_capacity(cell_count);
    let mut new_right_cells = Vec::with_capacity(cell_count);
    for i in 0..cell_count {
        if let Some(rc) = right_model.row_data(i) {
            new_left_cells.push(TableCellData {
                text: rc.text.clone(),
                status: 0,
                col_x_px: rc.col_x_px,
                col_width_px: rc.col_width_px,
            });
            new_right_cells.push(TableCellData {
                text: rc.text.clone(),
                status: 0,
                col_x_px: rc.col_x_px,
                col_width_px: rc.col_width_px,
            });
        }
    }

    let base_cells_clone = tab.table_rows[row_idx].base_cells.clone();
    tab.table_rows[row_idx] = TableRowData {
        row_number: tab.table_rows[row_idx].row_number,
        left_cells: ModelRc::new(VecModel::from(new_left_cells)),
        right_cells: ModelRc::new(VecModel::from(new_right_cells)),
        base_cells: base_cells_clone,
        row_status: 0,
    };

    apply_table_rows_to_window(window, state);
    rebuild_table_diff_positions(window, state);

    let tab = state.current_tab();
    if !tab.diff_positions.is_empty() {
        let new_idx = (diff_index as usize).min(tab.diff_positions.len() - 1);
        update_current_diff(window, state, new_idx as i32);
    }
}

pub fn table_copy_right_and_next(window: &MainWindow, state: &mut AppState) {
    let diff_index = state.current_tab().current_diff;
    table_copy_to_right(window, state, diff_index);
    navigate_diff(window, state, true);
}

pub fn table_copy_left_and_next(window: &MainWindow, state: &mut AppState) {
    let diff_index = state.current_tab().current_diff;
    table_copy_to_left(window, state, diff_index);
    navigate_diff(window, state, true);
}

/// 3-way table: copy left cells to base cells for a single diff row (use-left).
pub fn table_use_left(window: &MainWindow, state: &mut AppState, diff_index: i32) {
    table_resolve_to_base(window, state, diff_index, true);
}

/// 3-way table: copy right cells to base cells for a single diff row (use-right).
pub fn table_use_right(window: &MainWindow, state: &mut AppState, diff_index: i32) {
    table_resolve_to_base(window, state, diff_index, false);
}

pub fn table_use_left_and_next(window: &MainWindow, state: &mut AppState) {
    let diff_index = state.current_tab().current_diff;
    table_use_left(window, state, diff_index);
    navigate_diff(window, state, true);
}

pub fn table_use_right_and_next(window: &MainWindow, state: &mut AppState) {
    let diff_index = state.current_tab().current_diff;
    table_use_right(window, state, diff_index);
    navigate_diff(window, state, true);
}

pub fn table_use_all_left(window: &MainWindow, state: &mut AppState) {
    let positions: Vec<usize> = state.current_tab().diff_positions.clone();
    for (i, _) in positions.iter().enumerate() {
        table_resolve_to_base_inner(state, i, true);
    }
    apply_table_rows_to_window(window, state);
    rebuild_table_diff_positions(window, state);
}

pub fn table_use_all_right(window: &MainWindow, state: &mut AppState) {
    let positions: Vec<usize> = state.current_tab().diff_positions.clone();
    for (i, _) in positions.iter().enumerate() {
        table_resolve_to_base_inner(state, i, false);
    }
    apply_table_rows_to_window(window, state);
    rebuild_table_diff_positions(window, state);
}

/// Internal: copy source (left or right) cells to base cells for a single diff row.
fn table_resolve_to_base(
    window: &MainWindow,
    state: &mut AppState,
    diff_index: i32,
    use_left: bool,
) {
    {
        let tab = state.current_tab();
        if diff_index < 0 || diff_index as usize >= tab.diff_positions.len() {
            return;
        }
    }
    table_resolve_to_base_inner(state, diff_index as usize, use_left);

    apply_table_rows_to_window(window, state);
    rebuild_table_diff_positions(window, state);

    let tab = state.current_tab();
    if !tab.diff_positions.is_empty() {
        let new_idx = (diff_index as usize).min(tab.diff_positions.len() - 1);
        update_current_diff(window, state, new_idx as i32);
    }
    window.set_status_text(SharedString::from(if use_left {
        "Left → Base copied"
    } else {
        "Right → Base copied"
    }));
}

/// Copy source pane's cells to base for a single diff row.
/// When `use_left` is true, left → base; otherwise right → base.
/// The opposite pane's cells keep their values but get status re-evaluated.
fn table_resolve_to_base_inner(state: &mut AppState, diff_index: usize, use_left: bool) {
    let tab = state.current_tab();
    if diff_index >= tab.diff_positions.len() {
        return;
    }
    let row_idx = tab.diff_positions[diff_index];
    let row = &tab.table_rows[row_idx];
    let (src_model, other_model) = if use_left {
        (&row.left_cells, &row.right_cells)
    } else {
        (&row.right_cells, &row.left_cells)
    };
    let cell_count = src_model.row_count();
    let mut new_base = Vec::with_capacity(cell_count);
    let mut new_src = Vec::with_capacity(cell_count);
    let mut new_other = Vec::with_capacity(cell_count);
    for i in 0..cell_count {
        if let Some(sc) = src_model.row_data(i) {
            new_base.push(TableCellData {
                text: sc.text.clone(),
                status: 0,
                col_x_px: sc.col_x_px,
                col_width_px: sc.col_width_px,
            });
            new_src.push(TableCellData {
                text: sc.text.clone(),
                status: 0,
                col_x_px: sc.col_x_px,
                col_width_px: sc.col_width_px,
            });
            let oc = other_model.row_data(i);
            let ov = oc.as_ref().map(|c| c.text.clone()).unwrap_or_default();
            let st = if ov == sc.text { 0 } else { 1 };
            new_other.push(TableCellData {
                text: ov,
                status: st,
                col_x_px: sc.col_x_px,
                col_width_px: sc.col_width_px,
            });
        }
    }
    let has_diff = new_other.iter().any(|c| c.status != 0);
    let (new_left, new_right) = if use_left {
        (new_src, new_other)
    } else {
        (new_other, new_src)
    };
    let tab = state.current_tab_mut();
    tab.table_rows[row_idx] = TableRowData {
        row_number: tab.table_rows[row_idx].row_number,
        left_cells: ModelRc::new(VecModel::from(new_left)),
        right_cells: ModelRc::new(VecModel::from(new_right)),
        base_cells: ModelRc::new(VecModel::from(new_base)),
        row_status: if has_diff { 1 } else { 0 },
    };
}

/// Copy all left cells to right for all diff rows (table view).
pub fn table_copy_all_right(window: &MainWindow, state: &mut AppState) {
    let tab = state.current_tab();
    if tab.diff_positions.is_empty() {
        return;
    }
    let positions: Vec<usize> = tab.diff_positions.clone();

    let tab = state.current_tab_mut();
    for &row_idx in &positions {
        let row = &tab.table_rows[row_idx];
        let left_model = &row.left_cells;
        let cell_count = left_model.row_count();
        let mut new_right_cells = Vec::with_capacity(cell_count);
        let mut new_left_cells = Vec::with_capacity(cell_count);
        for i in 0..cell_count {
            if let Some(lc) = left_model.row_data(i) {
                new_right_cells.push(TableCellData {
                    text: lc.text.clone(),
                    status: 0,
                    col_x_px: lc.col_x_px,
                    col_width_px: lc.col_width_px,
                });
                new_left_cells.push(TableCellData {
                    text: lc.text.clone(),
                    status: 0,
                    col_x_px: lc.col_x_px,
                    col_width_px: lc.col_width_px,
                });
            }
        }
        let base_cells_clone = tab.table_rows[row_idx].base_cells.clone();
        tab.table_rows[row_idx] = TableRowData {
            row_number: tab.table_rows[row_idx].row_number,
            left_cells: ModelRc::new(VecModel::from(new_left_cells)),
            right_cells: ModelRc::new(VecModel::from(new_right_cells)),
            base_cells: base_cells_clone,
            row_status: 0,
        };
    }

    apply_table_rows_to_window(window, state);
    rebuild_table_diff_positions(window, state);
}

/// Copy all right cells to left for all diff rows (table view).
pub fn table_copy_all_left(window: &MainWindow, state: &mut AppState) {
    let tab = state.current_tab();
    if tab.diff_positions.is_empty() {
        return;
    }
    let positions: Vec<usize> = tab.diff_positions.clone();

    let tab = state.current_tab_mut();
    for &row_idx in &positions {
        let row = &tab.table_rows[row_idx];
        let right_model = &row.right_cells;
        let cell_count = right_model.row_count();
        let mut new_left_cells = Vec::with_capacity(cell_count);
        let mut new_right_cells = Vec::with_capacity(cell_count);
        for i in 0..cell_count {
            if let Some(rc) = right_model.row_data(i) {
                new_left_cells.push(TableCellData {
                    text: rc.text.clone(),
                    status: 0,
                    col_x_px: rc.col_x_px,
                    col_width_px: rc.col_width_px,
                });
                new_right_cells.push(TableCellData {
                    text: rc.text.clone(),
                    status: 0,
                    col_x_px: rc.col_x_px,
                    col_width_px: rc.col_width_px,
                });
            }
        }
        let base_cells_clone = tab.table_rows[row_idx].base_cells.clone();
        tab.table_rows[row_idx] = TableRowData {
            row_number: tab.table_rows[row_idx].row_number,
            left_cells: ModelRc::new(VecModel::from(new_left_cells)),
            right_cells: ModelRc::new(VecModel::from(new_right_cells)),
            base_cells: base_cells_clone,
            row_status: 0,
        };
    }

    apply_table_rows_to_window(window, state);
    rebuild_table_diff_positions(window, state);
}

/// Restore table cell texts from a grid snapshot.
fn restore_table_grid(rows: &[TableRowData], grid: &[Vec<String>], pane: i32) {
    for (r, row) in rows.iter().enumerate() {
        let cells = match pane {
            0 => &row.left_cells,
            1 => &row.base_cells,
            _ => &row.right_cells,
        };
        if let Some(grid_row) = grid.get(r) {
            for (c, text) in grid_row.iter().enumerate() {
                if let Some(mut cell) = cells.row_data(c) {
                    cell.text = SharedString::from(text.as_str());
                    cells.set_row_data(c, cell);
                }
            }
        }
    }
}

pub(super) fn table_undo(window: &MainWindow, state: &mut AppState) {
    let tab = state.current_tab_mut();
    if tab.table_undo_stack.is_empty() {
        return;
    }

    // Save current to redo
    let current = TableSnapshot {
        left_grid: extract_table_grid(&tab.table_rows, 0),
        right_grid: extract_table_grid(&tab.table_rows, 2),
        base_grid: extract_table_grid(&tab.table_rows, 1),
    };
    tab.table_redo_stack.push(current);

    let Some(snapshot) = tab.table_undo_stack.pop() else {
        return;
    };
    restore_table_grid(&tab.table_rows, &snapshot.left_grid, 0);
    restore_table_grid(&tab.table_rows, &snapshot.right_grid, 2);
    restore_table_grid(&tab.table_rows, &snapshot.base_grid, 1);

    apply_table_rows_to_window(window, state);

    let tab = state.current_tab();
    window.set_can_undo(!tab.table_undo_stack.is_empty());
    window.set_can_redo(!tab.table_redo_stack.is_empty());
    window.set_status_text(SharedString::from("Undo"));
}

pub(super) fn table_redo(window: &MainWindow, state: &mut AppState) {
    let tab = state.current_tab_mut();
    if tab.table_redo_stack.is_empty() {
        return;
    }

    // Save current to undo
    let current = TableSnapshot {
        left_grid: extract_table_grid(&tab.table_rows, 0),
        right_grid: extract_table_grid(&tab.table_rows, 2),
        base_grid: extract_table_grid(&tab.table_rows, 1),
    };
    tab.table_undo_stack.push(current);

    let Some(snapshot) = tab.table_redo_stack.pop() else {
        return;
    };
    restore_table_grid(&tab.table_rows, &snapshot.left_grid, 0);
    restore_table_grid(&tab.table_rows, &snapshot.right_grid, 2);
    restore_table_grid(&tab.table_rows, &snapshot.base_grid, 1);

    apply_table_rows_to_window(window, state);

    let tab = state.current_tab();
    window.set_can_undo(!tab.table_undo_stack.is_empty());
    window.set_can_redo(!tab.table_redo_stack.is_empty());
    window.set_status_text(SharedString::from("Redo"));
}

pub fn new_blank_table_3way(
    window: &MainWindow,
    state: &mut AppState,
    file_type: i32,
    delimiter: &str,
    newline_in_quotes: bool,
    quote_char: &str,
) {
    // 3-way table: base + left + right — reuse new_blank_table logic but with view_mode awareness
    // For now, open as a 2-pane table view since 3-way table merge is the same as 2-pane in WinMerge
    new_blank_table(
        window,
        state,
        file_type,
        delimiter,
        newline_in_quotes,
        quote_char,
    );
}

pub fn new_blank_table(
    window: &MainWindow,
    state: &mut AppState,
    _file_type: i32,
    delimiter: &str,
    _newline_in_quotes: bool,
    _quote_char: &str,
) {
    // "TAB" sentinel from Slint (can't pass \t directly)
    let delim_str = if delimiter == "TAB" { "\t" } else { delimiter };
    let delim_byte = delim_str.as_bytes().first().copied().unwrap_or(b',');
    let current_is_blank = {
        let tab = state.current_tab();
        tab.view_mode == ViewMode::Blank && tab.left_path.is_none() && tab.right_path.is_none()
    };
    if !current_is_blank {
        add_tab(window, state);
    }

    // Create an initial 10×5 empty grid so the user can start editing immediately
    let initial_rows = 10;
    let initial_cols = 5;
    let (columns, content_width_px) = build_columns(initial_cols, DEFAULT_COL_WIDTH_PX);
    let empty_grid: Vec<Vec<String>> = (0..initial_rows)
        .map(|_| vec![String::new(); initial_cols])
        .collect();
    let cell_status: Vec<Vec<i32>> = (0..initial_rows)
        .map(|_| vec![0i32; initial_cols])
        .collect();
    let table_rows = build_table_rows(
        &empty_grid,
        &empty_grid,
        &cell_status,
        initial_rows,
        initial_cols,
        &columns,
    );

    {
        let tab = state.current_tab_mut();
        tab.view_mode = ViewMode::CsvCompare;
        tab.left_path = None;
        tab.right_path = None;
        tab.title = "Untitled".to_string();
        tab.excel_cells = Vec::new();
        tab.table_rows = table_rows.clone();
        tab.table_columns = columns.clone();
        tab.table_content_width_px = content_width_px;
        tab.csv_delimiter = delim_byte;
        tab.excel_sheet_data.clear();
        tab.diff_positions.clear();
        tab.diff_stats = String::new();
        tab.has_unsaved_changes = false;
        tab.left_encoding = "UTF-8".to_string();
        tab.right_encoding = "UTF-8".to_string();
    }

    window.set_view_mode(ViewMode::CsvCompare.as_i32());
    window.set_table_rows(ModelRc::new(VecModel::from(table_rows)));
    window.set_table_columns(ModelRc::new(VecModel::from(columns)));
    window.set_table_content_width_px(content_width_px);
    window.set_diff_count(0);
    window.set_left_path(SharedString::from(""));
    window.set_right_path(SharedString::from(""));
    window.set_has_unsaved_changes(false);
    window.set_status_text(SharedString::from("New blank table document"));
    sync_tab_list(window, state);
}

/// Build TableRowData from full grids and cell_status matrix for the side-by-side table view.
pub(super) fn build_table_rows(
    left_grid: &[Vec<String>],
    right_grid: &[Vec<String>],
    cell_status: &[Vec<i32>],
    max_rows: usize,
    max_cols: usize,
    columns: &[TableColumnInfo],
) -> Vec<TableRowData> {
    let mut rows = Vec::with_capacity(max_rows);

    for r in 0..max_rows {
        let left_row = left_grid.get(r);
        let right_row = right_grid.get(r);
        let status_row = cell_status.get(r);

        let mut left_cells = Vec::with_capacity(max_cols);
        let mut right_cells = Vec::with_capacity(max_cols);
        let mut has_diff = false;
        let mut row_left_only = true;
        let mut row_right_only = true;

        for c in 0..max_cols {
            let lv = left_row
                .and_then(|row| row.get(c))
                .map(|s| s.as_str())
                .unwrap_or("");
            let rv = right_row
                .and_then(|row| row.get(c))
                .map(|s| s.as_str())
                .unwrap_or("");
            let status = status_row.and_then(|sr| sr.get(c)).copied().unwrap_or(0);

            if status != 0 {
                has_diff = true;
            }
            if status != 2 {
                row_left_only = false;
            }
            if status != 3 {
                row_right_only = false;
            }

            let col_x_px = columns.get(c).map(|col| col.x_px).unwrap_or(0);
            let col_width_px = columns
                .get(c)
                .map(|col| col.width_px)
                .unwrap_or(DEFAULT_COL_WIDTH_PX);

            left_cells.push(TableCellData {
                text: SharedString::from(lv),
                status: if status == 3 { 3 } else { status },
                col_x_px,
                col_width_px,
            });
            right_cells.push(TableCellData {
                text: SharedString::from(rv),
                status: if status == 2 { 2 } else { status },
                col_x_px,
                col_width_px,
            });
        }

        let row_status = if !has_diff {
            0
        } else if row_left_only {
            2
        } else if row_right_only {
            3
        } else {
            1
        };

        rows.push(TableRowData {
            row_number: (r + 1) as i32,
            left_cells: ModelRc::new(VecModel::from(left_cells)),
            right_cells: ModelRc::new(VecModel::from(right_cells)),
            base_cells: ModelRc::new(VecModel::from(Vec::<TableCellData>::new())),
            row_status,
        });
    }

    rows
}

/// Build column info (name, width, x-offset) for the given column count.
pub(super) fn build_columns(max_cols: usize, col_width: i32) -> (Vec<TableColumnInfo>, i32) {
    let mut columns = Vec::with_capacity(max_cols);
    let mut x = 0i32;
    for c in 0..max_cols {
        columns.push(TableColumnInfo {
            name: SharedString::from(crate::csv::col_to_name(c)),
            width_px: col_width,
            x_px: x,
        });
        x += col_width;
    }
    (columns, x) // x is total content width
}

/// Build table rows for 3-way comparison.
/// Maps csv.rs 3-way cell_status (0-7) to per-pane TableCellData statuses.
pub(super) fn build_table_rows_3way(
    base_grid: &[Vec<String>],
    left_grid: &[Vec<String>],
    right_grid: &[Vec<String>],
    cell_status: &[Vec<i32>],
    max_rows: usize,
    max_cols: usize,
    columns: &[TableColumnInfo],
) -> Vec<TableRowData> {
    let mut rows = Vec::with_capacity(max_rows);

    for r in 0..max_rows {
        let base_row = base_grid.get(r);
        let left_row = left_grid.get(r);
        let right_row = right_grid.get(r);
        let status_row = cell_status.get(r);

        let mut base_cells = Vec::with_capacity(max_cols);
        let mut left_cells = Vec::with_capacity(max_cols);
        let mut right_cells = Vec::with_capacity(max_cols);
        let mut has_diff = false;
        let mut has_conflict = false;
        let mut all_base_only = true;
        let mut all_left_only = true;
        let mut all_right_only = true;

        for c in 0..max_cols {
            let bv = base_row
                .and_then(|row| row.get(c))
                .map(|s| s.as_str())
                .unwrap_or("");
            let lv = left_row
                .and_then(|row| row.get(c))
                .map(|s| s.as_str())
                .unwrap_or("");
            let rv = right_row
                .and_then(|row| row.get(c))
                .map(|s| s.as_str())
                .unwrap_or("");
            let three_way_status = status_row.and_then(|sr| sr.get(c)).copied().unwrap_or(0);

            // Map 3-way cell status to per-pane display statuses
            // base_st: 0=normal, 2=removed(base-only)
            // left_st: 0=normal, 1=modified, 3=added(left-only), 5=conflict
            // right_st: 0=normal, 1=modified, 3=added(right-only), 5=conflict
            let (base_st, left_st, right_st) = match three_way_status {
                0 => (0, 0, 0), // identical
                1 => (0, 1, 0), // left-changed
                2 => (0, 0, 1), // right-changed
                3 => (0, 1, 1), // both-changed-same
                4 => (0, 5, 5), // conflict
                5 => (2, 4, 4), // base-only (deleted in both)
                6 => (4, 3, 4), // left-only
                7 => (4, 4, 3), // right-only
                _ => (0, 0, 0),
            };

            if three_way_status != 0 {
                has_diff = true;
            }
            if three_way_status == 4 {
                has_conflict = true;
            }
            if three_way_status != 5 {
                all_base_only = false;
            }
            if three_way_status != 6 {
                all_left_only = false;
            }
            if three_way_status != 7 {
                all_right_only = false;
            }

            let col_x_px = columns.get(c).map(|col| col.x_px).unwrap_or(0);
            let col_width_px = columns
                .get(c)
                .map(|col| col.width_px)
                .unwrap_or(DEFAULT_COL_WIDTH_PX);

            base_cells.push(TableCellData {
                text: SharedString::from(bv),
                status: base_st,
                col_x_px,
                col_width_px,
            });
            left_cells.push(TableCellData {
                text: SharedString::from(lv),
                status: left_st,
                col_x_px,
                col_width_px,
            });
            right_cells.push(TableCellData {
                text: SharedString::from(rv),
                status: right_st,
                col_x_px,
                col_width_px,
            });
        }

        // Row-level status for location pane
        let row_status = if !has_diff {
            0
        } else if has_conflict {
            4 // conflict
        } else if all_base_only {
            5 // base-only
        } else if all_left_only {
            6 // left-only
        } else if all_right_only {
            7 // right-only
        } else {
            1 // has changes
        };

        rows.push(TableRowData {
            row_number: (r + 1) as i32,
            left_cells: ModelRc::new(VecModel::from(left_cells)),
            right_cells: ModelRc::new(VecModel::from(right_cells)),
            base_cells: ModelRc::new(VecModel::from(base_cells)),
            row_status,
        });
    }

    rows
}

/// Recompute 2-way table diff from CSV text (used by rescan when editing is in progress).
pub(super) fn recompute_table_from_csv(
    window: &MainWindow,
    state: &mut AppState,
    left_csv: &str,
    right_csv: &str,
) {
    let result = compare_csv_full(left_csv, right_csv);
    let tab = state.current_tab();
    let old_cols = tab.table_columns.len();
    let need_rebuild_cols = result.max_cols != old_cols;

    let columns = if need_rebuild_cols {
        let (cols, width) = build_columns(result.max_cols, DEFAULT_COL_WIDTH_PX);
        let tab = state.current_tab_mut();
        tab.table_columns = cols.clone();
        tab.table_content_width_px = width;
        window.set_table_columns(ModelRc::new(VecModel::from(cols.clone())));
        window.set_table_content_width_px(width);
        cols
    } else {
        tab.table_columns.clone()
    };

    let table_rows = build_table_rows(
        &result.left_grid,
        &result.right_grid,
        &result.cell_status,
        result.max_rows,
        result.max_cols,
        &columns,
    );

    let diff_positions: Vec<usize> = table_rows
        .iter()
        .enumerate()
        .filter(|(_, r)| r.row_status != 0)
        .map(|(i, _)| i)
        .collect();
    let diff_count = diff_positions.len();

    let tab = state.current_tab_mut();
    window.set_table_rows(ModelRc::new(VecModel::from(table_rows.clone())));
    tab.table_rows = table_rows;
    tab.diff_positions = diff_positions;
    tab.current_diff = if diff_count > 0 { 0 } else { -1 };
    tab.editing_dirty = false;

    window.set_diff_count(diff_count as i32);
    window.set_current_diff_index(tab.current_diff);
}

/// Recompute 3-way table diff from CSV text (used by rescan when editing is in progress).
pub(super) fn recompute_table_from_csv_3way(
    window: &MainWindow,
    state: &mut AppState,
    base_csv: &str,
    left_csv: &str,
    right_csv: &str,
) {
    let result = compare_csv_full_3way(base_csv, left_csv, right_csv);
    let tab = state.current_tab();
    let old_cols = tab.table_columns.len();
    let need_rebuild_cols = result.max_cols != old_cols;

    let columns = if need_rebuild_cols {
        let (cols, width) = build_columns(result.max_cols, DEFAULT_COL_WIDTH_PX);
        let tab = state.current_tab_mut();
        tab.table_columns = cols.clone();
        tab.table_content_width_px = width;
        window.set_table_columns(ModelRc::new(VecModel::from(cols.clone())));
        window.set_table_content_width_px(width);
        cols
    } else {
        tab.table_columns.clone()
    };

    let table_rows = build_table_rows_3way(
        &result.base_grid,
        &result.left_grid,
        &result.right_grid,
        &result.cell_status,
        result.max_rows,
        result.max_cols,
        &columns,
    );

    let diff_positions: Vec<usize> = table_rows
        .iter()
        .enumerate()
        .filter(|(_, r)| r.row_status != 0)
        .map(|(i, _)| i)
        .collect();
    let diff_count = diff_positions.len();

    let tab = state.current_tab_mut();
    window.set_table_rows(ModelRc::new(VecModel::from(table_rows.clone())));
    tab.table_rows = table_rows;
    tab.diff_positions = diff_positions;
    tab.current_diff = if diff_count > 0 { 0 } else { -1 };
    tab.editing_dirty = false;

    window.set_diff_count(diff_count as i32);
    window.set_current_diff_index(tab.current_diff);
}

pub(super) fn run_excel_compare(
    window: &MainWindow,
    state: &mut AppState,
    left_bytes: &[u8],
    right_bytes: &[u8],
    left_path: &std::path::Path,
    right_path: &std::path::Path,
) {
    let result = compare_excel_full(left_bytes, right_bytes);

    let left_name = path_file_name(left_path);
    let right_name = path_file_name(right_path);

    // Build SheetTableCache for each sheet
    let mut sheet_cache: std::collections::BTreeMap<String, SheetTableCache> =
        std::collections::BTreeMap::new();
    for (sheet_name, data) in &result.sheets {
        let (columns, content_width_px) = build_columns(data.max_cols, DEFAULT_COL_WIDTH_PX);
        let rows = build_table_rows(
            &data.left_grid,
            &data.right_grid,
            &data.cell_status,
            data.max_rows,
            data.max_cols,
            &columns,
        );
        sheet_cache.insert(
            sheet_name.clone(),
            SheetTableCache {
                rows,
                columns,
                content_width_px,
            },
        );
    }

    // Build combined "All sheets" view: concatenate all sheets
    let mut all_rows = Vec::new();
    let mut all_max_cols: usize = 0;
    for data in result.sheets.values() {
        all_max_cols = all_max_cols.max(data.max_cols);
    }
    let (all_columns, all_content_width_px) = build_columns(all_max_cols, DEFAULT_COL_WIDTH_PX);
    for data in result.sheets.values() {
        let rows = build_table_rows(
            &data.left_grid,
            &data.right_grid,
            &data.cell_status,
            data.max_rows,
            all_max_cols,
            &all_columns,
        );
        all_rows.extend(rows);
    }

    let sheet_names = result.sheet_names.clone();
    let total_diff = result.total_diff_count;

    let tab = state.current_tab_mut();
    tab.view_mode = ViewMode::ExcelCompare;
    tab.title = format!("{} ↔ {}", left_name, right_name);
    tab.excel_sheet_names = sheet_names.clone();
    tab.excel_sheet_data = sheet_cache;
    tab.excel_cells = Vec::new();

    // Build diff positions from row statuses
    let diff_positions: Vec<usize> = all_rows
        .iter()
        .enumerate()
        .filter(|(_, r)| r.row_status != 0)
        .map(|(i, _)| i)
        .collect();
    let diff_pos_count = diff_positions.len();
    tab.diff_positions = diff_positions;
    tab.current_diff = if diff_pos_count > 0 { 0 } else { -1 };

    // Set window properties — clone for window, move into tab
    window.set_table_rows(ModelRc::new(VecModel::from(all_rows.clone())));
    window.set_table_columns(ModelRc::new(VecModel::from(all_columns.clone())));
    tab.table_rows = all_rows;
    tab.table_columns = all_columns;
    tab.table_content_width_px = all_content_width_px;
    window.set_table_content_width_px(all_content_width_px);
    window.set_diff_count(diff_pos_count as i32);
    window.set_current_diff_index(tab.current_diff);

    // Sheet selector: prepend empty string for "All sheets"
    let sheet_model: ModelRc<SharedString> = ModelRc::new(VecModel::from(
        std::iter::once(SharedString::from(""))
            .chain(sheet_names.iter().map(|s| SharedString::from(s.as_str())))
            .collect::<Vec<_>>(),
    ));
    window.set_excel_sheet_names(sheet_model);
    window.set_excel_active_sheet(SharedString::from(""));

    window.set_view_mode(ViewMode::ExcelCompare.as_i32());
    window.set_left_path(SharedString::from(left_path.to_string_lossy().to_string()));
    window.set_right_path(SharedString::from(right_path.to_string_lossy().to_string()));
    window.set_status_text(SharedString::from(format!(
        "[Excel] {} ↔ {} — {} cells changed",
        left_name, right_name, total_diff
    )));
    sync_tab_list(window, state);
}

/// Switch the displayed Excel sheet in table grid view.
/// Empty sheet_name means "All sheets".
pub fn switch_excel_sheet(window: &MainWindow, state: &mut AppState, sheet_name: &str) {
    let tab = state.current_tab_mut();
    if tab.view_mode != ViewMode::ExcelCompare {
        return;
    }

    if sheet_name.is_empty() {
        // "All sheets" — concatenate all cached sheets
        let mut all_rows = Vec::new();
        let mut max_content_width: i32 = 0;
        for cache in tab.excel_sheet_data.values() {
            max_content_width = max_content_width.max(cache.content_width_px);
        }
        // Compute max columns count from caches to build unified columns
        let mut max_cols: usize = 0;
        for cache in tab.excel_sheet_data.values() {
            max_cols = max_cols.max(cache.columns.len());
        }
        for cache in tab.excel_sheet_data.values() {
            all_rows.extend(cache.rows.clone());
        }
        let (columns, content_width_px) = build_columns(max_cols, DEFAULT_COL_WIDTH_PX);
        tab.table_rows = all_rows.clone();
        tab.table_columns = columns.clone();
        tab.table_content_width_px = content_width_px;

        window.set_table_rows(ModelRc::new(VecModel::from(all_rows)));
        window.set_table_columns(ModelRc::new(VecModel::from(columns)));
        window.set_table_content_width_px(content_width_px);
    } else if let Some(cache) = tab.excel_sheet_data.get(sheet_name) {
        let rows = cache.rows.clone();
        let columns = cache.columns.clone();
        let content_width_px = cache.content_width_px;

        tab.table_rows = rows.clone();
        tab.table_columns = columns.clone();
        tab.table_content_width_px = content_width_px;

        window.set_table_rows(ModelRc::new(VecModel::from(rows)));
        window.set_table_columns(ModelRc::new(VecModel::from(columns)));
        window.set_table_content_width_px(content_width_px);
    }

    // Rebuild diff positions for the new sheet
    let tab = state.current_tab_mut();
    let diff_positions: Vec<usize> = tab
        .table_rows
        .iter()
        .enumerate()
        .filter(|(_, r)| r.row_status != 0)
        .map(|(i, _)| i)
        .collect();
    let diff_pos_count = diff_positions.len();
    tab.diff_positions = diff_positions;
    tab.current_diff = if diff_pos_count > 0 { 0 } else { -1 };
    window.set_diff_count(diff_pos_count as i32);
    window.set_current_diff_index(tab.current_diff);
}

pub(super) fn run_csv_compare(
    window: &MainWindow,
    state: &mut AppState,
    left_bytes: &[u8],
    right_bytes: &[u8],
    left_path: &std::path::Path,
    right_path: &std::path::Path,
) {
    use crate::encoding::decode_file;

    let (left_text, _) = decode_file(left_bytes);
    let (right_text, _) = decode_file(right_bytes);

    let result = compare_csv_full(&left_text, &right_text);

    let (columns, content_width_px) = build_columns(result.max_cols, DEFAULT_COL_WIDTH_PX);
    let table_rows = build_table_rows(
        &result.left_grid,
        &result.right_grid,
        &result.cell_status,
        result.max_rows,
        result.max_cols,
        &columns,
    );

    let left_name = path_file_name(left_path);
    let right_name = path_file_name(right_path);

    let diff_count = result.diff_count;
    let delimiter_mismatch = result.delimiter_mismatch;

    // Detect delimiter from left file for save operations
    let left_delim = crate::csv::detect_delimiter(&left_text);

    let tab = state.current_tab_mut();
    tab.view_mode = ViewMode::CsvCompare;
    tab.title = format!("{} ↔ {}", left_name, right_name);
    tab.csv_delimiter = left_delim;
    tab.excel_cells = Vec::new();
    tab.excel_sheet_names = Vec::new();
    tab.excel_sheet_data.clear();

    // Build diff positions from row statuses before moving table_rows
    let diff_positions: Vec<usize> = table_rows
        .iter()
        .enumerate()
        .filter(|(_, r)| r.row_status != 0)
        .map(|(i, _)| i)
        .collect();
    let diff_pos_count = diff_positions.len();
    tab.diff_positions = diff_positions;
    tab.current_diff = if diff_pos_count > 0 { 0 } else { -1 };

    // Clone for window, move into tab
    window.set_table_rows(ModelRc::new(VecModel::from(table_rows.clone())));
    window.set_table_columns(ModelRc::new(VecModel::from(columns.clone())));
    window.set_table_content_width_px(content_width_px);
    tab.table_rows = table_rows;
    tab.table_columns = columns;
    tab.table_content_width_px = content_width_px;

    window.set_diff_count(diff_pos_count as i32);
    window.set_current_diff_index(tab.current_diff);

    window.set_view_mode(ViewMode::CsvCompare.as_i32());
    window.set_left_path(SharedString::from(left_path.to_string_lossy().to_string()));
    window.set_right_path(SharedString::from(right_path.to_string_lossy().to_string()));

    // Build status with warnings
    let mut status = format!(
        "[CSV] {} \u{2194} {} \u{2014} {} cells changed",
        left_name, right_name, diff_count
    );
    let left_ext = left_path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let right_ext = right_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    if !left_ext.eq_ignore_ascii_case(right_ext) {
        status.push_str(&format!(" \u{26a0} .{} vs .{}", left_ext, right_ext));
    }
    if delimiter_mismatch {
        status.push_str(" \u{26a0} different delimiters detected");
    }
    window.set_status_text(SharedString::from(status));
    sync_tab_list(window, state);
}

pub(super) fn run_csv_compare_3way(
    window: &MainWindow,
    state: &mut AppState,
    base_bytes: &[u8],
    left_bytes: &[u8],
    right_bytes: &[u8],
    base_path: &std::path::Path,
    left_path: &std::path::Path,
    right_path: &std::path::Path,
) {
    use crate::encoding::decode_file;

    let (base_text, _) = decode_file(base_bytes);
    let (left_text, _) = decode_file(left_bytes);
    let (right_text, _) = decode_file(right_bytes);

    let result = compare_csv_full_3way(&base_text, &left_text, &right_text);

    let (columns, content_width_px) = build_columns(result.max_cols, DEFAULT_COL_WIDTH_PX);
    let table_rows = build_table_rows_3way(
        &result.base_grid,
        &result.left_grid,
        &result.right_grid,
        &result.cell_status,
        result.max_rows,
        result.max_cols,
        &columns,
    );

    let left_name = path_file_name(left_path);
    let right_name = path_file_name(right_path);

    let diff_count = result.diff_count;
    let conflict_count = result.conflict_count;
    let delimiter_mismatch = result.delimiter_mismatch;

    let base_delim = crate::csv::detect_delimiter(&base_text);

    let tab = state.current_tab_mut();
    tab.view_mode = ViewMode::CsvThreeWay;
    tab.title = format!("{} ↔ {} (3-way)", left_name, right_name);
    tab.csv_delimiter = base_delim;
    tab.excel_cells = Vec::new();
    tab.excel_sheet_names = Vec::new();
    tab.excel_sheet_data.clear();

    // Build diff positions from row statuses before moving table_rows
    let diff_positions: Vec<usize> = table_rows
        .iter()
        .enumerate()
        .filter(|(_, r)| r.row_status != 0)
        .map(|(i, _)| i)
        .collect();
    let diff_pos_count = diff_positions.len();
    tab.diff_positions = diff_positions;
    tab.current_diff = if diff_pos_count > 0 { 0 } else { -1 };

    // Clone for window, move into tab
    window.set_table_rows(ModelRc::new(VecModel::from(table_rows.clone())));
    window.set_table_columns(ModelRc::new(VecModel::from(columns.clone())));
    window.set_table_content_width_px(content_width_px);
    tab.table_rows = table_rows;
    tab.table_columns = columns;
    tab.table_content_width_px = content_width_px;

    window.set_diff_count(diff_pos_count as i32);
    window.set_current_diff_index(tab.current_diff);

    window.set_view_mode(ViewMode::CsvThreeWay.as_i32());
    window.set_left_path(SharedString::from(left_path.to_string_lossy().to_string()));
    window.set_right_path(SharedString::from(right_path.to_string_lossy().to_string()));
    window.set_base_path(SharedString::from(base_path.to_string_lossy().to_string()));

    // Build status with warnings
    let mut status = format!(
        "[CSV 3-way] {} \u{2194} {} \u{2014} {} diffs, {} conflicts",
        left_name, right_name, diff_count, conflict_count
    );
    // Extension mismatch check across all 3 files
    let exts: Vec<&str> = [base_path, left_path, right_path]
        .iter()
        .map(|p| p.extension().and_then(|e| e.to_str()).unwrap_or(""))
        .collect();
    if !exts[0].eq_ignore_ascii_case(exts[1]) || !exts[0].eq_ignore_ascii_case(exts[2]) {
        status.push_str(" \u{26a0} mixed file extensions");
    }
    if delimiter_mismatch {
        status.push_str(" \u{26a0} different delimiters detected");
    }
    window.set_status_text(SharedString::from(status));
    sync_tab_list(window, state);
}
