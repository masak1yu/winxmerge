---
title: Syntax Highlighting
description: Language-aware syntax highlighting via tree-sitter.
---

WinXMerge provides syntax highlighting for source code files using [tree-sitter](https://tree-sitter.github.io/).

## Supported Languages

| Language | Extension |
|----------|-----------|
| Rust | `.rs` |
| JavaScript | `.js` |
| TypeScript | `.ts` |
| TSX | `.tsx` |
| Python | `.py` |
| Go | `.go` |
| C | `.c`, `.h` |
| C++ | `.cpp`, `.hpp`, `.cc` |
| C# | `.cs` |
| Java | `.java` |
| Ruby | `.rb` |
| JSON | `.json` |
| YAML | `.yml`, `.yaml` |
| TOML | `.toml` |
| Markdown | `.md` |

## Automatic Detection

File types are automatically detected based on the file extension. No manual configuration is required.

## Toggle

Syntax highlighting can be toggled on/off in **Edit → Options...**.

:::note
Syntax highlighting is available in the desktop version only. The WASM web version does not include tree-sitter support.
:::
