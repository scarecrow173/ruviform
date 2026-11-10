---
id: "PLAN-034V4uVEulNfghJYxbNL9Y"
type: "plan"
status: "approved"
title: "Issue #243 — Windows 実 OS multiwindow smoke 実装計画"
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
      - type: "file"
        path: "crates/ruviform-ui-gpui/examples/real_os_multiwindow_smoke.rs"
      - type: "file"
        path: "evidence/issue-243-multiwindow-verify/verdict-multiwindow-windows.json"
    modified:
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
    - "docs/specs/issue-243-windows-multiwindow-smoke.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-243-windows-multiwindow-smoke.md"
    - "docs/designs/issue-243-windows-multiwindow-smoke.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-243-gpui-realwin-windows.md"
    - "docs/plans/issue-313-windows-pointer-smoke.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-243-windows-multiwindow-smoke.md"
    - "docs/designs/issue-243-windows-multiwindow-smoke.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-243-windows-multiwindow-smoke.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #243 — Windows 実 OS multiwindow smoke 実装計画 Implementation Plan

## Goal

`real_os_multiwindow_smoke` example（3 実 Window を SendInput
実入力で駆動する multiwindow/Dock/Job 検証 fixture）、
`Cargo.toml`/`mise.toml` lane、verdict JSON 証拠、
capability matrix 追記を追加し、Windows 実機で
focus/close/detach/reattach/move/regenerate/job policy を
実測する（Windows 部分スコープ）。

## Scope

単一 implementation stream。`crates/ruviform-ui-gpui` の
example + 証拠 + docs に限定し、driver/capability 本体・
consumer repo・他 OS には触れない。

## Upstream Documents

- `docs/specs/issue-243-windows-multiwindow-smoke.md`
- `docs/designs/issue-243-windows-multiwindow-smoke.md`
- `docs/specs/issue-243-gpui-realwin-windows.md`（先行 slice）
- `docs/specs/issue-313-windows-pointer-smoke.md`（`os_input`
  pointer API・計測ジオメトリの正本）

## Design Inputs

- `docs/designs/issue-243-windows-multiwindow-smoke.md`

## File Map

| ファイル | 変更 |
| --- | --- |
| `crates/ruviform-ui-gpui/examples/real_os_multiwindow_smoke.rs` | 新規 — phase machine fixture（約 1900 行） |
| `crates/ruviform-ui-gpui/Cargo.toml` | `[[example]]` 宣言追加 |
| `mise.toml` | `example-ui-gpui-real-os-multiwindow-smoke` task 追加 |
| `evidence/issue-243-multiwindow-verify/verdict-multiwindow-windows.json` | 新規 — 実機 verdict |
| `evidence/rf313-os-capability-matrix/report.md` | multiwindow 結果行追記 |
| `evidence/rf313-os-capability-matrix/results.jsonl` | 同上（機械可読行） |
| `docs/{specs,designs,plans,tasks,test-specs,impl/ir}/issue-243-windows-multiwindow-smoke.md` | docs チェーン |

## Dependency Graph

fixture は `os_input`（#313, merged）と `GpuiDriver`/
`WindowDriver`/`DockWorkspace`/`UiRuntime`（#386/#181/#182,
merged）のみに依存。新規 crate 依存なし。

## Risk Register

| リスク | 対応 |
| --- | --- |
| 実 OS 入力が環境要因で届かない（kernel driver による SendInput drop・ユーザー物理入力との競合） | cursor read-back を独立 oracle 化し、retry 上限後は fail + 診断情報（`fg_log`/`failures`）を JSON に残す。oracle を弱めない |
| phase machine の遷移 bug で window を残したまま終了 | teardown を verdict 書き込み後・失敗時にも実行。charlie を最後に close する順序を不変条件化 |
| Dock duplicate-view 等の API 誤用 | WaitReady で attach 済み instance を操作。headless contract test で意味を事前確認 |
| clippy `too_many_lines`/厳格 lint | phase helper 分割で適合（実績あり） |

## Rollback Strategy

example + 証拠 + docs のみの追加で production コード不変の
ため、revert は当該ファイル群の削除で完結する。

## Verification Matrix

| 層 | 検証 |
| --- | --- |
| fmt/lint/type | `mise run fmt-check` `check` `lint-ui-gpui` |
| headless test | `mise run test-ui-gpui` `test-rt` `test` |
| real OS | `mise run example-ui-gpui-real-os-multiwindow-smoke` → verdict JSON（exit 0=pass） |
| docs | doc-maintenance plan→apply→doc-status 検証、index 再生成 |
