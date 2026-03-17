# WinXMerge

WinMerge にインスパイアされた、Rust + Slint UI によるクロスプラットフォームのファイル差分比較・マージツール。

## 機能

### ファイル比較 (2-way)
- 行単位の差分表示（追加/削除/変更/移動を色分け）
- 差分ナビゲーション（次/前の差分へジャンプ）
- マージ操作（左→右 / 右→左のブロック単位コピー）
- インライン差分マーカー付き2ペイン表示
- ロケーションペイン（差分位置のミニマップ）
- 行移動の自動検出（青色ハイライト）

### 3-way マージ（エンジン実装済み）
- ベースファイル・左・右の3ファイル比較
- 衝突検出と自動マージ

### フォルダ比較
- ディレクトリの再帰比較
- ファイル状態表示（同一/異なる/片方のみ）
- ダブルクリックでファイル差分ビューを開く

### タブ
- 複数の比較をタブで管理
- 各タブが独立した状態を保持
- Cmd+T で新規タブ、Cmd+W で閉じる

### シンタックスハイライト
- tree-sitter による行レベルのハイライト
- 対応言語: Rust, JavaScript, Python, JSON, C
- ファイルタイプの自動検出

### Undo / Redo
- マージ操作の取り消し・やり直し
- Cmd+Z / Cmd+Shift+Z

### 差分オプション
- 空白の無視
- 大文字小文字の無視

### エンコーディング
- 文字エンコーディング自動検出（UTF-8, UTF-16, Shift_JIS 等）
- BOM 対応
- 保存時に元のエンコーディングを維持

### 検索・置換
- テキスト検索（マッチ数表示、前/次ナビゲーション）
- 置換 / 全置換

### キーボードショートカット

| ショートカット | 動作 |
|---------------|------|
| Cmd+S | 左ファイル保存 |
| Cmd+F | 検索・置換の表示切替 |
| Cmd+Z | Undo |
| Cmd+Shift+Z | Redo |
| Cmd+T | 新規タブ |
| Cmd+W | タブを閉じる |
| Cmd+N | 新規比較 |
| Alt+↓ | 次の差分 |
| Alt+↑ | 前の差分 |

### その他
- WinMerge 風の初期選択ダイアログ
- WinMerge 風のオプション設定画面（Edit → Options...）
- 右クリックコンテキストメニュー（コピー、マージ、ナビゲーション）
- 未保存変更の確認ダイアログ
- HTML 差分レポートエクスポート（File → Export HTML Report...）
- ネイティブメニューバー（macOS / Windows）
- 設定の永続化（~/.config/winxmerge/settings.json）
- 大きなファイル向けパフォーマンス最適化

## 技術スタック

| 項目 | 技術 |
|------|------|
| 言語 | Rust 1.94.0 |
| UI フレームワーク | [Slint](https://slint.dev/) 1.15 |
| 差分アルゴリズム | [similar](https://crates.io/crates/similar) |
| シンタックスハイライト | [tree-sitter](https://crates.io/crates/tree-sitter) |
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

# 2つのファイルを指定して起動
cargo run -- file1.txt file2.txt
```

## Git 連携

WinXMerge を `git difftool` として使用できます。

### セットアップ

```bash
# ビルドしてパスの通る場所にインストール
cargo build --release
cp target/release/winxmerge ~/.local/bin/

# git difftool として設定
git config --global diff.tool winxmerge
git config --global difftool.winxmerge.cmd 'winxmerge "$LOCAL" "$REMOTE"'

# プロンプトなしで起動する場合
git config --global difftool.prompt false
```

### 使い方

```bash
# ワーキングツリーの変更を WinXMerge で確認
git difftool

# 特定のファイルの差分を確認
git difftool -- path/to/file.rs

# ブランチ間の差分を確認
git difftool main..feature-branch

# 特定のコミット間の差分を確認
git difftool HEAD~3..HEAD
```

## プロジェクト構成

```
winxmerge/
├── Cargo.toml
├── build.rs                    # Slint コンパイル設定
├── .tool-versions              # asdf バージョン管理
├── ui/
│   ├── main.slint              # メインウィンドウ（メニュー/ツールバー/ステータスバー）
│   ├── dialogs/
│   │   └── open-dialog.slint   # ファイル/フォルダ選択ダイアログ
│   └── widgets/
│       ├── diff-view.slint     # 差分表示ウィジェット（2ペイン + ロケーションペイン）
│       ├── folder-view.slint   # フォルダ比較ウィジェット
│       └── tab-bar.slint       # タブバーウィジェット
├── src/
│   ├── main.rs                 # エントリーポイント、コールバック接続
│   ├── app.rs                  # アプリケーション状態管理（タブ対応）、UI 連携
│   ├── encoding.rs             # エンコーディング検出・変換
│   ├── highlight.rs            # シンタックスハイライト（tree-sitter）
│   ├── diff/
│   │   ├── engine.rs           # 2-way 差分計算エンジン
│   │   ├── three_way.rs        # 3-way マージエンジン
│   │   └── folder.rs           # フォルダ再帰比較
│   └── models/
│       ├── diff_line.rs        # 差分行データモデル
│       └── folder_item.rs      # フォルダ比較アイテムモデル
└── testdata/                   # テスト用サンプルファイル
```

## 使い方

1. `cargo run` でアプリを起動
2. 初期画面で左右のファイル/フォルダパスを入力し「Compare」
3. **差分ナビゲーション:** ツールバーの ◀ Prev / Next ▶ またはAlt+↓/↑
4. **マージ:** Copy → / ← Copy ボタン、または差分行間の ▶ / ◀ ボタン
5. **Undo:** Cmd+Z で操作を元に戻す
6. **検索:** Cmd+F で検索・置換バーを表示
7. **タブ:** Cmd+T で新規タブ、複数の比較を並行管理
8. **オプション:** ツールバーの「Ignore WS」「Ignore Case」チェックボックス

## ライセンス

本プロジェクトは [Slint Royalty-Free Desktop, Mobile, and Web Applications License v2.0](https://slint.dev/terms-and-conditions#royalty-free) の下で配布されます。

Slint UI フレームワークを使用しているため、以下の条件が適用されます:

- デスクトップ / モバイル / Web アプリケーションとしての配布はロイヤリティフリー
- 組み込みシステムでの使用は対象外
- Slint の帰属表示（AboutSlint ウィジェットまたは Web バッジ）が必要
