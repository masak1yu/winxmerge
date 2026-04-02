# WinMerge 3-Way Diff アルゴリズム 技術解析書

WinMerge のソースコード（GitHub: WinMerge/winmerge）および GNU diff3 アルゴリズムの解析に基づく、3-way diff/merge の詳細技術仕様。

---

## 目次

1. [3-Way Diff アルゴリズム概要](#1-3-way-diff-アルゴリズム概要)
2. [Make3wayDiff: ハンクマージアルゴリズム](#2-make3waydiff-ハンクマージアルゴリズム)
3. [OP_TYPE 分類ロジック](#3-op_type-分類ロジック)
4. [行アラインメントとGhost行](#4-行アラインメントとghost行)
5. [コピー操作](#5-コピー操作)
6. [UI表示と着色](#6-ui表示と着色)
7. [ナビゲーション](#7-ナビゲーション)
8. [エッジケース](#8-エッジケース)
9. [WinXMerge 実装への適用](#9-winxmerge-実装への適用)

---

## 1. 3-Way Diff アルゴリズム概要

### 1.1 基本原理

WinMerge の 3-way diff は **GNU diff3 アルゴリズム** に基づいており、次の手順で動作する:

1. **3組のペアワイズ 2-way diff を実行**
2. **2-way diff の結果（ハンク列）をマージして 3-way diff ブロックを生成**
3. **各ブロックの変更タイプ（OP_TYPE）を決定**
4. **Ghost行を挿入して3ペインの行を同期**

### 1.2 ペアワイズ比較の構成

WinMerge は **3つのファイル（file[0]=左, file[1]=中央/Base, file[2]=右）** に対して3組の 2-way diff を実行する:

| 比較ID | 比較対象 | 記法 | 意味 |
|--------|---------|------|------|
| `diffdata10` | file[1] vs file[0] | Base vs Left | 中央から見た左の変更 |
| `diffdata12` | file[1] vs file[2] | Base vs Right | 中央から見た右の変更 |
| `diffdata02` | file[0] vs file[2] | Left vs Right | 左右の直接比較（コンフリクト判定用） |

**重要**: `diffdata10` と `diffdata12` は Base（file[1]）を基準にしている。これにより、Base の行番号空間でハンクのオーバーラップを検出できる。

`diffdata02`（左 vs 右）は `Make3wayDiff` 内でコンフリクト判定（左と右が同一変更か異なる変更か）に使用される。直接ハンクマージには参加しない。

### 1.3 実行フロー

```
RunFileDiff()
  ├── Diff2Files(&script10, file[1], file[0])  // Base vs Left
  ├── Diff2Files(&script12, file[1], file[2])  // Base vs Right
  ├── Diff2Files(&script02, file[0], file[2])  // Left vs Right (比較関数用)
  │
  └── LoadWinMergeDiffsFromDiffUtilsScript3()
        ├── script10 → diff10 (DiffRangeInfo vector に変換)
        ├── script12 → diff12 (DiffRangeInfo vector に変換)
        └── Make3wayDiff(diff3, diff10, diff12, Comp02Functor, has_trivial)
              └── diff3 = マージ済み3-wayブロック列
```

---

## 2. Make3wayDiff: ハンクマージアルゴリズム

### 2.1 データ構造

```cpp
struct DIFFRANGE {
    int begin[3];    // 各ファイルの開始行（0-indexed）
    int end[3];      // 各ファイルの終了行（inclusive）
    int dbegin;      // 同期済み（Ghost行追加後）開始行
    int dend;        // 同期済み終了行
    int blank[3];    // 各ファイルのBlank（Ghost）行数 (-1=なし)
    OP_TYPE op;      // 操作タイプ
};
```

入力の `diff10` と `diff12` は2要素配列（`begin[0]`, `begin[1]`, `end[0]`, `end[1]`）で、`begin[0]`/`end[0]` がBase（file[1]）側、`begin[1]`/`end[1]` が対象ファイル側の行範囲。

出力の `diff3` は3要素配列（`begin[0..2]`, `end[0..2]`）で、3ファイルそれぞれの行範囲を持つ。

### 2.2 マージアルゴリズムの詳細

`Make3wayDiff` は、`diff10`（Base vs Left）と `diff12`（Base vs Right）のハンク列を **Base の行番号空間で走査** し、オーバーラップするハンクをグループ化して3-wayブロックを生成する。

#### ステップ1: 先頭ハンクの選択

```
diff10 の次のハンク: dr10first （Base行 = dr10first.begin[0]）
diff12 の次のハンク: dr12first （Base行 = dr12first.begin[0]）

firstDiffBlockIsDiff12 = (dr12first.begin[0] <= dr10first.begin[0])
```

Base 行番号が小さい方を「最初のブロック」とする。

#### ステップ2: オーバーラップ検出ループ

2つのハンク列を交互にスキャンし、Base 行範囲がオーバーラップするハンクをすべてグループに取り込む:

```
while (diff10 と diff12 の両方にハンクが残っている):
    dr10 = diff10 の現在のハンク
    dr12 = diff12 の現在のハンク

    // 完全に終了行が一致 → 両方を消費して終了
    if dr10.end[0] == dr12.end[0]:
        consume both, break

    // 最後に処理したのが diff12 側のハンクなら
    if lastDiffBlockIsDiff12:
        if max(dr12.begin[0], dr12.end[0]) < dr10.begin[0]:
            break  // オーバーラップなし、グループ完成
    else:
        if max(dr10.begin[0], dr10.end[0]) < dr12.begin[0]:
            break

    // より大きい end[0] を持つ方を進める
    if dr12.end[0] > dr10.end[0]:
        advance diff10, lastDiffBlockIsDiff12 = true
    else:
        advance diff12, lastDiffBlockIsDiff12 = false
```

**核心**: このループは、Base 行空間で「重なり合う」すべてのハンクを1つのグループにまとめる。重なりがなくなった時点でグループが確定する。

#### ステップ3: 3-way ブロックの行範囲計算

グループ内のハンクから、3ファイルそれぞれの行範囲を算出する:

```
if firstDiffBlockIsDiff12:
    dr3.begin[1] = dr12first.begin[0]    // Base開始
    dr3.begin[2] = dr12first.begin[1]    // Right開始
    if diff10 がグループに参加していない:
        dr3.begin[0] = dr3.begin[1] - linelast1 + linelast0  // Leftはオフセット計算
    else:
        dr3.begin[0] = dr3.begin[1] - dr10first.begin[0] + dr10first.begin[1]

// end も同様に計算（lastDiffBlockIsDiff12 で分岐）
```

**行番号のオフセット計算**: `linelast0`, `linelast1`, `linelast2` は前のブロック終了時点の各ファイルの行位置（end+1）。ハンクに参加していない側の行範囲は、Baseからのオフセットで推定する。

#### ステップ4: OP_TYPE の決定

```
if diff10 がグループに不参加:
    op = OP_3RDONLY       // Base vs Right のみ差分 → 右だけ変更
else if diff12 がグループに不参加:
    op = OP_1STONLY       // Base vs Left のみ差分 → 左だけ変更
else:
    // 両方に差分がある
    if cmpfunc(dr3) == true:  // file[0] と file[2] が同一か？
        op = OP_2NDONLY   // 左=右 → 中央（Base）だけ変更
    else:
        op = OP_DIFF      // コンフリクト（3ファイルとも異なる）
```

#### ステップ5: 後処理（オーバーラップ修正）

最終的に、隣接する3-wayブロックの行範囲が重複する場合の修正:

```
for each adjacent pair (dr3[i], dr3[i+1]):
    for j in 0..3:
        if dr3[i].end[j] >= dr3[i+1].begin[j]:
            dr3[i].end[j] = dr3[i+1].begin[j] - 1
```

### 2.3 Comp02Functor: コンフリクト判定

両側に差分がある場合、`file[0]`（Left）と `file[2]`（Right）を直接行比較する:

```cpp
struct Comp02Functor {
    bool operator()(const DiffRangeInfo& dr3) {
        int line0 = dr3.begin[0], line0end = dr3.end[0];
        int line2 = dr3.begin[2], line2end = dr3.end[2];

        // 行数が異なれば不一致
        if ((line0end - line0) != (line2end - line2))
            return false;

        // 各行を比較
        for (int i = 0; i <= line0end - line0; ++i) {
            if (line_cmp(linbuf0[line0+i], linbuf2[line2+i]) != 0)
                return false;
        }
        return true;  // 完全一致 → OP_2NDONLY
    }
};
```

`line_cmp` は大文字小文字、空白、EOL の各オプションに応じた比較を行う。

---

## 3. OP_TYPE 分類ロジック

### 3.1 OP_TYPE 一覧

| OP_TYPE | 値 | 意味 | 条件 |
|---------|---|------|------|
| `OP_NONE` | 0 | 差分なし | 3ファイルとも同一（Equal領域） |
| `OP_1STONLY` | 1 | 左のみ変更 | diff12 に該当ハンクなし（Base=Right, Left だけ異なる） |
| `OP_2NDONLY` | 2 | 中央のみ変更 | diff10 と diff12 の両方に該当があり、Left=Right |
| `OP_3RDONLY` | 3 | 右のみ変更 | diff10 に該当ハンクなし（Base=Left, Right だけ異なる） |
| `OP_DIFF` | 4 | コンフリクト | diff10 と diff12 の両方に該当があり、Left!=Right |
| `OP_TRIVIAL` | 5 | 無視された差分 | 空白のみの差分など（オプション依存） |

### 3.2 判定フロー図

```
diff10にハンクあり?  diff12にハンクあり?
      No                  Yes          → OP_3RDONLY (右のみ変更)
      Yes                 No           → OP_1STONLY (左のみ変更)
      Yes                 Yes          →
         Left == Right?
            Yes → OP_2NDONLY (中央のみ変更 = 両側が同じ変更をした)
            No  → OP_DIFF (コンフリクト)
```

### 3.3 OP_TYPE と変更主体の関係

| OP_TYPE | Left | Base | Right | 解釈 |
|---------|------|------|-------|------|
| `OP_1STONLY` | 変更あり | 元 | 元と同じ | 左だけが変更した |
| `OP_2NDONLY` | 変更あり | 元 | 左と同じ | 左右が同じ変更をした（Baseだけ異なる） |
| `OP_3RDONLY` | 元と同じ | 元 | 変更あり | 右だけが変更した |
| `OP_DIFF` | 変更A | 元 | 変更B (A!=B) | 左右が異なる変更をした（コンフリクト） |

---

## 4. 行アラインメントとGhost行

### 4.1 Ghost行の目的

3-way diff では、各ファイルの行数が異なる差分ブロックが発生する。3ペインの表示を同期するために、行数が少ないペインに **Ghost行**（空行）を挿入して行数を揃える。

### 4.2 PrimeTextBuffers: Ghost行挿入

`PrimeTextBuffers()` で各差分ブロックに対してGhost行を挿入する:

```
for each DIFFRANGE curDiff:
    // 各ファイルの実行数
    nline[0] = curDiff.end[0] - curDiff.begin[0] + 1  // Left
    nline[1] = curDiff.end[1] - curDiff.begin[1] + 1  // Base
    nline[2] = curDiff.end[2] - curDiff.begin[2] + 1  // Right

    // 最大行数
    nmaxline = max(nline[0], nline[1], nline[2])

    for file in 0..3:
        nextra = nmaxline - nline[file]
        if nextra > 0:
            // Ghost行を挿入
            SetEmptyLine(position, nextra)
            // フラグを設定
            dflag = LF_GHOST
            if (file==0 && op==OP_3RDONLY) || (file==2 && op==OP_1STONLY):
                dflag |= LF_SNP
            SetLineFlag(each ghost line, dflag)
```

### 4.3 LF_SNP フラグ（Same in Non-active Pair）

`LF_SNP` は「このペインの内容はペアの他方と同一である」ことを示すフラグ:

| OP_TYPE | LF_SNP 設定先 | 意味 |
|---------|--------------|------|
| `OP_3RDONLY` | file[0]（Left）のGhost行 | Left = Base（左と中央は同じ）|
| `OP_1STONLY` | file[2]（Right）のGhost行 | Right = Base（右と中央は同じ）|

**用途**:
- **着色**: LF_SNP が設定されたペインは変更ハイライトをスキップ（変更がないため）
- **Detail ペイン**: LF_SNP のペインではワードレベル差分ハイライトを省略

### 4.4 同期済み行番号（dbegin / dend）

Ghost行挿入後、`DIFFRANGE` の `dbegin` / `dend` が計算される。これは **3ペインで共通の表示行番号** であり、すべてのペインで同じ行位置に同じ差分ブロックが表示される。

```
curDiff.dbegin = 累積表示行位置
curDiff.dend = dbegin + nmaxline - 1
```

### 4.5 行アラインメントの詳細ロジック

#### AdjustDiffBlocks3way

差分ブロック内で類似行を整列させる機能（「Align similar lines」オプション）:

1. 各差分ブロックに対して、3組のペアワイズ DiffMap を計算
2. `CreateVirtualLineToRealLineMap3way()` で仮想行マップを構築
3. 仮想行ごとに `ComputeOpType3way()` でサブ分類
4. 類似行同士が横に並ぶようにGhost行を再配置

#### ComputeOpType3way

仮想行レベルでの細粒度分類:

```
for each virtual line:
    pane0_has_content = (not ghost in pane 0)
    pane1_has_content = (not ghost in pane 1)
    pane2_has_content = (not ghost in pane 2)

    if only pane0: OP_1STONLY
    if only pane1: OP_2NDONLY
    if only pane2: OP_3RDONLY
    if pane0 && pane1 && !pane2:
        compare(pane0, pane1) → same: OP_3RDONLY, diff: OP_DIFF
    // ... 他の組み合わせも同様
    if all three:
        compare pairwise → determine OP_TYPE
```

### 4.6 具体例

**Base**:
```
line 1
line 2
line 3
```

**Left** (2行追加):
```
line 1
added-L1
added-L2
line 2
line 3
```

**Right** (line 2 を変更):
```
line 1
MODIFIED-2
line 3
```

**Ghost行挿入後の3ペイン表示**:

| 行 | Left | Base | Right | OP_TYPE |
|----|------|------|-------|---------|
| 1 | `line 1` | `line 1` | `line 1` | Equal |
| 2 | `added-L1` | _(ghost)_ | _(ghost)_ | OP_1STONLY |
| 3 | `added-L2` | _(ghost)_ | _(ghost)_ | OP_1STONLY |
| 4 | `line 2` | `line 2` | `MODIFIED-2` | OP_3RDONLY |
| 5 | `line 3` | `line 3` | `line 3` | Equal |

- 行2-3: Leftに2行追加。BaseとRightにGhost行2行挿入。RightのGhost行に `LF_SNP` 設定。
- 行4: Rightが変更。LeftのGhost行はなし（行数同じ）。LeftにLF_SNP設定可能。

---

## 5. コピー操作

### 5.1 隣接ペインコピー（Copy Left / Copy Right）

3-way では、Copy Left/Right は **常に隣接ペイン間** の操作:

| コマンド | アクティブペイン | コピー方向 |
|---------|---------------|-----------|
| Copy Right (L2R) | 0（Left） | Left → Base (0 → 1) |
| Copy Right (L2R) | 1（Base） | Base → Right (1 → 2) |
| Copy Left (R2L) | 1（Base） | Base → Left (1 → 0) |
| Copy Left (R2L) | 2（Right） | Right → Base (2 → 1) |

### 5.2 明示的コピー操作（コンテキストメニュー）

各ペインから任意のペインへのコピーが可能:

| アクティブペイン | 利用可能なコピー操作 |
|---------------|-------------------|
| Left (0) | Copy To Middle, Copy To Right, Copy From Middle, Copy From Right |
| Base (1) | Copy To Left, Copy To Right, Copy From Left, Copy From Right |
| Right (2) | Copy To Middle, Copy To Left, Copy From Middle, Copy From Left |

### 5.3 コピー操作の実装

```
ListCopy(srcPane, dstPane, nDiff):
    1. 対象の DIFFRANGE を取得
    2. Undo グループを開始
    3. dst ペインの begin[dst]..end[dst] の行を削除
    4. src ペインの begin[src]..end[src] の行を挿入
    5. Ghost行を再計算
    6. Undo グループを終了
    7. Rescan（再差分計算）
```

**重要: 下から上への処理**
複数の差分を一括コピーする場合、行番号のずれを防ぐため **下から上（末尾から先頭）** の順序で処理する。

### 5.4 Auto Merge

3-way 専用の自動マージ機能:

```
DoAutoMerge(dstPane):
    for each diff (bottom to top):
        srcIndex = GetMergeableSrcIndex(nDiff, dstPane)
        if srcIndex >= 0:
            ListCopy(srcIndex, dstPane, nDiff)
            autoMergedCount++
        else:
            unresolvedConflictCount++

    show message: "N diffs auto-merged, M conflicts remain"
    navigate to first unresolved conflict
```

#### GetMergeableSrcIndex のロジック

```
GetMergeableSrcIndex(nDiff, dstIndex):
    switch dstIndex:
        case 0 (Left) or 2 (Right):
            if op == OP_2NDONLY: return 1 (Base)  // Baseだけ変更 → Baseの内容をコピー
            else: return -1  // マージ不可
        case 1 (Base):
            if op == OP_1STONLY: return 0 (Left)   // 左だけ変更 → 左をBaseにコピー
            if op == OP_2NDONLY: return 0 (Left)   // Base変更（左右同じ） → 左をコピー
            if op == OP_3RDONLY: return 2 (Right)  // 右だけ変更 → 右をBaseにコピー
            else: return -1  // OP_DIFF（コンフリクト）は自動マージ不可
```

**OP_DIFF（コンフリクト）は自動マージ不可** — ユーザーが手動で解決する必要がある。

---

## 6. UI表示と着色

### 6.1 行の着色

WinMerge は各行の OP_TYPE に基づいて背景色を設定する:

| OP_TYPE | 変更ペインの色 | 同一ペインの色 | 意味 |
|---------|-------------|-------------|------|
| OP_NONE | なし | なし | 差分なし |
| OP_1STONLY | Left: 黄色系 | Base, Right: 薄い黄色 (LF_SNP) | 左のみ変更 |
| OP_2NDONLY | Base: シアン系 | Left, Right: 薄いシアン | 中央のみ変更 |
| OP_3RDONLY | Right: 黄色系 | Left, Base: 薄い黄色 (LF_SNP) | 右のみ変更 |
| OP_DIFF | 全ペイン: 赤/ピンク系 | — | コンフリクト |

**LF_SNP のペインは「同一」を示す淡い色** で表示される。変更があるペインは通常の差分色で表示。

### 6.2 ワードレベルハイライト

差分ブロック内で、行内のどの部分が変更されたかをハイライト:

- `OP_3RDONLY`: Left ペインのハイライトをスキップ（Left = Base で同一なので）
- `OP_1STONLY`: Right ペインのハイライトをスキップ（Right = Base で同一なので）
- `OP_DIFF`: 全ペインでハイライト
- `OP_2NDONLY`: Left と Right のハイライトをスキップ可能（同一なので）

### 6.3 Location ペイン（ミニマップ）

3-way 時は3本の縦バーを描画し、バー間にコネクタ領域を表示:

| 色 | 条件 | 意味 |
|----|------|------|
| 黄色 | 左-中央間で `OP_3RDONLY`、中央-右間で `OP_1STONLY` | このペアは同一 |
| シアン | `OP_2NDONLY` | 中央のみ異なる |
| 赤 | `OP_DIFF` | コンフリクト |

### 6.4 Detail ペイン

3-way 時は3セクション（Left / Base / Right）を表示:
- diff 選択時、各セクションはそのブロックの範囲にスクロール
- 範囲外の行はグレーアウト
- ワードレベル差分ハイライトあり（LF_SNP のペインはスキップ）

---

## 7. ナビゲーション

### 7.1 標準ナビゲーション

First / Prev / Next / Last: 2-way と同一。全有効差分ブロックを順にトラバース。

### 7.2 3-way 専用ナビゲーション

`THREEWAYDIFFTYPE` による分類でフィルタリング:

| タイプ | 意味 | 含む OP_TYPE | 除外する OP_TYPE |
|--------|------|-------------|----------------|
| `LEFTMIDDLE` | 左と中央が異なる | OP_1STONLY, OP_2NDONLY, OP_DIFF | OP_3RDONLY, OP_TRIVIAL |
| `LEFTRIGHT` | 左と右が異なる | OP_1STONLY, OP_3RDONLY, OP_DIFF | OP_2NDONLY, OP_TRIVIAL |
| `MIDDLERIGHT` | 中央と右が異なる | OP_2NDONLY, OP_3RDONLY, OP_DIFF | OP_1STONLY, OP_TRIVIAL |
| `LEFTONLY` | 左のみ変更 | OP_1STONLY | 他すべて |
| `MIDDLEONLY` | 中央のみ変更 | OP_2NDONLY | 他すべて |
| `RIGHTONLY` | 右のみ変更 | OP_3RDONLY | 他すべて |
| `CONFLICT` | コンフリクト | OP_DIFF | 他すべて |

### 7.3 ナビゲーション関数

```
NextSignificant3wayDiffFromLine(nLine, nDiffType):
    for each diff in order:
        if diff.dbegin >= nLine:
            if matches nDiffType filter:
                return diff index
    return -1

PrevSignificant3wayDiffFromLine(nLine, nDiffType):
    for each diff in reverse:
        if diff.dend <= nLine:
            if matches nDiffType filter:
                return diff index
    return -1
```

### 7.4 コンフリクトナビゲーション

`OnNextConflict` / `OnPrevConflict` は内部で `THREEWAYDIFFTYPE_CONFLICT` を指定して3-wayナビゲーション関数を呼ぶ。`OP_DIFF` のみをナビゲートする。

---

## 8. エッジケース

### 8.1 一方の側がより多い/少ない行を持つ場合

- **挿入**: 一方にのみ追加行がある場合、他の2ペインにGhost行が挿入される
- **削除**: 一方から行が削除された場合、そのペインにGhost行が挿入される
- **行数0のハンク**: base_start == base_end のハンク（純粋な挿入）が正しく処理される必要がある

### 8.2 空ファイル

- 3ファイルすべてが空: 差分なし
- 1ファイルのみ空: そのファイル全体がGhost行、他の2ファイルの全行が差分ブロック
- 2ファイルが空: 残り1ファイルの内容に応じた OP_TYPE

### 8.3 オーバーラップするハンク

Make3wayDiff のオーバーラップ検出ループが処理:

```
例:
  diff10: Base行 5-10 が変更
  diff12: Base行 8-15 が変更

→ Base行 5-15 が1つの3-wayブロックにマージされる
→ 両側に差分があるため OP_DIFF（Left vs Right を比較して確定）
```

### 8.4 隣接するが重ならないハンク

```
例:
  diff10: Base行 5-10
  diff12: Base行 11-15

→ 別々の3-wayブロック
→ diff10 のブロック: OP_1STONLY
→ diff12 のブロック: OP_3RDONLY
```

### 8.5 同一行の変更（BothChanged）

左右が BaseFrom の同じ行を同じ内容に変更した場合:
- `Comp02Functor` が true を返す
- `OP_2NDONLY` に分類される（「Baseだけが異なる」= 左右は一致）
- コンフリクトにはならない

### 8.6 末尾の改行

- 末尾改行の有無は `lines()` の分割結果に影響する
- WinMerge は EOL 差分を独立したオプション（`m_bIgnoreEol`）で制御
- 改行文字の違いのみの場合、OP_TRIVIAL にできる

### 8.7 大文字小文字・空白の無視

比較オプションによって OP_TYPE の判定結果が変わる:
- 大文字小文字を無視: `"ABC"` と `"abc"` は同一扱い
- 空白を無視: `"a b"` と `"a  b"` は同一扱い
- これらのオプションは `Comp02Functor` の `line_cmp` に影響

### 8.8 Ghost行内のコピー操作

Ghost行を含む差分ブロックでコピーを行う場合:
- Ghost行は削除される（実際の行に置き換わる）
- コピー後の行数変化により、後続ブロックの dbegin/dend が再計算される
- Rescan で全体が再同期される

---

## 9. WinXMerge 実装への適用

### 9.1 現在の WinXMerge 実装の分析

現在の `compute_three_way_diff`（`src/diff/three_way.rs`）は:

- **2組のペアワイズ diff** を使用（Base vs Left, Base vs Right）— WinMerge の3組ではなく2組
- **ハンクのオーバーラップ処理** が簡略化されている（同一 base_start のハンクのみ処理）
- **OP_TYPE** の代わりに `ThreeWayStatus` enum を使用（Equal, LeftChanged, RightChanged, BothChanged, Conflict）
- **Ghost行** は line_no が None で表現される

### 9.2 WinMerge との主な差異

| 項目 | WinMerge | WinXMerge 現状 | 改善点 |
|------|---------|---------------|--------|
| ペアワイズ比較 | 3組 (10, 12, 02) | 2組 (base-left, base-right) | 02 比較は Comp02 相当のインライン比較で代替可能 |
| ハンクマージ | オーバーラップ検出ループ | 同一 base_start のみ | **要改善**: base 範囲が重なるハンクのグループ化が必要 |
| OP_TYPE | 6種類 | 5種類 (ThreeWayStatus) | BothChanged ≒ OP_2NDONLY で概ね対応 |
| Ghost行 | LF_GHOST + LF_SNP フラグ | line_no: None | SNP 相当のフラグ追加を検討 |
| 行アラインメント | AdjustDiffBlocks3way | なし | 将来対応 |

### 9.3 最も重要な改善: ハンクオーバーラップ処理

現在の実装の最大の問題は、**異なる base_start を持つがオーバーラップするハンクの処理**:

```
現状の問題:
  left_hunk: base 5-10 (Left が変更)
  right_hunk: base 8-15 (Right が変更)

  現在: base_start が異なるため別々のブロックとして処理される
  正解: base 5-15 を1つのコンフリクトブロックとしてマージすべき
```

Make3wayDiff のオーバーラップ検出ループに相当するロジックの実装が必要。

### 9.4 推奨実装手順

1. **ハンクオーバーラップ検出の実装**: Make3wayDiff のマージループに相当するロジック
2. **Comp02 比較の実装**: 両側変更時に Left vs Right を直接比較
3. **LF_SNP 相当のフラグ追加**: ThreeWayLine に `snp_pane: Option<u8>` を追加
4. **行アラインメント改善**: 差分ブロック内の類似行整列
5. **Auto Merge 機能の実装**: GetMergeableSrcIndex ロジック

---

## 参考資料

### WinMerge ソースファイル

| ファイル | 役割 |
|---------|------|
| `Src/Diff3.h` | `Make3wayDiff()` テンプレート関数 — ハンクマージの核心 |
| `Src/DiffList.h` | `OP_TYPE`, `THREEWAYDIFFTYPE`, `DIFFRANGE` 構造体 |
| `Src/DiffList.cpp` | `GetMergeableSrcIndex()`, 3-way ナビゲーション関数 |
| `Src/DiffWrapper.cpp` | 3組ペアワイズ diff + Make3wayDiff 呼び出し |
| `Src/MergeDocDiffSync.cpp` | `ComputeOpType3way()`, Ghost行同期, 行アラインメント |
| `Src/MergeDocDiffCopy.cpp` | `DoAutoMerge()`, コピー操作 |
| `Src/MergeDoc.cpp` | `PrimeTextBuffers()` — Ghost行挿入, LF_SNP 設定 |
| `Src/MergeEditView.cpp` | UI操作: 着色, ナビゲーション, コンテキストメニュー |

### 外部参考

- GNU diffutils マニュアル: diff3 Hunks — https://www.gnu.org/software/diffutils/manual/html_node/diff3-Hunks.html
- James Coglan: "Merging with diff3" — https://blog.jcoglan.com/2017/05/08/merging-with-diff3/
- Wikipedia: Diff3 — https://en.wikipedia.org/wiki/Diff3
