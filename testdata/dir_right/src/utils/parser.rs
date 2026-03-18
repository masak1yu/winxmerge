use std::collections::HashMap;

/// Parse a URL query string (without the leading `?`) into a key-value map.
///
/// Values are percent-decoded. Duplicate keys keep the last value.
///
/// # Examples
///
/// ```
/// let map = parse_query_string("foo=bar&baz=42");
/// assert_eq!(map["foo"], "bar");
/// assert_eq!(map["baz"], "42");
/// ```
pub fn parse_query_string(query: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();

    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (key, value) = match pair.split_once('=') {
            Some((k, v)) => (k, v),
            None => (pair, ""),
        };
        let key = percent_decode(key);
        let value = percent_decode(value);
        map.insert(key, value);
    }

    map
}

/// Minimal percent-decoding: replaces `%XX` sequences with the corresponding byte.
/// Non-UTF-8 sequences are replaced with the Unicode replacement character.
pub fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (
                hex_val(bytes[i + 1]),
                hex_val(bytes[i + 2]),
            ) {
                output.push((hi << 4) | lo);
                i += 3;
                continue;
            }
        }
        if bytes[i] == b'+' {
            output.push(b' ');
        } else {
            output.push(bytes[i]);
        }
        i += 1;
    }

    String::from_utf8(output).unwrap_or_else(|e| {
        String::from_utf8_lossy(e.as_bytes()).into_owned()
    })
}

/// Percent-encode a string for safe inclusion in a URL query component.
pub fn percent_encode(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9'
            | b'-' | b'_' | b'.' | b'~' => output.push(byte as char),
            b' ' => output.push('+'),
            _ => output.push_str(&format!("%{byte:02X}")),
        }
    }
    output
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_pairs() {
        let m = parse_query_string("a=1&b=2");
        assert_eq!(m["a"], "1");
        assert_eq!(m["b"], "2");
    }

    #[test]
    fn decodes_percent_encoded() {
        let m = parse_query_string("name=hello%20world");
        assert_eq!(m["name"], "hello world");
    }

    #[test]
    fn handles_empty_value() {
        let m = parse_query_string("flag=");
        assert_eq!(m["flag"], "");
    }

    #[test]
    fn encode_decode_roundtrip() {
        let original = "hello world & more=stuff";
        let encoded = percent_encode(original);
        let decoded = percent_decode(&encoded);
        assert_eq!(decoded, original);
    }
}
