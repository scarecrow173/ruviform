---
id: "PLAN-034UjMMA7vTOdlkqHFOrca"
type: "plan"
status: "approved"
title: "Issue #313 — Windows 実 OS SendInput キーボード smoke 実装計画"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/313"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-313-windows-realos-smoke.md"
    modified:
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
    - "docs/specs/issue-313-windows-realos-smoke.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-313-windows-realos-smoke.md"
    - "docs/designs/issue-313-windows-realos-smoke.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-479-keyboard-value-controls.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-313-windows-realos-smoke.md"
    - "docs/designs/issue-313-windows-realos-smoke.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-313-windows-realos-smoke.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #313 — Windows 実 OS SendInput キーボード smoke 実装計画 Implementation Plan

## Goal

実 GPUI window に value control を mount し、Win32 `SendInput` で
Tab/矢印キーを注入して `WindowLifecycle::Action` intent の到達を
実測する `real_os_smoke` example、`os_input` モジュール、manual
mise lane、OS Capability Matrix 証拠を追加する（Windows 部分
スコープ）。

## Scope

単一の実装ストリーム。対象は `ruviform-ui-gpui` の example /
薄い注入モジュール / 証拠文書のみで、製品コードの挙動は変えない。

## Upstream Documents

- `docs/specs/issue-313-windows-realos-smoke.md`
- `docs/designs/issue-313-windows-realos-smoke.md`

## Design Inputs

- `docs/designs/issue-313-windows-realos-smoke.md`

## File Map

- `crates/ruviform-ui-gpui/src/gpui_driver/os_input.rs` - Win32
  `SendInput` キーストローク注入（`OsKey`/`press_key`）。
- `crates/ruviform-ui-gpui/src/lib.rs` - `pub mod os_input` と
  re-export の 2 行のみ。
- `crates/ruviform-ui-gpui/examples/real_os_smoke.rs` - smoke
  example（view mount、injector thread、verdict JSON）。
- `crates/ruviform-ui-gpui/Cargo.toml` - `[[example]]` 定義。
- `mise.toml` - `example-ui-gpui-real-os-smoke` manual lane。
- `evidence/rf313-os-capability-matrix/` - report.md /
  results.jsonl / verdict-windows.json。
- `docs/impl/ir/issue-313-windows-realos-smoke.md` - 実施記録。

## Implementation Sequence

- [x] T1: `os_input.rs` + lib.rs の `pub mod`/re-export。
- [x] T2: `real_os_smoke.rs` example + `Cargo.toml` `[[example]]`。
- [x] T3: `mise.toml` manual lane。
- [x] T4: build & Windows 実機で smoke 実行、verdict JSON 採取。
- [x] T5: `evidence/rf313-os-capability-matrix/` の report/results
  作成（#243/#314 リンク、SendInput の性質明記）。
- [x] T6: `test-ui-gpui` / `check` / `lint` / `fmt-check` /
  manual lane の検証。
- [x] T7: `origin/main` 取り込み後の再検証。
- [x] T8: review-protocol レビュー（blocker なしを確認）。
- [x] T9: PR 作成（`Refs #313`）。

## Dependencies and Risks

- 前提: GPUI Host・`capture_window`（#314）・#479 キーボード規約・
  #243 実窓証拠が main にある（確認済み）。
- リスク: foreground focus が取れない環境では Fail（誠実な記録）。
- リスク: SendInput が IME/secure desktop に阻まれる場合は Fail。

## Verification Matrix

| 検証 | 手段 |
| --- | --- |
| 型/lint | `mise run check-ui-gpui`, `lint-ui-gpui` |
| 既存回帰 | `mise run test-ui-gpui` |
| fmt | `mise run fmt-check` |
| 実 OS smoke | `mise run example-ui-gpui-real-os-smoke`（manual、Windows） |
| workspace | `mise run check`, `mise run lint` |
