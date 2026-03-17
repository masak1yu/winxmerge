use std::path::Path;
use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

/// Highlight names we recognize, mapped to color indices
const HIGHLIGHT_NAMES: &[&str] = &[
    "keyword",              // 0
    "string",               // 1
    "comment",              // 2
    "number",               // 3
    "function",             // 4
    "function.builtin",     // 5
    "type",                 // 6
    "type.builtin",         // 7
    "variable",             // 8
    "variable.builtin",     // 9
    "operator",             // 10
    "constant",             // 11
    "constant.builtin",     // 12
    "property",             // 13
    "attribute",            // 14
    "punctuation.bracket",  // 15
    "punctuation.delimiter",// 16
    "constructor",          // 17
    "module",               // 18
    "tag",                  // 19
    "embedded",             // 20
];

/// Per-line highlight: the dominant highlight type for the line
/// Returns a Vec of highlight indices (one per line), where:
///   -1 = no highlight (plain text)
///   0..N = index into HIGHLIGHT_NAMES
pub fn highlight_lines(source: &str, file_path: &str) -> Vec<i32> {
    let line_count = source.lines().count().max(1);
    let mut line_highlights = vec![-1i32; line_count];

    let ext = Path::new(file_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let config = match get_highlight_config(ext) {
        Some(c) => c,
        None => return line_highlights,
    };

    let mut highlighter = Highlighter::new();
    let highlights = highlighter.highlight(&config, source.as_bytes(), None, |_| None);

    let highlights = match highlights {
        Ok(h) => h,
        Err(_) => return line_highlights,
    };

    // Track which highlight is active and accumulate per-line dominant type
    let mut current_highlight: Option<usize> = None;
    let mut byte_offset = 0usize;
    // Count occurrences of each highlight type per line
    let mut line_type_counts: Vec<[u32; 21]> = vec![[0u32; 21]; line_count];

    let line_starts: Vec<usize> = std::iter::once(0)
        .chain(source.bytes().enumerate().filter_map(|(i, b)| {
            if b == b'\n' { Some(i + 1) } else { None }
        }))
        .collect();

    for event in highlights {
        match event {
            Ok(HighlightEvent::HighlightStart(s)) => {
                current_highlight = Some(s.0);
            }
            Ok(HighlightEvent::HighlightEnd) => {
                current_highlight = None;
            }
            Ok(HighlightEvent::Source { start, end }) => {
                if let Some(hl) = current_highlight {
                    if hl < 21 {
                        // Find which lines this span covers
                        let start_line = line_starts.partition_point(|&ls| ls <= start).saturating_sub(1);
                        let end_line = line_starts.partition_point(|&ls| ls <= end).saturating_sub(1);
                        let span_len = (end - start) as u32;
                        for line in start_line..=end_line.min(line_count - 1) {
                            line_type_counts[line][hl] += span_len;
                        }
                    }
                }
                byte_offset = end;
            }
            Err(_) => break,
        }
    }

    // For each line, pick the dominant highlight type (most bytes)
    for (line_idx, counts) in line_type_counts.iter().enumerate() {
        let mut best_type = -1i32;
        let mut best_count = 0u32;
        for (type_idx, &count) in counts.iter().enumerate() {
            if count > best_count {
                best_count = count;
                best_type = type_idx as i32;
            }
        }
        // Only assign if the highlight covers a meaningful portion of the line
        if best_count > 0 {
            line_highlights[line_idx] = best_type;
        }
    }

    let _ = byte_offset; // suppress unused warning
    line_highlights
}

fn get_highlight_config(ext: &str) -> Option<HighlightConfiguration> {
    let (language, highlights_query, injections_query, locals_query) = match ext {
        "rs" => (
            tree_sitter_rust::LANGUAGE.into(),
            tree_sitter_rust::HIGHLIGHTS_QUERY,
            "",
            "",
        ),
        "js" | "mjs" | "cjs" | "jsx" => (
            tree_sitter_javascript::LANGUAGE.into(),
            tree_sitter_javascript::HIGHLIGHT_QUERY,
            tree_sitter_javascript::INJECTIONS_QUERY,
            tree_sitter_javascript::LOCALS_QUERY,
        ),
        "py" | "pyw" => (
            tree_sitter_python::LANGUAGE.into(),
            tree_sitter_python::HIGHLIGHTS_QUERY,
            "",
            "",
        ),
        "json" => (
            tree_sitter_json::LANGUAGE.into(),
            tree_sitter_json::HIGHLIGHTS_QUERY,
            "",
            "",
        ),
        "c" | "h" => (
            tree_sitter_c::LANGUAGE.into(),
            tree_sitter_c::HIGHLIGHT_QUERY,
            "",
            "",
        ),
        _ => return None,
    };

    let mut config = HighlightConfiguration::new(
        language,
        ext,
        highlights_query,
        injections_query,
        locals_query,
    )
    .ok()?;

    config.configure(HIGHLIGHT_NAMES);
    Some(config)
}

/// Detect file type name from extension
pub fn detect_file_type(file_path: &str) -> &'static str {
    let ext = Path::new(file_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    match ext {
        "rs" => "Rust",
        "js" | "mjs" | "cjs" => "JavaScript",
        "jsx" => "JSX",
        "ts" => "TypeScript",
        "tsx" => "TSX",
        "py" | "pyw" => "Python",
        "json" => "JSON",
        "c" => "C",
        "h" => "C Header",
        "cpp" | "cc" | "cxx" => "C++",
        "hpp" | "hxx" => "C++ Header",
        "java" => "Java",
        "rb" => "Ruby",
        "go" => "Go",
        "swift" => "Swift",
        "kt" | "kts" => "Kotlin",
        "html" | "htm" => "HTML",
        "css" => "CSS",
        "xml" => "XML",
        "yaml" | "yml" => "YAML",
        "toml" => "TOML",
        "md" | "markdown" => "Markdown",
        "sh" | "bash" => "Shell",
        "sql" => "SQL",
        "txt" => "Text",
        _ => "Plain Text",
    }
}
