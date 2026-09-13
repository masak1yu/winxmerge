---
title: Project Files
description: Save compared paths and compare options as a WinMerge-compatible project file, and reopen them later.
---

WinXMerge can save a comparison's paths and options as a WinMerge-compatible `.WinMerge` project file, and reopen it later. WinXMerge doesn't restore sessions across launches, so this is the only way to get back to a saved comparison.

## Opening and Saving

- **File → Open Project...** opens a `.WinMerge` file and starts a comparison for each `<paths>` entry it contains, one tab per entry
- **File → Save Project...** saves the current tab only, prompting for a destination
- From the command line: `winxmerge project.WinMerge` — only when it's the sole positional argument; the `.WinMerge` extension is matched case-insensitively

## What's Read and Written

| Element | Meaning |
| --- | --- |
| `left` / `right` | The two compared paths |
| `middle` | The 3-way base file |
| `white-spaces` | 0 = compare all, 1 = ignore whitespace changes, 2 = ignore all whitespace |
| `ignore-case`, `ignore-blank-lines`, `ignore-carriage-return-diff` | Text compare toggles |
| `subfolders` | Recurse into subfolders during a folder compare |
| `compare-method` | Folder compare method, 0–6 (Full/Quick/Binary/Date/SizeDate/Size/Existence); WinMerge's method 7, Image, has no equivalent and is ignored |

Elements WinXMerge has no use for — `filter`, `*-desc`, `*-readonly`, `window-type`, `table-*`, `unpacker`/`prediffer`, other `ignore-*`, `hidden-items`, and anything unrecognized — are read and discarded; they never cause an open to fail.

## Behavior Notes

- An option missing from the file leaves the current setting untouched, the same as WinMerge; a value outside its valid range is likewise ignored.
- `subfolders`/`compare-method` change the app-wide folder compare settings — same as the `/m` command line flag — so they affect later folder comparisons in other tabs too, and are what gets saved on exit. Text compare options apply only to the tab being opened.
- `subfolders = 0` limits the recursion depth to the top level only; `subfolders = 1` restores unlimited depth, but only if the depth was previously limited to the top level (a deeper custom limit is left alone). Saving writes the reverse mapping.
- Relative paths resolve against the project file's own folder. Unlike WinMerge, environment variables in paths aren't expanded. Saved paths are always absolute.
- Several `<paths>` entries open several tabs. An entry missing its left or right path, or a 3-way entry where any of the three paths is a folder (3-way folder compare isn't supported), is skipped; the status bar reports how many entries opened and skipped.
- A project file that fails to parse as XML shows an error in the status bar instead of opening.
