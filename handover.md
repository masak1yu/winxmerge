# WinXMerge ハンドオーバードキュメント

## プロジェクト概要

WinMerge にインスパイアされた、Rust + Slint UI によるクロスプラットフォームのファイル差分比較・マージツール。
GitHub: `https://github.com/masak1yu/winxmerge`
Web アプリ: `https://winxmerge.app`

## 現在の状態

- **バージョン:** 0.33.0 開発中
- **ブランチ:** `v0.33.0`
- **CI:** GitHub Actions（Ubuntu / macOS (aarch64) / Windows）+ Cloudflare Pages (WASM)

## v0.33.0 の変更内容（開発中）

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
| v0.32.0 | #35 | WASM フルスクリーン、ダークテーマ、ペースト、diff 統計 |
| v0.31.0 | #34 | WASM 表示修正（lib+bin ハイブリッド、canvas DOM 挿入） |
| v0.30.0 | #33 | WASM ファイルアップロード、diff ナビゲーション |
| v0.29.0 | #32 | WASM scaffold（Cloudflare Pages デプロイ） |
| v0.28.0 | #31 | CSV/TSV テーブル比較、オレンジ diff ハイライト |

## デスクトップ版 未実装機能（優先順位順）

### HIGH
1. 自動リスキャン（編集停止 500ms 後に自動 diff 再計算）
2. バイナリ/Hex 比較
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
