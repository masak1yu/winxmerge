---
title: プラグインシステム
description: 外部コマンドで WinXMerge を拡張。
---

WinXMerge は、ファイルパスプレースホルダー付きの外部コマンドを実行できるプラグインシステムをサポートしています。

## 設定

プラグインは `~/.config/winxmerge/settings.json` で設定します：

```json
{
  "plugins": [
    {
      "name": "Prettier でフォーマット",
      "command": "prettier --write {LEFT} {RIGHT}"
    },
    {
      "name": "VS Code で開く",
      "command": "code --diff {LEFT} {RIGHT}"
    }
  ]
}
```

## プレースホルダー

| プレースホルダー | 説明 |
|-------------|-------------|
| `{LEFT}` | 左ファイルのパス |
| `{RIGHT}` | 右ファイルのパス |

## 使い方

設定したプラグインは **Plugins** メニューに表示されます。プラグインを選択すると、現在のファイルパスが代入されてコマンドが実行されます。
