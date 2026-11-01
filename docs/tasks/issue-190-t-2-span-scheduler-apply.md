---
id: "TASK-034Qh6uIoPqWsErTyU"
type: "task"
status: "done"
title: "Issue #190 T-2: transport controller + RT span scheduler"
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
    - "docs/impl/ir/issue-190-t-2-span-scheduler-apply.md"
  depends-on:
    - "docs/tasks/issue-190-t-1-canonical-contract-types.md"
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

# T-2: transport controller + RT span scheduler

## Deliverable

`transport_schedule.rs` 継続（T-1 同ファイル）:

- `TransportController`（非RT側）: `request(cmd, at) -> Result<CommandId,
  AppliedReceipt>`・`preflight(req, contract, cadence)`・`install_plan(plan)`・
  `device_restart(base_frame)`・`drain_receipts`・`drain_faults`。
  pending 用 pre-allocated Vec・receipts/faults bounded ledger。
- `TransportScheduler`（RT 状態）: `run_callback(start, frames, sink)` —
  pending applies + loop wrap を境界として半開 span に分割し
  `sink.span(TransportSpan, events_window)` を呼ぶ。apply の有界差替・
  receipt/fault 記録。`SpanContext`/`TransportSpan`/`DeliveredEvent` 定義。
- legality/Degraded 処理: exact では illegal 境界を
  `UnrepresentableBoundary` fault + Rejected receipt（状態不変）、
  degraded は `max_deferral` 有界の `next_legal_boundary` 走査後に適用し
  `DeferredApply` を receipt に記録。
- event window: `plan_generation`/`transport_generation` 一致判定・
  occurrence 内 position→engine_frame 変換・`max_events_per_span` 超過で
  `EventBudgetExceeded` fault。

## Acceptance

- media-core 単体テスト: Play/Seek/SetLoop/loop wrap/reject/fault/epoch。
- `rt_transport_scheduler_no_alloc`: `run_callback` 無割当（既存 rt_ 命名・
  assert_no_alloc 方針に従う）。
- `mise run check`/`lint`/`test`/`test-rt` 緑。

## Notes

- sink は generic `FnMut(&TransportSpan, &[DeliveredEvent])`。
- pending/faults/receipts は request 時・構築時に capacity 確保し RT は
  push のみ（capacity 内）。
