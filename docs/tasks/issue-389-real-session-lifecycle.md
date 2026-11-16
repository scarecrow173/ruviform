---
id: "TASK-rf389-real-session-lifecycle-01"
type: "task"
status: "in-progress"
title: "Issue #389 real session lifecycle remainder 実装タスク"
created: "2026-11-17"
updated: "2026-11-17"
owners: []
relations:
  source:
    - "docs/plans/issue-389-real-session-lifecycle.md"
    - "docs/specs/issue-389-real-session-lifecycle.md"
    - "docs/designs/issue-389-real-session-lifecycle.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-389-real-session-lifecycle.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-389-real-session-lifecycle.md"
    - "docs/designs/issue-389-real-session-lifecycle.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-389-real-session-lifecycle.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-389-real-session-lifecycle.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-389-real-session-lifecycle.md"
  verified-by:
    - "docs/test-specs/issue-389-real-session-lifecycle.md"
  references:
    - "evidence/rf389/"
  defers: []
  deferred-by: []
---

# Issue #389 real session lifecycle remainder 実装タスク

## Work

`crates/ruviform-plugin-dock-test/` に session lifecycle 残件
6 case（`real-session-degraded`/`real-session-recovery`/
`real-session-timeout`/`real-editor-visibility`/
`real-session-crash`/`real-session-hang`）を追加する。
`real_catalog.rs`/`real_dock.rs` を 21 行化し、
`attempt_real_attach` の open 駆動を `drive_open` seam へ
一般化して各 case の session op を駆動し、console test・
README を追従させ、検証 lane を実走して `evidence/rf389/` に
記録する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-389-real-session-lifecycle.md`
- `docs/specs/issue-389-real-session-lifecycle.md`
- `docs/designs/issue-389-real-session-lifecycle.md`
- `docs/test-specs/issue-389-real-session-lifecycle.md`

## Dependencies

- `docs/specs/issue-389-editor-session.md`（catalog 構造の先例）
- `crates/ruviform-plugin-host` session API（read-only 参照）
- `crates/ruviform-plugin-test-vst3` `test_editor` fixture
  （`mise run build ruviform-plugin-test-vst3` で cdylib 生成）

## Affected Files

- `crates/ruviform-plugin-dock-test/src/real_catalog.rs`
- `crates/ruviform-plugin-dock-test/src/real_dock.rs`
- `crates/ruviform-plugin-dock-test/tests/console.rs`
- `crates/ruviform-plugin-dock-test/README.md`
- `docs/{specs,designs,plans,tasks,test-specs,impl/ir,impl/exp,
  discovery}/issue-389-real-session-lifecycle.*`
- `docs/designs/issue-389-plugin-editor-session.md`
- `evidence/rf389/`

## Done When

- [x] `REAL_CATALOG`/`real_entries()` が 21 行で pairwise 一致し、
  `real_catalog_matches_entries`/`blocked_reports_fail_closed`
  が green。
- [x] `real-session-degraded` が `open_or_degrade(Floating→
  Docked)` の `OpenOutcome::Degraded` + `[Degraded{Docked},
  Closed]` + child HWND 契約で `Pass` 判定する。
- [x] `real-session-recovery` が refused open → `recover(Docked)`
  → `Recovered{Docked}` + `[Failed{Unsupported},
  Recovered{Docked}, Closed]` で `Pass` 判定する。
- [x] `real-session-timeout` が ZERO deadline + `begin_detach` +
  `tick_all` → `Failed{Timeout}` + `[Attached{Docked},
  Failed{Timeout}, Closed]` で `Pass` 判定する。
- [x] `real-editor-visibility` が `set_visibility` の defined
  refusal → `Attached` 維持・event 不変を観測し `Unsupported`
  verdict を返す。
- [x] `real-session-crash`/`real-session-hang` が in-process
  非生存を detail 明記で `Unsupported` を返す。
- [x] `tests/console.rs` が 29 行・新 case id を含む期待一覧で
  green。
- [x] `README.md` の行数記述と verdict 表が 21 行へ追従。
- [x] `mise run test-ui-plugin-dock-real-headless` が 21 行
  NotRun+environment_blocked・violation 0・exit 0。
- [x] `mise run test-ui-plugin-dock-real` を実走し新 case の実
  verdict を `evidence/rf389/` に記録（不可なら NotRun+
  environment_blocked を証拠化）。
- [x] `docs/impl/ir`・`docs/impl/exp`・exp.jsonl・discovery が
  実績で更新され、review-protocol 1 cycle・blocker 0。

## Verification

- [x] `mise run check/lint/test ruviform-plugin-dock-test`
  （default feature）。
- [x] `mise run check/lint/test ruviform-plugin-dock-test
  --features real-dock`。
- [x] `mise run check/lint ruviform-plugin-dock-test
  --features lab-gui`。
- [x] `mise run build ruviform-plugin-test-vst3`。
- [x] `mise run test ruviform-plugin-host --test editor_session
  --test editor_session_driver`。
- [x] `mise run test-ui-plugin-dock`。
- [x] `mise run test-ui-plugin-dock-console`。
- [x] `mise run test-ui-plugin-dock-real-headless`。
- [x] `mise run test-ui-plugin-dock-real`。
- [x] `mise run test-ui-scenario-contract`。
- [x] `mise run check-ui-driver-boundary`。
- [x] `mise exec -- prek run --all-files`（導入済みの場合）。
