---
title: Git 連携
description: WinXMerge を git difftool / git mergetool として使用。
---

WinXMerge は Git の `difftool` と `mergetool` の両方として連携できます。

## difftool の設定

```bash
# ビルドとインストール
cargo build --release --features desktop
cp target/release/winxmerge ~/.local/bin/

# git の設定
git config --global diff.tool winxmerge
git config --global difftool.winxmerge.cmd 'winxmerge "$LOCAL" "$REMOTE"'
git config --global difftool.prompt false
```

## mergetool の設定（3-Way マージ）

```bash
git config --global merge.tool winxmerge
git config --global mergetool.winxmerge.cmd 'winxmerge "$BASE" "$LOCAL" "$REMOTE"'
git config --global mergetool.winxmerge.trustExitCode true
```

## 使い方

```bash
# ワーキングツリーの変更を表示
git difftool

# 特定のファイルの差分
git difftool -- path/to/file.rs

# ブランチ間の差分
git difftool main..feature-branch

# マージコンフリクトの解決
git mergetool
```

## シングルインスタンスタブモード

`git difftool` が複数の変更ファイルを処理する際、WinXMerge は **IPC**（Unix ドメインソケット）を使用して実行中のインスタンスを検出します。複数のウィンドウを開く代わりに、以降の差分は既存ウィンドウの**新しいタブ**として開かれます。

2つ以上のファイルペアがこの方法で開かれると、**仮想フォルダ比較ビュー**として表示されます。フォルダビュー内のファイルをダブルクリックすると、新しいタブで詳細な差分が開きます。

この動作は自動的に行われ、追加の設定は不要です。
