//! ui/**/*.slint の @tr("...") がすべて ja の po に msgid として存在することを確認する。
//! 無いと日本語 UI に原文が混ざる (CLAUDE.md Gotcha 7)。

use std::fs;
use std::path::Path;

fn collect_slint(dir: &Path, out: &mut Vec<String>) {
    for e in fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            collect_slint(&p, out);
        } else if p.extension().is_some_and(|x| x == "slint") {
            out.push(fs::read_to_string(&p).unwrap());
        }
    }
}

/// `@tr("...")` のリテラルを、エスケープを保ったまま取り出す。
/// po の msgid も同じエスケープ表記なので、非エスケープ化せず文字列比較できる。
fn tr_literals(src: &str) -> Vec<String> {
    let mut res = Vec::new();
    let mut rest = src;
    while let Some(i) = rest.find("@tr(\"") {
        rest = &rest[i + 5..];
        let b = rest.as_bytes();
        let mut j = 0;
        while j < b.len() && b[j] != b'"' {
            j += if b[j] == b'\\' { 2 } else { 1 };
        }
        res.push(rest[..j].to_string());
        rest = &rest[j..];
    }
    res
}

#[test]
fn all_tr_strings_have_ja_msgid() {
    let po = fs::read_to_string("translations/ja/LC_MESSAGES/winxmerge.po").unwrap();
    // 行頭 `msgid "..."` のみ対象 (コメントアウトされた行は除外)
    let msgids: Vec<&str> = po
        .lines()
        .filter_map(|l| l.strip_prefix("msgid \"")?.strip_suffix('"'))
        .collect();

    let mut srcs = Vec::new();
    collect_slint(Path::new("ui"), &mut srcs);
    let mut missing: Vec<String> = srcs
        .iter()
        .flat_map(|s| tr_literals(s))
        .filter(|t| !msgids.contains(&t.as_str()))
        .collect();
    missing.sort();
    missing.dedup();
    assert!(missing.is_empty(), "ja po に msgid が無い @tr: {missing:?}");
}
