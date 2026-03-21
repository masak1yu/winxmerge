use crate::models::diff_line::{DiffResult, LineStatus};

/// One comment entry across all tabs
pub struct CommentEntry {
    pub tab_title: String,
    pub left_file: String,
    pub right_file: String,
    pub diff_block: usize,
    pub comment: String,
}

/// Export all comment entries as CSV
pub fn export_all_comments_csv(entries: &[CommentEntry]) -> String {
    let mut out = String::from("Tab,Left File,Right File,Diff Block,Comment\n");
    for e in entries {
        let esc = |s: &str| format!("\"{}\"", s.replace('"', "\"\""));
        out.push_str(&format!(
            "{},{},{},{},{}\n",
            esc(&e.tab_title),
            esc(&e.left_file),
            esc(&e.right_file),
            e.diff_block,
            esc(&e.comment)
        ));
    }
    out
}

/// Export all comment entries as JSON
pub fn export_all_comments_json(entries: &[CommentEntry]) -> String {
    let esc = |s: &str| {
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
    };
    let mut out = String::from("[\n");
    for (i, e) in entries.iter().enumerate() {
        out.push_str(&format!(
            "  {{\"tab\":\"{}\",\"left_file\":\"{}\",\"right_file\":\"{}\",\"diff_block\":{},\"comment\":\"{}\"}}{}",
            esc(&e.tab_title),
            esc(&e.left_file),
            esc(&e.right_file),
            e.diff_block,
            esc(&e.comment),
            if i + 1 < entries.len() { ",\n" } else { "\n" }
        ));
    }
    out.push_str("]\n");
    out
}

/// Generate HTML for printing: includes auto-print script and print-optimized CSS
pub fn export_html_for_print(
    diff_result: &DiffResult,
    left_title: &str,
    right_title: &str,
    comments: &std::collections::HashMap<usize, String>,
) -> String {
    let mut html = export_html(diff_result, left_title, right_title, comments);
    // Insert auto-print script before </body>
    let script = "<script>window.onload=function(){window.print();}</script>\n";
    if let Some(pos) = html.rfind("</body>") {
        html.insert_str(pos, script);
    } else {
        html.push_str(script);
    }
    html
}

/// Export diff result as an HTML file
pub fn export_html(
    diff_result: &DiffResult,
    left_title: &str,
    right_title: &str,
    comments: &std::collections::HashMap<usize, String>,
) -> String {
    let mut html = String::new();

    html.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
    html.push_str("<meta charset=\"utf-8\">\n");
    html.push_str("<title>WinXMerge - Diff Report</title>\n");
    html.push_str("<style>\n");
    html.push_str(CSS);
    html.push_str("</style>\n");
    html.push_str("</head>\n<body>\n");

    // Header
    html.push_str("<div class=\"header\">\n");
    html.push_str("<h1>WinXMerge - Diff Report</h1>\n");
    html.push_str(&format!(
        "<p class=\"summary\">{} differences found</p>\n",
        diff_result.diff_count
    ));
    html.push_str("</div>\n");

    // Table
    html.push_str("<table>\n<thead>\n<tr>\n");
    html.push_str(&format!(
        "<th class=\"line-no\">#</th><th class=\"content\">{}</th>\n",
        escape_html(left_title)
    ));
    html.push_str(&format!(
        "<th class=\"line-no\">#</th><th class=\"content\">{}</th>\n",
        escape_html(right_title)
    ));
    html.push_str("</tr>\n</thead>\n<tbody>\n");

    let mut diff_block_idx: usize = 0;
    for line in &diff_result.lines {
        let class = match line.status {
            LineStatus::Equal => "",
            LineStatus::Added => "added",
            LineStatus::Removed => "removed",
            LineStatus::Modified => "modified",
            LineStatus::Moved => "moved",
        };

        html.push_str(&format!("<tr class=\"{}\">\n", class));

        // Left side
        html.push_str(&format!(
            "<td class=\"line-no\">{}</td>",
            line.left_line_no.map(|n| n.to_string()).unwrap_or_default()
        ));
        html.push_str(&format!(
            "<td class=\"content\">{}</td>\n",
            escape_html(&line.left_text)
        ));

        // Right side
        html.push_str(&format!(
            "<td class=\"line-no\">{}</td>",
            line.right_line_no
                .map(|n| n.to_string())
                .unwrap_or_default()
        ));
        html.push_str(&format!(
            "<td class=\"content\">{}</td>\n",
            escape_html(&line.right_text)
        ));

        html.push_str("</tr>\n");

        // After each non-Equal row, emit comment row if present
        if line.status != LineStatus::Equal {
            if let Some(comment) = comments.get(&diff_block_idx) {
                if !comment.is_empty() {
                    html.push_str(&format!(
                        "<tr class=\"comment-row\"><td colspan=\"4\">💬 {}</td></tr>\n",
                        escape_html(comment)
                    ));
                }
            }
            diff_block_idx += 1;
        }
    }

    html.push_str("</tbody>\n</table>\n");
    html.push_str("<div class=\"footer\">Generated by WinXMerge</div>\n");
    html.push_str("</body>\n</html>");

    html
}

/// Export diff result as a unified diff (patch) file
pub fn export_unified_diff(
    diff_result: &DiffResult,
    left_title: &str,
    right_title: &str,
) -> String {
    let mut output = String::new();
    output.push_str(&format!("--- {}\n", left_title));
    output.push_str(&format!("+++ {}\n", right_title));

    // Group consecutive changes into hunks
    let lines = &diff_result.lines;
    if lines.is_empty() {
        return output;
    }

    let context_lines = 3usize;
    let mut i = 0;
    while i < lines.len() {
        // Find the start of a change
        if lines[i].status == LineStatus::Equal {
            i += 1;
            continue;
        }

        // Found a change; build a hunk with context
        let hunk_start = i.saturating_sub(context_lines);
        // Find the end of changes (merging nearby changes)
        let mut hunk_end = i;
        while hunk_end < lines.len() {
            if lines[hunk_end].status != LineStatus::Equal {
                hunk_end += 1;
            } else {
                // Check if another change is within context range
                let lookahead = (hunk_end + context_lines * 2 + 1).min(lines.len());
                let next_change =
                    (hunk_end..lookahead).find(|&j| lines[j].status != LineStatus::Equal);
                if let Some(nc) = next_change {
                    hunk_end = nc + 1;
                } else {
                    break;
                }
            }
        }
        let hunk_end = (hunk_end + context_lines).min(lines.len());

        // Count left/right lines in hunk
        let mut left_count = 0u32;
        let mut right_count = 0u32;
        let mut left_start = 0u32;
        let mut right_start = 0u32;
        let mut left_start_set = false;
        let mut right_start_set = false;

        for line in &lines[hunk_start..hunk_end] {
            if let Some(n) = line.left_line_no {
                if !left_start_set {
                    left_start = n as u32;
                    left_start_set = true;
                }
                left_count += 1;
            }
            if let Some(n) = line.right_line_no {
                if !right_start_set {
                    right_start = n as u32;
                    right_start_set = true;
                }
                right_count += 1;
            }
        }

        if !left_start_set {
            left_start = 1;
        }
        if !right_start_set {
            right_start = 1;
        }

        output.push_str(&format!(
            "@@ -{},{} +{},{} @@\n",
            left_start, left_count, right_start, right_count
        ));

        for line in &lines[hunk_start..hunk_end] {
            match line.status {
                LineStatus::Equal => {
                    output.push(' ');
                    output.push_str(&line.left_text);
                    output.push('\n');
                }
                LineStatus::Removed => {
                    output.push('-');
                    output.push_str(&line.left_text);
                    output.push('\n');
                }
                LineStatus::Added => {
                    output.push('+');
                    output.push_str(&line.right_text);
                    output.push('\n');
                }
                LineStatus::Modified | LineStatus::Moved => {
                    if !line.left_text.is_empty() {
                        output.push('-');
                        output.push_str(&line.left_text);
                        output.push('\n');
                    }
                    if !line.right_text.is_empty() {
                        output.push('+');
                        output.push_str(&line.right_text);
                        output.push('\n');
                    }
                }
            }
        }

        i = hunk_end;
    }

    output
}

/// Export diff as CSV (or TSV when sep='\t')
pub fn export_csv(result: &DiffResult, left_title: &str, right_title: &str, sep: char) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "Status{}Left Line{}Right Line{}Left Text{}Right Text\n",
        sep, sep, sep, sep
    ));
    for line in &result.lines {
        let status = match line.status {
            LineStatus::Equal => "Equal",
            LineStatus::Added => "Added",
            LineStatus::Removed => "Removed",
            LineStatus::Modified => "Modified",
            LineStatus::Moved => "Moved",
        };
        let left_no = line.left_line_no.map(|n| n.to_string()).unwrap_or_default();
        let right_no = line
            .right_line_no
            .map(|n| n.to_string())
            .unwrap_or_default();
        let escape = |s: &str| format!("\"{}\"", s.replace('"', "\"\""));
        out.push_str(&format!(
            "{}{}{}{}{}{}{}{}{}\n",
            status,
            sep,
            left_no,
            sep,
            right_no,
            sep,
            escape(&line.left_text),
            sep,
            escape(&line.right_text)
        ));
    }
    let _ = left_title;
    let _ = right_title;
    out
}

/// Export folder comparison as HTML
pub fn export_folder_html(
    items: &[crate::models::folder_item::FolderItem],
    left_title: &str,
    right_title: &str,
) -> String {
    use crate::models::folder_item::FileCompareStatus;
    let mut rows = String::new();
    for item in items {
        let status_text = match item.status {
            FileCompareStatus::Identical => "Identical",
            FileCompareStatus::Different => "Different",
            FileCompareStatus::LeftOnly => "Left only",
            FileCompareStatus::RightOnly => "Right only",
        };
        let color = match item.status {
            FileCompareStatus::Identical => "#888",
            FileCompareStatus::Different => "#b08800",
            FileCompareStatus::LeftOnly => "#cb2431",
            FileCompareStatus::RightOnly => "#22863a",
        };
        rows.push_str(&format!(
            "<tr><td>{}</td><td style=\"color:{}\">{}</td><td>{}</td><td>{}</td></tr>\n",
            escape_html(&item.relative_path),
            color,
            status_text,
            item.left_modified.as_deref().unwrap_or(""),
            item.right_modified.as_deref().unwrap_or("")
        ));
    }
    format!(
        r#"<!DOCTYPE html>
<html><head><meta charset="utf-8"><title>Folder Compare</title>
<style>body{{font-family:monospace;font-size:13px}} table{{border-collapse:collapse;width:100%}} th,td{{border:1px solid #ddd;padding:4px 8px}} th{{background:#f0f0f0}}</style>
</head><body>
<h2>Folder Compare: {} &#x2194; {}</h2>
<table><tr><th>Path</th><th>Status</th><th>Left Modified</th><th>Right Modified</th></tr>
{}
</table></body></html>"#,
        escape_html(left_title),
        escape_html(right_title),
        rows
    )
}

/// Export diff result as an Excel (.xlsx) file
pub fn export_xlsx(
    diff_result: &DiffResult,
    left_title: &str,
    right_title: &str,
    comments: &std::collections::HashMap<usize, String>,
) -> Result<Vec<u8>, String> {
    use rust_xlsxwriter::{Color, Format, Workbook};

    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();
    sheet.set_name("Diff Report").map_err(|e| e.to_string())?;

    // Formats
    let fmt_header = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0xF0F0F0));
    let fmt_added = Format::new().set_background_color(Color::RGB(0xE6FFEC));
    let fmt_removed = Format::new().set_background_color(Color::RGB(0xFFEBE9));
    let fmt_modified = Format::new().set_background_color(Color::RGB(0xFFF8C5));
    let fmt_moved = Format::new().set_background_color(Color::RGB(0xE0E8FF));
    let fmt_comment = Format::new()
        .set_background_color(Color::RGB(0xFFFBE6))
        .set_italic();

    // Header row
    sheet
        .write_string_with_format(0, 0, "Status", &fmt_header)
        .map_err(|e| e.to_string())?;
    sheet
        .write_string_with_format(0, 1, "Left Line", &fmt_header)
        .map_err(|e| e.to_string())?;
    sheet
        .write_string_with_format(0, 2, left_title, &fmt_header)
        .map_err(|e| e.to_string())?;
    sheet
        .write_string_with_format(0, 3, "Right Line", &fmt_header)
        .map_err(|e| e.to_string())?;
    sheet
        .write_string_with_format(0, 4, right_title, &fmt_header)
        .map_err(|e| e.to_string())?;
    sheet
        .write_string_with_format(0, 5, "Comment", &fmt_header)
        .map_err(|e| e.to_string())?;

    // Column widths
    sheet.set_column_width(0, 10).map_err(|e| e.to_string())?;
    sheet.set_column_width(1, 8).map_err(|e| e.to_string())?;
    sheet.set_column_width(2, 60).map_err(|e| e.to_string())?;
    sheet.set_column_width(3, 8).map_err(|e| e.to_string())?;
    sheet.set_column_width(4, 60).map_err(|e| e.to_string())?;
    sheet.set_column_width(5, 40).map_err(|e| e.to_string())?;

    let mut row: u32 = 1;
    let mut diff_block_idx: usize = 0;

    for line in &diff_result.lines {
        let (status_str, fmt) = match line.status {
            LineStatus::Equal => ("Equal", None),
            LineStatus::Added => ("Added", Some(&fmt_added)),
            LineStatus::Removed => ("Removed", Some(&fmt_removed)),
            LineStatus::Modified => ("Modified", Some(&fmt_modified)),
            LineStatus::Moved => ("Moved", Some(&fmt_moved)),
        };

        let left_no = line.left_line_no.map(|n| n as f64);
        let right_no = line.right_line_no.map(|n| n as f64);

        if let Some(fmt) = fmt {
            sheet
                .write_string_with_format(row, 0, status_str, fmt)
                .map_err(|e| e.to_string())?;
            if let Some(n) = left_no {
                sheet
                    .write_number_with_format(row, 1, n, fmt)
                    .map_err(|e| e.to_string())?;
            }
            sheet
                .write_string_with_format(row, 2, &line.left_text, fmt)
                .map_err(|e| e.to_string())?;
            if let Some(n) = right_no {
                sheet
                    .write_number_with_format(row, 3, n, fmt)
                    .map_err(|e| e.to_string())?;
            }
            sheet
                .write_string_with_format(row, 4, &line.right_text, fmt)
                .map_err(|e| e.to_string())?;

            // Comment column
            let comment = comments
                .get(&diff_block_idx)
                .map(|s| s.as_str())
                .unwrap_or("");
            if !comment.is_empty() {
                sheet
                    .write_string_with_format(row, 5, comment, fmt)
                    .map_err(|e| e.to_string())?;
            }
            diff_block_idx += 1;
        } else {
            sheet
                .write_string(row, 0, status_str)
                .map_err(|e| e.to_string())?;
            if let Some(n) = left_no {
                sheet.write_number(row, 1, n).map_err(|e| e.to_string())?;
            }
            sheet
                .write_string(row, 2, &line.left_text)
                .map_err(|e| e.to_string())?;
            if let Some(n) = right_no {
                sheet.write_number(row, 3, n).map_err(|e| e.to_string())?;
            }
            sheet
                .write_string(row, 4, &line.right_text)
                .map_err(|e| e.to_string())?;
        }

        row += 1;
    }

    // Freeze header row
    sheet.set_freeze_panes(1, 0).map_err(|e| e.to_string())?;

    // Auto-filter on header
    sheet
        .autofilter(0, 0, row - 1, 5)
        .map_err(|e| e.to_string())?;

    // Drop unused format warnings
    let _ = fmt_comment;

    workbook.save_to_buffer().map_err(|e| e.to_string())
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace(' ', "&nbsp;")
        .replace('\t', "&nbsp;&nbsp;&nbsp;&nbsp;")
}

const CSS: &str = r#"
body {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    margin: 0; padding: 20px;
    background: #f5f5f5;
}
.header { margin-bottom: 20px; }
.header h1 { margin: 0; font-size: 24px; }
.summary { color: #666; margin: 4px 0; }
table {
    width: 100%; border-collapse: collapse;
    font-family: "SF Mono", "Menlo", "Monaco", monospace;
    font-size: 13px; background: white;
    border: 1px solid #ddd;
}
thead { background: #f0f0f0; }
th { padding: 6px 8px; text-align: left; border: 1px solid #ddd; font-weight: 600; }
td { padding: 2px 8px; border: 1px solid #eee; white-space: pre; vertical-align: top; }
.line-no { width: 40px; color: #999; text-align: right; user-select: none; }
.content { min-width: 200px; }
tr.added { background: #e6ffec; }
tr.removed { background: #ffebe9; }
tr.modified { background: #fff8c5; }
tr.moved { background: #e0e8ff; }
.comment-row td { background: #fffbe6; color: #856404; font-style: italic; padding: 4px 8px; border: 1px solid #ffd700; }
.footer { margin-top: 20px; color: #999; font-size: 12px; }
@media print {
    body { background: white; padding: 0; }
    .header h1 { font-size: 16px; }
}
"#;
