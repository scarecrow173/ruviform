---
id: "TASK-034Ug9lk3rk5h8iKkSQY2c"
type: "task"
status: "in-progress"
title: "Issue #314 slice 3 — GPUI/OS capture backend"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/314"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/capture.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/golden_capture.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/scenario/visual.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-314-golden-capture.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-314-golden-capture.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-314-golden-scenario.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/test-specs/issue-314-golden-capture.md"
  verified-by:
    - "docs/test-specs/issue-314-golden-capture.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #314 slice 3 — GPUI/OS capture backend

## Work

`GpuiDriver` の実 OS window pixel capture を実装する。

- `gpui_driver/capture.rs` 新設: `OsCapturedFrame`（RGBA8 +
  scale_factor + backend tag）、`GpuiDriver::capture_window`
  （slot 検査 → OS handle → backend 委譲）、Windows backend
  （`PrintWindow` primary / screen `BitBlt` fallback、32bpp
  top-down DIB → BGRA→RGBA）、非 Windows `Unsupported` stub。
- `lib.rs`: `pub mod capture;` + `OsCapturedFrame` re-export
  のみ（#479 領域不変、coordinator 承認済み）。
- `tests/golden_capture.rs`: unknown/unopened window の error
  path test、実 window e2e（settle → capture → `RgbaImage` →
  `BaselineStore::compare`/`write_candidate`/`promote` →
  `pass/match` → `append_evidence`）。
- `scenario/visual.rs`: module doc を slice 3 実装済みの事実に
  合わせて更新（動作変更なし）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-314-golden-capture.md`（実装計画）
- `docs/designs/issue-314-golden-capture.md`（設計 — PrintWindow
  primary / BitBlt fallback、driver 層 API の根拠）
- `docs/specs/issue-314-golden-capture.md`（spec — R1〜R8/AC-1〜8）

## Dependencies

- slice 1（`docs/specs/issue-314-golden-core.md`、PR #481）—
  `BaselineStore`/`RgbaImage`/`GoldenKey`/`EnvironmentMeta`/
  `append_evidence`。
- slice 2（`docs/specs/issue-314-golden-scenario.md`、PR #483）—
  `CaptureOutcome`/`VisualGuiTest` 契約（adapter 層は不変）。
- #243（`docs/specs/issue-243-gpui-realwin-windows.md`、PR #482）—
  `GpuiDriver::run`・`Slot::raw_parent`・`surface::raw_parent_of`。

## Affected Files

- `crates/ruviform-ui-gpui/src/gpui_driver/capture.rs`（新規）
- `crates/ruviform-ui-gpui/src/lib.rs`（mod 宣言 + re-export）
- `crates/ruviform-ui-gpui/tests/golden_capture.rs`（新規）
- `crates/ruviform-ui-test/src/scenario/visual.rs`（doc のみ）
- `docs/{specs,designs,plans,tasks,test-specs}/issue-314-golden-capture.md`
  （新規）+ `docs/*/README.md`（index 自動生成）

## Done When

- [x] `capture_window` が実 window client area の RGBA8 を返す
      （`pixels.len() == w*h*4`、alpha=255、top-down）。
- [x] 未知/未 open window → `UiError::Host`、OS handle なし →
      `UiError::Unsupported`（pixel 捏造なし）。
- [x] Windows backend が `PrintWindow` primary・`BitBlt` fallback
      を実装し、`OsCapturedFrame::backend` に経路を記録する。
- [x] e2e test が `blocked/baseline_missing` → `pass/match` +
      evidence 2 行を検証して pass する。
- [x] `gpui-test`/Fake adapter の `capture_frame`・
      `capabilities()` が非回帰（`Unsupported`・`VisualCapture`
      非宣言を維持）。
- [ ] mise canonical 検証一式が pass する。

## Verification

- [x] `cargo check -p ruviform-ui-gpui --features gpui`（補助）。
- [x] `cargo test -p ruviform-ui-gpui --features gpui --test
      golden_capture` — 2 passed（補助、canonical = mise）。
- [ ] `mise run test-ui-golden` / `test-ui-scenario-contract` /
      `test-ui-scenario-contract-fake` /
      `test-ui-scenario-contract-gpui` / `check` / `lint` /
      `fmt-check` / `test-ui-test-boundary`。
