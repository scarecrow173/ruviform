---
id: "TASK-034Ue8ThHNXd5Ah4vY5uvG"
type: "task"
status: "done"
title: "Issue #451 FU-1b — Tab/Shift-Tab フォーカス巡回配線の実装タスク"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-451-focus-next-wiring.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-451-focus-next-wiring.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-451-focus-next-wiring.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #451 FU-1b — Tab/Shift-Tab フォーカス巡回配線の実装タスク

## Work

計画 `PLAN-034Ue8cy65F5L1MWmEzj3E` のスライスを実施する。

- T1: `Inner` に `focus_nav_wired` を追加し、`drain` 先頭で一度だけ `cx.observe_keystrokes` を登録する `wire_focus_navigation` を `lib.rs` に実装する（`tab` 無修飾→`focus_next`、shift のみ→`focus_prev`、その他修飾→無視）。
- T2: `RvfView::focused_tab_key`（`pub(crate)`）と `GpuiObserver::press_key` / `GpuiObserver::focused_tab`（`gpui-test` 限定 seam）を追加し、`tab_focus` の `FocusHandle` を `.tab_stop(true)` で作成する（`track_focus` handle には要素側 `tab_stop` が伝播しないため）。
- T3: `tests/consumer_observation.rs` に 2 つの selection 付き Tabs strip を持つ view で Tab / Shift-Tab / Ctrl+Tab のフォーカス遷移を検証するテストを追加する（test-spec E1–E4）。
- T4: 検証 lane（`test-ui-gpui` / `test-ui` / `check` / `lint` / `fmt-check`）を実行する。
- T5: main 取り込み・docs 索引再生成・review-protocol レビュー・PR 作成。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-451-focus-next-wiring.md`
- `docs/specs/issue-451-focus-next-wiring.md`
- `docs/designs/issue-451-focus-next-wiring.md`
- `docs/test-specs/issue-451-focus-next-wiring.md`

## Dependencies

- Issue #451 FU-1（PR #457/#460 で main 統合済み: tab-select intent・tab stop 登録・矢印キー操作）。

## Affected Files

- `crates/ruviform-ui-gpui/src/lib.rs`
- `crates/ruviform-ui-gpui/src/render.rs`
- `crates/ruviform-ui-gpui/src/gpui_driver/observe.rs`
- `crates/ruviform-ui-gpui/tests/consumer_observation.rs`
- `docs/**`（本 chain と索引）

## Done When

- [x] Tab が `focus_next`、Shift-Tab が `focus_prev` を呼ぶ配線が `ruviform-ui-gpui` 内にある。
- [x] headless テストでフォーカスが次 / 前の tab stop へ移り、循環と修飾キー除外が確認できる。
- [x] `mise run test-ui-gpui`・`test-ui`・`check`・`lint`・`fmt-check` が pass する。

## Verification

- [x] `mise run test-ui-gpui`
- [x] `mise run test-ui`
- [x] `mise run check`
- [x] `mise run lint`
- [x] `mise run fmt-check`
