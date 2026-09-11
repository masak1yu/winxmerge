---
title: Image Comparison
description: Pixel-level image diff with overlay, blend slider, and tolerance.
---

WinXMerge supports pixel-level comparison for image files.

## Supported Formats

PNG, JPEG, GIF, BMP, WebP, TIFF, ICO

## Three-Panel View

- **Left panel**: Original image
- **Right panel**: Modified image
- **Diff overlay**: Changed pixels shown in red, within-tolerance pixels in yellow, identical pixels in gray

## Controls

### Zoom

Continuous zoom slider from **10% to 400%**, plus a **Fit** mode that scales to the window size.

### Blend Slider

The blend slider (0–100%) overlays changed-pixel highlights directly on the left/right panels at adjustable opacity, making it easy to see exactly what changed in context.

### Diff Panel Toggle

Show or hide the diff overlay panel to focus on individual images.

### Tolerance

The tolerance slider (0–255) sets how large a pixel difference is still treated as a match. It is compared against the largest per-channel difference between the two pixels, including alpha. At 0 only exact matches count; raise it to ignore small color shifts such as those introduced by JPEG re-encoding. The comparison recomputes when the slider is released, not while dragging. The tolerance is per-tab and is not saved between sessions.
