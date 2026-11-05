---
id: "TASK-cE5wGhv6Qap0q8kw6DiFoR"
type: "task"
status: "done"
title: "T-2 rt_bridge transport + drop/loss/lag + OBS-02 lifecycle integration + README"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-observability/src/rt_bridge.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-observability/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-observability/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-observability/README.md"
      - type: "file"
        path: "crates/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "Cargo.lock"
  implements:
    - "docs/plans/implement-issue-280-rt-record-bridge.md"
  implemented-by:
    - "docs/impl/ir/implement-issue-280-rt-record-bridge.md"
  depends-on:
    - "docs/plans/implement-issue-280-rt-record-bridge.md"
    - "docs/tasks/p280-t1-record-vocabulary-clock-correlation-event.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# T-2 rt_bridge transport + drop/loss/lag + OBS-02 lifecycle integration + README

## Work

`src/rt_bridge.rs` に `bridge<R>(capacity)`・`RtRecorder<R>`（`try_record`/
`dropped`/`recorded`）・`RecordSink<R>`（`drain`/`drain_final`/`recorded`/
`dropped`/`drained_total`/`lag`/`recorded_reader`/`dropped_reader`）・
`ReceivedRecord<R>` を実装。輸送は `ruviform_rt::channel<Envelope<R>>`
（private `Envelope { record, dropped_before }`）。欠落は `Counter` 共有 +
envelope `dropped_before` で二重可視化。`Cargo.toml` に `ruviform-rt` path
依存追加。`rt_` 契約試験（`assert_no_alloc`）・FIFO/drop-on-full/lag/
quiescence/zero-capacity/跨スレッド試験・`SourceLifecycle` 統合試験を
付ける。`lib.rs` の crate doc・`README.md`（RT 契約表・依存方針差分）・
`crates/README.md` の `ruviform-observability` 行を更新。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-280-rt-record-bridge.md`
- `docs/designs/issue-280-rt-record-bridge-clock-correlation.md`
- `docs/specs/issue-280-rt-record-bridge-clock-correlation.md`

## Dependencies

- `docs/tasks/p280-t1-record-vocabulary-clock-correlation-event.md`
  （`EventRecord` を `bridge` の典型 `R` として統合試験で使用）

## Affected Files

- `crates/ruviform-observability/src/rt_bridge.rs`
- `crates/ruviform-observability/src/lib.rs`
- `crates/ruviform-observability/Cargo.toml`
- `crates/ruviform-observability/README.md`
- `crates/README.md`

## Verification

- `mise run fmt-check` / `mise run check` / `mise run lint` / `mise run test` /
  `mise run test-rt`
- `rt_` 試験: no-alloc・bounded latency・drop/loss/lag/quiescence
- OBS-02 `SourceLifecycle` 統合試験: retire → final drain → removed
