# WinXMerge ハンドオーバードキュメント

## プロジェクト概要

WinMerge にインスパイアされた、Rust + Slint UI によるクロスプラットフォームのファイル差分比較・マージツール。
GitHub: `git@github.com:masak1yu/winxmerge.git`

## 現在の状態

- **バージョン:** 0.7.0
- **ブランチ:** `feature/v0.7.0`
- **テスト:** 12件すべてパス
- **ビルド:** `cargo build` 成功
- **CI:** GitHub Actions（ubuntu / macOS）

## リリース履歴

| バージョン | PR | 内容 |
|-----------|-----|------|
| v0.1.0 | #1 | 差分表示、マージ、フォルダ比較、メニュー、エンコーディング検出、検索、diff オプション |
| v0.2.0 | #2 | 選択ダイアログ、未保存確認、検索置換、行移動検出、タブ、シンタックスハイライト |
| v0.3.0 | #3 | Undo/Redo、キーボードショートカット、3-way マージエンジン、パフォーマンス最適化 |
| v0.4.0 | #4 | Git 連携、オプション画面、コンテキストメニュー、HTML エクスポート、スクロール同期、設定永続化 |
| v0.5.0 | #5 | 3-way マージ UI 統合、オプション実効化、最近のファイル一覧 |
| v0.6.0 | #6 | フォルダ比較強化（更新日時、.gitignore、フィルタ）、GitHub Actions CI |
| v0.7.0 | - | テーマ切替（ライト/ダーク）、ThemeColors グローバルによるカラー一元管理 |

## 実装済み機能一覧

### ファイル比較 (2-way)
- 行単位の差分表示（追加=緑 / 削除=赤 / 変更=黄 / 移動=青）
- 差分ナビゲーション（次/前の差分へジャンプ、Alt+↓/↑）
- マージ操作（左→右 / 右→左コピー、ツールバー + インラインボタン）
- Undo/Redo（スナップショットベース、Cmd+Z / Cmd+Shift+Z）
- 左右2ペイン + マージボタン列 + ロケーションペイン（ミニマップ）
- 左右スクロール同期（viewport-y バインド）
- 行番号クリックで差分選択

### 3-way マージ
- 3ペイン表示（Left / Base / Right）、スクロール同期
- ベースファイルからの変更を自動検出（LeftChanged / RightChanged / BothChanged / Conflict）
- 衝突行の赤色ハイライト、L/R ボタンで衝突解決
- 衝突ナビゲーション（次/前）
- CLI: `winxmerge <base> <left> <right>` で3ファイル起動
- 選択ダイアログに「3-way merge」チェックボックス + ベースファイル入力

### フォルダ比較
- ディレクトリの再帰比較
- ファイル状態表示（同一/異なる/片方のみ）
- 左右の更新日時を表示
- .gitignore パターン自動読み込み（.git ディレクトリ自動除外）
- ファイル拡張子フィルタ対応
- ダブルクリックでファイル差分ビュー → 「< Back」ボタンで戻る

### タブ
- 複数比較をタブで管理（各タブ独立状態）
- Cmd+T で新規タブ、Cmd+W で閉じる
- 未保存マーク表示、動的ウィンドウタイトル

### シンタックスハイライト
- tree-sitter による行レベルハイライト
- 対応言語: Rust, JavaScript, Python, JSON, C, C++, Go, TypeScript, TSX, Ruby
- ファイルタイプ自動検出（ステータスバーに表示）

### 検索・置換
- テキスト検索（マッチ数表示、前/次ナビゲーション）
- 置換 / 全置換
- Cmd+F でトグル

### エンコーディング
- 文字エンコーディング自動検出（UTF-8, UTF-16, Shift_JIS 等）
- BOM 検出と保持
- 保存時に元のエンコーディングを維持

### 差分オプション
- 空白の無視、大文字小文字の無視
- 行末の違いを無視、空行の無視（設定あり、エンジン側は一部未適用）
- 行移動検出のオン/オフ

### Git 連携
- CLI 引数でファイルパスを受け取り直接比較: `winxmerge <left> <right>`
- `git difftool` として設定可能

### 右クリックコンテキストメニュー
- Copy to Right/Left、Copy Text（クリップボード）、Next/Prev Diff
- オプション画面でオン/オフ切替

### オプション画面（Edit → Options...）
- 比較: 空白無視、大文字無視、空行無視、行末無視、行移動検出
- エディタ: 行番号表示、ワードラップ、シンタックスハイライト、フォントサイズ、タブ幅
- コンテキストメニューのオン/オフ
- 全設定を `~/.config/winxmerge/settings.json` に永続化

### HTML エクスポート
- File → Export HTML Report... で差分レポートを HTML 出力
- 色分けテーブル、印刷用 CSS 付き

### キーボードショートカット
| キー | 動作 |
|------|------|
| Cmd+S | 左ファイル保存 |
| Cmd+F | 検索・置換 |
| Cmd+Z | Undo |
| Cmd+Shift+Z | Redo |
| Cmd+T | 新規タブ |
| Cmd+W | タブを閉じる |
| Cmd+N | 新規比較 |
| Alt+↓/↑ | 次/前の差分 |

### テーマ切替
- ライト / ダーク テーマ切替（オプション画面の Appearance セクション）
- `Palette.color-scheme` による Slint ウィジェットの自動テーマ適用
- `ThemeColors` グローバルで差分カラー・シンタックスハイライト色を一元管理（`ui/theme.slint`）
- ライト・ダーク各テーマに最適化された差分背景色・マーカー色・構文色
- 設定は `~/.config/winxmerge/settings.json` に永続化

### その他
- WinMerge 風の初期選択ダイアログ
- 未保存変更の確認ダイアログ
- ネイティブメニューバー（macOS ではシステムメニューに統合）
- 大ファイル最適化（Patience アルゴリズム、5秒タイムアウト）
- 最近開いたファイルの履歴（設定に保存）

## アーキテクチャ

### Rust 側 (src/)

| ファイル | 役割 |
|---------|------|
| `main.rs` | エントリーポイント。CLI 引数処理、Slint コールバック接続、設定の読み込み |
| `app.rs` | アプリケーション状態管理（**TabState** per-tab + **AppState** タブマネージャー）。全 UI 操作のロジック |
| `diff/engine.rs` | 2-way 差分計算。空白/大文字無視、行移動検出、大ファイル最適化 |
| `diff/three_way.rs` | 3-way マージエンジン。衝突検出・自動マージ。UI 統合済み |
| `diff/folder.rs` | フォルダ再帰比較 |
| `encoding.rs` | エンコーディング検出・変換。BOM 対応 |
| `highlight.rs` | tree-sitter シンタックスハイライト（10言語対応） |
| `export.rs` | HTML 差分レポート生成 |
| `settings.rs` | 設定の永続化（serde_json） |
| `models/diff_line.rs` | DiffLine, DiffResult, LineStatus |
| `models/folder_item.rs` | FolderItem, FileCompareStatus |

### Slint UI 側 (ui/)

| ファイル | 役割 |
|---------|------|
| `main.slint` | メインウィンドウ。メニューバー、ツールバー、検索/置換バー、FocusScope（ショートカット）、確認ダイアログ |
| `widgets/diff-view.slint` | 2-way 差分2ペイン + マージボタン列 + ロケーションペイン + コンテキストメニュー |
| `widgets/diff-view-3way.slint` | 3-way マージ3ペイン（Left/Base/Right）+ 衝突解決ボタン |
| `widgets/folder-view.slint` | フォルダ比較リスト表示 |
| `widgets/tab-bar.slint` | タブバー |
| `dialogs/open-dialog.slint` | 初期選択ダイアログ |
| `theme.slint` | テーマカラー定義（ThemeColors グローバル、ライト/ダーク対応） |
| `dialogs/options-dialog.slint` | オプション設定ダイアログ（テーマ選択含む） |

### 状態管理の構造

```
AppState
├── tabs: Vec<TabState>     # 各タブが独立した比較状態
├── active_tab: usize
│
└── TabState
    ├── left_path / right_path / base_path  # ファイルパス（base は 3-way 用）
    ├── diff_positions / current_diff   # 差分ナビゲーション
    ├── left_lines / right_lines        # テキスト内容
    ├── undo_stack / redo_stack         # TextSnapshot のスタック
    ├── diff_options                    # 空白/大文字無視
    ├── search_matches                  # 検索結果
    ├── view_mode                       # 0=diff, 1=folder, 2=open dialog
    ├── diff_line_data                  # UI 表示用キャッシュ
    ├── folder_items / folder_item_data # フォルダ比較結果
    └── left_encoding / right_encoding  # 検出されたエンコーディング

AppSettings (永続化)
├── 比較オプション（ignore_whitespace, ignore_case, etc.）
├── エディタ設定（font_size, tab_width, etc.）
├── UI 設定（show_toolbar, enable_context_menu）
└── recent_files: Vec<RecentEntry>
```

## 技術スタック

- **Rust 1.94.0**（asdf 管理、`.tool-versions` あり）
- **Slint 1.15.1** — UI フレームワーク
- **similar 2.6** — 差分アルゴリズム
- **tree-sitter 0.26** + tree-sitter-highlight — シンタックスハイライト（10言語）
- **rfd 0.15** — ネイティブファイルダイアログ
- **chardetng + encoding_rs** — エンコーディング検出
- **arboard 3** — クリップボード操作
- **serde + serde_json** — 設定の永続化
- **dirs 6** — 設定ファイルパス取得

## 既知の問題・制限

1. **シンタックスハイライトが行単位** — Slint の `Text` がリッチテキスト非対応のため
2. **ドラッグ＆ドロップ非対応** — Slint 1.15 が外部ファイル D&D をサポートしていない
3. **行内（文字レベル）差分非対応** — Slint のリッチテキスト制限
4. **word_wrap / tab_width** — 設定は保存されるが UI に未反映（Slint ListView の制約）

---

## 次にやるべき項目 (v0.8.0+)

### 優先度高

1. **国際化 (i18n)**
   - メニュー・ダイアログの日本語/英語切替
   - Slint の `@tr()` マクロ活用

2. **リリースバイナリの自動ビルド**
   - GitHub Actions で macOS / Linux / Windows のクロスコンパイル
   - GitHub Releases への自動アップロード

### 優先度低

3. **行内（文字レベル）差分**
   - `similar` の文字レベル diff で変更位置を計算
   - Slint のリッチテキスト対応を待つか、複数 Text 要素で近似実装

4. **プラグインシステム**
   - カスタム差分フィルタ（前処理）
   - 外部ツール連携

5. **アクセシビリティ**
    - スクリーンリーダー対応
    - キーボードのみでの全操作対応

## ビルド・テスト手順

```bash
asdf install                             # Rust 1.94.0 をインストール
cargo build                              # ビルド
cargo test                               # 12件のテスト実行
cargo run                                # アプリ起動（選択ダイアログ）
cargo run -- file1.txt file2.txt         # 2-way 比較
cargo run -- base.txt left.txt right.txt # 3-way マージ
cargo build --release                    # リリースビルド
```

### Git difftool 設定

```bash
cp target/release/winxmerge ~/.local/bin/
git config --global diff.tool winxmerge
git config --global difftool.winxmerge.cmd 'winxmerge "$LOCAL" "$REMOTE"'
git config --global difftool.prompt false
git difftool                             # 2-way diff

# mergetool（3-way マージ）
git config --global merge.tool winxmerge
git config --global mergetool.winxmerge.cmd 'winxmerge "$BASE" "$LOCAL" "$REMOTE"'
git mergetool                            # 衝突解決
```
