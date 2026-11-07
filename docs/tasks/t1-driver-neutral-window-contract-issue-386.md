---
id: "TASK-uFDZyczopm3crTR2lUgeGd"
type: "task"
status: "in-progress"
title: "T1 Driver-neutral Window/Host 共通契約（ruviform-ui driver module）"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "../plans/implement-issue-386-driver-neutral-gui-host.md"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui/src/driver.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/error.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/runtime.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-issue-386-driver-neutral-gui-host.md"
  implemented-by: []
  depends-on:
    - "../plans/implement-issue-386-driver-neutral-gui-host.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../test-specs/issue-386-driver-neutral-gui-host.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/issue-386-driver-neutral-gui-host.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# T1: Driver-neutral Window/Host 共通契約

## Work

`crates/ruviform-ui/src/driver.rs` を新設し、design の通り
`LogicalSize`/`LogicalPosition`/`WindowSpec`/`WindowLifecycle`/
`DriverEvent`/`Capability`/`WindowCapabilities`/`WindowDriver` を
定義。`UiError` に `Unsupported{capability,message}` variant +
`unsupported()` ctor を追加。`UiRuntime` に
`dispatch_window_intent(window, intent) -> Vec<RoutedEvent>` を追加
（`handle_window` の Message branch と同じ dispatch_for 経路）。
`lib.rs` に `pub mod driver` と re-export を追加。

## Acceptance

- `cargo check --workspace` が GPUI なしで pass（R1）。
- `WindowSpec::default()` / `WindowCapabilities::default()` /
  `UiError::unsupported` の unit test pass（R1）。
