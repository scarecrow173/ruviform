---
id: "TASK-HfEXpjuOcYSGP35UqpabKo"
type: "task"
status: "done"
title: "Issue #196 T-2: media-core receipt/fault pop + host bus enumeration"
created: "2026-11-02"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/196"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-196-typed-graph-plugin-adapter.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-196-t-1-plugin-core-bus-enumeration.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-196-typed-graph-plugin-adapter.md"
    - "docs/designs/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# T-2: media-core pop + host bus enumeration

## Deliverable

- `crates/ruviform-media-core/src/transport_schedule.rs`:
  `TransportScheduler::pop_receipt`/`pop_fault`（`Vec::remove(0)`・capacity
  bounded・RT-safe）。
- `crates/ruviform-plugin-host`: `BusReport::{Reported(Vec<BusDescriptor>),
  Unavailable}`、`PluginHost::buses`/`InProcessHost::buses`
  （L0 lock・L1/video→Unavailable・Failed→Err）。

## Acceptance

- `cargo test -p ruviform-media-core -p ruviform-plugin-host` 緑。
- L1 Unavailable 宣言の単体テスト。

## Notes

- pop は additive・bounded。host buses は非 RT のみ。
