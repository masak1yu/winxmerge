# WinMerge 3-Way Merge 仕様書

WinMerge のオリジナルソースコード（GitHub: WinMerge/winmerge）から解析した 3-way マージの仕様。

## 1. ビューアーキテクチャ

### ペイン構成
- `m_nBuffers` = 2 (2-way) or 3 (3-way) で切り替え
- `m_ptBuf[3]`: 左/中央/右の 3 つのテキストバッファ
- `m_pView[3][3]`: `[group][buffer]` の 2D 配列。group はメインビュー、detailビュー、追加分割
- ペイン番号: 0=左, 1=中央(Base), 2=右

### ペイン名称
- Pane 0: **Left** (Mine / Local)
- Pane 1: **Middle** (Base / Ancestor)
- Pane 2: **Right** (Theirs / Remote)

### ループ構造
全てのペイン操作は `for (nBuffer = 0; nBuffer < m_nBuffers; nBuffer++)` で回す。
2-way / 3-way でコードパスを分岐しない設計。

## 2. Diff エンジン（3-way）

### 3 組のペアワイズ比較
3-way diff は **3 組の 2-way diff** を実行して統合する：

| 比較ID | 比較対象 | 意味 |
|--------|---------|------|
| diffdata10 | file[1] vs file[0] | 中央 vs 左 |
| diffdata12 | file[1] vs file[2] | 中央 vs 右 |
| diffdata02 | file[0] vs file[2] | 左 vs 右 |

各ペアは GNU diffutils ベースの `Diff2Files()` で処理。
結果は `Make3wayDiff()` (Diff3.h) で統合される。

### OP_TYPE 分類

| OP_TYPE | 意味 | 条件 |
|---------|------|------|
| `OP_NONE` | 差分なし | 3 ファイルとも同一 |
| `OP_1STONLY` | 左のみ変更 | 中央=右、左だけ異なる |
| `OP_2NDONLY` | 中央のみ変更 | 左=右、中央だけ異なる |
| `OP_3RDONLY` | 右のみ変更 | 左=中央、右だけ異なる |
| `OP_DIFF` | コンフリクト | 3 ファイルとも異なる |
| `OP_TRIVIAL` | 無視された差分 | |

### Make3wayDiff のロジック
- diff10 のみ → `OP_3RDONLY`（右だけ変更）
- diff12 のみ → `OP_1STONLY`（左だけ変更）
- 両方 → file[0] と file[2] を比較:
  - 同一なら `OP_2NDONLY`（中央だけ変更）
  - 異なれば `OP_DIFF`（コンフリクト）

### Ghost 行
Ghost 行を挿入して 3 バッファの行数を揃える。
`LF_SNP` (Same in Non-active Pair) フラグ:
- `OP_3RDONLY` のとき pane 0 に設定（左=中央で同じ）
- `OP_1STONLY` のとき pane 2 に設定（右=中央で同じ）

## 3. Detail ペイン（3-way）

### 構造
- Detail ペインは **別のビューグループ** (`m_bDetailView = true`)
- メインビューと同じ数のペインを持つ（3-way なら **3 セクション**）
- つまり左/中央/右の 3 分割で詳細表示

### 動作
- diff 選択時、各 detail ペインはそのブロックの範囲 (`m_lineBegin` ～ `m_lineEnd`) にスクロール
- 範囲外の行はグレーアウト
- ワードレベルの差分ハイライトあり
  - Pane 0: `OP_3RDONLY` のハイライトをスキップ（左と中央が同じなので）
  - Pane 2: `OP_1STONLY` のハイライトをスキップ（右と中央が同じなので）

## 4. Location ペイン（ミニマップ）

### 3-way 時のバー構成
- `m_bar[3]`: 3 本の縦バーを描画
- バー幅: `clientWidth / (nBuffers * 2)` でマージン付き配置

### バー間のコネクタ領域
隣接バー間（左-中央間、中央-右間）に色付き矩形を描画:

| 色 | 条件 | 意味 |
|----|------|------|
| 黄色 | 左-中央間で `OP_3RDONLY`、中央-右間で `OP_1STONLY` | このペアは同一 |
| シアン | `OP_2NDONLY` | 中央のみ異なる |
| 赤 | `OP_DIFF` | コンフリクト（3ファイルとも異なる） |

## 5. コピー/マージ操作

### コピー方向（アクティブペイン依存）

| コマンド | アクティブペイン | src → dst |
|---------|---------------|-----------|
| Copy Right (L2R) | 0 (左) | 0 → 1 (左→中央) |
| Copy Right (L2R) | 1 (中央) | 1 → 2 (中央→右) |
| Copy Left (R2L) | 1 (中央) | 1 → 0 (中央→左) |
| Copy Left (R2L) | 2 (右) | 2 → 1 (右→中央) |

**重要**: 3-way では Copy Left/Right は常に **隣接ペイン** への操作。

### 明示的コピー操作（コンテキストメニュー）
各ペインから利用可能な操作:
- **左ペイン**: Copy To Middle, Copy To Right, Copy From Middle, Copy From Right
- **中央ペイン**: Copy To Left, Copy To Right, Copy From Left, Copy From Right
- **右ペイン**: Copy To Middle, Copy To Left, Copy From Middle, Copy From Left

### Auto Merge
- **3-way 専用** (`m_nBuffers == 3` のみ有効)
- 下から上に全 diff を走査
- マージ可能な判定 (`GetMergeableSrcIndex`):
  - dst=0 or dst=2: `OP_2NDONLY` のみマージ可（source=1, 中央）
  - dst=1 (中央): `OP_1STONLY` → src=0、`OP_3RDONLY` → src=2
  - `OP_DIFF` (コンフリクト) は **自動マージ不可**
- 結果: "N diffs auto-merged, M conflicts remain"

## 6. ツールバー

### 3-way 時の変更点
- **L2R / R2L**: アクティブペインに応じて隣接ペインへのコピー。メニューテキストが動的に変更
- **All Left / All Right**: 同様にテキスト変更
- **Auto Merge**: 3-way 専用ボタン。未編集かつ未マージ時のみ有効
- **パッチ生成**: 2-way 専用（3-way では無効）
- **3-way ナビゲーション**: LM/LR/MR/LO/MO/RO のナビゲーションボタンが 3-way 時のみ有効

### コピーボタンの非表示/無効化
2-way にない操作（Copy To Middle 等）は 2-way 時に無効化。
3-way ではアクティブペインに無関係な操作を削除（コンテキストメニュー）。

## 7. ナビゲーション

### 標準ナビゲーション (First/Prev/Next/Last)
2-way と同一。統合 diff リストの全有効 diff をトラバース。

### 3-way 専用ナビゲーション

`THREEWAYDIFFTYPE` による分類:

| タイプ | 意味 | 対応 OP_TYPE |
|--------|------|-------------|
| LEFTMIDDLE | 左と中央が異なる | `OP_1STONLY`, `OP_2NDONLY`, `OP_DIFF` を含む（`OP_3RDONLY` を除外）|
| LEFTRIGHT | 左と右が異なる | `OP_2NDONLY` を除外 |
| MIDDLERIGHT | 中央と右が異なる | `OP_1STONLY` を除外 |
| LEFTONLY | 左のみ変更 | `OP_1STONLY` のみ |
| MIDDLEONLY | 中央のみ変更 | `OP_2NDONLY` のみ |
| RIGHTONLY | 右のみ変更 | `OP_3RDONLY` のみ |
| CONFLICT | コンフリクト | `OP_DIFF` のみ |

### コンフリクトナビゲーション
`OnNextConflict` / `OnPrevConflict` は内部で `OnNext3wayDiff(THREEWAYDIFFTYPE_CONFLICT)` を呼ぶ。
コンフリクト = **3 ファイルとも異なる** (`OP_DIFF`) のみ。

### Copy & Advance
3-way では、コピー後の行数変化を考慮して次の diff 位置を計算する複雑なロジック。

## 8. ステータスバー

### 構造
- `CMergeStatusBar` に `MergeStatus m_status[3]` — ペインごとに 1 つ
- `m_nPanes` = `m_nBuffers`

### 表示内容（ペインごと）
- 行番号（実行番号。Ghost 行は "Line N-N+1" 表示）
- カラム位置（タブ展開後）
- 文字位置と文字数
- 選択行/文字数
- EOL タイプ（混在 EOL モード時）
- コードページと BOM

### Diff カウント
"Diff N of M" または "M Diffs" 形式。2-way / 3-way 共通。

## WinXMerge への適用ガイド

### 現状との差分

| 機能 | WinMerge | WinXMerge 現状 | 対応方針 |
|------|---------|---------------|---------|
| Diff エンジン | 3 組ペアワイズ + Make3wayDiff | `compute_three_way_diff` | 要確認 |
| Detail ペイン | 3 セクション（左/中央/右）| 2 セクション（左/右）| 3 セクションに拡張 |
| Location ペイン | 3 バー + コネクタ | 1 バー | 3 バー + コネクタ描画 |
| コピー操作 | 隣接ペイン基準 | use-left/use-right のみ | 6 方向コピー追加 |
| ナビゲーション | 7 種類の diff タイプ | conflict のみ | 段階的に追加 |
| ステータスバー | 3 ペイン分 | 1 行 | 3 ペイン情報追加 |
| Auto Merge | あり | なし | 将来対応 |

### 優先実装順序

1. **Detail ペイン 3 セクション化** — 左/中央/右の 3 分割
2. **Location ペイン 3 バー化** — 3 本バー + コネクタ色分け
3. **コピー操作の方向拡張** — 隣接ペイン間コピー
4. **ナビゲーション拡張** — diff タイプ別移動
5. **Auto Merge** — 非コンフリクト diff の自動マージ
6. **ステータスバー 3 ペイン化**

## 参照ソースファイル

| ファイル | 役割 |
|---------|------|
| `Src/MergeDoc.h` | ドキュメントクラス宣言、`m_nBuffers`, `m_ptBuf[3]`, `m_pView[3][3]` |
| `Src/MergeDoc.cpp` | Rescan, PrimeTextBuffers, ステータスバー |
| `Src/MergeEditView.cpp` | UI 操作全般: コピー、ナビゲーション、コンテキストメニュー、着色、detail ビュー |
| `Src/DiffList.h` | `OP_TYPE`, `THREEWAYDIFFTYPE`, `DIFFRANGE` 構造体 |
| `Src/Diff3.h` | `Make3wayDiff()` アルゴリズム |
| `Src/DiffWrapper.cpp` | 3 組ペアワイズ diff + Make3wayDiff 呼び出し |
| `Src/MergeDocDiffSync.cpp` | `ComputeOpType3way()`, Ghost 行同期 |
| `Src/MergeDocDiffCopy.cpp` | `DoAutoMerge()`, `GetMergeableSrcIndex()` |
| `Src/LocationView.cpp` | Location ペイン描画（3-way コネクタ領域）|
| `Src/MergeEditSplitterView.cpp` | ペイン生成（メイン + detail）|
| `Src/MergeFrameCommon.cpp` | `MenuIDtoXY()` マッピング、`ChangeMergeMenuText()` |
| `Src/MergeStatusBar.h` | ステータスバー（3 スロット）|
