---
title: File Comparison (2-Way)
description: Compare two files side by side with line-level and word-level diff.
---

WinXMerge provides a powerful 2-way file comparison view inspired by WinMerge.

## Diff Display

- **Block-level grouping**: Consecutive changes are merged into a single diff block (WinMerge-style)
- **Line-level diff**: Additions, deletions, changes, and moves are displayed with color coding
  - Green: Added lines
  - Red: Removed lines
  - Yellow: Modified lines
  - Blue: Moved lines
- **Word-level (character-level) diff**: Modified lines show exactly which characters changed with inline highlighting

## Diff Navigation

Navigate between diff blocks using the toolbar or keyboard:

| Action | Shortcut |
|--------|----------|
| First diff | Alt+Home |
| Previous diff | Alt+↑ |
| Next diff | Alt+↓ |
| Last diff | Alt+End |

Click a line number in either pane to jump to that diff block.

## Merge Operations

- **Copy Left → Right**: Copy the current diff block from the left pane to the right
- **Copy Right → Left**: Copy from the right to the left
- **Copy & Advance**: Copy and automatically move to the next diff block
- **Copy All**: Copy all diff blocks in one direction
- **Inline buttons**: Click ▶ / ◀ between diff lines for per-block merging

## Two-Pane Layout

The side-by-side display shows both files with synchronized scrolling. A **location pane** (minimap) on the right shows an overview of all diff positions in the file.

## Moved Line Detection

Lines that have been moved (not just added/removed) are automatically detected and highlighted in blue, making it easy to distinguish structural changes from content changes.

## Diff Detail Pane

The bottom pane shows the currently selected diff block in detail:
- Left panel: removed/modified lines
- Right panel: added/modified lines
- Character-level background highlighting on changed segments

The pane height is resizable by dragging the top handle.

## Diff Comments

Add per-block notes in the detail pane's **Note** field. Comments are:
- Persisted to the session
- Included in HTML and Excel export reports
- Exportable as CSV/JSON via **File → Export All Comments**
