---
title: 3-Way Merge
description: Resolve merge conflicts with three-pane comparison.
---

WinXMerge supports 3-way merge for resolving conflicts between two modified versions of a base file.

## Three-Pane Display

The 3-way view shows three panes side by side:
- **Left**: One modified version
- **Base** (center): The common ancestor
- **Right**: The other modified version

## Launching 3-Way Merge

```bash
# From command line
cargo run --features desktop -- base.txt left.txt right.txt

# As git mergetool (see Git Integration)
git mergetool
```

Or check **3-way merge** in the file selection dialog and specify the base file.

## Diff Algorithm

WinXMerge uses the WinMerge-style **Make3wayDiff** overlap-grouping algorithm to automatically detect:
- Changes made only on the left
- Changes made only on the right
- Changes made on both sides (conflicts or identical changes)

## Conflict Resolution

Conflicts are highlighted in **red**. For each conflict block:
- Click **L** to accept the left version
- Click **R** to accept the right version
- Navigate between conflicts with next/previous conflict buttons

When both sides have identical changes, they are **auto-merged** automatically.

## Merge Workflow

1. Open a 3-way comparison (base + left + right)
2. Review each conflict highlighted in red
3. For each conflict, click **L** or **R** to choose a side
4. Use **Copy & Advance** to resolve and move to the next conflict
5. When all conflicts are resolved, save the result

## Inline Editing

You can edit text directly in any pane. Press **F5** to rescan and recompute the diff from the edited content.

## Saving

Use the **Save dropdown** to save each pane individually:
- Save Left
- Save Middle (base)
- Save Right
