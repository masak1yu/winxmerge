use calamine::{Reader, open_workbook_auto_from_rs};
use std::io::Cursor;

#[derive(Debug, Clone)]
pub struct ExcelCellDiff {
    pub sheet: String,
    pub row: usize, // 1-based
    #[allow(dead_code)]
    pub col: usize, // 1-based
    pub col_name: String,
    pub left_value: String,
    pub right_value: String,
    /// 0=identical, 1=different, 2=left_only (cell/sheet exists only in left), 3=right_only
    pub status: i32,
}

/// Returns true if the path extension is a supported Excel format
pub fn is_excel_path(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| matches!(e.to_lowercase().as_str(), "xlsx" | "xls" | "xlsm" | "ods"))
        .unwrap_or(false)
}

/// Convert column index (0-based) to Excel column name (A, B, ... Z, AA, AB, ...)
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

fn read_workbook(data: &[u8]) -> std::collections::BTreeMap<String, Vec<Vec<String>>> {
    let cursor = Cursor::new(data.to_vec());
    let mut wb = match open_workbook_auto_from_rs(cursor) {
        Ok(w) => w,
        Err(_) => return std::collections::BTreeMap::new(),
    };
    let mut result = std::collections::BTreeMap::new();
    let sheet_names = wb.sheet_names().to_vec();
    for name in sheet_names {
        if let Ok(range) = wb.worksheet_range(&name) {
            let mut rows = Vec::new();
            for row in range.rows() {
                let cells: Vec<String> = row.iter().map(|c| c.to_string()).collect();
                rows.push(cells);
            }
            result.insert(name, rows);
        }
    }
    result
}

/// Compare two Excel files and return a list of cell differences.
pub fn compare_excel(left_data: &[u8], right_data: &[u8]) -> Vec<ExcelCellDiff> {
    let left_wb = read_workbook(left_data);
    let right_wb = read_workbook(right_data);

    let mut all_sheets: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for k in left_wb.keys() {
        all_sheets.insert(k.clone());
    }
    for k in right_wb.keys() {
        all_sheets.insert(k.clone());
    }

    let mut diffs = Vec::new();

    for sheet in &all_sheets {
        let left_rows = left_wb.get(sheet);
        let right_rows = right_wb.get(sheet);

        let max_rows = left_rows
            .map(|r| r.len())
            .unwrap_or(0)
            .max(right_rows.map(|r| r.len()).unwrap_or(0));

        for row_idx in 0..max_rows {
            let left_row = left_rows.and_then(|r| r.get(row_idx));
            let right_row = right_rows.and_then(|r| r.get(row_idx));

            let max_cols = left_row
                .map(|r| r.len())
                .unwrap_or(0)
                .max(right_row.map(|r| r.len()).unwrap_or(0));

            for col_idx in 0..max_cols {
                let left_val = left_row
                    .and_then(|r| r.get(col_idx))
                    .map(|s| s.as_str())
                    .unwrap_or("");
                let right_val = right_row
                    .and_then(|r| r.get(col_idx))
                    .map(|s| s.as_str())
                    .unwrap_or("");

                // Skip if both empty
                if left_val.is_empty() && right_val.is_empty() {
                    continue;
                }

                let status = if left_wb.contains_key(sheet) && !right_wb.contains_key(sheet) {
                    2 // left_only (whole sheet)
                } else if !left_wb.contains_key(sheet) && right_wb.contains_key(sheet) {
                    3 // right_only (whole sheet)
                } else if left_val == right_val {
                    0 // identical
                } else {
                    1 // different
                };

                // Only emit non-identical cells
                if status != 0 {
                    diffs.push(ExcelCellDiff {
                        sheet: sheet.clone(),
                        row: row_idx + 1,
                        col: col_idx + 1,
                        col_name: col_to_name(col_idx),
                        left_value: left_val.to_string(),
                        right_value: right_val.to_string(),
                        status,
                    });
                }
            }
        }
    }
    diffs
}
