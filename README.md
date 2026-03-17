# WinXMerge

[![CI](https://github.com/masak1yu/winxmerge/actions/workflows/ci.yml/badge.svg)](https://github.com/masak1yu/winxmerge/actions/workflows/ci.yml)

A cross-platform file diff comparison and merge tool inspired by WinMerge, built with Rust + Slint UI.

## Features

### File Comparison (2-way)
- Line-level diff display (additions/deletions/changes/moves with color coding)
- Diff navigation (first/previous/next/last diff)
- Merge operations (block-level copy: left→right / right→left)
- Two-pane display with inline diff markers
- Location pane (minimap of diff positions)
- Automatic detection of moved lines (blue highlight)
- Synchronized left/right scrolling
- Click line numbers to select diff blocks

### 3-way Merge
- Three-pane display (Left / Base / Right)
- Automatic detection of changes from base file
- Conflict highlighting (red) with L/R buttons for conflict resolution
- Conflict navigation (next/previous)
- Auto-merge when both sides have identical changes

### Folder Comparison
- Recursive directory comparison
- File status display (identical / different / one-side only)
- Left/right modification timestamps
- Automatic .gitignore pattern loading (.git directories auto-excluded)
- File extension filter
- Double-click to open file diff view
- Right-click context menu (copy to left/right, delete)
- "< Back" button to return to folder view

### Tabs
- Manage multiple comparisons with tabs
- Each tab maintains independent state
- Cmd+T to create new tab, Cmd+W to close

### Syntax Highlighting
- Line-level highlighting via tree-sitter
- Supported languages: Rust, JavaScript, Python, JSON, C, C++, Go, TypeScript, TSX, Ruby
- Automatic file type detection
- Toggle on/off in options

### Undo / Redo
- Undo and redo merge operations
- Cmd+Z / Cmd+Shift+Z

### Diff Options
- Ignore whitespace
- Ignore case
- Ignore blank lines
- Ignore line ending differences
- Toggle moved line detection

### Encoding
- Automatic character encoding detection (UTF-8, UTF-16, Shift_JIS, etc.)
- BOM support
- Preserves original encoding when saving

### Search & Replace
- Text search (match count display, previous/next navigation)
- Replace / Replace All

### Go to Line
- Jump to a specific line number (Cmd+G)

### Bookmarks
- Toggle bookmarks on diff lines (Cmd+M)
- Navigate between bookmarks (F2 / Navigate menu)

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Cmd+S | Save left file |
| Cmd+F | Toggle search & replace |
| Cmd+G | Go to line |
| Cmd+M | Toggle bookmark |
| Cmd+Z | Undo |
| Cmd+Shift+Z | Redo |
| Cmd+T | New tab |
| Cmd+W | Close tab |
| Cmd+N | New comparison |
| Alt+↓ | Next diff |
| Alt+↑ | Previous diff |
| Alt+Home | First diff |
| Alt+End | Last diff |
| F2 | Next bookmark |

### Internationalization (i18n)
- Japanese / English UI switching (Edit → Options... → Appearance → Language)
- Full translation support for menus, toolbar, dialogs, and status bar

### Theme Switching
- Light / Dark theme switching (Edit → Options... → Appearance)
- All widgets automatically follow theme via Slint Palette integration
- Diff colors and syntax highlighting colors optimized per theme
- Settings persisted across sessions

### Other
- WinMerge-style initial file selection dialog (with recent files list)
- WinMerge-style options dialog (Edit → Options...)
- Right-click context menu (copy, merge, navigation)
- Unsaved changes confirmation dialog
- HTML diff report export (File → Export HTML Report...)
- Native menu bar (macOS / Windows)
- Settings persistence (~/.config/winxmerge/settings.json)
- Performance optimizations for large files
- GitHub Actions CI (build, test, lint on Ubuntu / macOS / Windows)
- Automated release builds for Linux, macOS (x86_64 + aarch64), and Windows

## Tech Stack

| Component | Technology |
|-----------|-----------|
| Language | Rust 1.94.0 |
| UI Framework | [Slint](https://slint.dev/) 1.15 |
| Diff Algorithm | [similar](https://crates.io/crates/similar) |
| Syntax Highlighting | [tree-sitter](https://crates.io/crates/tree-sitter) |
| File Dialog | [rfd](https://crates.io/crates/rfd) |
| Encoding Detection | [chardetng](https://crates.io/crates/chardetng) + [encoding_rs](https://crates.io/crates/encoding_rs) |
| Clipboard | [arboard](https://crates.io/crates/arboard) |
| Settings Persistence | [serde](https://crates.io/crates/serde) + [serde_json](https://crates.io/crates/serde_json) |

## Setup

### Prerequisites

- [asdf](https://asdf-vm.com/) installed
- macOS / Linux / Windows (WSL)

### Getting Started

```bash
# Clone the repository
git clone git@github.com:masak1yu/winxmerge.git
cd winxmerge

# Install Rust via asdf
asdf plugin add rust
asdf install

# Build
cargo build

# Run tests
cargo test

# Launch the app
cargo run

# 2-way comparison
cargo run -- file1.txt file2.txt

# 3-way merge
cargo run -- base.txt left.txt right.txt
```

## Git Integration

WinXMerge can be used as a `git difftool` / `git mergetool`.

### difftool Setup

```bash
cargo build --release
cp target/release/winxmerge ~/.local/bin/

git config --global diff.tool winxmerge
git config --global difftool.winxmerge.cmd 'winxmerge "$LOCAL" "$REMOTE"'
git config --global difftool.prompt false
```

### mergetool Setup (3-way merge)

```bash
git config --global merge.tool winxmerge
git config --global mergetool.winxmerge.cmd 'winxmerge "$BASE" "$LOCAL" "$REMOTE"'
git config --global mergetool.winxmerge.trustExitCode true
```

### Usage

```bash
# View working tree changes
git difftool

# Diff a specific file
git difftool -- path/to/file.rs

# Diff between branches
git difftool main..feature-branch

# Resolve merge conflicts
git mergetool
```

## Project Structure

```
winxmerge/
├── Cargo.toml
├── build.rs                    # Slint build configuration
├── .tool-versions              # asdf version management
├── ui/
│   ├── main.slint              # Main window
│   ├── theme.slint             # Theme color definitions (light/dark)
│   ├── dialogs/
│   │   ├── open-dialog.slint   # File/folder selection dialog
│   │   └── options-dialog.slint # Options dialog
│   └── widgets/
│       ├── diff-view.slint     # 2-way diff display widget
│       ├── diff-view-3way.slint # 3-way merge display widget
│       ├── folder-view.slint   # Folder comparison widget
│       └── tab-bar.slint       # Tab bar widget
├── src/
│   ├── main.rs                 # Entry point, CLI argument handling
│   ├── app.rs                  # Application state management (tab support)
│   ├── encoding.rs             # Encoding detection and conversion
│   ├── export.rs               # HTML report export
│   ├── highlight.rs            # Syntax highlighting (10 languages)
│   ├── settings.rs             # Settings persistence
│   ├── diff/
│   │   ├── engine.rs           # 2-way diff engine
│   │   ├── three_way.rs        # 3-way merge engine
│   │   └── folder.rs           # Recursive folder comparison
│   └── models/
│       ├── diff_line.rs        # Diff line data model
│       └── folder_item.rs      # Folder comparison item model
├── translations/               # Translation files (gettext .po)
│   └── ja/LC_MESSAGES/         # Japanese translations
└── testdata/                   # Test sample files
```

## Usage

1. Launch the app with `cargo run`
2. Enter left/right file or folder paths in the initial dialog and click "Compare"
   - For 3-way merge: check "3-way merge" and specify a base file
   - Re-open recent files from the list with one click
3. **Diff navigation:** Use ◀ Prev / Next ▶ toolbar buttons or Alt+↓/↑
4. **Merge:** Use Copy → / ← Copy buttons, or inline ▶ / ◀ buttons between diff lines
5. **3-way conflict resolution:** Click L / R buttons on red (conflict) lines to choose left or right
6. **Undo:** Cmd+Z to undo operations
7. **Search:** Cmd+F to show the search & replace bar
8. **Tabs:** Cmd+T for a new tab, manage multiple comparisons in parallel
9. **Options:** Edit → Options... to configure settings

## License

This project is distributed under the [Slint Royalty-Free Desktop, Mobile, and Web Applications License v2.0](https://slint.dev/terms-and-conditions#royalty-free).

Since this project uses the Slint UI framework, the following conditions apply:

- Distribution as desktop / mobile / web applications is royalty-free
- Embedded system use is not covered
- Slint attribution (AboutSlint widget or web badge) is required
