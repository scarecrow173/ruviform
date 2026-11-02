---
id: "TASK-W49nvd39gPw0nVVlHofI5L"
type: "task"
status: "done"
title: "ruviform-ui: window contract types (WindowId/WindowEvent/RoutedEvent/CloseDecision/WindowHandle)"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui/src/window.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui/src/event.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/lib.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-181-multi-window.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-181-multi-window.md"
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

# ruviform-ui: window contract types

## Work

`ruviform-ui` に `window` モジュールを追加し、multi-window 契約型を実装する。

- `window.rs`: `WindowId`（`NonZeroU64` ベース・`new(u64)->Option<Self>`・`get()`・Display・Serialize/Deserialize）、`WindowEvent{Windowed{window,event}, Tick}`、`RoutedEvent{window: Option<WindowId>, event}`、`CloseDecision{Approve,Veto,Defer}`、`WindowConfig`（Default 付き — HostConfig の per-Window 相当）、`MonitorInfo`、`WindowHandle` trait（eval/resize/focus/create_window/request_close/respond_close/destroy_window/windows/monitors/exit）。全て `#[non_exhaustive]` 公開 enum/struct 規約・全公開 API doc・RT 非該当の明記。
- `event.rs`: `HostEvent` へ `Moved{x,y}`・`Destroyed` variant を additive 追加。`CloseRequested` doc を更新（単一 Window 経路は従来どおり、multi-window 経路では `CloseDecision` 対象）。
- `lib.rs`: `pub mod window;` + re-export（`WindowId`/`WindowEvent`/`RoutedEvent`/`CloseDecision`/`WindowConfig`/`MonitorInfo`/`WindowHandle`）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-181-multi-window.md`
- `docs/designs/issue-181-multi-window.md`（Components and Boundaries 節）
- `docs/adr/issue-181-multi-window-contract-placement-close-decision-ipc-routing.md`

## Dependencies

- なし（最初のタスク）。

## Verification

- `mise run fmt-check`・`check`・`lint` green。
- doctest/ユニット: `WindowId::new(0)` が `None`、serde round-trip、Display。
