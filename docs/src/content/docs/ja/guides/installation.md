---
title: インストール
description: WinXMerge のインストール方法。
---

## ダウンロード

ビルド済みバイナリは [GitHub Releases](https://github.com/masak1yu/winxmerge/releases) ページからダウンロードできます。

| プラットフォーム | ファイル |
|----------|------|
| macOS (Apple Silicon) | `winxmerge-macos-aarch64.tar.gz` |
| Linux (x86_64) | `winxmerge-linux-x86_64.tar.gz` |
| Windows (x86_64) | `winxmerge-windows-x86_64.zip` |

## Web版

インストール不要 — ブラウザで直接 **[winxmerge.app](https://winxmerge.app)** にアクセスしてお試しください。

## ソースからビルド

### 前提条件

- [asdf](https://asdf-vm.com/) または [mise](https://mise.jdx.dev/) がインストール済み
- macOS / Linux / Windows (WSL)

### 手順

```bash
# リポジトリをクローン
git clone git@github.com:masak1yu/winxmerge.git
cd winxmerge

# asdf で Rust をインストール
asdf plugin add rust
asdf install

# ビルド
cargo build --features desktop

# テスト実行
cargo test --features desktop

# アプリを起動
cargo run --features desktop
```

## WASM版のビルド

```bash
# WASMターゲットとtrunkをインストール
rustup target add wasm32-unknown-unknown
cargo install trunk

# 開発サーバー
trunk serve

# 本番ビルド（dist/ に出力）
trunk build --release
```

## macOS アプリバンドル

Finder Sync Extension 付きの `.app` バンドルをビルドするには：

```bash
./scripts/build-macos-bundle.sh
```

`target/` ディレクトリに `WinXMerge.app` が生成されます。
