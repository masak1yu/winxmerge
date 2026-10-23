use chardetng::EncodingDetector;
use encoding_rs::Encoding;

/// Return true if the byte slice looks like binary (contains null bytes in first 8KB).
pub fn is_binary(data: &[u8]) -> bool {
    let check = &data[..data.len().min(8192)];
    check.contains(&0u8)
}

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

/// Detect the dominant line ending type from raw file bytes.
pub fn detect_eol(bytes: &[u8]) -> &'static str {
    // F12: not `contains(&b'\n')` for LF — every CRLF has a \n too, so CRLF-only files read as Mixed.
    let truncated = bytes.len() > 65536;
    let check = &bytes[..bytes.len().min(65536)];
    let (mut crlf, mut cr, mut lf) = (false, false, false);
    let mut i = 0;
    while i < check.len() {
        if check[i] == b'\r' {
            if check.get(i + 1) == Some(&b'\n') {
                crlf = true;
                i += 2;
                continue;
            }
            // Not counted when cut off at the scan limit: it may be half of a CRLF.
            if i + 1 < check.len() || !truncated {
                cr = true;
            }
        } else if check[i] == b'\n' {
            lf = true;
        }
        i += 1;
    }
    match (crlf, cr, lf) {
        (false, false, false) => "",
        (true, false, false) => "CRLF",
        (false, true, false) => "CR",
        (false, false, true) => "LF",
        _ => "Mixed",
    }
}

/// 保存用に `\n` 区切りの本文を元ファイルの改行コードへ戻す (#75)。
/// `eol` は `detect_eol` の戻り値。"CRLF"/"CR" のみ変換し、LF/Mixed/空は LF のまま。
pub fn apply_eol(text: &str, eol: &str) -> String {
    match eol {
        "CRLF" => text.replace('\n', "\r\n"),
        "CR" => text.replace('\n', "\r"),
        _ => text.to_string(),
    }
}

/// 既存ファイルの改行コードを検出する。読めなければ空文字 (= LF 扱い)。
pub fn detect_eol_of_file(path: &std::path::Path) -> &'static str {
    std::fs::read(path).map(|b| detect_eol(&b)).unwrap_or("")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_eol_converts_only_crlf_and_cr() {
        // What: CRLF/CR は \n を置換し、LF/Mixed/空はそのまま。
        assert_eq!(apply_eol("a\nb\n", "CRLF"), "a\r\nb\r\n");
        assert_eq!(apply_eol("a\nb\n", "CR"), "a\rb\r");
        assert_eq!(apply_eol("a\nb\n", "LF"), "a\nb\n");
        assert_eq!(apply_eol("a\nb\n", "Mixed"), "a\nb\n");
        assert_eq!(apply_eol("a\nb\n", ""), "a\nb\n");
        // 往復: CRLF バイト列 -> 行分割 -> 結合 -> 戻すと元のバイト列になる
        let orig = "x\r\ny\r\n";
        let joined: String = orig.lines().map(|l| format!("{}\n", l)).collect();
        assert_eq!(apply_eol(&joined, detect_eol(orig.as_bytes())), orig);
    }

    #[test]
    fn detect_eol_classifies_lf_cr_crlf_mixed_empty_and_boundary_truncation() {
        // What: detect_eol tells LF/CR/CRLF/Mixed/"" apart, and a \r cut off at
        // the 64KiB scan limit changes nothing (F12).
        assert_eq!(detect_eol(b"a\nb\n"), "LF");
        assert_eq!(detect_eol(b"a\r\nb\r\n"), "CRLF");
        assert_eq!(detect_eol(b"a\rb\r"), "CR");
        assert_eq!(detect_eol(b"a\r\nb\n"), "Mixed");
        assert_eq!(detect_eol(b""), "");
        assert_eq!(detect_eol(b"abc"), "");

        // CRLF file whose last \r\n straddles the limit.
        let mut crlf_cut = b"a\r\n".to_vec();
        crlf_cut.resize(65535, b'a');
        crlf_cut.extend_from_slice(b"\r\n");
        assert_eq!(detect_eol(&crlf_cut), "CRLF");

        // CR file whose last scanned byte is a \r.
        let mut cr_cut = b"a\r".to_vec();
        cr_cut.resize(65535, b'a');
        cr_cut.extend_from_slice(b"\rb");
        assert_eq!(detect_eol(&cr_cut), "CR");
    }
}
