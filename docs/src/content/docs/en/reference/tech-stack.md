---
title: Tech Stack
description: Technologies and libraries used in WinXMerge.
---

## Core

| Component | Technology |
|-----------|-----------|
| Language | Rust 1.94.0 |
| UI Framework | [Slint](https://slint.dev/) 1.15 |
| Diff Algorithm | [similar](https://crates.io/crates/similar) |

## Desktop Features

| Component | Technology |
|-----------|-----------|
| Syntax Highlighting | [tree-sitter](https://crates.io/crates/tree-sitter) |
| File Dialog | [rfd](https://crates.io/crates/rfd) |
| Encoding Detection | [chardetng](https://crates.io/crates/chardetng) + [encoding_rs](https://crates.io/crates/encoding_rs) |
| Clipboard | [arboard](https://crates.io/crates/arboard) |
| Settings Persistence | [serde](https://crates.io/crates/serde) + [serde_json](https://crates.io/crates/serde_json) |
| ZIP Comparison | [zip](https://crates.io/crates/zip) |
| Excel Read | [calamine](https://crates.io/crates/calamine) |
| Excel Export | [rust_xlsxwriter](https://crates.io/crates/rust_xlsxwriter) |
| Image Comparison | [image](https://crates.io/crates/image) |

## WASM Build

| Component | Technology |
|-----------|-----------|
| Bindings | [wasm-bindgen](https://crates.io/crates/wasm-bindgen) |
| Build Tool | [trunk](https://trunkrs.dev/) |
| Deployment | Cloudflare Pages |

## Project Structure

```
winxmerge/
├── Cargo.toml
├── build.rs                    # Slint build configuration
├── ui/
│   ├── main.slint              # Main window
│   ├── theme.slint             # Theme color definitions
│   ├── icons/                  # SVG toolbar icons
│   ├── dialogs/                # Dialog components
│   └── widgets/                # UI widget components
├── src/
│   ├── main.rs                 # Entry point, CLI handling
│   ├── app.rs                  # Application state management
│   ├── diff/
│   │   ├── engine.rs           # 2-way diff engine
│   │   ├── three_way.rs        # 3-way merge engine
│   │   └── folder.rs           # Folder comparison
│   └── models/                 # Data models
├── macos/                      # macOS bundle + Finder extension
├── scripts/                    # Build scripts
└── translations/               # i18n files (gettext .po)
```
