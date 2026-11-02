---
id: "TASK-5bJ6JAK2lt9CHr2ZLSR42R"
type: "task"
status: "done"
title: "ruviform-ui: fake-host multi-window contract tests"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui/tests/multi_window.rs"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-181-multi-window.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-181-02-runtime-routing.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-181-multi-window.md"
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

# ruviform-ui: fake-host multi-window contract tests

## Work

`crates/ruviform-ui/tests/multi_window.rs` に、`WindowHandle` を実装する FakeHost を用意し、test-spec（`docs/test-specs/issue-181-multi-window.md`）の受入項目を headless 検証する。

- FakeHost: `HashMap<WindowId, FakeWindow{ready, pending, close_state}>` + `WindowEvent` 発行記録 + eval 配送記録 + close 決定機構（未回答既定 `Approve`）+ bounded pending（drop-oldest）+ `windows()`/`monitors()`/`exit()`。adapter が実装すべき契約セマンティクスの参照実装。
- テスト: (a) 2 Window イベント分離、(b) WebView 間 `id` 衝突の正しい routing、(c) CloseRequested の Approve/Veto/Defer 全経路、(d) 未回答既定 Approve、(e) `request_close` が veto-able 経路を通る、(f) 破棄済み宛先への eval silent drop + fallible op `Err`、(g) pre-`Ready` eval の bounded 保留と `Ready` flush、(h) `create_window` 失敗で Window を残さない、(i) Window Close で Job 非取消 + owner routing + owner 破棄後の安全 drop、(j) `cancel_window_jobs` opt-in、(k) 全 Window 破棄でも auto-exit しない。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-181-multi-window.md`
- `docs/test-specs/issue-181-multi-window.md`（全 Feature/Rule）
- `docs/designs/issue-181-multi-window.md`（失敗・フォールバック規約表）

## Dependencies

- `docs/tasks/issue-181-02-runtime-routing.md`（UiRuntime multi-window 経路）。

## Verification

- `cargo test -p ruviform-ui --test multi_window` green。
- `mise run fmt-check`・`check`・`lint` green。
