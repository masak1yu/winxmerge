# WinXMerge Editor Specification (現状)

WinXMerge (Rust/Slint) のインラインエディタの現状をソースコードから解析した結果。

## 1. データモデル

### DiffLineData (共有Diffモデル)
```
DiffLineData {
  left-line-no, right-line-no: string   // 行番号 ("1","2"...) or "" (Ghost行)
  left-text, right-text: string          // テキスト内容
  status: int                            // 0=Equal, 1=Added, 2=Removed, 3=Modified, 4=Moved
  ...
}
```
- 左右のペインが **同一の配列** を共有
- status=1 (Added) の行: 左ペインは空表示、右ペインにテキスト表示
- status=2 (Removed) の行: 右ペインは空表示、左ペインにテキスト表示
- これがWinMergeの「Ghost行」に相当する

### TabState (バックエンド)
```rust
left_lines: Vec<String>     // 左ファイルの実テキスト行
right_lines: Vec<String>    // 右ファイルの実テキスト行
diff_line_data: Vec<DiffLineData>  // UIモデルキャッシュ
editing_dirty: bool          // 編集済み (F5で再比較が必要)
editing_mode: bool           // 独立編集モード (現在の実装)
```

### EditLine (独立編集モデル — 現在の実装の問題の元凶)
```
EditLine {
  line-no: string,
  text: string,
}
```
- `editing_mode=true` 時に各ペインが独立したEditLine配列を使用
- 問題: Diffモデルとは完全に分離されるため、整列が崩れる

## 2. 現在の編集動作

### テキスト入力 (edit_line)
- **Diffモード** (`editing_mode=false`):
  - DiffLineData VecModelの該当行を直接更新
  - left_lines/right_lines も同時に更新
  - 他ペインには影響しない (同一行の片側のみ変更)
- **編集モード** (`editing_mode=true`):
  - left_lines/right_linesを直接更新
  - EditLine VecModelの該当行を更新
  - 他ペインには影響しない

### Enter キー (insert_line_after)
- **現在の動作**: `editing_mode=true` に遷移し、独立編集ビューに切り替え
- left_lines/right_lines に空行を挿入
- apply_edit_mode() で EditLine モデルを全再構築
- **問題点**: Diffビューから独立編集ビューへの突然の切り替えが発生

### Backspace (delete_line)
- **編集モード**: 空行のみ削除可能。left_lines/right_linesから削除
- **Diffモード**: VecModelから行を削除。但しleft_lines/right_linesは未同期

### Tab キー
- `key-pressed` でキーイベントを消費 (フォーカス移動防止)
- Slint 1.15.1 に `insert-text()` API がないため、タブ文字の挿入は未実装

## 3. リスキャン (F5)

### 処理フロー
```
rescan()
  → run_diff() or recompute_diff_from_text()
    → apply_diff_result()
      → editing_dirty = false
      → editing_mode = false
      → left_lines/right_lines を再解析
      → DiffLineData を再構築
      → diff_editing_mode = false (UIをDiffビューに戻す)
```

### 自動リスキャンは未実装
- WinMergeのような ~500ms後の自動リスキャンはない
- `editing_dirty=true` の状態で「Editing — press F5 to compare」を表示するのみ

## 4. ペイン整列

### Diffモード
- 左右のListViewが **同一のdiff-lines配列** を for ループで描画
- 全行が同じ高さ (font_size + 2 px)
- viewport-y を双方向同期 → 完全に整列

### 編集モード (editing_mode=true)
- 左右が別々のEditLine配列を描画
- 行数が異なる可能性がある → **整列が崩れる**
- viewport-y は同期しているがピクセルベースなので行の対応が取れない

## 5. 発見された問題点

### 問題1: 編集モードの導入が根本的に間違い
- WinMergeは「独立編集モード」を持たない
- WinMergeは常にGhost行込みの整列ビューで編集する
- 編集後~500msで自動リスキャンしてGhost行を再計算する
- 現在のWinXMergeの `editing_mode` は不要であり、むしろ害がある

### 問題2: Diffモードでのdelete_lineがleft_lines/right_linesと未同期
- VecModelから行を削除しても、left_lines/right_linesは更新されない
- F5でリスキャンすると削除した行が復活する

### 問題3: Diffモードでのdelete_lineでediting_dirtyが未設定
- ステータスバーに「F5で再比較」メッセージが表示されない

### 問題4: 編集モードでUndoスナップショットが取られない
- Undo/Redoスタックが不完全

### 問題5: delete_lineは空行のみ削除可能
- 内容のある行はBackspaceで削除できない

### 問題6: Tab文字の挿入が未実装
- Slint 1.15.1 の制限 (`insert-text()` API なし)
