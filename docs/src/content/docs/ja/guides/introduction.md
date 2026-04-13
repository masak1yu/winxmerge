---
title: WinXMerge とは
description: WinXMergeの概要と特徴。
---

## WinXMerge とは？

WinXMerge は、[WinMerge](https://winmerge.org/) にインスパイアされたクロスプラットフォームのファイル差分比較・マージツールです。**Rust** と [Slint UI](https://slint.dev/) で構築されています。

WinMerge は強力で広く使われている差分ツールですが、Windows でしか動作しません。WinXMerge は同じ使い慣れたワークフローを **macOS**、**Linux**、**Windows** で利用できるようにします。

## なぜ WinXMerge？

- **クロスプラットフォーム**: macOS (Apple Silicon)、Linux、Windows でネイティブ動作
- **WinMerge 互換**: 同じキーボードショートカット、マージワークフロー、ビジュアルスタイル
- **高速**: Rust によるネイティブパフォーマンス、大きなファイルでも快適
- **多機能**: 2-way差分、3-wayマージ、フォルダ比較、CSV/Excel/画像差分、シンタックスハイライトなど
- **Git連携**: `git difftool` / `git mergetool` として使用可能、シングルインスタンスタブモード対応
- **Web版**: [winxmerge.app](https://winxmerge.app) でインストール不要で即座に試用可能

## 対応プラットフォーム

| プラットフォーム | アーキテクチャ | 形式 |
|----------|-------------|--------|
| macOS | aarch64 (Apple Silicon) | .app バンドル |
| Linux | x86_64 | バイナリ |
| Windows | x86_64 | .exe |
| Web | WASM | [winxmerge.app](https://winxmerge.app) |

## ライセンス

WinXMerge は [Slint Royalty-Free Desktop, Mobile, and Web Applications License v2.0](https://slint.dev/terms-and-conditions#royalty-free) の下で配布されています。
