---
id: "TASK-034Rp2aMrgIhyU5ZATHDQz"
type: "task"
status: "done"
title: "T-1 core primitives: Gauge/Histogram + RT-lane tests"
created: "2026-11-03"
updated: "2026-11-03"
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
    - "docs/plans/implement-issue-277-observability-core-source.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-277-observability-core-source.md"
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

# T-1 core primitives: Gauge/Histogram + RT-lane tests

## Work

`src/lib.rs` の counter 実装を `src/core/counter.rs` へ移動し root re-export で
`ruviform_observability::{counter, Counter, CounterReader}` を不変維持。
`src/core/gauge.rs`（`gauge(u64)`/`Gauge::{set,read,reader}`/`GaugeReader::read`・
単一 `Arc<AtomicU64>`・Relaxed）と `src/core/histogram.rs`（`histogram(&[u64])`・
`Box<[u64]>` 境界検証・`Box<[AtomicU64]>` buckets・`observe` partition_point・
`HistogramReader::snapshot`）を実装。`assert_no_alloc` dev-dep + `rt_` 試験。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-277-observability-core-source.md`
- `docs/designs/issue-277-observability-core-source-contract.md`
- `docs/specs/issue-277-observability-core-source-contract.md`

## Dependencies

- なし（plan のみ）

## Affected Files

- `crates/ruviform-observability/src/{lib.rs,core/*}`、`Cargo.toml`（dev-dep）

## Verification

- [ ] `mise run check`/`lint`/`test`/`test-rt`（rt_ で no-alloc 実証）
- [ ] 既存 counter 4 テストが無改変 pass
