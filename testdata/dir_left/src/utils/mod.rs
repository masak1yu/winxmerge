pub mod parser;

pub use parser::parse_query_string;

/// Truncate a string to at most `max_chars` Unicode scalar values.
/// Appends `"…"` if truncation occurred.
pub fn truncate(s: &str, max_chars: usize) -> String {
    let mut chars = s.chars();
    let collected: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{collected}…")
    } else {
        collected
    }
}
