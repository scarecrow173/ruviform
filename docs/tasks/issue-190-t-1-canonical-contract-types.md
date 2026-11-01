---
id: "TASK-034Qh5tYhNjMkLoPqRs"
type: "task"
status: "done"
title: "Issue #190 T-1: canonical transport contract types (media-core)"
created: "2026-11-02"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/190"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-190-transport-span-scheduler.md"
  implemented-by:
    - "docs/impl/ir/issue-190-t-1-canonical-transport-contract-types.md"
  depends-on:
    - "docs/plans/implement-issue-190-transport-span-scheduler.md"
    - "docs/designs/issue-190-transport-span-scheduler.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-190-transport-span-scheduler.md"
    - "docs/designs/issue-190-transport-span-scheduler.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# T-1: canonical transport contract types

## Deliverable

`crates/ruviform-media-core/src/transport_schedule.rs`（新規）+ `lib.rs`
re-export:

- `EngineFrame`/`ClockEpochId`/`TransportGeneration`/`PlanGeneration`/
  `TempoMapRevision`/`LoopOccurrence`/`CommandId`（u64 newtype、Copy、
  epoch 混同防止のため EngineFrame に epoch フィールド）。
- `TransportCommand`、`DeliveryPolicy`、`ApplyStatus`、`RejectReason`、
  `ApplyDegradation`、`AppliedReceipt`、`ScheduleFault{kind,at}`、
  `FaultKind`、`ScheduleBudget`、`BlockContract`、`InBlockTransportSupport`。
- `ScheduledEvent<P>`（position・source_sequence・plan_generation・
  transport_generation: Option・payload）。
- `TransportPlan`（generation・tempo_revision・tempo_map: Arc・events・
  loop なし — loop は transport state）と `TransportPlanBuilder`
  （event 順検証・budget 検証・generation 付与）。
- 純粋関数: `legal_len(len, contract)`・`partition_len(len, contract) ->
  Option<chunk count>`・`FrameWindow`・`EngineFrame::offset_within`。

## Acceptance

- `cargo test -p ruviform-media-core transport_schedule` の単体テストが
  green（legal_len 表・partition・offset・plan builder 検証・stale tag）。
- `mise run fmt-check`/`check`/`lint` 緑。
- 新規 API は `Copy`/`no_std` 方針と media-core RT 方針に整合（alloc は
  builder/controller の非RT側のみ）。

## Notes

- #189 の `MediaTime`/`MediaRange`/`TempoMap`/`BeatPosition`/`Ppqn` を消費。
- 固定 960/3840 リテラル再導入禁止。
