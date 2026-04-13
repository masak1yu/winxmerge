---
title: Settings
description: Configure WinXMerge behavior and appearance.
---

Settings are accessed via **Edit → Options...** and persisted to `~/.config/winxmerge/settings.json`.

## Appearance

| Setting | Description |
|---------|-------------|
| Theme | Light or Dark |
| Language | English or Japanese |
| Font size | Editor font size (8–32pt), adjustable via View → Zoom In/Out |
| Line wrapping | Toggle line wrapping at the window edge |

## Comparison

| Setting | Description |
|---------|-------------|
| Ignore whitespace | Treat lines with different whitespace as equal |
| Ignore case | Case-insensitive comparison |
| Ignore blank lines | Skip empty lines in diff |
| Ignore line endings | Ignore CR/LF differences |
| Moved line detection | Detect and highlight moved lines |
| Syntax highlighting | Enable/disable tree-sitter highlighting |

## Filters

| Setting | Description |
|---------|-------------|
| Line filters | Regex patterns to exclude lines from comparison |
| Substitution filters | Regex find/replace applied before comparison |

See [Filters](/en/features/filters/) for details.

## Auto-rescan

| Setting | Description |
|---------|-------------|
| Auto-rescan | Automatic file change detection (polls every 500ms) |

When enabled, WinXMerge re-runs the diff when files are modified externally. Skips reload when there are unsaved inline edits.

## External Editor

| Setting | Description |
|---------|-------------|
| Editor command | Custom command to open files in an external editor |

Accessible via **File → Open in External Editor**.

## Plugins

See [Plugin System](/en/integrations/plugins/) for configuration details.

## Session Persistence

WinXMerge saves and restores the following per-tab state across sessions:
- File paths and encoding
- EOL type and tab width
- Diff-only mode and status filter
- Diff comments
- Window zoom level
