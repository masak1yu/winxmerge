---
title: Filters
description: Exclude or transform lines before comparison using regex patterns.
---

WinXMerge supports two types of filters to customize comparison behavior.

## Line Filters

Exclude lines matching regex patterns from the comparison entirely. Useful for ignoring:
- Comments
- Timestamps
- Auto-generated content
- Version numbers

### Configuration

1. Open **Edit → Options... → Filters**
2. Enter regex patterns separated by `|` (pipe)
3. Lines matching any pattern are excluded from the diff

## Substitution Filters

Apply regex find/replace transformations before comparison. The original files are not modified — substitutions are applied only during diff computation.

Use cases:
- Normalize date formats
- Ignore version number differences
- Standardize whitespace

### Configuration

1. Open **Edit → Options... → Filters**
2. Add substitution rules with a regex **Find** pattern and a **Replace** string
3. Multiple rules can be defined

## Diff Options

In addition to filters, WinXMerge provides these comparison options (available in the toolbar as toggle buttons):

| Option | Description |
|--------|-------------|
| Ignore whitespace | Treat lines with different whitespace as equal |
| Ignore case | Case-insensitive comparison |
| Ignore blank lines | Skip empty lines |
| Ignore line endings | Ignore CR/LF differences |
| Moved line detection | Toggle detection of moved lines (blue highlight) |

All settings are persisted across sessions.
