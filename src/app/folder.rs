use super::*;

// --- Folder comparison ---

pub fn run_folder_compare(window: &MainWindow, state: &mut AppState) {
    let tab = state.current_tab();
    let (left_folder, right_folder) = match (&tab.left_folder, &tab.right_folder) {
        (Some(l), Some(r)) => (l.clone(), r.clone()),
        _ => return,
    };

    let folder_max_depth = window.get_opt_folder_max_depth().max(0) as usize;
    let options = FolderCompareOptions {
        exclude_patterns: state.folder_exclude_patterns.clone(),
        max_depth: folder_max_depth,
        min_size: window.get_opt_folder_min_size() as u64,
        max_size: window.get_opt_folder_max_size() as u64,
        modified_after: window.get_opt_folder_modified_after().to_string(),
        modified_before: window.get_opt_folder_modified_before().to_string(),
        ..Default::default()
    };
    let items = compare_folders_with_options(&left_folder, &right_folder, &options);
    let (folder_item_data, summary) = build_folder_item_data(&items);

    let left_name = path_file_name(&left_folder);
    let right_name = path_file_name(&right_folder);

    let tab = state.current_tab_mut();
    tab.folder_items = items;
    tab.folder_item_data = folder_item_data.clone();
    tab.view_mode = ViewMode::FolderCompare;
    tab.title = format!("{} ↔ {}", left_name, right_name);
    tab.folder_summary = summary.clone();

    window.set_folder_items(ModelRc::new(VecModel::from(folder_item_data)));
    window.set_view_mode(ViewMode::FolderCompare.as_i32());
    window.set_folder_summary_text(SharedString::from(summary.clone()));
    window.set_status_text(SharedString::from(format!(
        "Folder: {} ↔ {} — {}",
        left_name, right_name, summary
    )));

    sync_tab_list(window, state);
}

/// Build FolderItemData from FolderItem list and compute summary.
pub(super) fn build_folder_item_data(
    items: &[crate::models::folder_item::FolderItem],
) -> (Vec<FolderItemData>, String) {
    let folder_item_data: Vec<FolderItemData> = items
        .iter()
        .map(|item| {
            let status: i32 = item.status.as_i32();
            let depth = item
                .relative_path
                .chars()
                .filter(|&c| c == '/' || c == '\\')
                .count() as i32;
            FolderItemData {
                relative_path: SharedString::from(&item.relative_path),
                is_directory: item.is_directory,
                status,
                left_size: item
                    .left_size
                    .map(|s| SharedString::from(format_size(s)))
                    .unwrap_or_default(),
                right_size: item
                    .right_size
                    .map(|s| SharedString::from(format_size(s)))
                    .unwrap_or_default(),
                left_modified: item
                    .left_modified
                    .as_ref()
                    .map(|s| SharedString::from(s.as_str()))
                    .unwrap_or_default(),
                right_modified: item
                    .right_modified
                    .as_ref()
                    .map(|s| SharedString::from(s.as_str()))
                    .unwrap_or_default(),
                depth,
            }
        })
        .collect();

    let identical = items
        .iter()
        .filter(|i| i.status == FileCompareStatus::Identical)
        .count();
    let different = items
        .iter()
        .filter(|i| i.status == FileCompareStatus::Different)
        .count();
    let left_only = items
        .iter()
        .filter(|i| i.status == FileCompareStatus::LeftOnly)
        .count();
    let right_only = items
        .iter()
        .filter(|i| i.status == FileCompareStatus::RightOnly)
        .count();
    let total = items.len();
    let summary = format!(
        "Identical: {} | Different: {} | Left only: {} | Right only: {} | Total: {}",
        identical, different, left_only, right_only, total
    );

    (folder_item_data, summary)
}

/// Display a virtual folder comparison built from IPC-received file pairs.
pub fn display_virtual_folder(
    window: &MainWindow,
    state: &mut AppState,
    items: Vec<crate::models::folder_item::FolderItem>,
) {
    let (folder_item_data, summary) = build_folder_item_data(&items);
    let count = items.len();

    let tab = state.current_tab_mut();
    tab.folder_items = items;
    tab.folder_item_data = folder_item_data.clone();
    tab.view_mode = ViewMode::FolderCompare;
    tab.is_virtual_folder = true;
    tab.title = format!("git diff ({} files)", count);
    tab.folder_summary = summary.clone();

    window.set_folder_items(ModelRc::new(VecModel::from(folder_item_data)));
    window.set_view_mode(ViewMode::FolderCompare.as_i32());
    window.set_folder_summary_text(SharedString::from(&summary));
    window.set_status_text(SharedString::from(format!("git diff — {}", summary)));
    window.set_has_folder_context(false);

    sync_tab_list(window, state);
}

pub fn open_folder_item(window: &MainWindow, state: &mut AppState, index: i32) {
    let tab = state.current_tab();
    if index < 0 || index as usize >= tab.folder_items.len() {
        return;
    }

    let item = &tab.folder_items[index as usize];
    if item.is_directory {
        return;
    }

    if let (Some(left), Some(right)) = (&item.left_path, &item.right_path) {
        let left = left.clone();
        let right = right.clone();
        let is_virtual = tab.is_virtual_folder;

        if is_virtual {
            // Virtual folder: open diff in a new tab (folder tab stays intact)
            add_tab(window, state);
        }

        {
            let tab = state.current_tab_mut();
            tab.left_path = Some(left);
            tab.right_path = Some(right);
            tab.view_mode = ViewMode::FileDiff;
        }
        window.set_view_mode(ViewMode::FileDiff.as_i32());
        if !is_virtual {
            window.set_has_folder_context(true);
        }
        run_diff(window, state);
    }
}

// --- Folder file operations ---

pub fn folder_copy_to_right(window: &MainWindow, state: &mut AppState, index: i32) {
    let tab = state.current_tab();
    if index < 0 || index as usize >= tab.folder_items.len() {
        return;
    }
    let item = &tab.folder_items[index as usize];
    if let (Some(src), Some(right_folder)) = (&item.left_path, &tab.right_folder) {
        let dest = right_folder.join(&item.relative_path);
        if let Some(parent) = dest.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if item.is_directory {
            copy_dir_recursive(src, &dest);
        } else {
            let _ = fs::copy(src, &dest);
        }
        window.set_status_text(SharedString::from(format!(
            "Copied '{}' to right",
            item.relative_path
        )));
        run_folder_compare(window, state);
    }
}

pub fn folder_copy_to_left(window: &MainWindow, state: &mut AppState, index: i32) {
    let tab = state.current_tab();
    if index < 0 || index as usize >= tab.folder_items.len() {
        return;
    }
    let item = &tab.folder_items[index as usize];
    if let (Some(src), Some(left_folder)) = (&item.right_path, &tab.left_folder) {
        let dest = left_folder.join(&item.relative_path);
        if let Some(parent) = dest.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if item.is_directory {
            copy_dir_recursive(src, &dest);
        } else {
            let _ = fs::copy(src, &dest);
        }
        window.set_status_text(SharedString::from(format!(
            "Copied '{}' to left",
            item.relative_path
        )));
        run_folder_compare(window, state);
    }
}

pub fn folder_delete_item(window: &MainWindow, state: &mut AppState, index: i32) {
    let tab = state.current_tab();
    if index < 0 || index as usize >= tab.folder_items.len() {
        return;
    }
    let item = &tab.folder_items[index as usize];
    if let Some(left) = &item.left_path {
        if left.exists() {
            if item.is_directory {
                let _ = fs::remove_dir_all(left);
            } else {
                let _ = fs::remove_file(left);
            }
        }
    }
    if let Some(right) = &item.right_path {
        if right.exists() {
            if item.is_directory {
                let _ = fs::remove_dir_all(right);
            } else {
                let _ = fs::remove_file(right);
            }
        }
    }
    window.set_status_text(SharedString::from(format!(
        "Deleted '{}'",
        item.relative_path
    )));
    run_folder_compare(window, state);
}

fn copy_dir_recursive(src: &std::path::Path, dest: &std::path::Path) {
    let _ = fs::create_dir_all(dest);
    if let Ok(entries) = fs::read_dir(src) {
        for entry in entries.flatten() {
            let src_path = entry.path();
            let dest_path = dest.join(entry.file_name());
            if src_path.is_dir() {
                copy_dir_recursive(&src_path, &dest_path);
            } else {
                let _ = fs::copy(&src_path, &dest_path);
            }
        }
    }
}

pub(super) fn run_zip_compare(
    window: &MainWindow,
    state: &mut AppState,
    left_bytes: &[u8],
    right_bytes: &[u8],
    left_path: &std::path::Path,
    right_path: &std::path::Path,
) {
    let left_str = left_path.to_string_lossy();
    let right_str = right_path.to_string_lossy();
    let items = compare_zip_archives(left_bytes, right_bytes, &left_str, &right_str);
    let (folder_item_data, summary) = build_folder_item_data(&items);

    let left_name = path_file_name(left_path);
    let right_name = path_file_name(right_path);

    let tab = state.current_tab_mut();
    tab.folder_items = items;
    tab.folder_item_data = folder_item_data.clone();
    tab.view_mode = ViewMode::FolderCompare;
    tab.title = format!("{} ↔ {}", left_name, right_name);
    tab.folder_summary = summary.clone();

    let model = ModelRc::new(VecModel::from(folder_item_data));
    window.set_folder_items(model);
    window.set_view_mode(ViewMode::FolderCompare.as_i32());
    window.set_folder_summary_text(SharedString::from(&summary));
    window.set_status_text(SharedString::from(format!(
        "[ZIP] {} ↔ {} — {}",
        left_name, right_name, summary
    )));
    window.set_left_path(SharedString::from(left_path.to_string_lossy().to_string()));
    window.set_right_path(SharedString::from(right_path.to_string_lossy().to_string()));
    sync_tab_list(window, state);
}

fn rgba_to_slint_image(rgba: &[u8], width: u32, height: u32) -> slint::Image {
    let mut pixel_buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(width, height);
    pixel_buffer.make_mut_bytes().copy_from_slice(rgba);
    slint::Image::from_rgba8(pixel_buffer)
}

pub(super) fn run_image_compare(
    window: &MainWindow,
    state: &mut AppState,
    left_bytes: &[u8],
    right_bytes: &[u8],
    left_path: &std::path::Path,
    right_path: &std::path::Path,
) {
    let left_name = path_file_name(left_path);
    let right_name = path_file_name(right_path);

    match compare_images(left_bytes, right_bytes) {
        Err(e) => {
            window.set_status_text(SharedString::from(format!("Image error: {e}")));
            sync_tab_list(window, state);
        }
        Ok(result) => {
            let diff_pct = if result.total_pixels > 0 {
                result.diff_pixels as f64 / result.total_pixels as f64 * 100.0
            } else {
                0.0
            };
            let stats = format!(
                "Left: {}×{}  Right: {}×{}  Changed: {} / {} px ({:.2}%)",
                result.left_width,
                result.left_height,
                result.right_width,
                result.right_height,
                result.diff_pixels,
                result.total_pixels,
                diff_pct,
            );

            let left_img =
                rgba_to_slint_image(&result.left_rgba, result.left_width, result.left_height);
            let right_img =
                rgba_to_slint_image(&result.right_rgba, result.right_width, result.right_height);
            let diff_img =
                rgba_to_slint_image(&result.diff_rgba, result.diff_width, result.diff_height);
            let overlay_img =
                rgba_to_slint_image(&result.overlay_rgba, result.diff_width, result.diff_height);

            let tab = state.current_tab_mut();
            tab.view_mode = ViewMode::ImageCompare;
            tab.title = format!("{} ↔ {}", left_name, right_name);
            tab.image_stats = stats.clone();
            tab.left_image = Some(left_img.clone());
            tab.right_image = Some(right_img.clone());
            tab.diff_image = Some(diff_img.clone());
            tab.overlay_image = Some(overlay_img.clone());
            tab.image_left_w = result.left_width as i32;
            tab.image_left_h = result.left_height as i32;
            tab.image_right_w = result.right_width as i32;
            tab.image_right_h = result.right_height as i32;

            window.set_view_mode(ViewMode::ImageCompare.as_i32());
            window.set_left_image(left_img);
            window.set_right_image(right_img);
            window.set_diff_image(diff_img);
            window.set_overlay_image(overlay_img);
            window.set_image_stats(SharedString::from(stats.clone()));
            window.set_image_left_width(result.left_width as i32);
            window.set_image_left_height(result.left_height as i32);
            window.set_image_right_width(result.right_width as i32);
            window.set_image_right_height(result.right_height as i32);
            window.set_left_path(SharedString::from(left_path.to_string_lossy().to_string()));
            window.set_right_path(SharedString::from(right_path.to_string_lossy().to_string()));
            window.set_status_text(SharedString::from(format!(
                "[Image] {} ↔ {} — {}",
                left_name, right_name, stats
            )));
            sync_tab_list(window, state);
        }
    }
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

// --- Feature: Folder sort ---

pub fn sort_folder(window: &MainWindow, state: &mut AppState, column: i32) {
    let tab = state.current_tab_mut();
    if tab.view_mode != ViewMode::FolderCompare || tab.folder_items.is_empty() {
        return;
    }

    // Toggle direction if same column, else reset to ascending
    if tab.folder_sort_column == column {
        tab.folder_sort_ascending = !tab.folder_sort_ascending;
    } else {
        tab.folder_sort_column = column;
        tab.folder_sort_ascending = true;
    }

    let ascending = tab.folder_sort_ascending;

    tab.folder_items.sort_by(|a, b| {
        let ord = match column {
            0 => a
                .relative_path
                .to_lowercase()
                .cmp(&b.relative_path.to_lowercase()),
            1 => a.status.as_i32().cmp(&b.status.as_i32()),
            2 => a.left_size.unwrap_or(0).cmp(&b.left_size.unwrap_or(0)),
            3 => a.right_size.unwrap_or(0).cmp(&b.right_size.unwrap_or(0)),
            4 => a
                .left_modified
                .as_deref()
                .unwrap_or("")
                .cmp(b.left_modified.as_deref().unwrap_or("")),
            5 => a
                .right_modified
                .as_deref()
                .unwrap_or("")
                .cmp(b.right_modified.as_deref().unwrap_or("")),
            _ => std::cmp::Ordering::Equal,
        };
        if ascending { ord } else { ord.reverse() }
    });

    let (data, _) = build_folder_item_data(&tab.folder_items);
    tab.folder_item_data = data.clone();
    let sort_col = tab.folder_sort_column;
    let sort_asc = tab.folder_sort_ascending;

    window.set_folder_items(ModelRc::new(VecModel::from(data)));
    window.set_folder_selected_index(-1);
    window.set_folder_sort_column(sort_col);
    window.set_folder_sort_ascending(sort_asc);
}

// --- Feature: Folder item preview ---

pub fn preview_folder_item(window: &MainWindow, state: &AppState, idx: i32) {
    let tab = state.current_tab();
    if idx < 0 || idx as usize >= tab.folder_items.len() {
        return;
    }
    let item = &tab.folder_items[idx as usize];
    let name = item
        .relative_path
        .split('/')
        .last()
        .unwrap_or(&item.relative_path)
        .to_string();

    if item.is_directory {
        window.set_folder_preview_name(SharedString::from(name));
        window.set_folder_preview_left(SharedString::from("(directory)"));
        window.set_folder_preview_right(SharedString::from("(directory)"));
        return;
    }

    let (left_folder, right_folder) = match (&tab.left_folder, &tab.right_folder) {
        (Some(l), Some(r)) => (l.clone(), r.clone()),
        _ => return,
    };

    let left_path = left_folder.join(&item.relative_path);
    let right_path = right_folder.join(&item.relative_path);

    window.set_folder_preview_name(SharedString::from(name));
    window.set_folder_preview_left(SharedString::from(load_text_preview(&left_path)));
    window.set_folder_preview_right(SharedString::from(load_text_preview(&right_path)));
}

fn load_text_preview(path: &std::path::Path) -> String {
    if !path.exists() {
        return "(file not found)".to_string();
    }
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) => return format!("(read error: {e})"),
    };
    if is_binary(&bytes) {
        return format!("[binary  {} bytes]", bytes.len());
    }
    let (text, _) = decode_file(&bytes);
    text.lines().take(20).collect::<Vec<_>>().join("\n")
}
