---
title: Plugin System
description: Extend WinXMerge with external commands.
---

WinXMerge supports a plugin system that lets you run external commands with file path placeholders.

## Configuration

Plugins are configured in `~/.config/winxmerge/settings.json`:

```json
{
  "plugins": [
    {
      "name": "Format with Prettier",
      "command": "prettier --write {LEFT} {RIGHT}"
    },
    {
      "name": "Open in VS Code",
      "command": "code --diff {LEFT} {RIGHT}"
    }
  ]
}
```

## Placeholders

| Placeholder | Description |
|-------------|-------------|
| `{LEFT}` | Path to the left file |
| `{RIGHT}` | Path to the right file |

## Usage

Configured plugins appear in the **Plugins** menu. Select a plugin to execute its command with the current file paths substituted.
