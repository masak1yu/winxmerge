/// CSV / TSV table comparison

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CsvCellDiff {
    pub row: usize, // 1-based
    #[allow(dead_code)]
    pub col: usize, // 1-based
    pub col_name: String,
    pub left_value: String,
    pub right_value: String,
    /// 0=identical, 1=different, 2=left_only, 3=right_only
    pub status: i32,
}

/// Returns true if the path is a CSV or TSV file
pub fn is_csv_path(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| matches!(e.to_lowercase().as_str(), "csv" | "tsv"))
        .unwrap_or(false)
}

fn detect_delimiter(text: &str) -> u8 {
    // Simple heuristic: count commas vs tabs in the first line
    let first_line = text.lines().next().unwrap_or("");
    let commas = first_line.bytes().filter(|&b| b == b',').count();
    let tabs = first_line.bytes().filter(|&b| b == b'\t').count();
    if tabs > commas { b'\t' } else { b',' }
}

pub fn col_to_name(mut col: usize) -> String {
    let mut name = String::new();
    loop {
        name.insert(0, (b'A' + (col % 26) as u8) as char);
        if col < 26 {
            break;
        }
        col = col / 26 - 1;
    }
    name
}

/// Parse CSV/TSV text into a 2-D table of cell strings.
/// Handles quoted fields with embedded delimiters and newlines.
pub fn parse_csv(text: &str, delimiter: u8) -> Vec<Vec<String>> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut chars = text.bytes().peekable();

    while let Some(&b) = chars.peek() {
        chars.next();
        if in_quotes {
            if b == b'"' {
                if chars.peek() == Some(&b'"') {
                    // Escaped double-quote
                    chars.next();
                    field.push('"');
                } else {
                    in_quotes = false;
                }
            } else {
                field.push(b as char);
            }
        } else if b == b'"' {
            in_quotes = true;
        } else if b == delimiter {
            row.push(field.clone());
            field.clear();
        } else if b == b'\n' {
            row.push(field.clone());
            field.clear();
            rows.push(row.clone());
            row.clear();
        } else if b == b'\r' {
            // skip CR in CRLF
        } else {
            field.push(b as char);
        }
    }

    // Flush last field / row
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }

    rows
}

/// Full comparison result including both grids and per-cell status.
#[derive(Debug, Clone)]
pub struct CsvTableResult {
    pub left_grid: Vec<Vec<String>>,
    pub right_grid: Vec<Vec<String>>,
    pub max_rows: usize,
    pub max_cols: usize,
    /// cell_status[row][col] = 0(identical)/1(different)/2(left-only)/3(right-only)
    pub cell_status: Vec<Vec<i32>>,
    pub diff_count: usize,
}

/// Compare two CSV/TSV texts and return full grids with per-cell status.
pub fn compare_csv_full(left_text: &str, right_text: &str) -> CsvTableResult {
    let delim = detect_delimiter(left_text);
    let left_grid = parse_csv(left_text, delim);
    let right_grid = parse_csv(right_text, delim);

    let max_rows = left_grid.len().max(right_grid.len());
    let max_cols = left_grid
        .iter()
        .chain(right_grid.iter())
        .map(|r| r.len())
        .max()
        .unwrap_or(0);

    let mut cell_status = Vec::with_capacity(max_rows);
    let mut diff_count = 0;

    for r in 0..max_rows {
        let left_row = left_grid.get(r);
        let right_row = right_grid.get(r);
        let mut row_status = Vec::with_capacity(max_cols);

        for c in 0..max_cols {
            let lv = left_row
                .and_then(|row| row.get(c))
                .map(|s| s.as_str())
                .unwrap_or("");
            let rv = right_row
                .and_then(|row| row.get(c))
                .map(|s| s.as_str())
                .unwrap_or("");

            let status = match (left_row.is_some(), right_row.is_some()) {
                (true, false) => 2,
                (false, true) => 3,
                _ if lv == rv => 0,
                _ => 1,
            };

            if status != 0 {
                diff_count += 1;
            }
            row_status.push(status);
        }

        cell_status.push(row_status);
    }

    CsvTableResult {
        left_grid,
        right_grid,
        max_rows,
        max_cols,
        cell_status,
        diff_count,
    }
}

/// 3-way full comparison result including all three grids and per-cell status.
#[derive(Debug, Clone)]
pub struct CsvTableResult3Way {
    pub base_grid: Vec<Vec<String>>,
    pub left_grid: Vec<Vec<String>>,
    pub right_grid: Vec<Vec<String>>,
    pub max_rows: usize,
    pub max_cols: usize,
    /// cell_status[row][col]:
    /// 0=identical (all three same)
    /// 1=left-changed (left differs from base, right same as base)
    /// 2=right-changed (right differs from base, left same as base)
    /// 3=both-changed-same (both differ from base, but same as each other)
    /// 4=conflict (both differ from base, and different from each other)
    /// 5=base-only (row only in base)
    /// 6=left-only (row only in left)
    /// 7=right-only (row only in right)
    pub cell_status: Vec<Vec<i32>>,
    pub diff_count: usize,
    pub conflict_count: usize,
}

/// Compare three CSV/TSV texts and return full grids with per-cell 3-way status.
pub fn compare_csv_full_3way(
    base_text: &str,
    left_text: &str,
    right_text: &str,
) -> CsvTableResult3Way {
    let delim = detect_delimiter(base_text);
    let base_grid = parse_csv(base_text, delim);
    let left_grid = parse_csv(left_text, delim);
    let right_grid = parse_csv(right_text, delim);

    let max_rows = base_grid.len().max(left_grid.len()).max(right_grid.len());
    let max_cols = base_grid
        .iter()
        .chain(left_grid.iter())
        .chain(right_grid.iter())
        .map(|r| r.len())
        .max()
        .unwrap_or(0);

    let mut cell_status = Vec::with_capacity(max_rows);
    let mut diff_count = 0;
    let mut conflict_count = 0;

    for r in 0..max_rows {
        let base_row = base_grid.get(r);
        let left_row = left_grid.get(r);
        let right_row = right_grid.get(r);
        let mut row_status = Vec::with_capacity(max_cols);

        let has_base = base_row.is_some();
        let has_left = left_row.is_some();
        let has_right = right_row.is_some();

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

            let status = if !has_base && has_left && !has_right {
                6 // left-only row
            } else if !has_base && !has_left && has_right {
                7 // right-only row
            } else if has_base && !has_left && !has_right {
                5 // base-only row (deleted in both)
            } else if !has_base && has_left && has_right {
                // Row added in both sides
                if lv == rv {
                    3 // both-changed-same
                } else {
                    4 // conflict
                }
            } else {
                // Normal 3-way compare
                let left_changed = lv != bv;
                let right_changed = rv != bv;
                match (left_changed, right_changed) {
                    (false, false) => 0, // identical
                    (true, false) => 1,  // left-changed
                    (false, true) => 2,  // right-changed
                    (true, true) => {
                        if lv == rv {
                            3 // both-changed-same
                        } else {
                            4 // conflict
                        }
                    }
                }
            };

            if status != 0 {
                diff_count += 1;
            }
            if status == 4 {
                conflict_count += 1;
            }
            row_status.push(status);
        }

        cell_status.push(row_status);
    }

    CsvTableResult3Way {
        base_grid,
        left_grid,
        right_grid,
        max_rows,
        max_cols,
        cell_status,
        diff_count,
        conflict_count,
    }
}

/// Compare two CSV/TSV texts and return cell-level differences.
#[allow(dead_code)]
pub fn compare_csv(left_text: &str, right_text: &str) -> Vec<CsvCellDiff> {
    let delim = detect_delimiter(left_text);
    let left_rows = parse_csv(left_text, delim);
    let right_rows = parse_csv(right_text, delim);

    let max_rows = left_rows.len().max(right_rows.len());
    let mut diffs = Vec::new();

    for r in 0..max_rows {
        let left_row = left_rows.get(r);
        let right_row = right_rows.get(r);

        let max_cols = left_row
            .map(|r| r.len())
            .unwrap_or(0)
            .max(right_row.map(|r| r.len()).unwrap_or(0));

        for c in 0..max_cols {
            let lv = left_row
                .and_then(|row| row.get(c))
                .map(|s| s.as_str())
                .unwrap_or("");
            let rv = right_row
                .and_then(|row| row.get(c))
                .map(|s| s.as_str())
                .unwrap_or("");

            let status = match (left_row.is_some(), right_row.is_some()) {
                (true, false) => 2,        // left only
                (false, true) => 3,        // right only
                _ if lv == rv => continue, // identical — skip
                _ => 1,                    // different
            };

            diffs.push(CsvCellDiff {
                row: r + 1,
                col: c + 1,
                col_name: col_to_name(c),
                left_value: lv.to_string(),
                right_value: rv.to_string(),
                status,
            });
        }
    }

    diffs
}
