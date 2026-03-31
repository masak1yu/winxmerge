# WinXMerge ハンドオーバードキュメント

## プロジェクト概要

WinMerge にインスパイアされた、Rust + Slint UI によるクロスプラットフォームのファイル差分比較・マージツール。
GitHub: `git@github.com:masak1yu/winxmerge.git`

## 現在の状態

- **バージョン:** 0.29.0 (デスクトップ) / v0.30.0 開発中
- **ブランチ:** `feature/v0.30.0`
- **テスト:** 19件すべてパス
- **ビルド:** `cargo build` 成功
- **CI:** GitHub Actions（Ubuntu / macOS (aarch64) / Windows）+ Cloudflare Pages (WASM)
- **Webアプリ:** https://winxmerge.pages.dev/

## 次バージョン (v0.30.0) の予定

### WASM版機能拡張（実装中）
- [x] **表示バグ修正** — `#[wasm_bindgen(start)]` を `wasm_entry` にリネームして blank screen 解消
- [x] **ファイルアップロード** — ブラウザ File API (`Blob.text()`) でローカルファイルを左右ペインに読み込み
- [x] **差分ナビゲーション** — Prev/Next ボタン、diff カウンタ表示、現在 diff のハイライト + スクロール

### デスクトップ版（ペンディング）
WinMergeマニュアルとの機能ギャップ調査（2026-03-29）に基づく優先順位:

#### HIGH PRIORITY（コア機能ギャップ）
1. **バイナリ/Hex比較** — 16進数ビューアでのバイナリファイル比較、バイト単位差分表示
2. **プロジェクトファイル** — よく使うパス・オプションを .json で保存/読み込み
3. **CLIオプション拡充** — `/e`, `/x`, `/wl`, `/wr`, `/enableexitcode` 等
4. **差分アルゴリズム選択** — Histogram / Minimal アルゴリズム追加（現在: Myers/Patience のみ）
5. **フォルダ比較: リネーム/移動検出** — リネームされたファイルを同一として検出
6. **フォルダ比較: ツリービュー** — 展開/折りたたみ可能なツリー表示（現状フラットリストのみ）

#### MEDIUM PRIORITY
7. 数値無視オプション (`Ignore numbers`)
8. コメント無視オプション (`Ignore comment differences`)
9. 差分番号による直接移動 ("Go to Difference #N")
10. 行揃え (Align Similar Lines) — 類似行に空白行を自動挿入

詳細は `/home/vscode/.claude/projects/-workspaces-winxmerge/memory/project_unimplemented_features.md` 参照。

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
| v0.20.0 | #22 | ステータス別差分ナビゲーション、複数行選択+ブロックコピー、CSV/TSVエクスポート+フォルダHTMLレポート、クリップボード比較、タブ並び替え、フォルダ比較サマリー |
| v0.21.0 | #23 | ZIPアーカイブ比較（仮想フォルダビューでエントリ差分表示）、Excelファイル比較（テーブルビューでセル差分表示） |
| v0.22.0 | #24 | 画像比較（ピクセルレベル差分、左右サイドバイサイド＋差分オーバーレイ表示） |
| v0.23.0 | #25 | 差分コメント、比較結果ステータスフィルタ、クリップボードパス貼り付け、画像比較ズーム＋差分パネル切替、フォルダ比較ファイルプレビュー |
| v0.24.0 | - | アプリアイコン、差分コメントHTMLエクスポート、画像連続ズームスライダー、差分コメントセッション保存、フォルダステータスフィルタUI、ショートカットダイアログ、セッション保存改善、差分統計ミニグラフ |
| v0.25.0 | - | 差分詳細ペイン改善（現在ブロックのみ表示・片方のみ対応・文字レベルハイライト）、View メニュー拡張（Zoom In/Out/Reset・行折り返し）、ウィンドウリサイズ対応、マージ列削除、GitHub Actions リリースワークフロー追加 |
| v0.26.0 | - | UIアイコン修正（📋ペーストボタン→SVGアイコン化、フォルダ一覧ファイルアイコン→SVG化）、ファイルブラウザフォーカスバグ修正、リリース対象からmacOS Intel除外 |
| v0.27.0 | - | Excel差分エクスポート（.xlsx、色分け+コメント列）、差分コメント印刷対応、フォルダ比較列ヘッダソート、画像比較オーバーレイ透明度スライダー、全タブコメント一括エクスポート（CSV/JSON） |
| v0.28.0 | #31 | CSV/TSV比較（セル単位差分テーブル表示）、Diffハイライト色をオレンジに変更（本家WinMerge準拠）、ビルド後比較履歴自動クリア（`--clear-history`フラグ）、Diffスクロール修正（フォントサイズ対応）、比較選択ダイアログスクロール対応 |
| v0.29.0 | #32 | WASM対応スキャフォールド — Cloudflare Pages向けWebビュー初期実装（テキスト貼り付け→diff表示）、デスクトップ版との条件コンパイル分離（`cfg(not(wasm32))`）、trunk/wasm-bindgen対応 |

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
- Linux (x86_64), macOS (aarch64), Windows (x86_64)
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
- **ZIPアーカイブ比較** — `.zip` ファイルを仮想フォルダとして比較。CRC+サイズ一致チェック、エントリの追加/削除/変更を FolderView に表示
- **Excelファイル比較** — `.xlsx/.xls/.xlsm/.ods` を calamine で読み込み、セル値を差分比較。変更セルのみを ExcelView（テーブルビュー）に表示、シートセレクタ付き
- **画像比較** — PNG/JPEG/GIF/BMP/WebP/TIFF/ICO をピクセルレベルで比較（`image` クレート使用）。左右サイドバイサイド＋差分オーバーレイ（変更画素=赤、同一画素=グレースケール）を ImageView（view_mode=5）に表示。変更画素数・率をステータスバーに表示
- **差分コメント** — 差分ブロックごとにメモを記入（詳細ペイン下部の Note フィールド）。タブ切替・ナビゲーション時に自動ロード
- **比較結果ステータスフィルタ** — diff view 上部のフィルタバーで All/Added/Removed/Modified/Moved を絞り込み表示
- **クリップボードパス貼り付け** — 選択ダイアログの 📋 ボタンでクリップボードのファイルパス（file:// URI も対応）を左右パス欄に貼り付け
- **画像比較ズーム＋差分パネル切替** — Fit/100%/200% ズームボタン（100%/200% 時は ScrollView）、"Diff Panel" トグルで差分オーバーレイパネルの表示切替
- **フォルダ比較ファイルプレビュー** — フォルダビューでアイテムをクリックすると下部プレビューパネルに左右ファイルの先頭20行を表示
- **アプリアイコン** — "Diff Panels" デザインの SVG アイコン（`assets/icons/`）。Windows 実行ファイル埋め込み（`.ico`）、macOS バンドル（`.icns`）、Slint ウィンドウアイコン（`app-icon-256.png`）
- **差分コメントの HTML エクスポート** — `export_html()` が `comments: &HashMap<usize, String>` を受け取り、差分ブロック後にコメント行（黄色ハイライト）を出力
- **画像比較の連続ズームスライダー** — `zoom-percent: float`（0=Fit, 10–400）+ Slider ウィジェット（`std-widgets` の `Slider`）。Fit ボタンで zoom-percent=0
- **差分コメントのセッション保存** — `SessionEntry` に `diff_comments: Vec<SessionComment>` 追加。起動時にコメントを復元
- **フォルダ比較ステータスフィルタ UI** — `FolderView` 上部に All/Identical/Different/Left only/Right only フィルタバー。フィルタ外の行は `height: 0px` で非表示
- **キーボードショートカット一覧ダイアログ** — `ui/dialogs/shortcuts-dialog.slint` 新規作成。Help メニュー → "Keyboard Shortcuts" で表示。File/Navigation/Merge/View セクション
- **タブのセッション保存改善** — `SessionEntry` に `left_encoding`, `right_encoding`, `left_eol`, `right_eol`, `tab_width`, `diff_only`, `diff_status_filter` 追加。復元時に TabState へ反映
- **差分統計ミニグラフ表示** — ステータスバーに緑/赤/黄の比例バー（`horizontal-stretch` 使用）を追加。`parse_diff_stats()` + `sync_diff_stats()` ヘルパーで `+A -R ~M` テキストと同時に3プロパティを更新
- **Excelエクスポート** — `rust_xlsxwriter` で `.xlsx` 生成。ヘッダ固定・オートフィルタ・差分色付き（追加=緑/削除=赤/変更=黄/移動=青）・コメント列付き
- **差分コメント印刷対応** — `export_html_for_print` に `comments` を渡すよう修正。印刷HTMLにもコメント行を含める
- **フォルダ比較列ヘッダソート** — Name/Status/Left Size/Right Size/Left Modified/Right Modified の各列ヘッダをクリックでソート。同列再クリックで昇順↔降順切り替え（▲▼インジケーター表示）
- **画像比較オーバーレイ透明度スライダー** — Blend スライダー（0〜100%）で変更画素（赤）を左右パネル上にオーバーレイ表示。fit/zoom 両モード対応。`overlay_rgba` は同一画素をα=0（透明）で生成
- **全タブコメント一括エクスポート** — File → Export All Comments (CSV/JSON) で全タブの差分コメントをタブタイトル・ファイルパス・差分ブロック番号付きで出力

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
| `archive.rs` | ZIPアーカイブ比較（magic bytes検出、CRC+サイズベース差分） |
| `excel.rs` | Excelファイル比較（calamine使用、セル差分抽出） |
| `image_compare.rs` | 画像比較（image クレート使用、ピクセルレベル差分・RGBA バッファ生成） |

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
| `widgets/excel-view.slint` | Excelセル差分テーブルビュー（シートセレクタ + ListView） |
| `widgets/excel-view.slint` | Excelセル差分テーブルビュー（シートセレクタ + ListView） |
| `widgets/image-view.slint` | 画像比較ビュー（左右サイドバイサイド + 差分オーバーレイパネル + 連続ズームスライダー） |
| `dialogs/shortcuts-dialog.slint` | キーボードショートカット一覧ダイアログ |

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
    ├── view_mode                       # 0=diff, 1=folder, 2=open dialog, 3=3-way, 4=excel, 5=image
    ├── diff_line_data                  # UI 表示用キャッシュ
    ├── folder_items / folder_item_data # フォルダ比較結果
    ├── left_encoding / right_encoding  # 検出されたエンコーディング
    ├── left_mtime / right_mtime        # ファイル更新時刻（自動再スキャン用）
    ├── diff_comments: HashMap<usize, String>  # 差分ブロックごとのコメント
    ├── diff_status_filter: i32         # 差分フィルタ（0=All, 1=Added, 2=Removed, 3=Modified, 4=Moved）
    └── image_left_w/h, image_right_w/h # 画像サイズ（ズーム計算用）

AppSettings (永続化)
├── 比較オプション（ignore_whitespace, ignore_case, etc.）
├── エディタ設定（font_size, tab_width, etc.）
├── UI 設定（show_toolbar, enable_context_menu）
├── フィルタ設定（line_filters, substitution_filters）
├── plugins: Vec<PluginEntry>   # プラグイン定義（name, command）
├── external_editor: String     # 外部エディタコマンド
├── auto_rescan: bool           # 自動再スキャン
├── recent_files: Vec<RecentEntry>
└── session: Vec<SessionEntry>  # タブ復元（left/right/base_path + encoding/EOL/tab_width/comments）
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
- **zip 2** — ZIPアーカイブ読み込み・比較
- **calamine 0.26** — Excel/ODS ファイル読み込み（xlsx, xls, xlsm, ods）
- **image 0.25** — 画像デコード（PNG, JPEG, GIF, BMP, WebP, TIFF, ICO）

## 既知の問題・制限

1. **シンタックスハイライトが行単位** — Slint の `Text` がリッチテキスト非対応のため
2. **ドラッグ＆ドロップ非対応** — Slint 1.15 が外部ファイル D&D をサポートしていない
3. **ワードレベル差分の表示** — リッチテキスト非対応のため、変更文字は別行で表示

---

## 次にやるべき項目 (v0.28.0+)

1. **ドラッグ＆ドロップ** — Slint の OS ファイル D&D サポート待ち（winit の DroppedFile が Slint 公開 API に未公開）

## ビルド・テスト手順

```bash
asdf install                             # Rust 1.94.0 をインストール
cargo build                              # ビルド
cargo test                               # 19件のテスト実行
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
