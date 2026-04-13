---
title: シンタックスハイライト
description: tree-sitterによる言語対応シンタックスハイライト。
---

WinXMerge は [tree-sitter](https://tree-sitter.github.io/) を使用してソースコードのシンタックスハイライトを提供します。

## 対応言語

| 言語 | 拡張子 |
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

## 自動検出

ファイルの拡張子に基づいてファイルタイプが自動検出されます。手動設定は不要です。

## 切替

シンタックスハイライトは **Edit → Options...** でオン/オフを切り替えられます。

:::note
シンタックスハイライトはデスクトップ版でのみ利用可能です。WASM Web版には tree-sitter が含まれていません。
:::
