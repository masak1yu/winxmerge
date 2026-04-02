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
- VecModelが**唯一の正（authoritative）データソース**。tab内部配列は補助。

## Gotchas（既知の罠）
1. **ghost行のbase_line_no**: コピー時にbase側ghost行（base_line_no空）にテキストを入れても、`rebuild_three_way_text`がスキップする。コピー時は必ず`base_line_no`に"+"等の非空値を設定すること。
2. **最後の行を削除すると入力不能**: `three_way_delete_line`でreal_line_countが1のときは削除禁止。
3. **F5(rescan)でテキスト復活**: rescanは必ずVecModelから`rebuild_three_way_text`で再構築。tab.*_linesを直接使わない。
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
