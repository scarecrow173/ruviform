---
id: "TASK-034T2PiH8yPGqaV8KE8r4M"
type: "task"
status: "done"
title: "Issue #289 OBS-07 Collector/Metrics/Snapshot 実装"
created: "2026-11-06"
updated: "2026-11-06"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/289"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-observability/src/metrics/mod.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/collector/mod.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/collector/config.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/collector/health.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/collector/history.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/collector/registry.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/collector/engine.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/collector/window.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/collector/batch.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/collector/snapshot.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/collector/tests.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-observability/src/core/counter.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/core/gauge.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/core/histogram.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/core/mod.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/source/metric_source.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/lib.rs"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-289-observability-collector-metrics.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-289-observability-collector-metrics.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-289-observability-collector-metrics.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-289-observability-collector-metrics.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-289-observability-collector-metrics-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/277"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/278"
    - "https://github.com/scarecrow173/ruviform/issues/291"
  deferred-by: []
---

# Issue #289 OBS-07 Collector/Metrics/Snapshot 実装

## Work

`docs/plans/implement-issue-289-observability-collector-metrics.md` の
Task-1〜Task-5 を実行する:

1. `src/metrics/`（`Measurement`/`MetricValue`/`MeasurementValidity`/
   `MeasurementIssue`）と `src/collector/` の値型（`CollectorConfig`/
   `CollectorConfigError`/`ObservationWindow`/`ObservationBatch`/
   `MetricsSnapshot`/`CollectorHealth`/`CollectorState`/有界 ring）を
   実装。
2. `src/collector/registry.rs`（`Registry`/`SourceEntry`/`KeyClaim`/
   `retained_removed`）と core reader の `shared_cell_ptr` `pub(crate)`/
   `#[cfg(test)]` cell constructor を実装。
3. `src/collector/collector.rs`（`Collector`/`CollectorRegistrar`/
   `CollectorHandle`/`PollRunner`/`PollError`）を実装。
4. `src/collector/tests.rs` に全 contract 試験を実装。
5. `lib.rs` doc 更新・`mise run` 全検証・`origin/main` merge・PR。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-289-observability-collector-metrics.md`（approved）
- `docs/designs/issue-289-observability-collector-metrics.md`（approved）
- `docs/specs/issue-289-observability-collector-metrics.md`（approved）
- `docs/test-specs/issue-289-observability-collector-metrics-tests.md`（approved）
- `docs/designs/issue-277-observability-core-source-contract.md`（approved）

## Dependencies

- `crates/ruviform-observability` の既存 `core`/`source`/`record`/
  `rt_bridge` は変更不要（最小の `pub(crate)` アクセサ追加のみ）。
- 既存 `docs/designs/issue-277-observability-core-source-contract.md`
  の `ObservationRegistrar`/`RegistrationLease`/`SourceState`/`reader`
  所有権は変更しない。
- 新規外部依存なし（std + `ruviform-media-core` の `ClockDomainId` のみ）。

## Affected Files

- `crates/ruviform-observability/src/metrics/mod.rs`（新規）
- `crates/ruviform-observability/src/collector/mod.rs`（新規）
- `crates/ruviform-observability/src/collector/{config,window,batch,
  snapshot,health,history,registry,collector,tests}.rs`（新規）
- `crates/ruviform-observability/src/core/{counter,gauge,histogram}.rs`
  （`shared_cell_ptr` `pub(crate)` 追加。counter のみ `#[cfg(test)]`
  cell constructor 追加）
- `crates/ruviform-observability/src/source/metric_source.rs`
  （`MetricReader::shared_cell_ptr` `pub(crate)` 追加）
- `crates/ruviform-observability/src/lib.rs`（`mod metrics`/`mod
  collector`・crate doc 更新）
- `crates/ruviform-observability/Cargo.toml`（変更なし）
- `docs/{specs,designs,plans,test-specs,tasks}/README.md`（index 再生成）

## Done When

- [x] `src/metrics/`・`src/collector/` 全実装が compile し `mise run
      check` が pass。
- [x] `src/collector/tests.rs` の全 contract 試験が `mise run test` で
      pass（Baseline/Rate/不連続/再登録/drain/gauge/histogram/window/
      stale/partial/unavailable/unsupported/logging-off/同時 poll
      型保証/競合/容量/履歴 evict/世代 gap/shutdown/poison）。
- [x] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` が全 pass。
- [x] `origin/main` merge 後に `mise run check`/`lint`/`test` が再 pass
      （branch HEAD = `origin/main` HEAD のため merge は no-op。
      その状態で全検証 pass）。
- [x] 全公開 enum が `#[non_exhaustive]`・`unsafe` なし・`Collector` が
      `!Clone`・`Drop=shutdown`・`latest_batch` が `Stopped` 後も参照可。
- [x] 既存 `ruviform-observability` の test が無改変で pass。

## Verification

- [x] `mise run fmt-check`（`rustfmt --check` 相当）
- [x] `mise run check`（`cargo check --workspace --all-targets` 相当）
- [x] `mise run lint`（`clippy -D warnings` 相当）
- [x] `mise run test`（`cargo test --workspace` 相当）
- [x] `mise run test-rt`（既存 RT 試験・`rt` feature 有効時の回帰なし）
- [x] `docs/*/README.md` index が再生成される
- [x] `git status` が `docs/`・`crates/` のみ差分を持つ（他 worker の
      worktree 外の変更を持ち込まない）
