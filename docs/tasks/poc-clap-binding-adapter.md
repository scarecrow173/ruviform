---
id: "TASK-034JgsGOtyuy2MkLRLZQI5"
type: "task"
status: "todo"
title: "clack 系による CLAP binding PoC（最小ゲインプラグイン）"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/29"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-plugin-foundation.md"
  implemented-by: []
  depends-on:
    - "../plans/implement-plugin-foundation.md"
    - "create-plugin-core-foundation.md"
    - "implement-borrowed-planar-view.md"
  blocks:
    - "decide-plugin-binding-method-adr.md"
    - "sync-docs-and-verify-plugin-foundation.md"
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
    - "../../Cargo.toml"
    - "../../Cargo.lock"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-plugin-foundation.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/plugin-foundation.md"
  references:
    - "https://github.com/prokopyl/clack"
    - "https://github.com/scarecrow173/ruviform/issues/22"
  defers: []
  deferred-by: []
---

# clack 系による CLAP binding PoC（最小ゲインプラグイン）

## Work

plan `PLAN-034JcvpHIlt5JmqvgOquvI` の T3（Task Breakdown の `poc-clap-binding-clack` 仮称行に対応）。`clack`/`clack-plugin` 系で CLAP の規格エントリポイント成立と process 経路のゼロコピー借用を実測し、方式 ADR（T5）の入力となる発見事項を記録する。PoC コードは実クレート内（scratch ではない）に置く。T4（VST3 PoC）とは相互独立・並行実行可能だが、ルート `Cargo.toml` の `members` 同時追加で競合し得るため統合時に調停する。

- [ ] `crates/ruviform-plugin-clap/` を作成し `members` へ追加、`clack-plugin`（必要に応じ `clack-extensions`）を依存に追加。`Cargo.toml` は **`crate-type = ["cdylib", "rlib"]`**（cdylib は規格エントリポイントのエクスポート、rlib は doctest/`cargo test` 用 — File Map 参照）。検証ハーネスは dev-dependency に `clack-host` を使うかエントリ構造体を直接駆動するかを実装時に選択（lean: 規格エントリポイント経由の構築を実証できる `clack-host` 経路を優先）
- [ ] `clack_export_entry!` による最小ゲインプラグインで規格エントリポイントの成立を確認（cdylib エクスポートまたはホストハーネス経由のインスタンス化）
- [ ] `process(Process, Audio, Events)` の `PortPair` → `channels()` → `ChannelPair` が返す per-channel `&[f32]`/`&mut [f32]` を T2 の view にラップして読み書きし、ゼロコピー借用を実測
- [ ] `ChannelPair::InPlace`（入出力同一バッファ）は `clack` 自身がポインタ比較で分類する。アダプタは同バリアントを中立契約へ写像して T2 で決定した方式（専用 in-place view 分岐またはスクラッチ経路）を適用する。アダプタが生ポインタを比較する箇所では per-channel ポインタ対の粒度を維持し、エイリアス領域上の `&[f32]`+`&mut [f32]` 同時構築を避ける
- [ ] 発見事項を記録する（方式 ADR の入力）: ゼロコピー可否・アダプタ側が書く unsafe の量・process 経路の alloc 有無・ビルド要件・`rust_version` ≤ 1.85・ライセンス適合・拡張カバレッジ（params・events・audio-ports・thread-check 等）・保守状況・依存木サイズ

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-plugin-foundation.md](../plans/implement-plugin-foundation.md) — `PLAN-034JcvpHIlt5JmqvgOquvI`（approved）。T3 チェックリスト・V-3 の正本
- [../discovery/evaluate-plugin-binding-options.md](../discovery/evaluate-plugin-binding-options.md) — `DISC-034JccH31pBpKOSW5fst0g`。`clack` 0.2.0 が CLAP 第一候補・退路は `clap-sys` 自前薄ラッパーの根拠
- [../test-specs/plugin-foundation.md](../test-specs/plugin-foundation.md) — `TSPEC-034Jg6YSP6CLjJHnFGG5oL`。in-place 検出・エントリポイント実証の不変条件

## Dependencies

- 前提タスク: [create-plugin-core-foundation.md](create-plugin-core-foundation.md)（`TASK-034JgsFV6CWAjSnPuekBgs`）— T1 の規格中立型・処理契約が前提（plan 依存グラフ T3←T1）。[implement-borrowed-planar-view.md](implement-borrowed-planar-view.md)（`TASK-034JgsGmcPdPRoeRTvicjW`）— 借用 view と in-place 分岐方針が前提
- 後続をブロック: [decide-plugin-binding-method-adr.md](decide-plugin-binding-method-adr.md) — 方式 ADR の入力は PoC 実測結果
- Issue #22 の alloc 検証基盤は未整備前提。process 経路の alloc 有無は `rt_` テスト + ドキュメント契約・発見記録で代替する

## Affected Files

- `crates/ruviform-plugin-clap/Cargo.toml` — 新規（`crate-type = ["cdylib", "rlib"]`）
- `crates/ruviform-plugin-clap/src/lib.rs` — 新規（標準エントリポイント・ABI→中立型変換・最小ゲインプラグイン PoC）
- `Cargo.toml` — `members` 追加・層内順序コメント
- `Cargo.lock` — `clack` 系依存の記録

## Done When

- [ ] `clack_export_entry!` の最小ゲインプラグインが規格エントリポイント経由で構築できる
- [ ] `ChannelPair` per-channel スライスを T2 view にラップして読み書きでき、ゼロコピーが実測される
- [ ] `ChannelPair::InPlace` でポインタ一致を境界検出し、エイリアス領域上の `&[f32]`+`&mut [f32]` 同時構築がない
- [ ] cdylib 成果物に `clap_entry` エクスポートシンボルが存在する
- [ ] 方式 ADR 入力となる発見記録（ゼロコピー可否・unsafe 量・alloc・ビルド要件・MSRV・ライセンス・拡張カバレッジ・保守状況・依存木サイズ）が揃っている

## Verification

- [ ] `mise exec -- cargo test -p ruviform-plugin-clap` exits 0
- [ ] `mise exec -- cargo clippy -p ruviform-plugin-clap --all-targets -- -D warnings` exits 0
- [ ] `mise exec -- cargo build -p ruviform-plugin-clap` で cdylib 成果物（`*.dll`/`*.so`/`*.dylib`）が生成され、エクスポートシンボルに `clap_entry` が存在する（`nm`/`dumpbin` 等で確認、または dev-dep のホストハーネスでロード実証。**`cargo test` 単独ではこの検証を満たさない**）
- [ ] process 経路に `rt_` プレフィックスの no-alloc 契約テスト + レビュー可能なドキュメント契約を置く（`rt_` 名フィルタは alloc を観測できないため、本確認はドキュメント契約 + レビュー対象として扱う Issue #22 基盤未整備の代替 — spec 受け入れ基準準拠）
- [ ] 発見記録が方式 ADR 入力として揃っていることを確認
