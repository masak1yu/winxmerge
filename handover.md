# WinXMerge ハンドオーバードキュメント

## プロジェクト概要

WinMerge にインスパイアされた、Rust + Slint UI によるクロスプラットフォームのファイル差分比較・マージツール。
GitHub: `git@github.com:masak1yu/winxmerge.git`

## 現在の状態

- **バージョン:** 0.3.0
- **ブランチ:** `main` にマージ済み、`v0.4.0` ブランチ作成済み（作業開始前）
- **テスト:** 11件すべてパス
- **ビルド:** `cargo build` 成功（three_way の dead code warning あり — UI未統合のため）

## リリース履歴

| バージョン | PR | 内容 |
|-----------|-----|------|
| v0.1.0 | #1 | Phase 1-4: 差分表示、マージ、フォルダ比較、メニュー、エンコーディング検出、検索、diff オプション |
| v0.2.0 | #2 | 選択ダイアログ、未保存確認、検索置換、行移動検出、タブ、シンタックスハイライト |
| v0.3.0 | #3 | Undo/Redo、キーボードショートカット、3-way マージエンジン、パフォーマンス最適化 |

## アーキテクチャ

### Rust 側 (src/)

| ファイル | 役割 |
|---------|------|
| `main.rs` | エントリーポイント。Slint コールバックと AppState の接続 |
| `app.rs` | アプリケーション状態管理（**TabState** per-tab + **AppState** タブマネージャー）。全 UI 操作のロジック |
| `diff/engine.rs` | 2-way 差分計算。`similar` crate 使用。空白/大文字無視、行移動検出、大ファイル最適化 |
| `diff/three_way.rs` | 3-way マージエンジン。衝突検出・自動マージ。**UI 未統合** |
| `diff/folder.rs` | フォルダ再帰比較 |
| `encoding.rs` | `chardetng` + `encoding_rs` によるエンコーディング検出・変換。BOM 対応 |
| `highlight.rs` | tree-sitter によるシンタックスハイライト。行の主要トークンで色決定 |
| `models/diff_line.rs` | DiffLine, DiffResult, LineStatus（Equal/Added/Removed/Modified/Moved） |
| `models/folder_item.rs` | FolderItem, FileCompareStatus |

### Slint UI 側 (ui/)

| ファイル | 役割 |
|---------|------|
| `main.slint` | メインウィンドウ。メニューバー、ツールバー、検索バー、確認ダイアログ、FocusScope（ショートカット） |
| `widgets/diff-view.slint` | 差分2ペイン表示 + マージボタン列 + ロケーションペイン |
| `widgets/folder-view.slint` | フォルダ比較リスト表示 |
| `widgets/tab-bar.slint` | タブバー（アクティブ表示、未保存マーク、閉じるボタン、+ボタン） |
| `dialogs/open-dialog.slint` | 初期選択ダイアログ（ファイル/フォルダモード切替） |

### 状態管理の構造

```
AppState
├── tabs: Vec<TabState>     # 各タブが独立した比較状態
├── active_tab: usize
│
└── TabState
    ├── left_path / right_path          # ファイルパス
    ├── diff_positions / current_diff   # 差分ナビゲーション
    ├── left_lines / right_lines        # テキスト内容
    ├── undo_stack / redo_stack         # TextSnapshot のスタック
    ├── diff_options                    # 空白/大文字無視
    ├── search_matches                  # 検索結果
    ├── view_mode                       # 0=diff, 1=folder, 2=open dialog
    ├── diff_line_data                  # UI 表示用キャッシュ
    ├── folder_items / folder_item_data # フォルダ比較結果
    └── left_encoding / right_encoding  # 検出されたエンコーディング
```

### 重要な設計判断

- **シンタックスハイライト:** Slint の `Text` がリッチテキスト非対応のため、行全体を1色（行の主要トークン）で色付け。トークン単位の色分けは不可。
- **3-way マージ:** エンジンは実装済みだが、UI（3ペイン表示）は未統合。`diff/three_way.rs` に衝突検出ロジックがある。
- **Undo/Redo:** マージ操作前に左右テキストのスナップショットをスタックに保存。新しいマージ操作で redo スタックはクリアされる。
- **大ファイル対応:** 10,000行超で Patience アルゴリズムに切替、5秒タイムアウト。

## 技術スタック

- **Rust 1.94.0**（asdf 管理、`.tool-versions` あり）
- **Slint 1.15.1** — UI フレームワーク
- **similar 2.6** — 差分アルゴリズム
- **tree-sitter 0.26** + tree-sitter-highlight — シンタックスハイライト
- **rfd 0.15** — ネイティブファイルダイアログ
- **chardetng + encoding_rs** — エンコーディング検出

## 既知の問題・制限

1. **three_way.rs の dead code warnings** — UI 統合されていないため。`#[allow(dead_code)]` を付けるか UI 統合すれば解消
2. **シンタックスハイライトが行単位** — Slint の制約。将来 Slint がリッチテキスト対応すれば改善可能
3. **左右スクロール非同期** — 2つの `ListView` が独立しており、スクロール位置の同期が未実装
4. **フォルダ比較からファイル比較に遷移後の「戻る」** — `back-to-folder-view` コールバックが定義されているが UI からのトリガーが不完全

---

## v0.4.0 で取り組むべき項目

### 優先度高

1. **3-way マージ UI 統合**
   - `three_way.rs` のエンジンは完成済み
   - 3ペイン表示の `diff-view-3way.slint` を新規作成
   - `view_mode = 3` を追加
   - 選択ダイアログにベースファイル入力欄を追加
   - 衝突行のハイライト（赤系）と解決操作（左を採用/右を採用/手動編集）

2. **左右スクロール同期**
   - 現在の `ListView` × 2 は各自独立スクロール
   - Slint の `ListView` の `viewport-y` プロパティを双方向バインドで同期
   - マージボタン列の `ListView` も同期対象

3. **フォルダ比較の「戻る」ボタン**
   - フォルダ比較からファイル差分に入った後、フォルダビューに戻る導線
   - ツールバーに「← Back to folder」ボタンを `view_mode == 0` かつ `left_folder.is_some()` の場合に表示

### 優先度中

4. **ドラッグ＆ドロップでファイルを開く**
   - Slint の `DropEvent` を使ってファイルパスを受け取り比較開始
   - 選択ダイアログとツールバーの両方で対応

5. **行番号クリックで差分選択**
   - 行番号エリアに `TouchArea` を追加
   - クリックした行が差分ブロックなら `current_diff` を更新

6. **設定の永続化**
   - ウィンドウサイズ・位置の保存
   - 最近開いたファイルの履歴
   - diff オプション（ignore_whitespace 等）のデフォルト値

7. **言語サポートの拡充**
   - tree-sitter-typescript, tree-sitter-go, tree-sitter-cpp の追加
   - `highlight.rs` の `get_highlight_config` にエントリ追加するだけ

### 優先度低

8. **行内（文字レベル）差分のハイライト**
   - Modified 行の中で具体的にどの文字が変わったかを `similar` の文字レベル diff で計算
   - 現状の `DiffLineData` に文字位置情報を追加し、Slint 側で部分的に背景色を変える（実装難易度高）

9. **印刷 / HTML エクスポート**
   - 差分結果を HTML に出力
   - 印刷用スタイルシート付き

10. **プラグインシステム**
    - カスタム差分フィルタ（前処理）
    - 外部ツール連携

## ビルド・テスト手順

```bash
asdf install          # Rust 1.94.0 をインストール
cargo build           # ビルド
cargo test            # 11件のテスト実行
cargo run             # アプリ起動
```
