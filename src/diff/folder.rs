use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::models::folder_item::{FileCompareStatus, FolderItem};

pub fn compare_folders(left_dir: &Path, right_dir: &Path) -> Vec<FolderItem> {
    let left_entries = collect_entries(left_dir, left_dir);
    let right_entries = collect_entries(right_dir, right_dir);

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
            },
            (None, Some(re)) => FolderItem {
                relative_path: rel_path.clone(),
                is_directory: re.is_dir,
                status: FileCompareStatus::RightOnly,
                left_path: None,
                right_path: Some(re.full_path.clone()),
                left_size: None,
                right_size: if re.is_dir { None } else { Some(re.size) },
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
}

fn collect_entries(dir: &Path, base: &Path) -> Vec<(String, EntryInfo)> {
    let mut entries = Vec::new();
    if let Ok(read_dir) = fs::read_dir(dir) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            let rel = path
                .strip_prefix(base)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();
            let metadata = entry.metadata();
            let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
            let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);

            entries.push((
                rel.clone(),
                EntryInfo {
                    full_path: path.clone(),
                    is_dir,
                    size,
                },
            ));

            if is_dir {
                entries.extend(collect_entries(&path, base));
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
