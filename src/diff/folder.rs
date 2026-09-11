use std::collections::BTreeSet;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::time::SystemTime;

use crate::models::folder_item::{FileCompareStatus, FolderItem};

/// Folder-compare method, selectable via the Options dialog or the `/m` CLI
/// option. Declaration order matches the Options dialog ComboBox index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompareMethod {
    #[default]
    Full,
    Quick,
    Binary,
    Date,
    SizeDate,
    Size,
    Existence,
}

impl CompareMethod {
    /// Parses a `/m` CLI value (WinMerge-style keyword, case-insensitive).
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "full" => Some(Self::Full),
            "quick" => Some(Self::Quick),
            "binary" => Some(Self::Binary),
            "date" => Some(Self::Date),
            "sizedate" => Some(Self::SizeDate),
            "size" => Some(Self::Size),
            "existence" => Some(Self::Existence),
            _ => None,
        }
    }

    /// Maps an Options dialog ComboBox index to a method; out-of-range falls back to Full.
    pub fn from_index(index: i32) -> Self {
        match index {
            0 => Self::Full,
            1 => Self::Quick,
            2 => Self::Binary,
            3 => Self::Date,
            4 => Self::SizeDate,
            5 => Self::Size,
            6 => Self::Existence,
            _ => Self::Full,
        }
    }
}

/// Options for folder comparison
#[derive(Debug, Clone, Default)]
pub struct FolderCompareOptions {
    /// File extension filter (e.g., ["rs", "toml"]). Empty = all files.
    pub extension_filter: Vec<String>,
    /// Whether to respect .gitignore files
    pub respect_gitignore: bool,
    /// Exclude patterns (e.g., ["*.log", "build/", "node_modules"])
    pub exclude_patterns: Vec<String>,
    /// Maximum recursion depth (0 = unlimited)
    pub max_depth: usize,
    /// Minimum file size in bytes (0 = no limit)
    pub min_size: u64,
    /// Maximum file size in bytes (0 = no limit)
    pub max_size: u64,
    /// Only include files modified after this date (format "YYYY-MM-DD", empty = no filter)
    pub modified_after: String,
    /// Only include files modified before this date (format "YYYY-MM-DD", empty = no filter)
    pub modified_before: String,
    /// How to decide whether two files are identical
    pub compare_method: CompareMethod,
}

pub fn compare_folders_with_options(
    left_dir: &Path,
    right_dir: &Path,
    options: &FolderCompareOptions,
) -> Vec<FolderItem> {
    let gitignore_patterns = if options.respect_gitignore {
        load_gitignore_patterns(left_dir)
            .into_iter()
            .chain(load_gitignore_patterns(right_dir))
            .collect()
    } else {
        Vec::new()
    };

    let left_entries = collect_entries(left_dir, left_dir, &gitignore_patterns, options, 0);
    let right_entries = collect_entries(right_dir, right_dir, &gitignore_patterns, options, 0);

    let all_paths: BTreeSet<&String> = left_entries
        .iter()
        .chain(right_entries.iter())
        .map(|(path, _)| path)
        .collect();

    let mut items = Vec::new();

    for rel_path in all_paths {
        let left_entry = left_entries
            .iter()
            .find(|(p, _)| p == rel_path)
            .map(|(_, e)| e);
        let right_entry = right_entries
            .iter()
            .find(|(p, _)| p == rel_path)
            .map(|(_, e)| e);

        let item = match (left_entry, right_entry) {
            (Some(le), Some(re)) => {
                let status = if le.is_dir && re.is_dir {
                    FileCompareStatus::Identical
                } else if le.is_dir || re.is_dir {
                    FileCompareStatus::Different
                } else {
                    compare_entries(le, re, options.compare_method)
                };
                FolderItem {
                    relative_path: rel_path.clone(),
                    is_directory: le.is_dir && re.is_dir,
                    status,
                    left_path: Some(le.full_path.clone()),
                    right_path: Some(re.full_path.clone()),
                    left_size: if le.is_dir { None } else { Some(le.size) },
                    right_size: if re.is_dir { None } else { Some(re.size) },
                    left_modified: le.modified.clone(),
                    right_modified: re.modified.clone(),
                }
            }
            (Some(le), None) => FolderItem {
                relative_path: rel_path.clone(),
                is_directory: le.is_dir,
                status: FileCompareStatus::LeftOnly,
                left_path: Some(le.full_path.clone()),
                right_path: None,
                left_size: if le.is_dir { None } else { Some(le.size) },
                right_size: None,
                left_modified: le.modified.clone(),
                right_modified: None,
            },
            (None, Some(re)) => FolderItem {
                relative_path: rel_path.clone(),
                is_directory: re.is_dir,
                status: FileCompareStatus::RightOnly,
                left_path: None,
                right_path: Some(re.full_path.clone()),
                left_size: None,
                right_size: if re.is_dir { None } else { Some(re.size) },
                left_modified: None,
                right_modified: re.modified.clone(),
            },
            (None, None) => unreachable!(),
        };
        items.push(item);
    }

    items
}

struct EntryInfo {
    full_path: std::path::PathBuf,
    is_dir: bool,
    size: u64,
    modified: Option<String>,
    mtime: Option<SystemTime>,
}

fn collect_entries(
    dir: &Path,
    base: &Path,
    gitignore_patterns: &[String],
    options: &FolderCompareOptions,
    depth: usize,
) -> Vec<(String, EntryInfo)> {
    let mut entries = Vec::new();
    if let Ok(read_dir) = fs::read_dir(dir) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            let rel = path
                .strip_prefix(base)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();

            // Skip .git directory
            if path.file_name().map(|n| n == ".git").unwrap_or(false) {
                continue;
            }

            // Check gitignore
            if should_ignore(&rel, gitignore_patterns) {
                continue;
            }

            // Check exclude patterns
            if should_ignore(&rel, &options.exclude_patterns) {
                continue;
            }

            let metadata = entry.metadata();
            let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
            let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
            let mtime = metadata.as_ref().ok().and_then(|m| m.modified().ok());
            let modified = mtime.map(format_time);

            // Extension filter (only for files)
            if !is_dir && !options.extension_filter.is_empty() {
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                if !options
                    .extension_filter
                    .iter()
                    .any(|f| f.to_lowercase() == ext)
                {
                    continue;
                }
            }

            // Size filter (only for files)
            if !is_dir {
                if options.min_size > 0 && size < options.min_size {
                    continue;
                }
                if options.max_size > 0 && size > options.max_size {
                    continue;
                }
            }
            // Date filter
            if !is_dir && modified.is_some() {
                let mod_str = modified.as_deref().unwrap_or("");
                let date_part = &mod_str[..mod_str.len().min(10)]; // "YYYY-MM-DD"
                if !options.modified_after.is_empty() && date_part < options.modified_after.as_str()
                {
                    continue;
                }
                if !options.modified_before.is_empty()
                    && date_part > options.modified_before.as_str()
                {
                    continue;
                }
            }

            entries.push((
                rel.clone(),
                EntryInfo {
                    full_path: path.clone(),
                    is_dir,
                    size,
                    modified,
                    mtime,
                },
            ));

            if is_dir {
                let next_depth = depth + 1;
                let at_limit = options.max_depth > 0 && next_depth >= options.max_depth;
                if !at_limit {
                    entries.extend(collect_entries(
                        &path,
                        base,
                        gitignore_patterns,
                        options,
                        next_depth,
                    ));
                }
            }
        }
    }
    entries
}

/// Compares two files byte-for-byte.
// Reads fixed-size chunks instead of fs::read so large files are never held in memory whole.
pub fn compare_file_contents(left: &Path, right: &Path) -> FileCompareStatus {
    const CHUNK_SIZE: usize = 64 * 1024;

    let (left_len, right_len) = match (fs::metadata(left), fs::metadata(right)) {
        (Ok(l), Ok(r)) => (l.len(), r.len()),
        _ => return FileCompareStatus::Different,
    };
    if left_len != right_len {
        return FileCompareStatus::Different;
    }

    let (mut left_file, mut right_file) = match (fs::File::open(left), fs::File::open(right)) {
        (Ok(l), Ok(r)) => (l, r),
        _ => return FileCompareStatus::Different,
    };

    let mut left_buf = vec![0u8; CHUNK_SIZE];
    let mut right_buf = vec![0u8; CHUNK_SIZE];
    let mut remaining = left_len;
    while remaining > 0 {
        let take = remaining.min(CHUNK_SIZE as u64) as usize;
        if left_file.read_exact(&mut left_buf[..take]).is_err()
            || right_file.read_exact(&mut right_buf[..take]).is_err()
        {
            return FileCompareStatus::Different;
        }
        if left_buf[..take] != right_buf[..take] {
            return FileCompareStatus::Different;
        }
        remaining -= take as u64;
    }
    FileCompareStatus::Identical
}

/// Decides whether two (non-directory) entries are identical under `method`.
fn compare_entries(
    left: &EntryInfo,
    right: &EntryInfo,
    method: CompareMethod,
) -> FileCompareStatus {
    match method {
        // ponytail: this implementation's Full doesn't normalize text (whitespace,
        // case, ...) before comparing, so Full/Quick/Binary all reduce to the same
        // byte-for-byte comparison here. Split Full out once it applies DiffOptions.
        CompareMethod::Full | CompareMethod::Quick | CompareMethod::Binary => {
            compare_file_contents(&left.full_path, &right.full_path)
        }
        CompareMethod::Date => match (left.mtime, right.mtime) {
            (Some(l), Some(r)) if l == r => FileCompareStatus::Identical,
            _ => FileCompareStatus::Different,
        },
        CompareMethod::SizeDate => match (left.mtime, right.mtime) {
            (Some(l), Some(r)) if l == r && left.size == right.size => FileCompareStatus::Identical,
            _ => FileCompareStatus::Different,
        },
        CompareMethod::Size => {
            if left.size == right.size {
                FileCompareStatus::Identical
            } else {
                FileCompareStatus::Different
            }
        }
        CompareMethod::Existence => FileCompareStatus::Identical,
    }
}

fn load_gitignore_patterns(dir: &Path) -> Vec<String> {
    let gitignore_path = dir.join(".gitignore");
    match fs::read_to_string(&gitignore_path) {
        Ok(content) => content
            .lines()
            .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
            .map(|l| l.trim().to_string())
            .collect(),
        Err(_) => Vec::new(),
    }
}

fn should_ignore(rel_path: &str, patterns: &[String]) -> bool {
    let filename = rel_path.rsplit('/').next().unwrap_or(rel_path);
    for pattern in patterns {
        let pat = pattern.trim_start_matches('/');
        if pat.is_empty() {
            continue;
        }
        // Simple glob matching: exact match, prefix match, or extension match
        if rel_path == pat || rel_path.starts_with(&format!("{}/", pat)) {
            return true;
        }
        // *.ext pattern — match against filename
        if let Some(ext) = pat.strip_prefix("*.") {
            if filename.ends_with(&format!(".{}", ext)) {
                return true;
            }
        }
        // dir/ pattern
        if let Some(dir) = pat.strip_suffix('/') {
            if rel_path == dir || rel_path.starts_with(&format!("{}/", dir)) || filename == dir {
                return true;
            }
        }
        // Exact filename match
        if filename == pat {
            return true;
        }
    }
    false
}

fn format_time(time: SystemTime) -> String {
    let duration = time
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    // Simple UTC format: YYYY-MM-DD HH:MM
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;

    // Approximate date calculation
    let mut year = 1970u64;
    let mut remaining_days = days;
    loop {
        let days_in_year = if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
            366
        } else {
            365
        };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let month_days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1u64;
    for &md in &month_days {
        if remaining_days < md {
            break;
        }
        remaining_days -= md;
        month += 1;
    }
    let day = remaining_days + 1;

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        year, month, day, hours, minutes
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::Duration;

    // Unique per-test dir: `cargo test` runs tests in parallel, so a shared
    // fixed path would race between tests.
    fn unique_temp_dir(label: &str) -> std::path::PathBuf {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "winxmerge_folder_test_{}_{}_{}",
            std::process::id(),
            label,
            n
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn entry_info(path: &Path) -> EntryInfo {
        let metadata = fs::metadata(path).unwrap();
        EntryInfo {
            full_path: path.to_path_buf(),
            is_dir: false,
            size: metadata.len(),
            modified: None,
            mtime: metadata.modified().ok(),
        }
    }

    #[test]
    fn same_size_different_content_is_different_except_size_and_existence() {
        let dir = unique_temp_dir("same_size_diff_content");
        let left = dir.join("left.txt");
        let right = dir.join("right.txt");
        fs::write(&left, b"AAAA").unwrap();
        fs::write(&right, b"BBBB").unwrap();
        let le = entry_info(&left);
        let re = entry_info(&right);

        assert_eq!(
            compare_entries(&le, &re, CompareMethod::Full),
            FileCompareStatus::Different
        );
        assert_eq!(
            compare_entries(&le, &re, CompareMethod::Quick),
            FileCompareStatus::Different
        );
        assert_eq!(
            compare_entries(&le, &re, CompareMethod::Binary),
            FileCompareStatus::Different
        );
        assert_eq!(
            compare_entries(&le, &re, CompareMethod::Size),
            FileCompareStatus::Identical
        );
        assert_eq!(
            compare_entries(&le, &re, CompareMethod::Existence),
            FileCompareStatus::Identical
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn same_content_different_mtime_is_different_for_date_methods_only() {
        let dir = unique_temp_dir("same_content_diff_mtime");
        let left = dir.join("left.txt");
        let right = dir.join("right.txt");
        fs::write(&left, b"identical content").unwrap();
        fs::write(&right, b"identical content").unwrap();
        fs::File::options()
            .write(true)
            .open(&left)
            .unwrap()
            .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(1_000))
            .unwrap();
        fs::File::options()
            .write(true)
            .open(&right)
            .unwrap()
            .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(2_000))
            .unwrap();
        let le = entry_info(&left);
        let re = entry_info(&right);

        assert_eq!(
            compare_entries(&le, &re, CompareMethod::Date),
            FileCompareStatus::Different
        );
        assert_eq!(
            compare_entries(&le, &re, CompareMethod::SizeDate),
            FileCompareStatus::Different
        );
        assert_eq!(
            compare_entries(&le, &re, CompareMethod::Full),
            FileCompareStatus::Identical
        );
        assert_eq!(
            compare_entries(&le, &re, CompareMethod::Size),
            FileCompareStatus::Identical
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn same_mtime_different_size_splits_date_from_sizedate() {
        let dir = unique_temp_dir("same_mtime_diff_size");
        let left = dir.join("left.txt");
        let right = dir.join("right.txt");
        fs::write(&left, b"a").unwrap();
        fs::write(&right, b"ab").unwrap();
        let same_time = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
        fs::File::options()
            .write(true)
            .open(&left)
            .unwrap()
            .set_modified(same_time)
            .unwrap();
        fs::File::options()
            .write(true)
            .open(&right)
            .unwrap()
            .set_modified(same_time)
            .unwrap();
        let le = entry_info(&left);
        let re = entry_info(&right);

        assert_eq!(
            compare_entries(&le, &re, CompareMethod::Date),
            FileCompareStatus::Identical
        );
        assert_eq!(
            compare_entries(&le, &re, CompareMethod::SizeDate),
            FileCompareStatus::Different
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn full_compares_past_the_first_64kb_chunk() {
        let dir = unique_temp_dir("chunked_tail_diff");
        let left = dir.join("left.bin");
        let right = dir.join("right.bin");
        // Larger than one 64KB chunk so a truncated comparison would miss the
        // difference at the very end of the file.
        let mut left_bytes = vec![0u8; 70_000];
        let mut right_bytes = vec![0u8; 70_000];
        left_bytes[69_999] = 1;
        right_bytes[69_999] = 2;
        fs::write(&left, &left_bytes).unwrap();
        fs::write(&right, &right_bytes).unwrap();

        assert_eq!(
            compare_file_contents(&left, &right),
            FileCompareStatus::Different
        );

        let _ = fs::remove_dir_all(&dir);
    }
}
