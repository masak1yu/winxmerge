# WinMerge vs WinXMerge エディタ仕様差異と対応方針

## 差異一覧

| # | 項目 | WinMerge | WinXMerge (現状) | 差異 |
|---|------|----------|-----------------|------|
| 1 | 編集ビュー | 常にGhost行込み整列ビュー | editing_mode=trueで独立ビューに切替 | **重大** |
| 2 | Enter後の整列 | Ghost行付き整列を維持 | 独立ビューに切替→整列崩壊 | **重大** |
| 3 | 自動リスキャン | 編集停止~500ms後に自動実行 | なし (手動F5のみ) | **中** |
| 4 | Ghost行の表現 | 専用フラグ(LF_GHOST)で管理 | status=1/2の行で空側がGhost相当 | 機能的に等価 |
| 5 | 行番号マッピング | Real/Apparent変換テーブル | line_noフィールドで直接管理 | 機能的に等価 |
| 6 | Enter動作 | 行分割+Ghost行維持 | left_lines挿入→独立ビュー | **重大** |
| 7 | Backspace動作 | 行結合+Ghost行維持 | 空行のみ削除、同期不完全 | **重大** |
| 8 | Tab動作 | タブ文字挿入 | キーイベント消費のみ (挿入不可) | Slint制限 |
| 9 | テキスト入力 | バッファ直接編集 | DiffLineData更新+left_lines同期 | 機能的に等価 |
| 10 | カーソル保持 | Real Line+Ghost Offsetで保存/復元 | edit-focus-rowで管理 | 簡易版 |
| 11 | Undo (編集モード) | 全操作でUndo可能 | 編集モードではUndo未対応 | **中** |
| 12 | delete時のleft_lines同期 | バッファ直接操作 | Diffモデルのみ削除、left_lines未同期 | **重大** |

## 対応方針

### 方針: WinMerge仕様に準拠 (Slint制限のみ妥協)

---

### 対応1: editing_modeを廃止し、常にDiffビューで編集 [重大]

**WinMerge仕様**: 編集は常にGhost行込みの整列ビューで行う。独立編集モードは存在しない。

**対応**:
- `editing_mode` フラグ、`EditLine` 構造体、`left-edit-lines`/`right-edit-lines` を全て削除
- `edit-left-list`/`edit-right-list` ListView を削除
- 全ての編集は常にDiffLineData VecModel上で行う
- left_lines/right_lines は常にDiffLineDataと同期する

---

### 対応2: Enterで行挿入 → Ghost行付きで整列維持 [重大]

**WinMerge仕様**: Enter押下で行を分割。他ペインにはGhost行が自動挿入され整列を維持。リスキャン後にGhost行が再計算される。

**対応**:
- `insert_line_after(is_left=true)` の動作を変更:
  1. DiffLineData VecModel に新行を挿入
  2. 左ペイン挿入時: `status=2 (Removed)`, `left_line_no=N`, `right_line_no=""`
  3. 右ペイン挿入時: `status=1 (Added)`, `right_line_no=N`, `left_line_no=""`
  4. **これは以前の実装と同じだが、editing_modeに遷移しない**
  5. left_lines/right_lines も同時に更新
  6. 行番号を再計算
  7. `editing_dirty=true` をセット

- **行位置のズレについて**: WinMerge でも Insert 後は他ペインにGhost行が入り後続行はずれる。
  これは正常な動作。ただし自動リスキャン(対応3)によりすぐに再整列される。

---

### 対応3: 自動リスキャン (遅延実行) [中]

**WinMerge仕様**: 編集停止後~500msで自動リスキャン。

**対応**:
- Slintの`Timer`を使用して遅延リスキャンを実装
- 編集操作 (edit_line, insert_line_after, delete_line) のたびにタイマーをリセット
- 500ms間操作がなければ `rescan()` を自動実行
- `editing_dirty` フラグは不要になる (常に自動リスキャンされるため)
- ステータスバーのメッセージも不要に

**Slint Timer の使い方**:
```rust
let timer = slint::Timer::default();
timer.start(slint::TimerMode::SingleShot, Duration::from_millis(500), move || {
    rescan(&window, &mut state.borrow_mut());
});
```

---

### 対応4: Backspaceで行結合 + left_lines同期 [重大]

**WinMerge仕様**: Backspaceは現在行が空なら削除、空でなければ前行と結合。

**対応**:
- `delete_line` を修正:
  1. 空行の場合: VecModel から行を削除 + left_lines/right_lines からも削除
  2. 非空行の場合 (cursor at position 0): 前行のテキストと結合
  3. 行番号を再計算
  4. 自動リスキャンタイマーをリセット

---

### 対応5: Tab文字挿入 [Slint制限により妥協]

**WinMerge仕様**: Tab キーでタブ文字 (またはスペース) を挿入。

**Slint制限**: Slint 1.15.1 には `TextInput.insert-text()` メソッドがない。

**妥協案**:
- Tab キーのイベントは消費 (フォーカス移動防止) するが、文字挿入はできない
- Slint のバージョンアップで `insert-text()` が追加されたら対応
- 代替: Ctrl+Tab や別のキーバインドでの挿入を検討 (優先度低)

---

### 対応6: Undo対応の統一 [中]

**対応**:
- insert_line_after, delete_line でも push_undo_snapshot() を呼ぶ
- editing_mode 廃止により、全操作がDiffモデル上で行われるため統一される

---

## 実装順序

1. **editing_mode 廃止** — EditLine, edit-left-list, edit-right-list を削除
2. **insert_line_after 修正** — Ghost行付き整列+left_lines同期
3. **delete_line 修正** — left_lines同期+editing_dirty設定
4. **自動リスキャンタイマー** — 500ms遅延リスキャン実装
5. **Undo統一** — insert/delete にスナップショット追加
6. (将来) Tab文字挿入 — Slint API追加待ち
