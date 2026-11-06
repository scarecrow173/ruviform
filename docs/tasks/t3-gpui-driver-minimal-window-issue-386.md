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
        path: "crates/ruviform-ui-gpui/examples/minimal_window.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/README.md"
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
  ※実装では未導入 — semantic 契約検証は
  `ruviform-ui-test/tests/fake_driver.rs`（14 fixture）と
  `ruviform-ui/tests/runtime.rs`（dispatch parity）に集約し、
  GPUI 側は compile lane + example 実 OS 検証で担保
  （`test-ui-gpui` の実行 test 数は 0、design §構成の deviation
  記録に一致）。`#[gpui::test]` suite は RF390 へ defer。

## Acceptance

- `mise run check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui` pass（R3/R4）。
- 2 window・close veto・Destroyed の event 系列が fake と一致
  （FakeDriver fixture で固定 — GPUI 側は同一契約実装 +
  example 実 OS 検証）。
