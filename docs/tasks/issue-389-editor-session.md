---
id: "TASK-034YtASlLiwI44BLXyY1zl"
type: "task"
status: "in-progress"
title: "Issue #389 editor-session real-session-terminate 実装タスク"
created: "2026-11-16"
updated: "2026-11-16"
owners: []
relations:
  source:
    - "docs/plans/issue-389-editor-session.md"
    - "docs/specs/issue-389-editor-session.md"
    - "docs/designs/issue-389-editor-session.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-389-editor-session.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-389-editor-session.md"
    - "docs/designs/issue-389-editor-session.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-389-editor-session.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-389-editor-session.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-389-editor-session.md"
  verified-by:
    - "docs/test-specs/issue-389-editor-session.md"
  references:
    - "evidence/rf389/"
  defers: []
  deferred-by: []
---

# Issue #389 editor-session real-session-terminate 実装タスク

## Work

`crates/ruviform-plugin-dock-test/` に `real-session-terminate`
case を追加する。`real_catalog.rs` の `REAL_CATALOG` と
`real_dock.rs` の `real_entries()` を 13 行化し、`observe_open`
seam で attach 中の `HostCommand::Destroy` →
`notify_terminated(HostLost)` → `Terminated` exactly-once →
child HWND teardown → host 存続を観測する case 本体を実装し、
console test・crate doc・README を追従させ、検証 lane を実走して
`evidence/rf389/` に記録する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-389-editor-session.md`
- `docs/specs/issue-389-editor-session.md`
- `docs/designs/issue-389-editor-session.md`
- `docs/test-specs/issue-389-editor-session.md`

## Dependencies

- `docs/specs/issue-631-real-row.md`（catalog 構造の先例）
- `crates/ruviform-plugin-host` session API（read-only 参照）
- `crates/ruviform-plugin-test-vst3` `test_editor` fixture
  （`mise run build ruviform-plugin-test-vst3` で cdylib 生成）

## Affected Files

- `crates/ruviform-plugin-dock-test/src/real_catalog.rs`
- `crates/ruviform-plugin-dock-test/src/real_dock.rs`
- `crates/ruviform-plugin-dock-test/tests/console.rs`
- `crates/ruviform-plugin-dock-test/src/lib.rs`
- `crates/ruviform-plugin-dock-test/src/bin/plugin-dock-console.rs`
- `crates/ruviform-plugin-dock-test/src/bin/plugin-dock-real-scenario.rs`
- `crates/ruviform-plugin-dock-test/src/bin/plugin-dock-lab.rs`
- `crates/ruviform-plugin-dock-test/README.md`
- `docs/{specs,designs,plans,tasks,test-specs,impl/ir,impl/exp,
  discovery}/issue-389-editor-session.*`
- `evidence/rf389/`

## Done When

- [ ] `REAL_CATALOG`/`real_entries()` が 13 行で pairwise 一致し、
  `real_catalog_matches_entries`/`blocked_reports_fail_closed`
  が green。
- [ ] `real-session-terminate` case 本体が observe_open seam で
  全 step（child HWND 記録→Deactivate+Destroy→
  notify_terminated(HostLost)→Closed+concluded→events 列
  `[Attached{Docked}, Terminated{HostLost}]`→再 notify no-op→
  IsWindow(child)==0→Scan 成功+破棄 handle InvalidHandle）を
  実装し、refused open→`Unsupported`・他失敗→`Fail`・全成立→
  `Pass` の verdict 写像を共有 helper 経由で持つ。
- [ ] `tests/console.rs` が 21 行・新 case id を含む期待一覧で
  green。
- [ ] crate doc・README の "twelve"/12 行記述と verdict 表が
  13 行へ追従。
- [ ] `mise run test-ui-plugin-dock-real-headless` が 13 行
  NotRun+environment_blocked・violation 0・exit 0。
- [ ] `mise run test-ui-plugin-dock-real` を実走し新 case の実
  verdict を `evidence/rf389/` に記録（不可なら NotRun+
  environment_blocked を証拠化）。
- [ ] `docs/impl/ir`・`docs/impl/exp`・exp.jsonl・discovery が
  実績で更新され、review-protocol 1 cycle・blocker 0。

## Verification

- [ ] `mise run check/lint/test ruviform-plugin-dock-test`
  （default feature）。
- [ ] `mise run check/lint/test ruviform-plugin-dock-test
  --features real-dock`。
- [ ] `mise run check/lint ruviform-plugin-dock-test
  --features lab-gui`。
- [ ] `mise run build ruviform-plugin-test-vst3`。
- [ ] `mise run test ruviform-plugin-host --test editor_session
  --test editor_session_driver`。
- [ ] `mise run test-ui-plugin-dock`。
- [ ] `mise run test-ui-plugin-dock-real-headless`。
- [ ] `mise run test-ui-plugin-dock-real`。
- [ ] `mise run test-ui-plugin-dock-console`。
- [ ] `mise run check-ui-driver-boundary`。
- [ ] `mise exec -- prek run --all-files`（導入済みの場合）。
