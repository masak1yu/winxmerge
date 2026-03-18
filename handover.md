# WinXMerge ハンドオーバードキュメント

## プロジェクト概要

WinMerge にインスパイアされた、Rust + Slint UI によるクロスプラットフォームのファイル差分比較・マージツール。
GitHub: `git@github.com:masak1yu/winxmerge.git`

## 現在の状態

- **バージョン:** 0.20.0
- **ブランチ:** `feature/v0.20.0`
- **テスト:** 19件すべてパス
- **ビルド:** `cargo build` 成功
- **CI:** GitHub Actions（ubuntu / macOS / Windows）

## リリース履歴

| バージョン | PR | 内容 |
|-----------|-----|------|
| v0.1.0 | #1 | 差分表示、マージ、フォルダ比較、メニュー、エンコーディング検出、検索、diff オプション |
| v0.2.0 | #2 | 選択ダイアログ、未保存確認、検索置換、行移動検出、タブ、シンタックスハイライト |
| v0.3.0 | #3 | Undo/Redo、キーボードショートカット、3-way マージエンジン、パフォーマンス最適化 |
| v0.4.0 | #4 | Git 連携、オプション画面、コンテキストメニュー、HTML エクスポート、スクロール同期、設定永続化 |
| v0.5.0 | #5 | 3-way マージ UI 統合、オプション実効化、最近のファイル一覧 |
| v0.6.0 | #6 | フォルダ比較強化（更新日時、.gitignore、フィルタ）、GitHub Actions CI |
| v0.7.0 | #7 | テーマ切替（ライト/ダーク）、ThemeColors グローバルによるカラー一元管理 |
| v0.8.0 | #8 | 国際化 (i18n)：日本語/英語切替、@tr() マクロ、gettext .po ファイル |
| v0.9.0 | #9 | First/Last diff、Go to Line、ブックマーク、フォルダ操作、リリースCI、Windows CI |
| v0.10.0 | #10 | 行フィルタ、置換フィルタ（正規表現）、オプション画面拡張 |
| v0.11.0 | #11 | ワードレベル差分、プラグインシステム、アクセシビリティ、自動再スキャン、フォルダツリー表示、外部エディタ連携 |
| v0.12.0 | #12 | スプリッター、インライン編集、パッチエクスポート、スクロール同期、差分統計 |
| v0.13.0 | #13 | SVGアイコンツールバー、差分詳細ペイン、コピーして次へ、全差分コピー、ウィンドウサイズ保存 |
| v0.14.0 | #14 | 組み込みファイルブラウザ（WSL2対応）、左右スクロール双方向同期、絵文字→SVGアイコン化 |
| v0.15.0 | #15 | ブロック単位差分グループ化（WinMerge相当）、大ファイルでのスクロールカクつき解消 |
| v0.16.0 | - | 印刷機能、ワードdiffパフォーマンス最適化、差分詳細ペイン改善（リサイズ・ワードdiffハイライト）、tab_width/word_wrap実効化、プラグインメニュー動的生成、シンタックスハイライト行数制限 |
| v0.17.0 | #19 | 大ファイル非同期diff計算（30K行超をバックグラウンドスレッドで処理）、プラグイン実行修正（各メニュー項目が個別コマンドを実行） |
| v0.18.0 | #20 | 差分行のみ表示モード、バイナリファイル検出、検索ハイライト（アンバー色）、セッション復元（前回の全タブを起動時に自動再オープン）、フォルダ比較最大深度設定 |
| v0.19.0 | #21 | プラグイン非同期実行、差分統計常時表示、エンコーディング/行末文字のステータスバー表示、タブごとのdiffオプション独立化、フォルダ比較サイズ・日付フィルタ |
| v0.20.0 | - | ステータス別差分ナビゲーション、複数行選択+ブロックコピー、CSV/TSVエクスポート+フォルダHTMLレポート、クリップボード比較、タブ並び替え、フォルダ比較サマリー |

## 実装済み機能一覧

### ファイル比較 (2-way)
- 行単位の差分表示（追加=緑 / 削除=赤 / 変更=黄 / 移動=青）
- ワードレベル（文字レベル）差分表示（変更行の下に差分文字を表示）
- 差分ナビゲーション（次/前の差分へジャンプ、Alt+↓/↑）
- ブロック単位差分グループ化（連続する Delete/Insert を1ブロックに統合、WinMerge相当）
  - N:N ペアを Modified、余りを Removed/Added として表示
  - ナビゲーションはブロック単位（1ブロック = 1回の Alt+↓）
- マージ操作（左→右 / 右→左コピー、ツールバー + インラインボタン）
- Undo/Redo（スナップショットベース、Cmd+Z / Cmd+Shift+Z）
- 左右2ペイン + マージボタン列 + ロケーションペイン（ミニマップ）
- 左右スクロール双方向同期（どちらのペインをスクロールしても両側が追従）
- 左右ペインの行高を統一（ワードdiff有無の OR 条件）→ スクロール位置ズレなし
- 行番号クリックで差分選択

### 3-way マージ
- 3ペイン表示（Left / Base / Right）、スクロール同期
- ベースファイルからの変更を自動検出（LeftChanged / RightChanged / BothChanged / Conflict）
- 衝突行の赤色ハイライト、L/R ボタンで衝突解決
- 衝突ナビゲーション（次/前）
- CLI: `winxmerge <base> <left> <right>` で3ファイル起動
- 選択ダイアログに「3-way merge」チェックボックス + ベースファイル入力

### フォルダ比較
- ディレクトリの再帰比較（ツリー表示：パス深度に応じたインデント）
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
- 対応言語: Rust, JavaScript, Python, JSON, C, C++, Go, TypeScript, TSX, Ruby, Java, C#, YAML, TOML, Markdown
- ファイルタイプ自動検出（ステータスバーに表示）
- 5000行超のファイルはハイライトをスキップ（パフォーマンス最適化）

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
- エディタ: 行番号表示、ワードラップ（DiffView 行高 72px、折り返し表示）、シンタックスハイライト、フォントサイズ、タブ幅（タブ→スペース展開）
- コンテキストメニューのオン/オフ
- 全設定を `~/.config/winxmerge/settings.json` に永続化

### HTML エクスポート / 印刷
- File → Export HTML Report... で差分レポートを HTML 出力
- File → Print... でブラウザ印刷（HTML を一時ファイルに書き出し、`window.print()` JS を注入してデフォルトブラウザで開く）
- 色分けテーブル、印刷用 CSS 付き（`@media print` 対応）

### 差分ナビゲーション拡張
- First Diff / Last Diff（⏮ / ⏭ ボタン、Alt+Home / Alt+End）
- Go to Line ダイアログ（Cmd+G）
- ブックマーク切替（Cmd+M）、次/前のブックマーク（F2、Navigate メニュー）

### フォルダ操作
- 右クリックコンテキストメニュー（開く、左→右コピー、右→左コピー、削除）
- クリックで行選択

### リリースバイナリ自動ビルド
- GitHub Actions でタグ push 時に自動ビルド
- Linux (x86_64), macOS (x86_64 + aarch64), Windows (x86_64)
- GitHub Releases への自動アップロード

### 行フィルタ / 置換フィルタ
- 行フィルタ: 正規表現でマッチする行を比較から除外（`|` 区切りで複数指定可）
- 置換フィルタ: 正規表現で比較前にテキスト置換（タイムスタンプ、バージョン番号等の無視に有用）
- 複数ルール対応（パイプ `|` 区切り）
- オプション画面の Filters セクションで設定
- `regex` クレート使用、無効な正規表現は安全にスキップ
- 設定は `settings.json` に永続化

### プラグインシステム
- 外部コマンドをプラグインとして実行
- `{LEFT}` / `{RIGHT}` プレースホルダーでファイルパスを渡す
- `settings.json` の `plugins` フィールドで設定
- Plugins メニューから実行（プラグインごとに動的メニュー項目を生成、未設定時は "(No plugins configured)" を表示）

### 外部エディタ連携
- File メニューから左/右ファイルをシステムデフォルトエディタで開く
- `open` クレート使用（`open::that_detached()`）
- カスタムエディタコマンドを `settings.json` の `external_editor` で設定可能

### 自動再スキャン
- ファイル変更を自動検出（2秒間隔でポーリング）
- 変更検出時に差分を自動再計算
- オプション画面で Auto-rescan オン/オフ切替
- F5 キーで手動再スキャン

### アクセシビリティ
- Slint の `accessible-role` / `accessible-label` を主要 UI コンポーネントに設定
- DiffView、FolderView にスクリーンリーダー対応ラベル

### CI 拡張
- Windows をビルド・テストマトリクスに追加

### キーボードショートカット
| キー | 動作 |
|------|------|
| Cmd+S | 左ファイル保存 |
| Cmd+F | 検索・置換 |
| Cmd+G | 指定行へ移動 |
| Cmd+M | ブックマーク切替 |
| Cmd+Z | Undo |
| Cmd+Shift+Z | Redo |
| Cmd+T | 新規タブ |
| Cmd+W | タブを閉じる |
| Cmd+N | 新規比較 |
| Alt+↓/↑ | 次/前の差分 |
| Alt+Home/End | 最初/最後の差分 |
| F2 | 次のブックマーク |
| F5 | 再スキャン |

### 国際化 (i18n)
- 日本語 / 英語の UI 切替（オプション画面の Appearance セクション）
- Slint `@tr()` マクロによる全 UI 文字列の翻訳対応
- GNU gettext `.po` ファイル形式（`translations/ja/LC_MESSAGES/winxmerge.po`）
- `slint::select_bundled_translation()` による実行時言語切替
- メニュー、ツールバー、ダイアログ、ステータスバー、コンテキストメニュー全対応
- 設定は `~/.config/winxmerge/settings.json` に永続化

### テーマ切替
- ライト / ダーク テーマ切替（オプション画面の Appearance セクション）
- `Palette.color-scheme` による Slint ウィジェットの自動テーマ適用
- `ThemeColors` グローバルで差分カラー・シンタックスハイライト色を一元管理（`ui/theme.slint`）
- ライト・ダーク各テーマに最適化された差分背景色・マーカー色・構文色
- 設定は `~/.config/winxmerge/settings.json` に永続化

### SVG アイコンツールバー
- WinMerge 風のアイコンツールバー（1段、SVG アイコン 20 個）
- ToolBtn コンポーネント（32x32、ホバーハイライト、無効時半透明）
- ホバー時にステータスバーにヒント表示
- トグルボタン（Ignore WS / Ignore Case）はアクティブ時にハイライト
- アイコン: New, Back, Open L/R, Save L/R, Undo, Redo, Rescan, Settings, First/Prev/Next/Last, Copy R/L, Copy & Next R/L, Copy All R/L, WS, Aa

### 差分詳細ペイン
- ウィンドウ下部に差分ブロックの内容を上下表示（上=Left/削除、下=Right/追加）
- ScrollView 付き、選択差分の全行を表示
- 差分ナビゲーション・タブ切替・再計算時に自動更新
- ドラッグリサイズ対応（ペイン上端のハンドルで高さ調整、60px〜400px）
- ワードレベル差分ハイライト表示（Word Diff オンのとき変更文字列を下段に表示）

### コピーして次へ / 全差分コピー
- Copy Right and Advance / Copy Left and Advance（コピー後に次の差分へ自動移動）
- Copy All Left to Right / Copy All Right to Left（全差分を一括マージ）

### ウィンドウサイズ保存
- `window().on_close_requested()` でウィンドウサイズを `settings.json` に保存
- 起動時に `window().set_size()` で復元

### 組み込みファイルブラウザ
- GTK 非依存のファイル/フォルダピッカー（WSL2 等でネイティブダイアログが使えない環境に対応）
- ディレクトリナビゲーション（フォルダ/ファイル SVG アイコン、上のフォルダボタン、編集可能パスバー）
- ネイティブダイアログ（rfd）が利用不可の場合に自動フォールバック
- シングルクリックで選択、ダブルクリックでフォルダに入る/ファイルを確定

### その他
- WinMerge 風の初期選択ダイアログ
- 未保存変更の確認ダイアログ
- ネイティブメニューバー（macOS ではシステムメニューに統合）
- 大ファイル最適化（Patience アルゴリズム、5秒タイムアウト、20000行超はワードdiffスキップ）
- 30000行超の大ファイルは差分計算をバックグラウンドスレッドで実行（UI非ブロック）、完了後100msポーリングで反映
- 最近開いたファイルの履歴（設定に保存）
- **差分行のみ表示モード** — View → Diff Only で Equal 行を非表示（行高 0px）
- **バイナリファイル検出** — 先頭8KBにNULLバイトがある場合はテキスト差分をスキップし状態表示
- **検索ハイライト** — マッチ行の背景をアンバー色で強調表示
- **セッション復元** — 終了時に全タブのパスを `settings.json` に保存、次回起動時に自動再オープン
- **フォルダ比較最大深度** — オプション画面で再帰上限を設定（0=無制限）
- **プラグイン非同期実行** — `thread::spawn` + `upgrade_in_event_loop` でUIをブロックしない
- **差分統計常時表示** — ステータスバー右側に `+A -R ~M` を常に表示
- **エンコーディング/行末文字表示** — ステータスバー右側にファイル検出情報を表示（`UTF-8 | Shift_JIS`, `LF | CRLF`）
- **タブごとのdiffオプション独立化** — タブ切替時に全diffオプション（ignore flags, line filters, 置換フィルタ）を独立保持・復元
- **フォルダ比較サイズ/日付フィルタ** — 最小/最大ファイルサイズ、変更日付（YYYY-MM-DD）によるフィルタリング

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
| `highlight.rs` | tree-sitter シンタックスハイライト（15言語対応） |
| `export.rs` | HTML 差分レポート生成 |
| `settings.rs` | 設定の永続化（serde_json） |
| `models/diff_line.rs` | DiffLine, DiffResult, LineStatus |
| `models/folder_item.rs` | FolderItem, FileCompareStatus |

### Slint UI 側 (ui/)

| ファイル | 役割 |
|---------|------|
| `main.slint` | メインウィンドウ。メニューバー、SVGアイコンツールバー、差分詳細ペイン、検索/置換バー、FocusScope（ショートカット）、確認ダイアログ |
| `widgets/diff-view.slint` | 2-way 差分2ペイン + マージボタン列 + ロケーションペイン + コンテキストメニュー |
| `widgets/diff-view-3way.slint` | 3-way マージ3ペイン（Left/Base/Right）+ 衝突解決ボタン |
| `widgets/folder-view.slint` | フォルダ比較リスト表示 |
| `widgets/tab-bar.slint` | タブバー |
| `dialogs/open-dialog.slint` | 初期選択ダイアログ |
| `dialogs/file-browser.slint` | 組み込みファイルブラウザ（WSL2 対応、rfd フォールバック） |
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
    ├── left_encoding / right_encoding  # 検出されたエンコーディング
    └── left_mtime / right_mtime        # ファイル更新時刻（自動再スキャン用）

AppSettings (永続化)
├── 比較オプション（ignore_whitespace, ignore_case, etc.）
├── エディタ設定（font_size, tab_width, etc.）
├── UI 設定（show_toolbar, enable_context_menu）
├── フィルタ設定（line_filters, substitution_filters）
├── plugins: Vec<PluginEntry>   # プラグイン定義（name, command）
├── external_editor: String     # 外部エディタコマンド
├── auto_rescan: bool           # 自動再スキャン
└── recent_files: Vec<RecentEntry>
```

## 技術スタック

- **Rust 1.94.0**（asdf 管理、`.tool-versions` あり）
- **Slint 1.15.1** — UI フレームワーク
- **similar 2.6** — 差分アルゴリズム
- **tree-sitter 0.26** + tree-sitter-highlight — シンタックスハイライト（15言語）
- **rfd 0.15** — ネイティブファイルダイアログ
- **chardetng + encoding_rs** — エンコーディング検出
- **arboard 3** — クリップボード操作
- **serde + serde_json** — 設定の永続化
- **dirs 6** — 設定ファイルパス取得
- **open 5** — 外部エディタ / デフォルトアプリでファイルを開く
- **regex 1** — 行フィルタ・置換フィルタの正規表現

## 既知の問題・制限

1. **シンタックスハイライトが行単位** — Slint の `Text` がリッチテキスト非対応のため
2. **ドラッグ＆ドロップ非対応** — Slint 1.15 が外部ファイル D&D をサポートしていない
3. **ワードレベル差分の表示** — リッチテキスト非対応のため、変更文字は別行で表示

---

## 次にやるべき項目 (v0.21.0+)

1. **ドラッグ＆ドロップ** — Slint の D&D サポート待ち
2. **差分コメント** — 差分ブロックにメモを付ける機能
3. **比較結果フィルタ** — diff view でステータスを絞り込み表示（差分のみ/追加のみ等）

## ビルド・テスト手順

```bash
asdf install                             # Rust 1.94.0 をインストール
cargo build                              # ビルド
cargo test                               # 16件のテスト実行
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
