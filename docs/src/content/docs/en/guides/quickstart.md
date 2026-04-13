---
title: Quick Start
description: Get up and running with WinXMerge in minutes.
---

## Launch WinXMerge

```bash
# Launch with no arguments — opens the file selection dialog
cargo run --features desktop

# 2-way file comparison
cargo run --features desktop -- file1.txt file2.txt

# 3-way merge
cargo run --features desktop -- base.txt left.txt right.txt
```

## Basic Workflow

### 1. Open Files

When you launch WinXMerge, the **file selection dialog** appears. Enter the paths for the left and right files (or folders), then click **Compare**.

- Check **3-way merge** to specify a base file for three-way comparison
- Select from **recent files** for quick access

### 2. Navigate Diffs

Use the toolbar buttons or keyboard shortcuts to navigate between diff blocks:

| Action | Shortcut |
|--------|----------|
| Next diff | Alt+↓ |
| Previous diff | Alt+↑ |
| First diff | Alt+Home |
| Last diff | Alt+End |

### 3. Merge Changes

- Click **Copy →** or **← Copy** in the toolbar to copy the current diff block
- Use inline **▶** / **◀** buttons between diff lines for per-block merging
- **Copy & Advance** copies the current block and moves to the next one

### 4. Save

Press **Cmd+S** (macOS) or **Ctrl+S** to save the merged file.

### 5. Tabs

Use **Cmd+T** to open a new tab for a separate comparison. Each tab maintains its own independent state.

## Folder Comparison

Select two folders in the open dialog to start a recursive directory comparison. Double-click any file in the list to open a detailed diff view.

## Next Steps

- [File Comparison](/en/features/file-comparison/) — detailed guide on 2-way diff
- [3-Way Merge](/en/features/three-way-merge/) — conflict resolution workflow
- [Git Integration](/en/integrations/git/) — use as `git difftool` / `git mergetool`
