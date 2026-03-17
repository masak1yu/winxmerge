# WinXMerge

WinMerge にインスパイアされた、Rust + Slint UI によるクロスプラットフォームのファイル差分比較・マージツール。

## 機能

### ファイル比較 (2-way)
- 行単位の差分表示（追加/削除/変更を色分け）
- 差分ナビゲーション（次/前の差分へジャンプ）
- マージ操作（左→右 / 右→左のブロック単位コピー）
- インライン差分マーカー付き2ペイン表示
- ロケーションペイン（差分位置のミニマップ）

### フォルダ比較
- ディレクトリの再帰比較
- ファイル状態表示（同一/異なる/片方のみ）
- ダブルクリックでファイル差分ビューを開く

### 差分オプション
- 空白の無視
- 大文字小文字の無視

### エンコーディング
- 文字エンコーディング自動検出（UTF-8, UTF-16, Shift_JIS 等）
- BOM 対応
- 保存時に元のエンコーディングを維持

### その他
- テキスト検索（マッチ数表示、前/次ナビゲーション）
- ファイル保存（未保存変更の検知）
- ネイティブメニューバー（macOS / Windows）

## 技術スタック

| 項目 | 技術 |
|------|------|
| 言語 | Rust 1.94.0 |
| UI フレームワーク | [Slint](https://slint.dev/) 1.15 |
| 差分アルゴリズム | [similar](https://crates.io/crates/similar) |
| ファイルダイアログ | [rfd](https://crates.io/crates/rfd) |
| エンコーディング検出 | [chardetng](https://crates.io/crates/chardetng) + [encoding_rs](https://crates.io/crates/encoding_rs) |

## 環境構築

### 前提条件

- [asdf](https://asdf-vm.com/) がインストールされていること
- macOS / Linux / Windows（WSL）

### セットアップ

```bash
# リポジトリをクローン
git clone git@github.com:masak1yu/winxmerge.git
cd winxmerge

# asdf で Rust をインストール
asdf plugin add rust
asdf install

# ビルド
cargo build

# テスト実行
cargo test

# アプリ起動
cargo run
```

## プロジェクト構成

```
winxmerge/
├── Cargo.toml
├── build.rs                    # Slint コンパイル設定
├── .tool-versions              # asdf バージョン管理
├── ui/
│   ├── main.slint              # メインウィンドウ（メニュー/ツールバー/ステータスバー）
│   └── widgets/
│       ├── diff-view.slint     # 差分表示ウィジェット（2ペイン + ロケーションペイン）
│       └── folder-view.slint   # フォルダ比較ウィジェット
├── src/
│   ├── main.rs                 # エントリーポイント、コールバック接続
│   ├── app.rs                  # アプリケーション状態管理、UI 連携
│   ├── encoding.rs             # エンコーディング検出・変換
│   ├── diff/
│   │   ├── engine.rs           # 差分計算エンジン（空白/大文字小文字無視対応）
│   │   └── folder.rs           # フォルダ再帰比較
│   └── models/
│       ├── diff_line.rs        # 差分行データモデル
│       └── folder_item.rs      # フォルダ比較アイテムモデル
└── testdata/                   # テスト用サンプルファイル
```

## 使い方

1. `cargo run` でアプリを起動
2. **ファイル比較:** 「Open Left...」「Open Right...」で2つのファイルを選択
3. **フォルダ比較:** メニュー File → Open Left/Right Folder...
4. **差分ナビゲーション:** ツールバーの ◀ Prev / Next ▶ ボタン
5. **マージ:** Copy → / ← Copy ボタン、または差分行間の ▶ / ◀ ボタン
6. **検索:** メニュー Edit → Find...
7. **オプション:** ツールバーの「Ignore WS」「Ignore Case」チェックボックス

## ライセンス

本プロジェクトは [Slint Royalty-Free Desktop, Mobile, and Web Applications License v2.0](https://slint.dev/terms-and-conditions#royalty-free) の下で配布されます。

Slint UI フレームワークを使用しているため、以下の条件が適用されます:

- デスクトップ / モバイル / Web アプリケーションとしての配布はロイヤリティフリー
- 組み込みシステムでの使用は対象外
- Slint の帰属表示（AboutSlint ウィジェットまたは Web バッジ）が必要
