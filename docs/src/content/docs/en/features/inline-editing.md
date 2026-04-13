---
title: Inline Editing
description: Edit text directly in diff panes without leaving the comparison view.
---

WinXMerge supports WinMerge-style inline editing — you can edit text directly in the diff panes.

## How It Works

Editing is always in **aligned Ghost-line view** (no separate edit mode). Changes are reflected immediately in the diff display.

## Key Operations

| Action | Key |
|--------|-----|
| Insert new line | Enter |
| Delete empty line | Backspace (at start of line) |
| Move between rows | Arrow Up / Down |
| Rescan diff | F5 |
| Undo | Cmd+Z |
| Redo | Cmd+Shift+Z |

## Ghost Lines

When you insert a new line in one pane, a **ghost line** is automatically added to the other pane to maintain alignment.

## Rescan

Press **F5** to recompute the diff from the current edited content. This is useful after making multiple edits to see the updated diff state.

:::caution
If you have unsaved edits, auto-rescan will skip file reload to prevent data loss. Manual F5 rescan always recomputes from the current editor content.
:::

## New Blank Documents

Create empty comparison documents for editing from scratch:

- **File → New → Text**: Empty 2-way comparison
- **File → New → Table**: Empty CSV/TSV comparison (10x5 grid)
- **File → New (3-pane) → Text / Table**: Empty 3-way comparison
