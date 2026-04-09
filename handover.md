# WinXMerge ハンドオーバードキュメント

## プロジェクト概要

WinMerge にインスパイアされた、Rust + Slint UI によるクロスプラットフォームのファイル差分比較・マージツール。
GitHub: `https://github.com/masak1yu/winxmerge`
Web アプリ: `https://winxmerge.app`

## 現在の状態

- **バージョン:** 0.34.2
- **ブランチ:** `feature/v0.34.2`
- **テスト:** 29件すべてパス
- **ビルド:** `cargo build --features desktop` 成功
- **CI:** GitHub Actions（Ubuntu / macOS (aarch64) / Windows）+ Cloudflare Pages (WASM)

## v0.34.2 の変更内容

### IPC 仮想フォルダ比較
- `git difftool` で複数ファイルを受信した場合、100+タブではなく仮想フォルダ比較（view_mode=1）として1タブで表示
- 500ms デバウンスで IPC ペアをバッファリング
- 1ペアのみ → 従来通り 2-way diff タブ
- 2ペア以上 → 仮想フォルダ比較
- フォルダビューでダブルクリック → 新規タブで詳細 diff
- Back ボタンはキャッシュからフォルダ復元（ディスク再スキャンなし）

### 3-way diff 検索機能
- 3-way ビューで Cmd+F → 検索バー表示
- left/base/right 全ペインでマッチハイライト（黄色）
- Next/Prev ナビゲーション、Replace / Replace All 対応

### Auto-rescan（外部ファイル変更検知）
- 500ms ポーリングで外部ファイル変更を検知
- 未保存の編集がある間はリロードをスキップ（ユーザー編集を絶対に消さない）
- Options → Apply 時も未保存編集を保護

### 検索バー SVG アイコン
- Unicode ▲▼✕ がレンダリングされない問題を SVG アイコンに置換

### パスバー即時表示
- 片側のみファイル選択時もヘッダーバーにパスを即表示
- view_mode 7（ブランク画面）から 0 への自動切替

### i18n 翻訳拡充
- 89件の未翻訳 @tr() 文字列を日本語翻訳追加
- セーブ確認ダイアログ、メニュー、フィルタバー、ショートカット、CSV/Excel/画像ビュー等

## v0.34.1 の変更内容

### IPC によるシングルインスタンス・タブ追加
- Unix domain socket (`/tmp/winxmerge-<user>.sock`) による IPC
- 2回目以降の起動は既存インスタンスにファイルパスを送信して即終了
- 既存インスタンスが新しいタブとして diff を開く
- `git difftool` で複数ファイル差分が1ウィンドウのタブとして表示可能に

## v0.33.0 の変更内容

### インライン編集（WinMerge 仕様準拠）
- [x] `editing_mode` 廃止 — 常に Ghost 行込み整列ビューで編集
- [x] Enter で行挿入 — Ghost 行付きで整列維持、`left_lines`/`right_lines` 同期
- [x] Backspace で空行削除 — `left_lines`/`right_lines` 同期 + `editing_dirty` 設定
- [x] 矢印キー (Up/Down) でフォーカス移動（Ghost 行スキップ）
- [x] Undo スナップショット統一（insert/delete でも push_undo_snapshot）

### Copy Left/Right バグ修正
- [x] `is_current_diff`（常に false）→ `diff_index == target_diff_index` で判定に修正
- [x] `status=2` (Removed) の Copy Left→Right で行が消えるバグ修正

### 新規ドキュメント
- [x] File → New → Text: 空の 2-way 比較ドキュメント作成
- [x] File → New → Table: 空の CSV/TSV 比較ドキュメント作成
- [x] File → New (3-pane) → Text / Table: 空の 3-way 比較作成
- [x] ツールバー New ボタンにドロップダウンで種類選択
- [x] `left_lines`/`right_lines` 初期化修正（リスキャンで文字消え���バグ修正）

### リスキャン修正
- [x] 編集中・新規テキストは常に VecModel（画面表示内容）から再構築
- [x] ファイル有りかつ未編集のみディスク再読み込み

### Save As ダイアログ（WinMerge 風）
- [x] rfd ネイティブダイアログ → in-app ファイルブラウザに置き換え（WSL2 対応）
- [x] ディレク��リ一覧 + ファイル名入力バー
- [x] デフォルトディレクトリ = カレントディレクトリ

### 終了時保存導線（WinMerge 仕様）
- [x] Save(S) / Don't Save(D) / Cancel の 3 択
- [x] Cancel → 終了自体をキャンセル（アプリに戻る）
- [x] Save As でキャンセル → 終了キャンセル
- [x] `has_unsaved_changes` がキャンセル時に消えるバグ修正

### UI 改善
- [x] `view_mode=7`（ブランク開始画面）追加
- [x] メニュー再構成（File → New → Text/Table）
- [x] Open ダイアログをモーダル化
- [x] `SaveConfirmDialog`, `NewTableDialog` 追加
- [x] 3-way 編集コールバック追加

## クレート構成（重要）

lib + bin ハイブリッド構成：
- `src/lib.rs` — WASM エントリポイント（`#[wasm_bindgen(start)]`）
- `src/main.rs` — デスクトップ専用バイナリ
- `Cargo.toml` の `[[bin]]` に `required-features = ["desktop"]` → trunk が bin をスキップし cdylib をビルド
- ネイティブ CI は `--features desktop` を付ける

## WASM ビルド

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk
trunk serve        # 開発サーバー (localhost:8080)
trunk build --release  # 本番ビルド → dist/
```

`RUSTFLAGS=--cfg=web_sys_unstable_apis` が必要（Trunk.toml に設定済み）。

詳細は `doc/slint.md` 参照。

## リリース履歴

| バージョン | PR | 内容 |
|-----------|-----|------|
| v0.34.2 | #39 | IPC 仮想フォルダ比較、3-way 検索、auto-rescan、i18n 翻訳拡充 |
| v0.34.1 | #38 | IPC シングルインスタンス — git difftool でタブ式に複数ファイル差分を開く |
| v0.34.0 | #37 | 3-way diff エンジン全面書き換え、コピー/F5 バグ修正、保存ドロップダウン |
| v0.33.0 | #36 | インライン編集、新規ドキュメント、Save As ファイルブラウザ、終了時保存導線 |
| v0.32.0 | #35 | WASM フルスクリーン、ダークテーマ、ペースト、diff 統計 |
| v0.31.0 | #34 | WASM 表示修正（lib+bin ハイブリッド、canvas DOM 挿入） |
| v0.30.0 | #33 | WASM ファイルアップロード、diff ナビゲーション |
| v0.29.0 | #32 | WASM scaffold（Cloudflare Pages デプロイ） |
| v0.28.0 | #31 | CSV/TSV テーブル比較、オレンジ diff ハイライト |

## デスクトップ版 未実装機能（優先順位順）

### HIGH
1. バイナリ/Hex 比較
3. プロジェクトファイル保存/読み込み
4. CLI オプション拡充（`/e`, `/x`, `/wl`, `/wr` など）
5. diff アルゴリズム選択（Histogram / Minimal）
6. フォルダ比較：リネーム/移動検出
7. フォルダ比較：ツリービュー（展開/折りたたみ）

### MEDIUM
8. Tab 文字挿入（Slint `insert-text()` API 追加待ち）
9. 数値無視オプション
10. コメント無視オプション
11. diff 番号による直接移動
12. 行揃え（Align Similar Lines）
