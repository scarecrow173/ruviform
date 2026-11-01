---
id: "TASK-034QfdqgoGMqtqJ2HHFDdG"
type: "task"
status: "done"
title: "issue-189 T-3 golden/RT verification + impl record + output contract"
created: "2026-11-01"
updated: "2026-11-01"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-189-m3-rational-beat-variable-ppqn.md"
  implemented-by:
    - "docs/impl/ir/issue-189-m3-rational-beat-ppqn.md"
  depends-on:
    - "docs/plans/implement-issue-189-m3-rational-beat-variable-ppqn.md"
    - "docs/tasks/issue-189-t-2-tempomap-beat-basis.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-189-m3-rational-beat-ppqn-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# issue-189 T-3 golden/RT verification + impl record + output contract

## Work

plan Step 4-5: Golden/RT 検証と証拠記録。

- Golden: PPQN 480/960/1000/1920 の往復（±1 tick）、1/7 Beat 保持
  （PPQN-04）、異 PPQN 誤混合防止（PPQN-05）、tempo boundary exact・負
  pre-roll（PPQN-07）、旧 960 全テストケースの新 API 同一結果
  （PPQN-11）、overflow/den bound/範囲（PPQN-12）。
- `rt_` no-alloc テスト（全クエリ API + BeatPosition 演算）。
- CLAP `beats_to_ticks` 2^31 golden（adapter テストで spec 式と照合、
  half-away 丸め・負位置・境界）。
- `docs/impl/ir/issue-189-m3-rational-beat-ppqn.md` に実行コマンド・
  結果・PPQN-01〜12 判定（Synvelis 所有 01/02/06/08/10 は範囲外明示）・
  Time crate 不抽出の根拠を記録。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-189-m3-rational-beat-variable-ppqn.md`
- `docs/test-specs/issue-189-m3-rational-beat-ppqn-tests.md`
- `docs/designs/issue-189-rational-beat-variable-ppqn-tempomap.md`（出力契約表）

## Dependencies

- T-2（Beat 基準 TempoMap）。

## Affected Files

- `crates/ruviform-media-core/src/music.rs`（tests）
- `crates/ruviform-plugin-clap/src/adapt.rs`（tests のみ）
- `docs/impl/ir/issue-189-m3-rational-beat-ppqn.md`

## Done When

- [x] PPQN-03/04/05/07/09/11/12 の実測証拠が impl record に記録され、
  検証 lane が全て緑。

## Verification

- [x] `mise run fmt-check`・`check`・`lint`・`test`・`test-rt` 全緑。
- [x] `mise run lint-l1-real-plugin`/`test-l1-real-plugin`（adapter 変更が
  tests のみでも lane 回帰確認）。
