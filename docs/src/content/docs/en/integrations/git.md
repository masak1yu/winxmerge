---
title: Git Integration
description: Use WinXMerge as git difftool and git mergetool.
---

WinXMerge integrates with Git as both a `difftool` and a `mergetool`.

## difftool Setup

```bash
# Build and install
cargo build --release --features desktop
cp target/release/winxmerge ~/.local/bin/

# Configure git
git config --global diff.tool winxmerge
git config --global difftool.winxmerge.cmd 'winxmerge "$LOCAL" "$REMOTE"'
git config --global difftool.prompt false
```

## mergetool Setup (3-Way Merge)

```bash
git config --global merge.tool winxmerge
git config --global mergetool.winxmerge.cmd 'winxmerge "$BASE" "$LOCAL" "$REMOTE"'
git config --global mergetool.winxmerge.trustExitCode true
```

## Usage

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

## Single-Instance Tab Mode

When `git difftool` processes multiple changed files, WinXMerge uses **IPC** (Unix domain socket) to detect a running instance. Instead of opening multiple windows, subsequent diffs are opened as **new tabs** in the existing window.

When two or more file pairs are opened this way, they are displayed as a **virtual folder comparison view**. Double-click a file in the folder view to open its detailed diff in a new tab.

This behavior is automatic — no additional configuration is needed.
