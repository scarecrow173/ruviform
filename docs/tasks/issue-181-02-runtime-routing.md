---
id: "TASK-klq30A6pIf2U4FUAdY3iuL"
type: "task"
status: "done"
title: "ruviform-ui: UiRuntime multi-window routing (handle_window/drain_jobs_routed/cancel_window_jobs)"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-ui/src/runtime.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/service.rs"
      - type: "file"
        path: "crates/ruviform-ui/tests/runtime.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-181-multi-window.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-181-01-window-contract-types.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-181-multi-window.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-181-multi-window.md"
  references: []
  defers: []
  deferred-by: []
---

# ruviform-ui: UiRuntime multi-window routing

## Work

`UiRuntime` に multi-window 経路を additive 追加する。

- `runtime.rs`: `handle_window(window, HostEvent) -> Vec<RoutedEvent>`（Message→宛先 Some(w)、spawn Job の owner=Some(w)、`CloseRequested`→`closing` を Some(w) 宛に返すが Job は取消さない、`Tick`→`drain_jobs_routed`）、`drain_jobs_routed() -> Vec<RoutedEvent>`（owner Window 宛、ownerless は `None`）、`cancel_window_jobs(window) -> usize`（owner Window 所有 Job のみ opt-in cancel）。`JobRecord{owner: Option<WindowId>}`、`dispatch` を owner 付き内部経路へ拡張（`handle`/`dispatch_op` は `None`）。
- `service.rs`: `ServiceContext` に `origin: Option<WindowId>` を additive 追加。
- `tests/runtime.rs`: `handle_window` の routing（宛先 Some(w)・`closing` で Job 非取消・`Tick` の owner routing）と、既存 `handle` 経路の非回帰を直接単体テスト。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-181-multi-window.md`
- `docs/designs/issue-181-multi-window.md`（UiRuntime 節・Job/Window 寿命分離）

## Dependencies

- `docs/tasks/issue-181-01-window-contract-types.md`（WindowId/RoutedEvent 型）。

## Verification

- `cargo test -p ruviform-ui --test runtime` green（新規 + 既存非回帰）。
- `mise run fmt-check`・`check`・`lint` green。
