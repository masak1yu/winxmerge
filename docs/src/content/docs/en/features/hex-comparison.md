---
title: Hex Comparison
description: Byte-for-byte hex/ASCII view for binary files, with differing-byte highlighting.
---

WinXMerge shows a read-only hex/ASCII view for files it can't diff as text.

## When It's Used

A file with a NUL byte in its first 8 KB is treated as binary and automatically opens in Hex view. You can also force Hex view explicitly:

- **File > Recompare As > Hex** — switch the current tab's file pair into Hex view (and **Normal** to switch back)
- Folder compare's item context menu **Compare as Hex**
- The command line option `/t Binary`

## Layout

Each row shows 16 bytes: an 8-digit offset, followed by the left file's hex bytes and ASCII rendering, then the right file's. Non-printable bytes show as `.` in the ASCII column.

## Differences

Bytes are compared at the same offset — there's no realignment for inserted or deleted bytes, so a single inserted byte will mark everything after it as different. Once one file runs out of bytes, every remaining byte in the longer file counts as different too. Differing bytes are highlighted; use the diff navigation controls to jump between differing blocks, with the current row highlighted.

## Limitations

- No realignment: an insertion or deletion shifts the rest of the file out of alignment, unlike the text diff engine
- External file changes aren't auto-detected in Hex view; use Rescan (F5) to reload
- View > Zoom doesn't apply — hex view has no zoom control
- The Hex view choice isn't remembered across sessions — a forced Hex tab reopens through normal auto-detection next time
- `/t` is dropped when a path is forwarded to an already-running instance (e.g. via `git difftool`); only the file paths are passed along
- Read-only: no editing, saving, copying, exporting, or printing from Hex view
