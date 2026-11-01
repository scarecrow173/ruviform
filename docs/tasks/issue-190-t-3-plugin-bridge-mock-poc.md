---
id: "TASK-034Qh7vBnMxZaSdFgH"
type: "task"
status: "done"
title: "Issue #190 T-3: plugin transport bridge + Mock Driver PoC"
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
    - "docs/impl/ir/issue-190-t-3-plugin-bridge-mock-poc.md"
  depends-on:
    - "docs/tasks/issue-190-t-2-span-scheduler-apply.md"
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

# T-3: plugin transport bridge + Mock Driver PoC

## Deliverable

- `crates/ruviform-plugin-core/src/transport_bridge.rs`（新規）:
  `BlockContract::from_process_config(&ProcessConfig, InBlockTransportSupport)`、
  `CallPlan::for_span(len, contract)`、`SpanDelivery::for_call(span_ctx,
  call_start, call_frames, events_window, musical_ppqn) -> (TransportInfo,
  EventList)`。
- `lib.rs` re-export。
- `tests/transport_scheduler.rs`（統合）: `MockDriver` + `RecordingProcessor`
  （`PluginProcessor` 実装・call 引数を記録）で test-spec matrix
  （T-006/025/026/027/028/029/010/008/009/012/030/TA-008）を検証。
- `docs/impl/ir/issue-190-transport-span-scheduler.md`（impl record）。

## Acceptance

- AC-1〜AC-12 を test-spec matrix で green。
- `min_frames` 違反の process call が 1 件も記録されないことを
  `RecordingProcessor` 側でも assert。
- `EventList::new` 検証（非減少・offset<frames・zero-frame flush）に全件
  通過。
- `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` 緑。

## Notes

- `SpanDelivery` は caller が供給する `&mut [Event]` scratch に Event を
  書き込み `EventList::new(&scratch, frames)` で検証後 `process` へ渡す
  （RT alloc なし）。
- L0/L1 capability 差は `from_process_config` の引数で区別してテスト。
