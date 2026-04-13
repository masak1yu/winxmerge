---
title: Export & Reports
description: Export diff results as HTML, Excel, CSV, or JSON reports.
---

## HTML Report

**File → Export HTML Report...** generates a styled HTML file containing:
- Color-coded diff display (matching the app's visual style)
- All diff comments embedded in the report
- Suitable for sharing and printing

The HTML report is also used when printing via the system print dialog.

## Excel Report

**File → Export Excel (.xlsx)...** generates an Excel workbook with:
- Color-coded rows (green/red/yellow for added/removed/modified)
- A comments column containing diff notes
- Compatible with Excel, LibreOffice, Google Sheets

## Export All Comments

**File → Export All Comments (CSV/JSON)...** collects all diff comments across all open tabs and exports them as:
- **CSV**: Spreadsheet-compatible format
- **JSON**: Machine-readable format

This is useful for code review workflows where comments need to be tracked or shared.
