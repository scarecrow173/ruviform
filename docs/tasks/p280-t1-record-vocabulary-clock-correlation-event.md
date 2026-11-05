---
id: "TASK-nZDg5tn3BVGCSd5dBIm3Tv"
type: "task"
status: "done"
title: "T-1 record vocabulary: ClockContext/CorrelationBook/EventRecord + tests"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-observability/src/record/mod.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/record/clock.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/record/correlation.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/record/event.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-observability/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-observability/src/lib.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-280-rt-record-bridge.md"
  implemented-by:
    - "docs/impl/ir/implement-issue-280-rt-record-bridge.md"
  depends-on:
    - "docs/plans/implement-issue-280-rt-record-bridge.md"
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

# T-1 record vocabulary: ClockContext/CorrelationBook/EventRecord + tests

## Work

`src/record/` を新設し、`clock.rs`（`ClockContext` = `InputStamp` +
`ClockEpochId`）・`correlation.rs`（`CorrelationToken(NonZeroU64)`・
`CorrelationLink{token,epoch}`・`CorrelationBook`（非 RT
issue/remint/expire/classify）・`CorrelationStatus`/`UnlinkedReason`
`#[non_exhaustive]`）・`event.rs`（`EventId(u32)`・`RecordHeader`・
`EventRecord{header,value,reserved:[u64;2]}`）を実装。`Cargo.toml` に
`ruviform-media-core` path 依存追加。`ClockContext` の golden（domain/
epoch/provenance/None vs Estimated の区別・変換 API 不在）・Correlation の
linked/unlinked 試験・`EventRecord` の形状/size 上限/`reserved` 固定試験を
付ける。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-280-rt-record-bridge.md`
- `docs/designs/issue-280-rt-record-bridge-clock-correlation.md`
- `docs/specs/issue-280-rt-record-bridge-clock-correlation.md`

## Dependencies

- `docs/tasks/p280-t2-rt-bridge-transport-drop-loss-lifecycle.md` とは
  同一 `Cargo.toml` を共有（T-1 が `ruviform-media-core`、T-2 が
  `ruviform-rt` を追加）。

## Affected Files

- `crates/ruviform-observability/src/record/{mod.rs,clock.rs,correlation.rs,event.rs}`
- `crates/ruviform-observability/src/lib.rs`
- `crates/ruviform-observability/Cargo.toml`

## Verification

- `mise run fmt-check` / `mise run check` / `mise run lint` / `mise run test`
- golden 試験: clock/correlation/record shape の区別と `size_of` 上限
