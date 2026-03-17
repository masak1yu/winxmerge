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
- 左右スクロール同期
- 行番号クリックで差分選択

### 3-way マージ
- 3ペイン表示（Left / Base / Right）
- ベースファイルからの変更を自動検出
- 衝突行のハイライト（赤色）と L/R ボタンによる衝突解決
- 衝突ナビゲーション（次/前）
- 自動マージ（両側が同じ変更の場合）

### フォルダ比較
- ディレクトリの再帰比較
- ファイル状態表示（同一/異なる/片方のみ）
- 左右の更新日時を表示
- .gitignore パターンの自動読み込み（.git ディレクトリは自動除外）
- ファイル拡張子フィルタ
- ダブルクリックでファイル差分ビューを開く
- 「< Back」ボタンでフォルダビューに戻る

### タブ
- 複数の比較をタブで管理
- 各タブが独立した状態を保持
- Cmd+T で新規タブ、Cmd+W で閉じる

### シンタックスハイライト
- tree-sitter による行レベルのハイライト
- 対応言語: Rust, JavaScript, Python, JSON, C, C++, Go, TypeScript, TSX, Ruby
- ファイルタイプの自動検出
- オプションでオン/オフ切替

### Undo / Redo
- マージ操作の取り消し・やり直し
- Cmd+Z / Cmd+Shift+Z

### 差分オプション
- 空白の無視
- 大文字小文字の無視
- 空行の無視
- 行末の違いを無視
- 行移動検出のオン/オフ

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
- WinMerge 風の初期選択ダイアログ（最近のファイル一覧付き）
- WinMerge 風のオプション設定画面（Edit → Options...）
- 右クリックコンテキストメニュー（コピー、マージ、ナビゲーション）
- 未保存変更の確認ダイアログ
- HTML 差分レポートエクスポート（File → Export HTML Report...）
- ネイティブメニューバー（macOS / Windows）
- 設定の永続化（~/.config/winxmerge/settings.json）
- 大きなファイル向けパフォーマンス最適化
- GitHub Actions CI（ubuntu / macOS でビルド・テスト・lint）

## 技術スタック

| 項目 | 技術 |
|------|------|
| 言語 | Rust 1.94.0 |
| UI フレームワーク | [Slint](https://slint.dev/) 1.15 |
| 差分アルゴリズム | [similar](https://crates.io/crates/similar) |
| シンタックスハイライト | [tree-sitter](https://crates.io/crates/tree-sitter) |
| ファイルダイアログ | [rfd](https://crates.io/crates/rfd) |
| エンコーディング検出 | [chardetng](https://crates.io/crates/chardetng) + [encoding_rs](https://crates.io/crates/encoding_rs) |
| クリップボード | [arboard](https://crates.io/crates/arboard) |
| 設定永続化 | [serde](https://crates.io/crates/serde) + [serde_json](https://crates.io/crates/serde_json) |

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

# 2-way 比較
cargo run -- file1.txt file2.txt

# 3-way マージ
cargo run -- base.txt left.txt right.txt
```

## Git 連携

WinXMerge を `git difftool` / `git mergetool` として使用できます。

### difftool セットアップ

```bash
cargo build --release
cp target/release/winxmerge ~/.local/bin/

git config --global diff.tool winxmerge
git config --global difftool.winxmerge.cmd 'winxmerge "$LOCAL" "$REMOTE"'
git config --global difftool.prompt false
```

### mergetool セットアップ（3-way マージ）

```bash
git config --global merge.tool winxmerge
git config --global mergetool.winxmerge.cmd 'winxmerge "$BASE" "$LOCAL" "$REMOTE"'
git config --global mergetool.winxmerge.trustExitCode true
```

### 使い方

```bash
# ワーキングツリーの変更を確認
git difftool

# 特定のファイルの差分を確認
git difftool -- path/to/file.rs

# ブランチ間の差分を確認
git difftool main..feature-branch

# マージ衝突の解決
git mergetool
```

## プロジェクト構成

```
winxmerge/
├── Cargo.toml
├── build.rs                    # Slint コンパイル設定
├── .tool-versions              # asdf バージョン管理
├── ui/
│   ├── main.slint              # メインウィンドウ
│   ├── dialogs/
│   │   ├── open-dialog.slint   # ファイル/フォルダ選択ダイアログ
│   │   └── options-dialog.slint # オプション設定ダイアログ
│   └── widgets/
│       ├── diff-view.slint     # 2-way 差分表示ウィジェット
│       ├── diff-view-3way.slint # 3-way マージ表示ウィジェット
│       ├── folder-view.slint   # フォルダ比較ウィジェット
│       └── tab-bar.slint       # タブバーウィジェット
├── src/
│   ├── main.rs                 # エントリーポイント、CLI 引数処理
│   ├── app.rs                  # アプリケーション状態管理（タブ対応）
│   ├── encoding.rs             # エンコーディング検出・変換
│   ├── export.rs               # HTML レポートエクスポート
│   ├── highlight.rs            # シンタックスハイライト（10言語対応）
│   ├── settings.rs             # 設定永続化
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
   - 3-way マージ: 「3-way merge」をチェックしてベースファイルも指定
   - 最近のファイル一覧からワンクリックで再オープン
3. **差分ナビゲーション:** ツールバーの ◀ Prev / Next ▶ または Alt+↓/↑
4. **マージ:** Copy → / ← Copy ボタン、または差分行間の ▶ / ◀ ボタン
5. **3-way 衝突解決:** 赤い行の L / R ボタンで左右どちらを採用するか選択
6. **Undo:** Cmd+Z で操作を元に戻す
7. **検索:** Cmd+F で検索・置換バーを表示
8. **タブ:** Cmd+T で新規タブ、複数の比較を並行管理
9. **オプション:** Edit → Options... で各種設定を変更

## ライセンス

本プロジェクトは [Slint Royalty-Free Desktop, Mobile, and Web Applications License v2.0](https://slint.dev/terms-and-conditions#royalty-free) の下で配布されます。

Slint UI フレームワークを使用しているため、以下の条件が適用されます:

- デスクトップ / モバイル / Web アプリケーションとしての配布はロイヤリティフリー
- 組み込みシステムでの使用は対象外
- Slint の帰属表示（AboutSlint ウィジェットまたは Web バッジ）が必要
