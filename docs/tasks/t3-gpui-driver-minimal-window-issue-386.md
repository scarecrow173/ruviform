---
id: "TASK-LlEFlV8p3toRfliU2UWnVn"
type: "task"
status: "proposed"
title: "T3 GPUI Driver + 最小 Window fixture（ruviform-ui-gpui）"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "../plans/implement-issue-386-driver-neutral-gui-host.md"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/detect.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/driver.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/session.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/view.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/examples/minimal_window.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/driver_contract.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/Cargo.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-issue-386-driver-neutral-gui-host.md"
  implemented-by: []
  depends-on:
    - "../tasks/t1-driver-neutral-window-contract-issue-386.md"
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

# T3: GPUI Driver + 最小 Window fixture

## Work

- `GpuiDriver`（`WindowDriver` 実装）+ `run` 入口（gpui
  `Application` loop 上で `FnMut(DriverEvent, &mut dyn WindowDriver)`
  を駆動）。`on_window_should_close`→`CloseRequested`、
  `on_window_closed`→`Destroyed`、frame/state 差分→
  `Resized`/`Moved`/`Focused`/`ScaleFactorChanged`。
- `FixtureView`（最小 root view: Action(Intent) emit + `post_event`
  inbox 表示 + close policy 切替）。
- `examples/minimal_window.rs`（`required-features=["gpui"]`）。
- `tests/driver_contract.rs`（`#[gpui::test]` + `TestAppContext`
  で FakeDriver と同一シナリオを GPUI Driver で検証）。

## Acceptance

- `mise run check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui` pass（R3/R4）。
- 2 window・close veto・Destroyed の event 系列が fake と一致。
