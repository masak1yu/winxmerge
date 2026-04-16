# CLAUDE.md — WinXMerge

## Build & Test
```bash
cargo build --features desktop      # デスクトップビルド
cargo test --features desktop       # 全テスト実行（必ずfeatures指定）
cargo fmt                           # フォーマット
```

## Architecture
- **Slint 1.15.1** UI framework + **Rust** backend
- `src/app.rs` — アプリロジック全体（4000行超、最大ファイル）
- `src/main.rs` — コールバック配線
- `src/diff/three_way.rs` — 3-way diffエンジン（merge_hunks）
- `src/diff/engine.rs` — 2-way diffエンジン
- `ui/main.slint` — メインUI・ツールバー・ダイアログ
- `ui/widgets/diff-view-3way.slint` — 3-wayペイン（ThreeWayLineRow）
- `ui/widgets/diff-view.slint` — 2-wayペイン
- **PaneBuffer**（per-pane `VecModel<PaneLineData>`）が**唯一の正（authoritative）データソース**。tab内部配列は補助。

## Gotchas（既知の罠）
1. **ghost行のis_ghost**: コピー時にbase側ghost行（`is_ghost=true`）にテキストを入れる場合、`is_ghost=false`+`line_no="+"`に変更すること。ghost行はsave/rescan時にスキップされる。
2. **最後の行を削除すると入力不能**: `three_way_delete_line`でreal_line_countが1のときは削除禁止。
3. **F5(rescan)で編集内容消失**: rescanは`editing_dirty || has_unsaved_changes`のとき必ずVecModelから再構築すること。ファイル再読み込みは未編集時のみ。テキストdiff（view_mode 0/3）もテーブル（view_mode 4/6/8）も同じルール。新しいview_modeに編集機能を追加したら、`rescan()`にもガード分岐を必ず追加すること。
4. **merge_hunksのファイル側レンジ計算**: hunkの`new_start/new_end`を直接使うとequal行を見落とす。`net = Σ(new_count - old_count)`で計算すること。
5. **Slint TextInput**: 行ごとに独立widget。クロス行ドラッグ選択は不可（Slint仕様）。
6. **cargo test単体ではテストが見つからない**: `--features desktop`が必要。
7. **`@tr()`の罠**: 翻訳エントリがない文字列は原文のまま表示→言語混在。ペインラベル等は`@tr()`を使わない。

## Work Rules
- **featureブランチで作業**。main直接作業禁止。
- **新機能はplanモードで設計→合意→実装**の順。即実装しない。
- **1機能1コミット**。小さく区切る。
- 変更後は必ず`cargo test --features desktop`と`cargo build --features desktop`で確認。
- UI変更はGotchasを確認してからコードに触る。

## 編集機能追加チェックリスト（必須）
view_modeに編集機能を追加する際、以下を**プラン設計段階で全てステップに含める**こと。実装漏れで同じバグを繰り返した実績あり。
1. UI: Text→TextInput + read-only制御 + edited callback
2. callback配線: slint → main.rs → app.rs
3. app.rs: 編集関数 + VecModel更新 + dirty flag
4. undo/redo: snapshot + push/pop + view_mode分岐
5. save: 再構築関数 + save関数 + main.rsの全save系callbackにview_mode分岐
6. **rescan(): editing_dirty/has_unsaved_changesガード + VecModelから再構築**（過去2回漏れた最重要項目）
7. new_blank: 初期グリッド作成
