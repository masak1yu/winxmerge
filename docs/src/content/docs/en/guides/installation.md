---
title: Installation
description: How to install WinXMerge on your system.
---

## Download

Pre-built binaries are available from the [GitHub Releases](https://github.com/masak1yu/winxmerge/releases) page.

| Platform | File |
|----------|------|
| macOS (Apple Silicon) | `winxmerge-macos-aarch64.tar.gz` |
| Linux (x86_64) | `winxmerge-linux-x86_64.tar.gz` |
| Windows (x86_64) | `winxmerge-windows-x86_64.zip` |

## Web Version

No installation required — try WinXMerge directly in your browser at **[winxmerge.app](https://winxmerge.app)**.

## Build from Source

### Prerequisites

- [asdf](https://asdf-vm.com/) or [mise](https://mise.jdx.dev/) installed
- macOS / Linux / Windows (WSL)

### Steps

```bash
# Clone the repository
git clone git@github.com:masak1yu/winxmerge.git
cd winxmerge

# Install Rust via asdf
asdf plugin add rust
asdf install

# Build
cargo build --features desktop

# Run tests
cargo test --features desktop

# Launch the app
cargo run --features desktop
```

## Build WASM Version

```bash
# Install WASM target and trunk
rustup target add wasm32-unknown-unknown
cargo install trunk

# Development server
trunk serve

# Production build (outputs to dist/)
trunk build --release
```

## macOS App Bundle

To build a `.app` bundle with the Finder Sync Extension:

```bash
./scripts/build-macos-bundle.sh
```

The resulting `WinXMerge.app` will be in the `target/` directory.
