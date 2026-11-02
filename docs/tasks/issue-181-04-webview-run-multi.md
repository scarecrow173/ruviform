---
id: "TASK-kIMP2tx8yXfzMncvaRoJVK"
type: "task"
status: "done"
title: "ruviform-ui-webview: SessionConfig + run_multi (wry/tao multi-window host)"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-ui-webview/src/host.rs"
      - type: "file"
        path: "crates/ruviform-ui-webview/src/lib.rs"
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

# ruviform-ui-webview: SessionConfig + run_multi

## Work

`host.rs` に multi-window ホスト経路を additive 追加する（`webview` feature 配下、`host::run` は不変）。

- `SessionConfig{data_dir, tick, windows: Vec<WindowConfig>}` + `run_multi(session, assets, data, on_event: FnMut(WindowEvent, &mut dyn WindowHandle))`。
- Window レジストリ: `HashMap<WindowId, Entry{window, webview, ready, pending, close_state}>` + `tao::window::WindowId` → `WindowId` 写像、`next_id` 単調採番。
- 共有 `WebContext` 1 基（`data_dir` 既定 `rvf-udf-multi-<pid>` — WebView2 UDF シングルトン）、全 WebView が `new_with_web_context` で共有、`WebContext` は全 WebView より長命（`run_multi` ローカル）。
- `WindowEvent{window_id}` → `Windowed{w,...}` 変換、IPC/pageload callback は `WindowId` 付き共有キュー + `EventLoopProxy` 起床（`run` と同構造）。
- close 決定機構: `CloseRequested` → `Requested` state + emit → `respond_close`（Approve=即 destroy+`Destroyed` enqueue / Veto=clear / Defer=ClosePending）→ 未回答は既定 Approve。`request_close` は同一経路の再 emit。`destroy_window` は即破棄 + `Destroyed`。
- 動的生成: `create_window` が `EventLoopWindowTarget` 経由で `WindowBuilder`+共有 `WebContext` の `build_webview` → `Ok(WindowId)`。失敗は `Err` で巻き戻し。
- pre-`Ready` eval の bounded pending（cap 64、drop-oldest、Ready flush）、不明/破棄済み eval の silent drop、fallible op の `Err(UiError)`。
- `windows()`/`monitors()`（`available_monitors` → `MonitorInfo`）/`exit()`（`want_exit` → `MainEventsCleared` で Exit）。Window 全滅で終了しない。
- `lib.rs`: `SessionConfig`/`run_multi` re-export。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-181-multi-window.md`
- `docs/designs/issue-181-multi-window.md`（`ruviform-ui-webview` 実装構造節）

## Dependencies

- `docs/tasks/issue-181-01-window-contract-types.md`（window 契約型）。

## Verification

- `mise run check-ui-webview`・`lint-ui-webview`・`test-ui-webview` green（`webview` feature lane — 型・lint は headless、窓付き実動作は手動検証へ記録）。
