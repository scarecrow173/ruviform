---
id: "TASK-1N5a7PAIOH2xfWl6PJFte5"
type: "task"
status: "completed"
title: "T5 Fake/GPUI 共通 Window/Dock lifecycle fixture（Issue #386 第 2 PR）"
created: "2026-11-08"
updated: "2026-11-08"
owners: ["scarecrow173"]
relations:
  source:
    - "../plans/implement-issue-386-driver-neutral-gui-host.md"
    - "../test-specs/issue-386-driver-neutral-gui-host.md"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-test/src/lifecycle.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/lifecycle_fixture.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lifecycle_fixture.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/surface.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-ui-test/src/lib.rs"
      - type: "file"
        path: "docs/designs/issue-386-driver-neutral-gui-host.md"
      - type: "file"
        path: "docs/test-specs/issue-386-driver-neutral-gui-host.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-issue-386-driver-neutral-gui-host.md"
  implemented-by: []
  depends-on:
    - "../tasks/t2-fake-driver-issue-386.md"
    - "../tasks/t3-gpui-driver-minimal-window-issue-386.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../designs/issue-386-driver-neutral-gui-host.md"
  derived-by: []
  verifies:
    - "../test-specs/issue-386-driver-neutral-gui-host.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# T5: Fake/GPUI 共通 Window/Dock lifecycle fixture

## Work

- `ruviform-ui-test::lifecycle` に driver 中立の共通シナリオ
  `run_lifecycle` と `expected_trace()` を追加する（2 Window・Dock 2-phase
  move/rollback・detach/reattach・close veto/defer/approve・unanswered close
  既定・late receipt・id 非再利用・tear-down）。
- Fake は `tests/lifecycle_fixture.rs`（workspace `test`）で同じ trace を検証する。
- GPUI は `cfg(all(test, gpui-test))` 専用の crate 内 seam（`GpuiDriver::settle_for_test`・
  `inbox_len_for_test`、`pub(crate)`）と `src/lifecycle_fixture.rs`（`#[gpui::test]`）で同じ
  trace を検証する。`gpui-test` ビルドでは surface 親 HWND を取得しない
  （test platform は window handle で panic するため）。

## Acceptance

- Fake / GPUI が同一 `expected_trace()` で通る（R6）。
- 旧 WebView path・本番 `run` は変更しない。
- 実 OS/IME/DPI は NotRun（design の検証節に記録）。
