---
id: "TASK-034Qidce7Pq9AxEezYMaNE"
type: "task"
status: "todo"
title: "T-3 typed::probe contract: capability resolver, ProbeBroker, ProbeDeck/RtProbe, ProbeCollector"
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
    - "docs/plans/implement-issue-185-generic-probe-poc.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-185-generic-probe-poc.md"
    - "docs/tasks/p185-t2-runtime-probe-plumbing-taps.md"
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

# T-3 typed::probe contract: capability resolver, ProbeBroker, ProbeDeck/RtProbe, ProbeCollector

## Work

plan Step 4: `typed/probe.rs` に契約型群（`ProbeId`/`ProbePoint`/`TapKind`/`PointAvailability`/`ProbeMode`/`CaptureConfig`/`RecordMeta`/`MetricRecord`/`CaptureChunk`/`StatusRecord`/`StatusKind`/`ProbeEvent`/`ObservationReport`/`ProbeError`・全公開 enum `#[non_exhaustive]`）、capability resolver（物理解決先行→宣言確認）、`ProbeBroker`（install/set_mode/capture/stop/remove/on_publish/maintain/probe_status）、`ProbeDeck`/`RtProbe`（`bound_generation` guard・stamp 監査・`active_taps`）、`ProbeCollector`（`drain`/`recycle`/`status_mirror`）を実装する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-185-generic-probe-poc.md`
- `docs/designs/issue-185-generic-probe-typed-runtime-observation-rt-capture.md`
- `docs/specs/issue-185-generic-probe-rt-capture.md`

## Dependencies

- `docs/tasks/p185-t2-runtime-probe-plumbing-taps.md`

## Affected Files

- `crates/ruviform-audio/src/typed/probe.rs`（新規）、`typed/mod.rs`（re-export）

## Done When

- [ ] capability 列挙が 5 分類を物理解決先行で返し決定的
- [ ] broker の全失敗面が `ProbeError` で返る
- [ ] `chunks_out` capacity = pool で push 不満杯・telemetry は drop-newest+loss
- [ ] `Off` で slot borrow/record 0・`bound_generation` 不一致で `Unbound`

## Verification

- [ ] `mise run check`/`lint` exit 0
- [ ] `cargo test -p ruviform-audio` exit 0
