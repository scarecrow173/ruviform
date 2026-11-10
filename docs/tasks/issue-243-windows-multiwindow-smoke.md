---
id: "TASK-034V4uYcdcMjdFzOg15rFm"
type: "task"
status: "in-progress"
title: "Issue #243 — Windows 実 OS multiwindow smoke 実装タスク"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/243"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-243-windows-multiwindow-smoke.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-243-windows-multiwindow-smoke.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-243-windows-multiwindow-smoke.md"
    - "docs/designs/issue-243-windows-multiwindow-smoke.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-243-windows-multiwindow-smoke.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-243-windows-multiwindow-smoke.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-243-windows-multiwindow-smoke.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #243 — Windows 実 OS multiwindow smoke 実装タスク

計画 `PLAN-034V4uVEulNfghJYxbNL9Y` のスライスを実施順の
タスクに分けたもの。

## Work

`real_os_multiwindow_smoke` example・`Cargo.toml`/
`mise.toml` lane・Windows 実機 verdict・capability matrix
追記・docs チェーン・PR（`Refs #243`）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-243-windows-multiwindow-smoke.md`
- `docs/specs/issue-243-windows-multiwindow-smoke.md`
- `docs/designs/issue-243-windows-multiwindow-smoke.md`

## Dependencies

- `os_input` SendInput API・`window_geometry`/
  `component_bounds`（#313, PR #541 merged）
- `GpuiDriver`/`WindowDriver`/`DockWorkspace`/`UiRuntime`
  （#386/#181/#182 merged）
- 先行 slice `window_lifecycle_probe`（#243, PR #482 merged）

## Affected Files

- `crates/ruviform-ui-gpui/examples/real_os_multiwindow_smoke.rs`（新規）
- `crates/ruviform-ui-gpui/Cargo.toml`
- `mise.toml`
- `evidence/issue-243-multiwindow-verify/verdict-multiwindow-windows.json`（新規）
- `evidence/rf313-os-capability-matrix/{report.md,results.jsonl}`
- `docs/{specs,designs,plans,tasks,test-specs,impl/ir}/issue-243-windows-multiwindow-smoke.md`

## Tasks

- [x] T1 `real_os_multiwindow_smoke.rs` fixture 実装
  （phase machine・3 window・focus/keyboard/Dock/move/
  close/regenerate/job policy・OS oracle・cursor 復元）。
- [x] T2 `Cargo.toml` `[[example]]` 宣言。
- [x] T3 Windows 実機 smoke 実行 → verdict JSON 採取
  （17 checks pass、verdict=pass を 1 回実測）。
- [x] T4 事後 run で SendInput が kernel driver に drop される
  環境退行を検出 → cursor read-back oracle 強化
  （commanded point・cursor 実位置・hit-test・foreground を
  記録）で診断可能化。
- [x] T5 `mise.toml` manual lane
  （`example-ui-gpui-real-os-multiwindow-smoke`）。
- [x] T6 `evidence/rf313-os-capability-matrix/` report/results
  更新（multiwindow 行を実測で追記 — Pass（前回実測）と
  NotRun（再検証・環境ブロック）を誠実に分類）。
- [x] T7 `fmt-check`/`check-ui-gpui`/`lint-ui-gpui`/
  `test-ui-gpui`/`test-rt`/`check`/`test`/`lint` 検証
  （全 lane exit 0）。
- [x] T8 docs チェーン + index 再生成、PR 作成
  （`Refs #243`、Issue は Open 継続）。実 OS 再検証は
  SendInput 環境ブロックにより NotRun — 健全 host での
  再実行を follow-up として記録。

## Upstream

- `docs/test-specs/issue-243-windows-multiwindow-smoke.md`
