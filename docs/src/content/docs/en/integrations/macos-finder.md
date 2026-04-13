---
title: macOS Finder Integration
description: Compare files directly from Finder's right-click menu.
---

WinXMerge includes a **Finder Sync Extension** that adds right-click compare actions to macOS Finder.

## Setup

1. Build the `.app` bundle with the Finder extension:
   ```bash
   ./scripts/build-macos-bundle.sh
   ```
2. Move `WinXMerge.app` to `/Applications`
3. Enable the extension in **System Settings → General → Login Items & Extensions**

## Usage

### Compare Two Files

1. Select **2 files** in Finder
2. Right-click → **Compare with WinXMerge**
3. Opens a 2-way diff view

### Compare Three Files

1. Select **3 files** in Finder
2. Right-click → **Compare with WinXMerge**
3. Opens a 3-way merge view

### Mark and Compare

1. Right-click a file → **Mark for Compare**
2. Navigate to another file
3. Right-click → **Compare with [marked file]**

## Integration with Running Instance

If WinXMerge is already running, the Finder extension adds a **new tab** in the existing window instead of launching a new instance.
