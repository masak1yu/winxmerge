use std::collections::BTreeMap;
use std::path::Path;

use crate::models::folder_item::{FileCompareStatus, FolderItem};

/// Returns true if the byte slice starts with the ZIP magic bytes PK\x03\x04
pub fn is_zip_bytes(data: &[u8]) -> bool {
    data.len() >= 4 && data[0] == 0x50 && data[1] == 0x4B && data[2] == 0x03 && data[3] == 0x04
}

/// Returns true if the path has a .zip extension
pub fn is_zip_path(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("zip"))
        .unwrap_or(false)
}

struct ZipEntry {
    name: String,
    size: u64,
    crc: u32,
}

fn read_zip_entries(data: &[u8]) -> Vec<ZipEntry> {
    let cursor = std::io::Cursor::new(data);
    let mut archive = match zip::ZipArchive::new(cursor) {
        Ok(a) => a,
        Err(_) => return Vec::new(),
    };
    let mut entries = Vec::new();
    for i in 0..archive.len() {
        if let Ok(file) = archive.by_index(i) {
            entries.push(ZipEntry {
                name: file.name().to_string(),
                size: file.size(),
                crc: file.crc32(),
            });
        }
    }
    entries
}

/// Compare two ZIP archive byte slices and return FolderItem list for display in FolderView.
pub fn compare_zip_archives(
    left_data: &[u8],
    right_data: &[u8],
    left_path_str: &str,
    right_path_str: &str,
) -> Vec<FolderItem> {
    let left_entries = read_zip_entries(left_data);
    let right_entries = read_zip_entries(right_data);

    let mut left_map: BTreeMap<String, &ZipEntry> = BTreeMap::new();
    for e in &left_entries {
        left_map.insert(e.name.clone(), e);
    }
    let mut right_map: BTreeMap<String, &ZipEntry> = BTreeMap::new();
    for e in &right_entries {
        right_map.insert(e.name.clone(), e);
    }

    let mut all_names: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for e in &left_entries {
        all_names.insert(e.name.clone());
    }
    for e in &right_entries {
        all_names.insert(e.name.clone());
    }

    let mut items = Vec::new();
    for name in &all_names {
        let is_dir = name.ends_with('/');
        let left = left_map.get(name);
        let right = right_map.get(name);

        let status = match (left, right) {
            (Some(l), Some(r)) => {
                if is_dir {
                    FileCompareStatus::Identical
                } else if l.crc == r.crc && l.size == r.size {
                    FileCompareStatus::Identical
                } else {
                    FileCompareStatus::Different
                }
            }
            (Some(_), None) => FileCompareStatus::LeftOnly,
            (None, Some(_)) => FileCompareStatus::RightOnly,
            (None, None) => unreachable!(),
        };

        // Compute depth from slashes
        let depth = name.chars().filter(|&c| c == '/').count() as i32;
        let _depth = if is_dir && depth > 0 {
            depth - 1
        } else {
            depth
        };

        let left_size_str = left
            .filter(|_| !is_dir)
            .map(|e| format_size(e.size))
            .unwrap_or_default();
        let right_size_str = right
            .filter(|_| !is_dir)
            .map(|e| format_size(e.size))
            .unwrap_or_default();

        let left_full =
            left.map(|_| std::path::PathBuf::from(format!("{}::{}", left_path_str, name)));
        let right_full =
            right.map(|_| std::path::PathBuf::from(format!("{}::{}", right_path_str, name)));

        items.push(FolderItem {
            relative_path: name.trim_end_matches('/').to_string(),
            is_directory: is_dir,
            status,
            left_path: left_full,
            right_path: right_full,
            left_size: left.filter(|_| !is_dir).map(|e| e.size),
            right_size: right.filter(|_| !is_dir).map(|e| e.size),
            left_modified: left.map(|_| left_size_str.clone()),
            right_modified: right.map(|_| right_size_str.clone()),
        });
    }
    items
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
