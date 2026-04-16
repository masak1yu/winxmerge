use std::path::Path;

/// Return true if the path looks like a supported image file.
pub fn is_image_path(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .as_deref(),
        Some("png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "tiff" | "tif" | "ico")
    )
}

/// Result of a pixel-level image comparison.
pub struct ImageCompareResult {
    pub left_width: u32,
    pub left_height: u32,
    /// Raw RGBA8 pixel data for the left image
    pub left_rgba: Vec<u8>,
    pub right_width: u32,
    pub right_height: u32,
    /// Raw RGBA8 pixel data for the right image
    pub right_rgba: Vec<u8>,
    /// Number of pixels that differ (including out-of-bounds regions)
    pub diff_pixels: u64,
    /// Total pixels in the diff canvas (max_w × max_h)
    pub total_pixels: u64,
    pub diff_width: u32,
    pub diff_height: u32,
    /// Diff image: differing pixels → red, identical pixels → dimmed grayscale
    pub diff_rgba: Vec<u8>,
    /// Overlay image: differing pixels → red (alpha=255), identical pixels → transparent (alpha=0)
    pub overlay_rgba: Vec<u8>,
}

/// Decode both images and compute a pixel-level diff.
/// Returns Err if either image fails to decode.
pub fn compare_images(left_data: &[u8], right_data: &[u8]) -> Result<ImageCompareResult, String> {
    let left_img = image::load_from_memory(left_data)
        .map_err(|e| format!("Left: {e}"))?
        .into_rgba8();
    let right_img = image::load_from_memory(right_data)
        .map_err(|e| format!("Right: {e}"))?
        .into_rgba8();

    let lw = left_img.width();
    let lh = left_img.height();
    let rw = right_img.width();
    let rh = right_img.height();

    // Diff canvas spans the maximum dimensions of both images
    let dw = lw.max(rw);
    let dh = lh.max(rh);
    let total = (dw as u64) * (dh as u64);

    let buf_size = (dw as usize) * (dh as usize) * 4;
    let mut diff_rgba = vec![255u8; buf_size];
    let mut overlay_rgba = vec![0u8; buf_size];
    let mut diff_pixels = 0u64;

    for y in 0..dh {
        for x in 0..dw {
            let in_left = x < lw && y < lh;
            let in_right = x < rw && y < rh;
            let lp = if in_left {
                *left_img.get_pixel(x, y)
            } else {
                image::Rgba([0u8, 0, 0, 0])
            };
            let rp = if in_right {
                *right_img.get_pixel(x, y)
            } else {
                image::Rgba([0u8, 0, 0, 0])
            };

            let idx = ((y as usize) * (dw as usize) + (x as usize)) * 4;
            if !in_left || !in_right || lp != rp {
                // Highlight differences in red
                diff_rgba[idx] = 255;
                diff_rgba[idx + 1] = 0;
                diff_rgba[idx + 2] = 0;
                diff_rgba[idx + 3] = 255;
                // Overlay: red with full alpha
                overlay_rgba[idx] = 220;
                overlay_rgba[idx + 1] = 30;
                overlay_rgba[idx + 2] = 30;
                overlay_rgba[idx + 3] = 255;
                if in_left || in_right {
                    diff_pixels += 1;
                }
            } else {
                // Identical pixel: render as dimmed grayscale using standard Rec.601 weights
                let gray =
                    ((lp[0] as u32 * 77 + lp[1] as u32 * 150 + lp[2] as u32 * 29) >> 8) as u8;
                let dim = (gray as u32 * 3 / 4 + 48) as u8;
                diff_rgba[idx] = dim;
                diff_rgba[idx + 1] = dim;
                diff_rgba[idx + 2] = dim;
                diff_rgba[idx + 3] = 255;
            }
        }
    }

    Ok(ImageCompareResult {
        left_width: lw,
        left_height: lh,
        left_rgba: left_img.into_raw(),
        right_width: rw,
        right_height: rh,
        right_rgba: right_img.into_raw(),
        diff_pixels,
        total_pixels: total,
        diff_width: dw,
        diff_height: dh,
        diff_rgba,
        overlay_rgba,
    })
}
