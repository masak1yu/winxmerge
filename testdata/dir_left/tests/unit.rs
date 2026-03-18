use myapp::utils::parser::{parse_query_string, percent_decode};
use myapp::utils::truncate;

#[test]
fn truncate_short_string_unchanged() {
    assert_eq!(truncate("hello", 10), "hello");
}

#[test]
fn truncate_long_string_appends_ellipsis() {
    let result = truncate("hello world", 5);
    assert!(result.ends_with('…'));
    assert_eq!(&result[..5], "hello");
}

#[test]
fn percent_decode_plus_as_space() {
    assert_eq!(percent_decode("hello+world"), "hello world");
}

#[test]
fn percent_decode_hex_sequence() {
    assert_eq!(percent_decode("%41%42%43"), "ABC");
}

#[test]
fn parse_query_string_duplicate_key_last_wins() {
    let m = parse_query_string("x=1&x=2");
    assert_eq!(m["x"], "2");
}
