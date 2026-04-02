# WinMerge Editor Specification

WinMerge (C++/MFC) のインラインエディタの仕様をソースコードから解析した結果。

## 1. Ghost Line (幽霊行) アーキテクチャ

### 概要
- 2つのペインの行数が異なる場合、少ない側に **Ghost Line** (空行プレースホルダー) を挿入して視覚的に整列させる
- Ghost Line には `LF_GHOST` フラグ (0x00400000UL) が付与される
- Ghost Line はバッファ上は実在する空行だが、ファイル保存時には除外される

### 行番号のマッピング
- **Apparent Line** (見かけの行番号): Ghost Line を含む画面上の行番号
- **Real Line** (実行番号): Ghost Line を除いた実際のファイル行番号
- `RealityBlock` 構造体で変換テーブルを管理
  - `{nStartReal, nStartApparent, nCount}` の配列
  - バイナリサーチで高速変換

### 例
```
ファイル内容: A, B, C (3行)
Diff結果で2行のGhost追加後:
  Apparent[0]=A (Real 0), Apparent[1]=B (Real 1),
  Apparent[2]=Ghost, Apparent[3]=Ghost,
  Apparent[4]=C (Real 2)
```

## 2. 編集動作

### テキスト入力 (InsertText)
1. **通常行に入力**: その行のテキストを変更。他ペインに影響なし
2. **Ghost行に入力**: Ghost行が実行に変換される。挿入テキストの行数分だけ後続のGhost行が削除される
3. `RecomputeRealityMapping()` で行番号マッピングを再計算
4. 行のリビジョン番号を更新 (変更追跡用)

### Enter キー
1. 現在行をカーソル位置で分割し、新しい実行を作成
2. Ghost行上でEnter → Ghost行が実行に変換される
3. `OnEditOperation(CE_ACTION_TYPING)` が発火
4. リスキャンタイマーが開始される

### Backspace / Delete キー
1. 現在のペインのバッファのみを変更
2. Ghost行に隣接する削除は、Ghost行も含めて処理
3. 行がマージされた場合は `RecomputeRealityMapping()` を呼ぶ

### Tab キー
- 通常のテキストエディタと同じ動作 (タブ文字またはスペースを挿入)

### **重要**: 他ペインへの影響
- **編集は現在のペインのバッファのみを変更する**
- 他ペインはリスキャンが実行されるまで一切変わらない
- Ghost行はペインごとにローカル

## 3. リスキャン (再比較)

### 自動リスキャン (`m_bAutomaticRescan = true`)
1. 編集操作発生 → `RESCAN_TIMEOUT` (~500ms) タイマー開始
2. タイマー発火 → `FLAG_RESCAN_WAITS_FOR_IDLE` をセット
3. アプリがアイドル状態になったら `RescanIfNeeded()` を実行
4. 実質的に **入力停止後 ~500ms で自動リスキャン**

### 手動リスキャン (F5)
- `FlushAndRescan(true)` を即座に実行

### FlushAndRescan の処理
```
1. 全ビューのカーソル位置を保存 (PushCursors: Real Line + Ghost Offset)
2. 全Ghost行を削除 (RemoveAllGhostLines)
3. 両ファイルの実テキストでDiffアルゴリズム実行
4. 新しいDiff結果に基づいてGhost行を再挿入 (PrimeTextBuffers)
5. カーソル位置を復元 (PopCursors)
6. 全ビューを更新
```

### PrimeTextBuffers (Ghost行再挿入)
- Diff結果リストを **後ろから前に** 処理
- 各Diffブロックで、行数が少ないペインにGhost行を挿入
- 例: 左ペインが3行、右ペインが5行 → 左に2行のGhost追加
- 各行に `LF_DIFF`, `LF_TRIVIAL`, `LF_MOVED` 等のフラグを設定

## 4. カーソル保持

### リスキャン前後のカーソル位置保存
```cpp
struct SCursorPushed {
    int x;              // 文字位置
    int y;              // Real Line 番号 (Apparent ではない)
    int nToFirstReal;   // Ghost行上の場合、次の実行までの距離
};
```
- Push: Apparent → Real + Ghost Offset に変換して保存
- Pop: Real + Ghost Offset → 新しい Apparent に変換して復元

## 5. スクロール同期

- ペイン間のスクロールは `UpdateSiblingScrollPos()` で同期
- **行インデックスベース** (Ghost行込み) で同期するため、自然に整列される
- 両ペインは常に同じ行数 (Ghost行込み) を持つ

## 6. 操作まとめ

| 操作 | 影響範囲 | リスキャン | Ghost行への影響 |
|------|---------|-----------|---------------|
| テキスト入力 | 現在のペインのみ | ~500ms後に自動 | リスキャンまで変化なし |
| Enter | 現在のペインのみ | ~500ms後に自動 | リスキャンまで変化なし |
| Backspace/Delete | 現在のペインのみ | ~500ms後に自動 | リスキャンまで変化なし |
| F5 (手動リスキャン) | 両ペイン再構築 | 即座 | 全削除→再挿入 |

## 7. 核心的な設計思想

1. **各ペインは独立したテキストバッファを持つ** (CGhostTextBuffer)
2. **編集は片方のペインのバッファのみを変更する**
3. **Ghost行はDiff結果の視覚化のためだけに存在する**
4. **編集後は一定時間で自動リスキャンし、Ghost行を再計算する**
5. **リスキャン時に全Ghost行を削除→Diff→再挿入のサイクルを回す**
