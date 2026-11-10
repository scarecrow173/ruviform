---
id: "PLAN-034UkPTR1W1N5M0K3P04"
type: "plan"
status: "approved"
title: "Issue #313 — Windows 実 OS SendInput ポインタ smoke 実装計画"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/313"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-313-windows-pointer-smoke.md"
      - type: "file"
        path: "crates/ruviform-ui-gpui/examples/real_os_pointer_smoke.rs"
      - type: "file"
        path: "evidence/rf313-os-capability-matrix/verdict-pointer-windows.json"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/os_input.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/capture.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/Cargo.toml"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "evidence/rf313-os-capability-matrix/report.md"
      - type: "file"
        path: "evidence/rf313-os-capability-matrix/results.jsonl"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-313-windows-pointer-smoke.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-313-windows-pointer-smoke.md"
    - "docs/designs/issue-313-windows-pointer-smoke.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-313-windows-realos-smoke.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-313-windows-pointer-smoke.md"
    - "docs/designs/issue-313-windows-pointer-smoke.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-313-windows-pointer-smoke.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #313 — Windows 実 OS SendInput ポインタ smoke 実装計画 Implementation Plan

## Goal

`os_input` ポインタ API、driver 計測ジオメトリ
（`component_bounds`/`window_geometry`）、`real_os_pointer_smoke`
example、manual mise lane、capability matrix 証拠更新を追加し、
Windows 実機で drag/click → gesture intent を実測する
（Windows 部分スコープ）。

## Scope

`ruviform-ui-gpui` の検証 surface（`gpui` feature 内）と証拠/
文書のみ。`WindowDriver` neutral trait・製品 Scenario DSL・
consumer 依存は変更しない。

## Upstream Documents

- `docs/specs/issue-313-windows-pointer-smoke.md`
- `docs/designs/issue-313-windows-pointer-smoke.md`

## Design Inputs

- `docs/designs/issue-313-windows-pointer-smoke.md`

## File Map

- `crates/ruviform-ui-gpui/src/gpui_driver/os_input.rs` —
  `cursor_position`/`move_to`/`mouse_down`/`mouse_up` 追加、
  `send` 共通ヘルパーで部分注入を `Host` 化。
- `crates/ruviform-ui-gpui/src/render.rs` — `ComponentBounds`
  alias、`ViewChannels` bundle、`RvfView::mounted` 引数整理、
  `logical_rect_of` の `pub(crate)` 化。
- `crates/ruviform-ui-gpui/src/lib.rs` — slot の bounds store、
  `mount_view` の `ViewChannels` 化、`component_bounds` query、
  `OsWindowGeometry` re-export。
- `crates/ruviform-ui-gpui/src/gpui_driver/capture.rs` —
  `OsWindowGeometry`、`window_geometry`、Windows backend
  `client_geometry`、非 Windows `Unsupported` stub。
- `crates/ruviform-ui-gpui/examples/real_os_pointer_smoke.rs`
  （新規）— 照準・注入系列・read-back・カーソル復元・verdict。
- `crates/ruviform-ui-gpui/Cargo.toml` — `[[example]]` 追加。
- `mise.toml` — `example-ui-gpui-real-os-pointer-smoke` lane。
- `evidence/rf313-os-capability-matrix/` — verdict JSON 追加、
  report/results 更新。
- `docs/{specs,designs,plans,tasks,test-specs,impl}` — 本
  スライスの文書チェーン。

## Dependency Graph

os_input API → capture geometry → render bounds store →
example（照準に両者を使用）→ mise lane → 実機実行 → 証拠更新
→ docs チェーン → PR。

## Risk Register

- カーソル read-back 逸脱（SendInput は受理されても OS 側で
  補正される場合）→ check で Fail として記録、Pass 偽装なし。
- `Focused(true)` 後も click-to-focus で初回 click が吸収
  される OS 設定 → focus ゲート + FOCUS_SETTLE で緩和、
  残れば Fail として正直に記録。
- paint 前の bounds 未取得 → tick 毎 poll + TIMEOUT で
  Fail（ゼロサイズ捏造なし）。
- カーソル復元失敗 → best-effort + stderr 記録、verdict 不変。

## Verification Matrix

| 検証 | lane |
| --- | --- |
| 型/lint | `check-ui-gpui` / `lint-ui-gpui` |
| headless test | `test-ui-gpui`（`gpui-test`） |
| fmt | `fmt-check` |
| 実機 smoke | `example-ui-gpui-real-os-pointer-smoke`（manual） |
| workspace | `check` / `lint` / `test`（headless lane） |
