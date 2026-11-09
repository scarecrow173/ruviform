---
id: "TASK-JYzFc0i8qIPnwLLfOMWSJa"
type: "task"
status: "done"
title: "Issue #290 OBS-08 RT Timing 実装タスク"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/290"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-290-observability-rt-timing.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-290-observability-rt-timing.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/implement-issue-290-observability-allocation-probe.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-290-observability-rt-timing.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-290-observability-rt-timing-tests.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/275"
    - "https://github.com/scarecrow173/ruviform/issues/280"
    - "https://github.com/scarecrow173/ruviform/issues/281"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/293"
  deferred-by: []
---

# Issue #290 OBS-08 RT Timing 実装タスク

## Tasks

- [x] T1. `src/rt_timing/record.rs` — `TIMING_SCHEMA_VERSION`/
  `TimingClock`/`TimingSample`/`TimingRecord` + accessor 実装
- [x] T2. `src/rt_timing/bridge.rs` — `timing_bridge()`/
  `TimingRecorder` 実装
- [x] T3. `src/rt_timing/sink.rs` — `TimingSink`/`TimingWindow`/
  `TimingSnapshot`/`QuantileEstimate` 実装
- [x] T4. `src/rt_timing/tests.rs` — test-spec R1–R10 試験実装
  （21 tests PASS）
- [x] T5. `src/rt_timing/mod.rs` + `src/lib.rs` — module 登録・
  re-export・crate doc 追記
- [x] T6. mise 検証（fmt-check/check/lint/test/test-rt）— 全 PASS
  （`test` は rustc OOM flake で 1 回失敗後 `CARGO_BUILD_JOBS=4`
  で再実行 PASS。`test-msrv` task は存在しない）
- [x] T7. IR 作成 + audit 証跡（#290/#291/#292 照合結果）— 3 IR に
  `## Audit` 節を追記済み
- [x] T8. doc index 再生成 + commit + PR — `doc_maintenance apply`
  で 5 index 再生成（findings 645→630、残りは既存の manual/migration
  finding で本変更と無関係）

## Notes

- 既存モジュール（`rt_bridge`/`record`/`histogram`/`collector`/
  `metrics`/`diagnostics`/`debug`/`profiling`）は変更しない。
- cross-repo gate（#281 証拠・実 OS・両 Consumer 回帰）は NotRun
  として記録し、実装対象外。
