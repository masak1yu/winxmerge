use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum FileCompareStatus {
    Identical,
    Different,
    LeftOnly,
    RightOnly,
}

#[derive(Debug, Clone)]
pub struct FolderItem {
    pub relative_path: String,
    pub is_directory: bool,
    pub status: FileCompareStatus,
    pub left_path: Option<PathBuf>,
    pub right_path: Option<PathBuf>,
    pub left_size: Option<u64>,
    pub right_size: Option<u64>,
    pub left_modified: Option<String>,
    pub right_modified: Option<String>,
}
