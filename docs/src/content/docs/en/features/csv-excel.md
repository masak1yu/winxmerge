---
title: CSV / Excel Comparison
description: Compare spreadsheets and delimited files with cell-level diff.
---

## CSV / TSV Comparison

WinXMerge provides a dedicated table view for comparing `.csv` and `.tsv` files.

### Features

- **Cell-level diff**: Each cell is compared individually, with changed cells highlighted
- **Auto-delimiter detection**: Automatically detects comma (CSV) or tab (TSV) delimiters independently per file
- **Quoted fields**: Handles quoted fields with embedded delimiters and newlines
- **Column resizing**: Drag column header borders to resize
- **Delimiter mismatch warning**: Alerts when comparing .csv vs .tsv files

### Editing

- **Inline cell editing**: Click a cell to edit its content
- **Undo/Redo**: Cmd+Z / Cmd+Shift+Z for cell edit operations
- **Save**: Cmd+S saves as CSV, with Save As dialog for new documents
- **Rescan**: F5 recomputes the diff from edited cells

### New Table Document

Create a blank table comparison via **File → New → Table** (10x5 initial grid).

## Excel / Spreadsheet Comparison

WinXMerge can compare Excel and spreadsheet files with a table-view cell diff display.

### Supported Formats

| Format | Extension |
|--------|-----------|
| Excel (modern) | `.xlsx` |
| Excel (legacy) | `.xls` |
| Excel with macros | `.xlsm` |
| OpenDocument | `.ods` |

### Features

- Changed cells are highlighted
- **Sheet selector** for multi-sheet files — compare sheet by sheet
- Read-only comparison view
