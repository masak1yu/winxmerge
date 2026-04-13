---
title: macOS Finder 連携
description: Finderの右クリックメニューから直接ファイルを比較。
---

WinXMerge には、macOS Finder に右クリック比較アクションを追加する **Finder Sync Extension** が含まれています。

## セットアップ

1. Finder 拡張付きの `.app` バンドルをビルド：
   ```bash
   ./scripts/build-macos-bundle.sh
   ```
2. `WinXMerge.app` を `/Applications` に移動
3. **システム設定 → 一般 → ログイン項目と機能拡張** で拡張を有効化

## 使い方

### 2ファイルの比較

1. Finder で **2つのファイル**を選択
2. 右クリック → **Compare with WinXMerge**
3. 2-way 差分ビューが開く

### 3ファイルの比較

1. Finder で **3つのファイル**を選択
2. 右クリック → **Compare with WinXMerge**
3. 3-way マージビューが開く

### マーク＆比較

1. ファイルを右クリック → **Mark for Compare**
2. 別のファイルに移動
3. 右クリック → **Compare with [マークしたファイル]**

## 実行中インスタンスとの連携

WinXMerge が既に実行中の場合、Finder 拡張は新しいインスタンスを起動する代わりに、既存ウィンドウに**新しいタブ**を追加します。
