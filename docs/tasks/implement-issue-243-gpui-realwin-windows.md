---
id: "TASK-034UdR18snUStfTR5NpZge"
type: "task"
status: "done"
title: "Issue #243 実装タスク"
created: "2026-11-09"
updated: "2026-11-11"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/243"
  changes:
    added:
      - "crates/ruviform-ui-gpui/examples/window_lifecycle_probe.rs"
      - "docs/impl/ir/issue-243-gpui-realwin-windows.md"
    modified:
      - "crates/ruviform-ui-gpui/Cargo.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-issue-243-gpui-realwin-windows.md"
  implemented-by: []
  depends-on:
    - "../plans/implement-issue-243-gpui-realwin-windows.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../test-specs/issue-243-gpui-realwin-windows.md"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-issue-243-gpui-realwin-windows.md"
  derived-by: []
  verifies:
    - "../test-specs/issue-243-gpui-realwin-windows.md"
  verified-by:
    - "../impl/ir/issue-243-gpui-realwin-windows.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #243 実装タスク

## Work

Windows 実機での GPUI Window lifecycle 実測（Issue #243 部分スコープ）。
観測用 example `window_lifecycle_probe` を追加し、probe 既定モード・
degenerate モード・`real_window_lifecycle`・`minimal_window` を実行して
証拠を impl record に記録する。

| # | タスク | 状態 |
| --- | --- | --- |
| T1 | `window_lifecycle_probe.rs` 実装 + `Cargo.toml` `[[example]]` エントリ | done |
| T2 | probe 既定モードを `wm-close.ps1 -Steps "veto,veto,veto,main"` で実機実行 | done（PASS・exit 0） |
| T3 | probe `--degenerate-create` を実機実行し分類記録 | done（受理+clamp を記録） |
| T4 | `real_window_lifecycle` を `wm-close.ps1 -Steps "inspector,inspector,main"` で再実行 | done（PASS・exit 0） |
| T5 | `minimal_window` を `wm-close.ps1 -Steps "minimal"` で再実行 | done（exit 0） |
| T6 | impl record に証拠・NotRun/Unsupported 分類・再現手順を記録 | done |
| T7 | `git merge origin/main` 後に fmt-check / check / lint-ui-gpui / test-ui-gpui | done（全 exit 0・impl record Validation 参照） |

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-243-gpui-realwin-windows.md`
- `docs/specs/issue-243-gpui-realwin-windows.md`
- `docs/test-specs/issue-243-gpui-realwin-windows.md`

## Dependencies

- `evidence/issue-431-real-verify/wm-close.ps1`（既存・実行のみ）
- 実 OS session（Windows 11・対話 session で window が表示できること）

## Affected Files

- `crates/ruviform-ui-gpui/examples/window_lifecycle_probe.rs`（新規）
- `crates/ruviform-ui-gpui/Cargo.toml`（`[[example]]` エントリ追加のみ）
- `docs/impl/ir/issue-243-gpui-realwin-windows.md`（新規・証拠記録）

## Done When

- [ ] probe 既定モードが `[probe] PASS` + exit 0（実機）
- [ ] degenerate モードの観測結果を分類して記録
- [ ] `real_window_lifecycle`・`minimal_window` の再実行が exit 0
- [ ] impl record に Pass/Fail/NotRun/Unsupported 分類と再現手順がある
- [ ] `mise run fmt-check` / `check` / `lint-ui-gpui` / `test-ui-gpui` が exit 0

## Verification

- [ ] 実機ログを impl record に引用（コマンド・exit code・stderr 全量）
- [ ] review-protocol レビューで blocker なし
