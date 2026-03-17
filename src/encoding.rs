use chardetng::EncodingDetector;
use encoding_rs::Encoding;

/// Detect encoding of raw bytes and decode to String.
/// Returns (decoded_text, encoding_name).
pub fn decode_file(bytes: &[u8]) -> (String, &'static str) {
    // Check for BOM first
    if let Some((text, encoding)) = try_bom(bytes) {
        return (text, encoding);
    }

    // Use chardetng for detection
    let mut detector = EncodingDetector::new();
    detector.feed(bytes, true);
    let encoding = detector.guess(None, true);

    let (cow, _, had_errors) = encoding.decode(bytes);
    let name = encoding.name();

    if had_errors {
        // Fallback to lossy UTF-8
        (String::from_utf8_lossy(bytes).into_owned(), "UTF-8 (lossy)")
    } else {
        (cow.into_owned(), name)
    }
}

fn try_bom(bytes: &[u8]) -> Option<(String, &'static str)> {
    // UTF-8 BOM
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        let text = String::from_utf8_lossy(&bytes[3..]).into_owned();
        return Some((text, "UTF-8 (BOM)"));
    }
    // UTF-16 LE BOM
    if bytes.starts_with(&[0xFF, 0xFE]) {
        let (cow, _, _) = encoding_rs::UTF_16LE.decode(bytes);
        return Some((cow.into_owned(), "UTF-16 LE"));
    }
    // UTF-16 BE BOM
    if bytes.starts_with(&[0xFE, 0xFF]) {
        let (cow, _, _) = encoding_rs::UTF_16BE.decode(bytes);
        return Some((cow.into_owned(), "UTF-16 BE"));
    }
    None
}

/// Encode text back to the specified encoding.
pub fn encode_text(text: &str, encoding_name: &str) -> Vec<u8> {
    if encoding_name.starts_with("UTF-8") {
        if encoding_name.contains("BOM") {
            let mut bytes = vec![0xEF, 0xBB, 0xBF];
            bytes.extend_from_slice(text.as_bytes());
            bytes
        } else {
            text.as_bytes().to_vec()
        }
    } else if let Some(encoding) = Encoding::for_label(encoding_name.as_bytes()) {
        let (cow, _, _) = encoding.encode(text);
        cow.into_owned()
    } else {
        text.as_bytes().to_vec()
    }
}
