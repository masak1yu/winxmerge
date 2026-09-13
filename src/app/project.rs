use std::ops::RangeInclusive;
use std::path::Path;

use roxmltree::Document;

use super::*;

/// One `<paths>` entry from a WinMerge-compatible project file.
#[derive(Debug, Default, PartialEq)]
pub struct ProjectEntry {
    pub left: Option<PathBuf>,
    pub middle: Option<PathBuf>,
    pub right: Option<PathBuf>,
    pub white_spaces: Option<i32>,
    pub ignore_case: Option<bool>,
    pub ignore_blank_lines: Option<bool>,
    pub ignore_eol: Option<bool>,
    pub subfolders: Option<bool>,
    pub compare_method: Option<i32>,
}

/// Parses a WinMerge project XML into one entry per `<paths>` element.
///
/// Elements winxmerge has no equivalent for (`filter`, `*-desc`, `*-readonly`,
/// `window-type`, `table-*`, `hidden-list`, other `ignore-*`, plugins, ...)
/// are silently skipped rather than rejected — WinMerge project files
/// routinely carry them, and failing to open over an unsupported option would
/// surprise users more than ignoring it.
pub fn parse_project(xml: &str, base_dir: &Path) -> Result<Vec<ProjectEntry>, String> {
    let xml = xml.strip_prefix('\u{FEFF}').unwrap_or(xml);
    let doc = Document::parse(xml).map_err(|e| e.to_string())?;
    let root = doc.root_element();
    if root.tag_name().name() != "project" {
        return Err(format!(
            "expected <project> as the root element, found <{}>",
            root.tag_name().name()
        ));
    }

    Ok(root
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "paths")
        .map(|paths| parse_paths_entry(paths, base_dir))
        .collect())
}

fn parse_paths_entry(paths: roxmltree::Node, base_dir: &Path) -> ProjectEntry {
    let mut entry = ProjectEntry::default();
    for child in paths.children().filter(|n| n.is_element()) {
        match child.tag_name().name() {
            "left" => entry.left = parse_path(child, base_dir),
            "middle" => entry.middle = parse_path(child, base_dir),
            "right" => entry.right = parse_path(child, base_dir),
            "white-spaces" => entry.white_spaces = parse_ranged_int(child, 0..=2),
            "ignore-case" => entry.ignore_case = Some(parse_int(child) != 0),
            "ignore-blank-lines" => entry.ignore_blank_lines = Some(parse_int(child) != 0),
            "ignore-carriage-return-diff" => entry.ignore_eol = Some(parse_int(child) != 0),
            "subfolders" => entry.subfolders = Some(parse_int(child) != 0),
            "compare-method" => entry.compare_method = parse_ranged_int(child, 0..=6),
            _ => {}
        }
    }
    entry
}

fn parse_path(node: roxmltree::Node, base_dir: &Path) -> Option<PathBuf> {
    let text = node.text().unwrap_or("").trim();
    if text.is_empty() {
        return None;
    }
    let p = PathBuf::from(text);
    if p.is_absolute() {
        Some(p)
    } else {
        Some(base_dir.join(p))
    }
}

/// WinMerge reads these fields with atoi, where non-numeric text parses as 0;
/// `str::parse` is stricter (e.g. rejects "1abc"), which project.rs accepts as
/// a simplification since real project files always write plain integers.
fn parse_int(node: roxmltree::Node) -> i32 {
    node.text().unwrap_or("").trim().parse::<i32>().unwrap_or(0)
}

fn parse_ranged_int(node: roxmltree::Node, range: RangeInclusive<i32>) -> Option<i32> {
    let v = parse_int(node);
    range.contains(&v).then_some(v)
}

/// Serializes one project entry into WinMerge-compatible project XML,
/// mirroring the field order ProjectFile.cpp writes: left, middle, right,
/// subfolders, white-spaces, ignore-blank-lines, ignore-case,
/// ignore-carriage-return-diff, compare-method. Only `Some` fields are written.
pub fn write_project(entry: &ProjectEntry) -> String {
    let mut out =
        String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<project>\n\t<paths>\n");
    if let Some(p) = &entry.left {
        push_path_element(&mut out, "left", p);
    }
    if let Some(p) = &entry.middle {
        push_path_element(&mut out, "middle", p);
    }
    if let Some(p) = &entry.right {
        push_path_element(&mut out, "right", p);
    }
    if let Some(v) = entry.subfolders {
        push_bool_element(&mut out, "subfolders", v);
    }
    if let Some(v) = entry.white_spaces {
        push_int_element(&mut out, "white-spaces", v);
    }
    if let Some(v) = entry.ignore_blank_lines {
        push_bool_element(&mut out, "ignore-blank-lines", v);
    }
    if let Some(v) = entry.ignore_case {
        push_bool_element(&mut out, "ignore-case", v);
    }
    if let Some(v) = entry.ignore_eol {
        push_bool_element(&mut out, "ignore-carriage-return-diff", v);
    }
    if let Some(v) = entry.compare_method {
        push_int_element(&mut out, "compare-method", v);
    }
    out.push_str("\t</paths>\n</project>\n");
    out
}

fn push_path_element(out: &mut String, tag: &str, path: &Path) {
    push_text_element(out, tag, &path.to_string_lossy());
}

fn push_text_element(out: &mut String, tag: &str, text: &str) {
    out.push_str(&format!("\t\t<{tag}>{}</{tag}>\n", escape_xml_text(text)));
}

fn push_bool_element(out: &mut String, tag: &str, value: bool) {
    out.push_str(&format!("\t\t<{tag}>{}</{tag}>\n", value as i32));
}

fn push_int_element(out: &mut String, tag: &str, value: i32) {
    out.push_str(&format!("\t\t<{tag}>{value}</{tag}>\n"));
}

/// Escapes the five XML predefined entities in element text content.
fn escape_xml_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

/// Opens every `<paths>` entry in the project file at `path`, one tab each.
pub fn open_project(window: &MainWindow, state: &mut AppState, path: &Path) {
    let xml = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            window.set_status_text(SharedString::from(format!(
                "Error reading project file {}: {}",
                path.display(),
                e
            )));
            return;
        }
    };

    let base_dir = path.parent().unwrap_or_else(|| Path::new(""));
    let entries = match parse_project(&xml, base_dir) {
        Ok(entries) => entries,
        Err(e) => {
            window.set_status_text(SharedString::from(format!(
                "Error parsing project file {}: {}",
                path.display(),
                e
            )));
            return;
        }
    };

    let mut opened = 0usize;
    let mut skipped_missing_path = 0usize;
    let mut skipped_3way_folder = 0usize;

    for entry in &entries {
        let (Some(left), Some(right)) = (&entry.left, &entry.right) else {
            skipped_missing_path += 1;
            continue;
        };
        if let Some(middle) = &entry.middle
            && (left.is_dir() || right.is_dir() || middle.is_dir())
        {
            skipped_3way_folder += 1;
            continue;
        }
        let is_folder = left.is_dir() && right.is_dir();

        let reuse_current_tab = opened == 0 && {
            let tab = state.current_tab();
            tab.view_mode == ViewMode::Blank && tab.left_path.is_none() && tab.right_path.is_none()
        };
        if !reuse_current_tab {
            add_tab(window, state);
        }

        apply_text_options(window, state, entry);
        if entry.middle.is_none() && is_folder {
            apply_folder_options(window, entry);
        }

        if let Some(middle) = &entry.middle {
            start_three_way_compare(
                window,
                state,
                &middle.to_string_lossy(),
                &left.to_string_lossy(),
                &right.to_string_lossy(),
            );
        } else {
            start_compare(
                window,
                state,
                &left.to_string_lossy(),
                &right.to_string_lossy(),
                is_folder,
                false,
            );
        }
        opened += 1;
    }

    let skipped = skipped_missing_path + skipped_3way_folder;
    if skipped > 0 || opened == 0 {
        window.set_status_text(SharedString::from(format!(
            "Project: opened {opened}, skipped {skipped} \
             (missing left/right path: {skipped_missing_path}, \
             3-way folder compare not supported: {skipped_3way_folder})"
        )));
    }
}

/// Applies the entry's text-compare options (Some values only) to the current
/// tab and the window toggles, mirroring the CLI override pattern in main.rs.
fn apply_text_options(window: &MainWindow, state: &mut AppState, entry: &ProjectEntry) {
    if let Some(white_spaces) = entry.white_spaces {
        // Matches /ignorews:N: 0=compare all, 1=ignore changes, 2=ignore all.
        let (ignore_whitespace, ignore_whitespace_all) = match white_spaces {
            0 => (false, false),
            1 => (true, false),
            _ => (true, true),
        };
        state.current_tab_mut().diff_options.ignore_whitespace = ignore_whitespace;
        state.current_tab_mut().diff_options.ignore_whitespace_all = ignore_whitespace_all;
        window.set_ignore_whitespace(ignore_whitespace);
        window.set_opt_ignore_whitespace_all(ignore_whitespace_all);
    }
    if let Some(v) = entry.ignore_case {
        state.current_tab_mut().diff_options.ignore_case = v;
        window.set_ignore_case(v);
    }
    if let Some(v) = entry.ignore_blank_lines {
        state.current_tab_mut().diff_options.ignore_blank_lines = v;
        window.set_opt_ignore_blank_lines(v);
    }
    if let Some(v) = entry.ignore_eol {
        state.current_tab_mut().diff_options.ignore_eol = v;
        window.set_opt_ignore_eol(v);
    }
}

/// Applies the entry's folder-compare options (Some values only) globally,
/// same as the Options dialog / `/m` CLI flag — winxmerge has no per-tab
/// folder-compare settings.
fn apply_folder_options(window: &MainWindow, entry: &ProjectEntry) {
    if let Some(subfolders) = entry.subfolders {
        if subfolders {
            if window.get_opt_folder_max_depth() == 1 {
                window.set_opt_folder_max_depth(0);
            }
        } else {
            window.set_opt_folder_max_depth(1);
        }
    }
    if let Some(m) = entry.compare_method {
        window.set_opt_folder_compare_method(m);
    }
}

/// Saves the current tab's paths and compare options as a WinMerge project
/// file, after prompting for a destination with a native save dialog.
pub fn save_project(window: &MainWindow, state: &mut AppState) {
    let tab = state.current_tab();
    let is_folder = tab.view_mode == ViewMode::FolderCompare;
    let (left, right, middle) = if is_folder {
        (tab.left_folder.clone(), tab.right_folder.clone(), None)
    } else {
        (
            tab.left_path.clone(),
            tab.right_path.clone(),
            tab.base_path.clone(),
        )
    };
    let (Some(left), Some(right)) = (left, right) else {
        window.set_status_text(SharedString::from("No paths to save in the current tab"));
        return;
    };

    let mut entry = ProjectEntry {
        left: Some(absolute_or_original(&left)),
        middle: middle.map(|m| absolute_or_original(&m)),
        right: Some(absolute_or_original(&right)),
        ..ProjectEntry::default()
    };

    // Read from the window toggles rather than tab.diff_options, which is
    // only synced back to the tab on tab switch (tab.rs:55-59) and can be
    // stale for the tab currently on screen.
    let ignore_whitespace = window.get_ignore_whitespace();
    let ignore_whitespace_all = window.get_opt_ignore_whitespace_all();
    entry.white_spaces = Some(if !ignore_whitespace {
        0
    } else if ignore_whitespace_all {
        2
    } else {
        1
    });
    entry.ignore_case = Some(window.get_ignore_case());
    entry.ignore_blank_lines = Some(window.get_opt_ignore_blank_lines());
    entry.ignore_eol = Some(window.get_opt_ignore_eol());

    if is_folder {
        entry.subfolders = Some(window.get_opt_folder_max_depth() != 1);
        let compare_method = window.get_opt_folder_compare_method();
        if (0..=6).contains(&compare_method) {
            entry.compare_method = Some(compare_method);
        }
    }

    let Some(path) = rfd::FileDialog::new()
        .set_title("Save Project")
        .set_file_name("project.WinMerge")
        .add_filter("WinMerge Project", &["WinMerge"])
        .save_file()
    else {
        return;
    };

    match fs::write(&path, write_project(&entry)) {
        Ok(()) => {
            window.set_status_text(SharedString::from(format!(
                "Saved project to {}",
                path.display()
            )));
        }
        Err(e) => {
            window.set_status_text(SharedString::from(format!(
                "Error saving project file {}: {}",
                path.display(),
                e
            )));
        }
    }
}

/// `std::path::absolute` fails only for unusual inputs (e.g. an empty path);
/// falling back to the original path keeps `save_project` infallible here.
fn absolute_or_original(path: &Path) -> PathBuf {
    std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> std::path::PathBuf {
        std::path::PathBuf::from("/base/dir")
    }

    #[test]
    fn reads_supported_elements_and_ignores_unknown_ones() {
        let xml = r#"<project>
            <paths>
                <left>left.txt</left>
                <middle>base.txt</middle>
                <right>right.txt</right>
                <subfolders>0</subfolders>
                <filter>*.log</filter>
                <left-readonly>0</left-readonly>
                <ignore-case>1</ignore-case>
                <ignore-blank-lines>1</ignore-blank-lines>
                <ignore-carriage-return-diff>1</ignore-carriage-return-diff>
            </paths>
        </project>"#;
        let entries = parse_project(xml, &base()).unwrap();
        assert_eq!(entries.len(), 1);
        let e = &entries[0];
        assert_eq!(e.left, Some(base().join("left.txt")));
        assert_eq!(e.middle, Some(base().join("base.txt")));
        assert_eq!(e.right, Some(base().join("right.txt")));
        assert_eq!(e.subfolders, Some(false));
        assert_eq!(e.ignore_case, Some(true));
        assert_eq!(e.ignore_blank_lines, Some(true));
        assert_eq!(e.ignore_eol, Some(true));
    }

    #[test]
    fn multiple_paths_elements_become_multiple_entries() {
        let xml = r#"<project>
            <paths><left>a1.txt</left><right>a2.txt</right></paths>
            <paths><left>b1.txt</left><right>b2.txt</right></paths>
        </project>"#;
        let entries = parse_project(xml, &base()).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].left, Some(base().join("a1.txt")));
        assert_eq!(entries[1].left, Some(base().join("b1.txt")));
    }

    #[test]
    fn relative_path_resolves_against_base_dir_absolute_path_kept() {
        let xml = r#"<project>
            <paths>
                <left>sub/rel.txt</left>
                <right>/abs/right.txt</right>
            </paths>
        </project>"#;
        let entries = parse_project(xml, &base()).unwrap();
        assert_eq!(entries[0].left, Some(base().join("sub/rel.txt")));
        assert_eq!(
            entries[0].right,
            Some(std::path::PathBuf::from("/abs/right.txt"))
        );
    }

    #[test]
    fn white_spaces_out_of_range_is_none_in_range_is_some() {
        for (value, expected) in [("0", Some(0)), ("1", Some(1)), ("2", Some(2)), ("3", None)] {
            let xml = format!(
                r#"<project><paths><white-spaces>{value}</white-spaces></paths></project>"#
            );
            let entries = parse_project(&xml, &base()).unwrap();
            assert_eq!(entries[0].white_spaces, expected, "white-spaces={value}");
        }
    }

    #[test]
    fn compare_method_out_of_range_is_none_in_range_is_some() {
        for (value, expected) in [("0", Some(0)), ("6", Some(6)), ("7", None)] {
            let xml = format!(
                r#"<project><paths><compare-method>{value}</compare-method></paths></project>"#
            );
            let entries = parse_project(&xml, &base()).unwrap();
            assert_eq!(
                entries[0].compare_method, expected,
                "compare-method={value}"
            );
        }
    }

    #[test]
    fn bom_prefixed_input_parses() {
        let xml = "\u{FEFF}<project><paths><left>a.txt</left></paths></project>";
        let entries = parse_project(xml, &base()).unwrap();
        assert_eq!(entries[0].left, Some(base().join("a.txt")));
    }

    #[test]
    fn malformed_xml_is_err() {
        let xml = "<project><paths></paths";
        assert!(parse_project(xml, &base()).is_err());
    }

    #[test]
    fn root_element_other_than_project_is_err() {
        let xml = "<not-a-project></not-a-project>";
        assert!(parse_project(xml, &base()).is_err());
    }

    #[test]
    fn write_then_parse_round_trip_keeps_paths_needing_escape_intact() {
        let entry = ProjectEntry {
            left: Some(std::path::PathBuf::from("/a & b/<left>.txt")),
            right: Some(std::path::PathBuf::from("/a \"b\" 'c'/right.txt")),
            ..ProjectEntry::default()
        };
        let xml = write_project(&entry);
        let parsed = parse_project(&xml, &base()).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].left, entry.left);
        assert_eq!(parsed[0].right, entry.right);
    }

    #[test]
    fn three_way_entry_writes_middle_two_file_entry_does_not() {
        let three_way = ProjectEntry {
            left: Some(std::path::PathBuf::from("/l.txt")),
            middle: Some(std::path::PathBuf::from("/m.txt")),
            right: Some(std::path::PathBuf::from("/r.txt")),
            ..ProjectEntry::default()
        };
        assert!(write_project(&three_way).contains("<middle>"));

        let two_file = ProjectEntry {
            left: Some(std::path::PathBuf::from("/l.txt")),
            right: Some(std::path::PathBuf::from("/r.txt")),
            ..ProjectEntry::default()
        };
        assert!(!write_project(&two_file).contains("<middle>"));
    }

    #[test]
    fn folder_entry_writes_subfolders_and_compare_method_file_entry_does_not() {
        let folder = ProjectEntry {
            left: Some(std::path::PathBuf::from("/left-dir")),
            right: Some(std::path::PathBuf::from("/right-dir")),
            subfolders: Some(true),
            compare_method: Some(2),
            ..ProjectEntry::default()
        };
        let xml = write_project(&folder);
        assert!(xml.contains("<subfolders>1</subfolders>"));
        assert!(xml.contains("<compare-method>2</compare-method>"));

        let file = ProjectEntry {
            left: Some(std::path::PathBuf::from("/l.txt")),
            right: Some(std::path::PathBuf::from("/r.txt")),
            ..ProjectEntry::default()
        };
        let xml = write_project(&file);
        assert!(!xml.contains("<subfolders>"));
        assert!(!xml.contains("<compare-method>"));
    }

    #[test]
    fn text_options_are_always_written_and_white_spaces_round_trips() {
        for (value, expected) in [(0, Some(0)), (1, Some(1)), (2, Some(2))] {
            let entry = ProjectEntry {
                left: Some(std::path::PathBuf::from("/l.txt")),
                right: Some(std::path::PathBuf::from("/r.txt")),
                white_spaces: Some(value),
                ignore_case: Some(true),
                ignore_blank_lines: Some(false),
                ignore_eol: Some(true),
                ..ProjectEntry::default()
            };
            let xml = write_project(&entry);
            assert!(xml.contains("<ignore-case>1</ignore-case>"));
            assert!(xml.contains("<ignore-blank-lines>0</ignore-blank-lines>"));
            assert!(xml.contains("<ignore-carriage-return-diff>1</ignore-carriage-return-diff>"));
            let parsed = parse_project(&xml, &base()).unwrap();
            assert_eq!(parsed[0].white_spaces, expected, "white_spaces={value}");
        }
    }
}
