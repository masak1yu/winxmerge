---
title: 技術スタック
description: WinXMerge で使用されている技術とライブラリ。
---

## コア

| コンポーネント | 技術 |
|-----------|-----------|
| 言語 | Rust 1.94.0 |
| UIフレームワーク | [Slint](https://slint.dev/) 1.15 |
| 差分アルゴリズム | [similar](https://crates.io/crates/similar) |

## デスクトップ機能

| コンポーネント | 技術 |
|-----------|-----------|
| シンタックスハイライト | [tree-sitter](https://crates.io/crates/tree-sitter) |
| ファイルダイアログ | [rfd](https://crates.io/crates/rfd) |
| エンコーディング検出 | [chardetng](https://crates.io/crates/chardetng) + [encoding_rs](https://crates.io/crates/encoding_rs) |
| クリップボード | [arboard](https://crates.io/crates/arboard) |
| 設定永続化 | [serde](https://crates.io/crates/serde) + [serde_json](https://crates.io/crates/serde_json) |
| ZIP比較 | [zip](https://crates.io/crates/zip) |
| Excel読み込み | [calamine](https://crates.io/crates/calamine) |
| Excelエクスポート | [rust_xlsxwriter](https://crates.io/crates/rust_xlsxwriter) |
| 画像比較 | [image](https://crates.io/crates/image) |

## WASMビルド

| コンポーネント | 技術 |
|-----------|-----------|
| バインディング | [wasm-bindgen](https://crates.io/crates/wasm-bindgen) |
| ビルドツール | [trunk](https://trunkrs.dev/) |
| デプロイ先 | Cloudflare Pages |

## プロジェクト構成

```
winxmerge/
├── Cargo.toml
├── build.rs                    # Slint ビルド設定
├── ui/
│   ├── main.slint              # メインウィンドウ
│   ├── theme.slint             # テーマカラー定義
│   ├── icons/                  # SVG ツールバーアイコン
│   ├── dialogs/                # ダイアログコンポーネント
│   └── widgets/                # UI ウィジェットコンポーネント
├── src/
│   ├── main.rs                 # エントリーポイント、CLI処理
│   ├── app.rs                  # アプリケーション状態管理
│   ├── diff/
│   │   ├── engine.rs           # 2-way 差分エンジン
│   │   ├── three_way.rs        # 3-way マージエンジン
│   │   └── folder.rs           # フォルダ比較
│   └── models/                 # データモデル
├── macos/                      # macOS バンドル + Finder 拡張
├── scripts/                    # ビルドスクリプト
└── translations/               # 多言語ファイル (gettext .po)
```
