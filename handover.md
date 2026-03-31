# WinXMerge ハンドオーバードキュメント

## プロジェクト概要

WinMerge にインスパイアされた、Rust + Slint UI によるクロスプラットフォームのファイル差分比較・マージツール。
GitHub: `https://github.com/masak1yu/winxmerge`
Web アプリ: `https://winxmerge.app`

## 現在の状態

- **バージョン:** 0.31.0 (リリース済み) / v0.32.0 開発中
- **ブランチ:** `feature/v0.32.0`
- **CI:** GitHub Actions（Ubuntu / macOS (aarch64) / Windows）+ Cloudflare Pages (WASM)

## v0.32.0 の変更内容（開発中）

### WASM 版
- [x] WASM フルスクリーン対応 — MutationObserver で canvas style 変化を検知して resize dispatch
- [x] ダークテーマ強制 — `Palette.color-scheme = ColorScheme.dark`
- [x] UI レイアウト改善 — ヘッダー、パネルタイトルバー、区切り線
- [x] Paste ボタン — クリップボードから直接テキスト入力（`web_sys_unstable_apis` 必要）
- [x] diff 統計表示 — ナビバー右端に `+N / -N / ~N`
- [x] ウィンドウリサイズ対応 — resize イベントで `set_size(Logical)` 呼び出し

### ドキュメント
- [x] `doc/slint.md` — Slint WASM 固有の挙動と対処法まとめ

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
| v0.31.0 | #34 | WASM 表示修正（lib+bin ハイブリッド、canvas DOM 挿入） |
| v0.30.0 | #33 | WASM ファイルアップロード、diff ナビゲーション |
| v0.29.0 | #32 | WASM scaffold（Cloudflare Pages デプロイ） |
| v0.28.0 | #31 | CSV/TSV テーブル比較、オレンジ diff ハイライト |
| v0.27.0 | — | Diff Stats バーグラフ、キーボードショートカットダイアログ |
| v0.26.0 | — | Diff コメント、HTML/Excel エクスポートへのコメント埋め込み |
| v0.25.0 | — | 画像比較（ピクセルレベル diff、ブレンドスライダー） |
| v0.24.0 | — | Excel/スプレッドシート比較 |
| v0.23.0 | — | ZIP アーカイブ比較 |
| v0.22.0 | — | 内蔵ファイルブラウザ（WSL2 対応） |

## デスクトップ版 未実装機能（優先順位順）

### HIGH
1. バイナリ/Hex 比較
2. プロジェクトファイル保存/読み込み
3. CLI オプション拡充（`/e`, `/x`, `/wl`, `/wr` など）
4. diff アルゴリズム選択（Histogram / Minimal）
5. フォルダ比較：リネーム/移動検出
6. フォルダ比較：ツリービュー（展開/折りたたみ）

### MEDIUM
7. 数値無視オプション
8. コメント無視オプション
9. diff 番号による直接移動
10. 行揃え（Align Similar Lines）
