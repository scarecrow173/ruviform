---
id: "TASK-034VOSINPDRV4REAL0SIN5"
type: "task"
status: "done"
title: "Issue #313 — `RealOsPointerOrKeyboard` capability 実装タスク（slice S1）"
created: "2026-11-11"
updated: "2026-11-11"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/313"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-313-task1-real-os-input-driver.md"
      - type: "file"
        path: "docs/impl/ir/issue-313-real-os-input-driver-slice.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/real_window.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/os_input.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/real_window.rs"
      - type: "file"
        path: "mise.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-313-real-os-input-driver-plan.md"
  implemented-by:
    - "docs/impl/ir/issue-313-real-os-input-driver-slice.md"
  depends-on:
    - "docs/plans/issue-313-real-os-input-driver-plan.md"
    - "docs/designs/issue-313-real-os-input-driver-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-313-windows-pointer-smoke.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #313 — `RealOsPointerOrKeyboard` capability 実装タスク（slice S1）

## T1: OS-input port on `RealOsGuiTest`

- [ ] `os_input.rs` に `OsInputOutcome`（`Injected`/`Unsupported`/
      `EnvironmentBlocked`/`NotRun`、public `#[non_exhaustive]`）を追加。
- [ ] `real_window.rs`: `run` handler で `Focused(active)` を
      `focused: Arc<Mutex<BTreeMap<WindowId,bool>>>` に記録。
- [ ] `supported_capabilities()` に `os_input::supported()` 条件で
      `RealOsPointerOrKeyboard` を追加。module doc の
      「Issue #313 scope」注記を実装済み契約へ更新。
- [ ] `os_preflight` cache + `os_input_preflight(scope)` +
      `window_focused` + `request_focus` + `os_press_key` +
      `os_pointer_move_to` + `os_mouse_down` + `os_mouse_up`。
- [ ] 単体テスト: capability 集合・preflight 合成・aim 計算。

## T2: OS 注入 scenario テスト + lane

- [ ] `tests/real_window.rs` に OS 注入 scenario テスト
      （smoke spec: Panel[Fader(gain), Seek(seek)]）。
      Windows: preflight → focus → `run_scenario` で keyboard/pointer
      ops → intents → receipt → observe → verdict JSON
      （`RUVIFORM_REAL_OS_INPUT_VERDICT`）+ driver_lab emit。
      非 Windows: `unsupported` 証拠。
- [ ] `mise.toml` に `test-ui-real-os-scenario` 追加。

## T3: 実走・証拠化・matrix

- [ ] headless lanes 全 pass。
- [ ] `test-ui-real-os-scenario` 実走 → verdict JSON。
- [ ] smoke/回帰 lanes 実走。
- [ ] `evidence/rf313-os-capability-matrix/` 更新 + impl record。
