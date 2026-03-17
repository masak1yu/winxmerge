use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::time::SystemTime;

use crate::models::folder_item::{FileCompareStatus, FolderItem};

/// Options for folder comparison
#[derive(Debug, Clone, Default)]
pub struct FolderCompareOptions {
    /// File extension filter (e.g., ["rs", "toml"]). Empty = all files.
    pub extension_filter: Vec<String>,
    /// Whether to respect .gitignore files
    pub respect_gitignore: bool,
    /// Exclude patterns (e.g., ["*.log", "build/", "node_modules"])
    pub exclude_patterns: Vec<String>,
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

    let left_entries = collect_entries(left_dir, left_dir, &gitignore_patterns, options);
    let right_entries = collect_entries(right_dir, right_dir, &gitignore_patterns, options);

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
                    compare_file_contents(&le.full_path, &re.full_path)
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
}

fn collect_entries(
    dir: &Path,
    base: &Path,
    gitignore_patterns: &[String],
    options: &FolderCompareOptions,
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
            let modified = metadata
                .as_ref()
                .ok()
                .and_then(|m| m.modified().ok())
                .map(format_time);

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

            entries.push((
                rel.clone(),
                EntryInfo {
                    full_path: path.clone(),
                    is_dir,
                    size,
                    modified,
                },
            ));

            if is_dir {
                entries.extend(collect_entries(&path, base, gitignore_patterns, options));
            }
        }
    }
    entries
}

fn compare_file_contents(left: &Path, right: &Path) -> FileCompareStatus {
    let left_content = fs::read(left);
    let right_content = fs::read(right);
    match (left_content, right_content) {
        (Ok(l), Ok(r)) => {
            if l == r {
                FileCompareStatus::Identical
            } else {
                FileCompareStatus::Different
            }
        }
        _ => FileCompareStatus::Different,
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
