---
title: Folder Comparison
description: Compare directories recursively with filtering and sorting.
---

WinXMerge can recursively compare two directories, showing the status of every file.

## Overview

Select two folders in the open dialog to start a folder comparison. The view displays:
- **Tree-style indentation** for nested directories
- **File status**: Identical / Different / Left only / Right only
- **Modification timestamps** for both sides
- **File sizes** for both sides

## Filters

### Status Filter Bar

Filter the file list by clicking the status buttons above the list:
- **All**: Show all files
- **Identical**: Only files that are the same
- **Different**: Only files that differ
- **Left only**: Files present only in the left directory
- **Right only**: Files present only in the right directory

### .gitignore Support

WinXMerge automatically loads `.gitignore` patterns and excludes matching files. `.git` directories are always excluded.

### File Extension Filter

Filter the comparison by specific file extensions.

## Compare Method

Choose how two files with the same relative path are judged identical, in the Options
dialog or with the `/m` command line option:

- **Full Contents**: byte-for-byte comparison
- **Quick Contents**: same as Full for now (kept separate for a future faster path)
- **Binary Contents**: same as Full for now (kept separate for a future binary-only path)
- **Modified Date**: identical when both modification timestamps match exactly
- **Modified Date and Size**: Modified Date, and the file sizes also match
- **File Size**: identical when the file sizes match
- **Existence**: files present on both sides always count as identical

## Column Sorting

Click any column header to sort the file list:
- Name
- Status
- Left Size / Right Size
- Left Modified / Right Modified

Click again to toggle ascending/descending (indicated by ▲/▼).

## Actions

- **Double-click** a file to open a detailed diff view in a new tab
- **Right-click** for context menu:
  - Copy to left / Copy to right
  - Delete
- **< Back** button to return to the folder view from a file diff

## ZIP Archive Comparison

`.zip` files can be compared as virtual folders, showing added/removed/changed entries based on CRC and file size.
