---
title: クイックスタート
description: WinXMerge をすぐに使い始めるためのガイド。
---

## WinXMerge の起動

```bash
# 引数なしで起動 — ファイル選択ダイアログが開く
cargo run --features desktop

# 2-way ファイル比較
cargo run --features desktop -- file1.txt file2.txt

# 3-way マージ
cargo run --features desktop -- base.txt left.txt right.txt
```

## 基本的なワークフロー

### 1. ファイルを開く

WinXMerge を起動すると、**ファイル選択ダイアログ**が表示されます。左右のファイル（またはフォルダ）のパスを入力し、**Compare** をクリックします。

- **3-way merge** にチェックを入れると、ベースファイルを指定して3-way比較ができます
- **最近使ったファイル**リストからワンクリックで再度開けます

### 2. 差分のナビゲーション

ツールバーボタンまたはキーボードショートカットで差分ブロック間を移動：

| 操作 | ショートカット |
|--------|----------|
| 次の差分 | Alt+↓ |
| 前の差分 | Alt+↑ |
| 最初の差分 | Alt+Home |
| 最後の差分 | Alt+End |

### 3. マージ

- ツールバーの **Copy →** / **← Copy** ボタンで現在の差分ブロックをコピー
- 差分行間のインライン **▶** / **◀** ボタンでブロック単位のマージ
- **Copy & Advance** で現在のブロックをコピーして次に移動

### 4. 保存

**Cmd+S**（macOS）または **Ctrl+S** でマージ結果を保存します。

### 5. タブ

**Cmd+T** で新しいタブを開き、複数の比較を並行して管理できます。各タブは独立した状態を持ちます。

## フォルダ比較

ファイル選択ダイアログで2つのフォルダを選択すると、再帰的なディレクトリ比較が始まります。リスト内のファイルをダブルクリックすると、詳細な差分ビューが開きます。

## 次のステップ

- [ファイル比較](/ja/features/file-comparison/) — 2-way差分の詳細ガイド
- [3-Wayマージ](/ja/features/three-way-merge/) — コンフリクト解決ワークフロー
- [Git連携](/ja/integrations/git/) — `git difftool` / `git mergetool` として使う
