---
id: "PLAN-034Ug8XG3cIM1VEYu5sQpv"
type: "plan"
status: "approved"
title: "Issue #314 slice 3 — GPUI/OS capture backend 実装計画"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/314"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-314-golden-capture.md"
      - type: "file"
        path: "docs/tasks/issue-314-golden-capture.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/designs/issue-314-golden-capture.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-314-golden-capture.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-314-golden-scenario.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-314-golden-capture.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #314 slice 3 — GPUI/OS capture backend 実装計画 Implementation Plan

## Goal

`GpuiDriver` が開く実 OS window の client area pixel を OS backend
（Windows: `PrintWindow` primary / screen `BitBlt` fallback）で取得し、
`OsCapturedFrame` → `RgbaImage` → `BaselineStore` の golden e2e を
Windows 実機で検証する。

## Scope

単一実装ストリーム。capture backend・driver wiring・e2e test・
文書チェーンを 1 slice で完結させる。scenario adapter 層・
consumer fixture・非 Windows backend は別 slice/別 issue。

## Upstream Documents

- `docs/specs/issue-314-golden-capture.md`（spec — 本 plan の正本）
- `docs/specs/issue-314-golden-scenario.md`（slice 2 — 契約）
- `docs/specs/issue-314-golden-core.md`（slice 1 — Golden Utility）
- `docs/specs/issue-243-gpui-realwin-windows.md`（実 window 経路）

## Design Inputs

- `docs/designs/issue-314-golden-capture.md`（approved）

## File Map

- `crates/ruviform-ui-gpui/src/gpui_driver/capture.rs` — 新規。
  `OsCapturedFrame` DTO、`GpuiDriver::capture_window`、per-OS
  backend（Windows: PrintWindow/BitBlt、非 Windows: Unsupported）。
- `crates/ruviform-ui-gpui/src/lib.rs` — `pub mod capture;` 宣言 +
  `OsCapturedFrame` re-export のみ（#479 領域に触れない）。
- `crates/ruviform-ui-gpui/tests/golden_capture.rs` — 新規。
  error path test + e2e golden test（Windows + gpui feature）。
- `crates/ruviform-ui-test/src/scenario/visual.rs` — module doc
  更新のみ（動作変更なし）。
- `docs/{specs,designs,plans,tasks,test-specs}/issue-314-golden-capture.md`
  — 本 slice の文書チェーン。

## Implementation Sequence

- [x] `capture.rs` 新設: `OsCapturedFrame`、`capture_window` の
      slot 検査、Windows backend（probe → client rect →
      PrintWindow → BitBlt fallback → DIB → BGRA→RGBA）。
- [x] `lib.rs` wiring: mod 宣言 + re-export。
- [x] `golden_capture.rs`: unknown/unopened error test +
      e2e golden test（settle → capture → compare → promote →
      compare → evidence）。
- [x] `visual.rs` doc 更新。
- [x] 文書チェーン記述。
- [ ] `git fetch origin main && git merge origin/main` で最新化。
- [ ] mise canonical 検証一式。
- [ ] review-protocol レビュー。
- [ ] commit / push / PR / worker_done。

## Dependencies and Risks

- #243 の `Slot::raw_parent` / `surface::raw_parent_of` が前提
  （main 入り済み）。
- `PrintWindow` は `extern "system"` 局所宣言 — windows-sys
  `Win32_Storage_Xps` feature を避けるため（Cargo.toml boundary）。
- 実 window test はデスクトップ環境を必要とする — CI/ヘッドレス
  環境では実行不可（`#[cfg(all(windows, feature = "gpui"))]` で
  gate、非対応環境では test 対象外）。
- frame 決定性は settle tick 数に依存 — 実測で 0 diff を確認したが
  環境差の可能性は残る（test-spec に明記）。

## Task Breakdown

- TASK-034Ug9lk3rk5h8iKkSQY2c（`docs/tasks/issue-314-golden-capture.md`）
  — 本 slice 全体を 1 task として管理。

## Verification

- [x] `cargo check -p ruviform-ui-gpui --features gpui` — 通過。
- [x] `cargo test -p ruviform-ui-gpui --features gpui --test
      golden_capture` — 2 passed（補助証拠、canonical = mise）。
- [ ] `mise run test-ui-golden` — Golden Utility 非回帰。
- [ ] `mise run test-ui-scenario-contract` /
      `test-ui-scenario-contract-fake` /
      `test-ui-scenario-contract-gpui` — adapter 非回帰。
- [ ] `mise run check` / `lint` / `fmt-check` /
      `test-ui-test-boundary`。
- [ ] `mise run check-ui-driver-boundary`（存在すれば）— 公開
      signature の driver-neutral 性。

## Open Questions / Gaps

- macOS/Linux backend — 未実装（`Unsupported`）。実機環境確保後。
- `PrintWindow` と表示内容の一致保証 — `PW_RENDERFULLCONTENT`
  で DWM 合成面を取得するが、window class 依存の差異は残る
  可能性。実利用で観測されたら再検討。
- settle tick 数の環境依存性 — Windows 実測値。CI での実行は
  デスクトップ環境が前提。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（Orca dispatched worker）。
- Independent work streams: none。
- User approval required before dispatch: yes（coordinator dispatch
  済み）。
- If approved: discover available implementation and delegation
  capabilities in the current environment, then execute task-by-task
  with review checkpoints.
- If declined or unavailable: execute inline using the same
  dependency graph and verification matrix.

## Review Handoff

- `PrintWindow` の `extern "system"` 宣言が windows-sys feature
  追加より適切か（boundary 制約下での選択）。
- `OsCapturedFrame::backend` の `&'static str` が evidence として
  十分か（将来 enum 化の可能性）。
- settle tick 数の根拠と flaky リスクの評価。
