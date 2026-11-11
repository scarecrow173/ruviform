---
id: "TASK-034WIND0WEDTESTI0S2TK"
type: "task"
status: "done"
title: "Issue #313 — windowed `TestIo`/`rvf.uitest.v1` 実装タスク（slice S2）"
created: "2026-11-11"
updated: "2026-11-11"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/313"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-313-windowed-testio-slice.md"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/windowed_io.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/bin/windowed_test_app.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/windowed_uitest.rs"
      - type: "file"
        path: "evidence/issue-313-windowed-uitest/verdict-windowed-uitest.json"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/src/io.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/fixture.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/headless.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/journal.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/agent.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/fake_driver.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/bin/dummy_app.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/Cargo.toml"
      - type: "file"
        path: "mise.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-313-windowed-testio-plan.md"
  implemented-by:
    - "docs/impl/ir/issue-313-windowed-testio-slice.md"
  depends-on:
    - "docs/plans/issue-313-windowed-testio-plan.md"
    - "docs/designs/issue-313-windowed-testio-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-313-task1-real-os-input-driver.md"
    - "docs/tasks/task-312-t2-control-plane.md"
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

# Issue #313 — windowed `TestIo`/`rvf.uitest.v1` 実装タスク（slice S2）

## T1: neutral 語彙化（`ruviform-ui-test`）

- [x] `io.rs`: `TestIo` を `emit(DriverEvent)`・
      `emit_ipc_body(WindowId, &str)`（新規）・
      `create_window(&WindowSpec)` へ。trait/method doc を neutral
      契約 + `emit_ipc_body` の malformed/suppressed/valid 三分岐
      语义へ更新。
- [x] `fixture.rs`: `TestApp::on_event(DriverEvent, &mut dyn
      WindowDriver)` + `shutdown(mode, &mut dyn WindowDriver)`。
- [x] `journal.rs`: `record_in(&DriverEvent)`（lifecycle kind
      対応表）・`record_suppressed(window, bytes)`・
      `JournalHandle` を `WindowDriver` decorator へ
      （`post_event`/`create_window`/`resize`/`focus`/
      `request_close`/`respond_close`/`destroy_window`/`exit` を
      `Out` 記録）・`deliver_routed` → `post_event`。
- [x] `fake_driver.rs`: `ready(&self, WindowId) -> bool` accessor
      追加。
- [x] `headless.rs`: `FakeDriver` 上に再基盤化。interceptor で
      `window.created`/`window.close_requested`/`window.destroyed`/
      `ipc.suppressed` notice を lifecycle event から生成。
      `emit_ipc_body`（armed→suppressed / valid→`Action` 注入 /
      malformed→`In "message"` + `post_event(Event::error)`）。
      `windows()` は `spec_of`/`ready`/`close_pending`/`is_ipc_down`
      の実 state。`url: None`。
- [x] `agent.rs`: `emit` 2 箇所を `DriverEvent::Tick` /
      `Windowed{CloseRequested}` へ。
- [x] `dummy_app.rs`: `WindowSpec`/`DriverEvent`/`WindowLifecycle`
      へ。`UiRuntime::dispatch_window_intent` +
      `deliver_routed`（post_event）。`ipc_emit` fixture →
      `emit_ipc_body`。`windows` fixture → `emit(Ready)`。
- [x] tests 移行 + `emit_ipc_body` 三分岐・metadata・close flow
      の検証追加。

## T2: windowed adapter（`ruviform-ui-gpui`）

- [x] `Cargo.toml`: `uitest` feature + `base64` optional dep +
      `[[bin]] windowed_test_app`。
- [x] `lib.rs`: `UiJob`/`set_ui_jobs`/`drain_ui_jobs`/`ui_jobs`
      field の cfg を `any(real-scenario, uitest)` に widen、
      `#[cfg(feature = "uitest")] pub mod windowed_io`。
- [x] `windowed_io.rs`: `WindowedIo`（`call`+settle 2-job、emit
      per-event mapping、`windows()` job 実 slot 読み、
      `emit_ipc_body` 三分岐 job、screenshot→`capture_window`、
      `set_fault`、capabilities/backend 正直 claim）+
      `UiSide`（`deliver` interceptor: journal+notices+ipc 抑止
      +`pending_shutdown` 処理）。
- [x] `bin/windowed_test_app.rs`: AUT（main=serve、app
      thread=`driver.run`+`catch_unwind`→`exit(3)`、stdout 予約・
      stderr diagnostics、`--fail-init`/`--windows N`、fixtures
      `windows`（real Ready bounded poll）/`ipc_emit`/
      `close_policy`）。

## T3: lane・検証・証拠

- [x] `tests/windowed_uitest.rs`（`RVF_WINDOWED_UITEST=1` opt-in、
      verdict JSON 出力）+ `mise.toml` `test-ui-windowed-uitest`。
- [x] headless lanes 全 pass（canonical mise）。
- [x] windowed lane 実走（desktop 不可なら environment_blocked
      記録）。
- [x] `evidence/issue-313-windowed-uitest/` 記録 + impl IR 作成。
