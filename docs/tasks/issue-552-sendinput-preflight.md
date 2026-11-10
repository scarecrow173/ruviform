---
id: "TASK-034Veln6e33I4gj6qIv08d"
type: "task"
status: "in-progress"
title: "Issue #552 — SendInput preflight 実装タスク"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/552"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-552-sendinput-preflight.md"
  implemented-by:
    - "docs/impl/ir/issue-552-sendinput-preflight-read-back-environment-blocked.md"
  depends-on:
    - "docs/plans/issue-552-sendinput-preflight.md"
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
    - "docs/test-specs/issue-552-sendinput-preflight.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #552 — SendInput preflight 実装タスク

## Work

`os_input` に preflight API（`PreflightScope`/`PreflightReport`/
`PreflightCheck`/`preflight` + 内部 `Probe` seam）を実装し、
pointer probe（絶対移動→`GetCursorPos` read-back→復帰）と
keyboard probe（`VK_F24` down→`GetAsyncKeyState` read-back→必ず up）
を追加する。mock probe ユニットテストで blocked/effective 分岐を
固定した後、3 つの real-OS example（`real_os_smoke` /
`real_os_pointer_smoke` / `real_os_multiwindow_smoke`）の `main()`
で lane 本体前に preflight を実行し、不通時は verdict
`environment_blocked`・exit code 3・schema v2 で終了させる。
通過時は verdict JSON の `preflight` フィールドに結果を記録する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-552-sendinput-preflight.md`
- `docs/designs/issue-552-sendinput-preflight.md`
- `docs/specs/issue-552-sendinput-preflight.md`
- `docs/test-specs/issue-552-sendinput-preflight.md`

## Dependencies

- なし（単一スライス。T1→T2→T3 は plan の Implementation Sequence
  に従う）。

## Affected Files

- `crates/ruviform-ui-gpui/src/gpui_driver/os_input.rs`
- `crates/ruviform-ui-gpui/examples/real_os_smoke.rs`
- `crates/ruviform-ui-gpui/examples/real_os_pointer_smoke.rs`
- `crates/ruviform-ui-gpui/examples/real_os_multiwindow_smoke.rs`
- `evidence/rf552-sendinput-preflight/`（新規）
- `docs/impl/ir/issue-552-sendinput-preflight.md`（新規）

## Done When

- [ ] `os_input::preflight` が scope 指定で pointer/keyboard の
  実効を read-back 検証し `PreflightReport` を返す。
- [ ] mock probe ユニットテストが blocked/effective・scope 選択・
  key up 必達・`probe_target` 端処理を検証し通る。
- [ ] 3 example が preflight 不通時に `environment_blocked` verdict
  （schema v2・`preflight` 記録）と exit code 3 で終了し、
  通過時は本体 lane が従来どおり走る。
- [ ] 指定の mise 検証タスクが全て通る。
- [ ] 実機 lane を可能な範囲で実行し verdict JSON を evidence に
  保存（不能なら NotRun 記録）。

## Verification

- [ ] `cargo test -p ruviform-ui-gpui --features gpui-test`
- [ ] `mise run fmt-check` / `check` / `lint` / `test` /
  `test-ui-gpui` / `test-ui-golden` / `test-ui-scenario-contract` /
  `test-ui-parity` / `test-ui-component-view` /
  `check-ui-driver-boundary`
- [ ] `mise run example-ui-gpui-real-os-smoke`（および可能なら
  pointer/multiwindow）を対話 session で実行
