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
    /// Number of pixels that differ beyond tolerance (including out-of-bounds regions)
    pub diff_pixels: u64,
    /// Number of pixels whose max per-channel difference is within tolerance (0 < diff <= tolerance)
    pub tolerated_pixels: u64,
    /// Total pixels in the diff canvas (max_w × max_h)
    pub total_pixels: u64,
    pub diff_width: u32,
    pub diff_height: u32,
    /// Diff image: differing pixels → red, within-tolerance pixels → yellow, identical pixels → dimmed grayscale
    pub diff_rgba: Vec<u8>,
    /// Overlay image: differing pixels → red (alpha=255), identical/within-tolerance pixels → transparent (alpha=0)
    pub overlay_rgba: Vec<u8>,
}

/// Decode both images and compute a pixel-level diff.
/// `tolerance` is the maximum per-channel (R, G, B, A) absolute difference still
/// treated as a match; 0 reproduces exact-match comparison.
/// Returns Err if either image fails to decode.
pub fn compare_images(
    left_data: &[u8],
    right_data: &[u8],
    tolerance: u8,
) -> Result<ImageCompareResult, String> {
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
    let mut tolerated_pixels = 0u64;

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
            let max_diff = (0..4).map(|c| lp[c].abs_diff(rp[c])).max().unwrap_or(0);

            if !in_left || !in_right || max_diff > tolerance {
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
            } else if max_diff == 0 {
                // Identical pixel: render as dimmed grayscale using standard Rec.601 weights
                let gray =
                    ((lp[0] as u32 * 77 + lp[1] as u32 * 150 + lp[2] as u32 * 29) >> 8) as u8;
                let dim = (gray as u32 * 3 / 4 + 48) as u8;
                diff_rgba[idx] = dim;
                diff_rgba[idx + 1] = dim;
                diff_rgba[idx + 2] = dim;
                diff_rgba[idx + 3] = 255;
            } else {
                // Within tolerance: treated as a match (overlay stays transparent), shown as yellow
                diff_rgba[idx] = 255;
                diff_rgba[idx + 1] = 200;
                diff_rgba[idx + 2] = 0;
                diff_rgba[idx + 3] = 255;
                tolerated_pixels += 1;
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
        tolerated_pixels,
        total_pixels: total,
        diff_width: dw,
        diff_height: dh,
        diff_rgba,
        overlay_rgba,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Encode a single solid-color RGBA image as an in-memory PNG.
    fn png(w: u32, h: u32, px: [u8; 4]) -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(w, h, image::Rgba(px));
        let mut buf = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .unwrap();
        buf
    }

    /// Compares two 1x1 images with the given pixel colors.
    fn compare_1x1(left: [u8; 4], right: [u8; 4], tolerance: u8) -> ImageCompareResult {
        let left_data = png(1, 1, left);
        let right_data = png(1, 1, right);
        compare_images(&left_data, &right_data, tolerance).unwrap()
    }

    #[test]
    fn zero_tolerance_one_step_diff_is_changed() {
        let result = compare_1x1([10, 10, 10, 255], [11, 10, 10, 255], 0);
        assert_eq!(result.diff_pixels, 1);
        assert_eq!(result.tolerated_pixels, 0);
    }

    #[test]
    fn diff_within_tolerance_is_tolerated_not_changed() {
        let result = compare_1x1([10, 10, 10, 255], [15, 10, 10, 255], 5);
        assert_eq!(result.diff_pixels, 0);
        assert_eq!(result.tolerated_pixels, 1);
        assert_eq!(&result.diff_rgba[0..4], &[255, 200, 0, 255]);
        assert_eq!(result.overlay_rgba[3], 0);
    }

    #[test]
    fn diff_above_tolerance_is_changed() {
        let result = compare_1x1([10, 10, 10, 255], [20, 10, 10, 255], 5);
        assert_eq!(result.diff_pixels, 1);
        assert_eq!(result.tolerated_pixels, 0);
    }

    #[test]
    fn alpha_only_diff_above_tolerance_is_changed() {
        let result = compare_1x1([10, 10, 10, 200], [10, 10, 10, 220], 10);
        assert_eq!(result.diff_pixels, 1);
        assert_eq!(result.tolerated_pixels, 0);
    }

    #[test]
    fn out_of_bounds_region_is_changed_even_at_max_tolerance() {
        let left_data = png(1, 1, [10, 10, 10, 255]);
        let right_data = png(2, 1, [10, 10, 10, 255]);
        let result = compare_images(&left_data, &right_data, 255).unwrap();
        assert_eq!(result.total_pixels, 2);
        assert_eq!(result.diff_pixels, 1);
        assert_eq!(result.tolerated_pixels, 0);
    }
}
