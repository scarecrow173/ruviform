---
id: "TASK-034XGt0rhQewWd8KFPsSIY"
type: "task"
status: "done"
title: "Issue #389 dock-test session-lifecycle 実装"
created: "2026-11-14"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "docs/specs/issue-389-session-lifecycle.md"
  changes:
    added:
      - type: "doc"
        path: "docs/tasks/issue-389-session-lifecycle.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-389-session-lifecycle.md"
  implemented-by:
    - "docs/impl/ir/issue-389-session-lifecycle.md"
  depends-on:
    - "docs/plans/issue-389-session-lifecycle.md"
    - "docs/specs/issue-389-session-lifecycle.md"
    - "docs/designs/issue-389-session-lifecycle.md"
    - "docs/test-specs/issue-389-session-lifecycle.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-631-plugin-dock-scenario.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-389-session-lifecycle.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #389 dock-test session-lifecycle 実装

## Work

`docs/plans/issue-389-session-lifecycle.md` の File Map / Implementation Sequence を実施する: `scenario.rs` に `session-lifecycle`/`host-terminate`/`open-or-degrade` の 3 case と共有 helper を追加し、`tests/plugin_dock.rs`・`README.md` を追従させ、`docs/designs/issue-389-plugin-editor-session.md` の stale verification-status を reconcile する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-389-session-lifecycle.md`
- `docs/specs/issue-389-session-lifecycle.md`
- `docs/designs/issue-389-session-lifecycle.md`
- `docs/test-specs/issue-389-session-lifecycle.md`

## Dependencies

- `ruviform-plugin-test-vst3` の cdylib（`mise run build ruviform-plugin-test-vst3` で生成 — 読み取りと build のみ、ソース変更なし）
- `PluginEditorSessions`/`InProcessEditorOps`/`FakeNativeSlot` の既存 API（plugin-host/plugin-ui-test — 読み取り参照のみ）

## Affected Files

- `crates/ruviform-plugin-dock-test/src/scenario.rs`（3 case + 共有 helper + module doc）
- `crates/ruviform-plugin-dock-test/tests/plugin_dock.rs`（case 数 5→8・verdict 断言）
- `crates/ruviform-plugin-dock-test/README.md`（verdict 表）
- `docs/designs/issue-389-plugin-editor-session.md`（verification-status reconcile）
- `evidence/rf389-dock/`（新規証拠）
- `docs/*/README.md` index（doc-maintenance で再生成）

## Done When

- [ ] `run_all` が 8 case を返し、新 3 case が実バイナリ実走で `Pass`・artifact 不在で `NotRun`
- [ ] 既存 5 case の verdict が非回帰（`editor-attach`=unsupported・`dock-pixels`=not_run 維持）
- [ ] `mise run test-ui-plugin-dock` 実走 JSONL が `evidence/rf389-dock/` に記録される
- [ ] `docs/designs/issue-389-plugin-editor-session.md` の R5/D8/D5/R2/R4/R6/`WIRE_VERSION` が現行事実と一致
- [ ] review-protocol 1 cycle 以上・blocker なし

## Verification

- [ ] `mise run fmt-check` / `mise run check|lint|test ruviform-plugin-dock-test`
- [ ] `mise run build ruviform-plugin-test-vst3` → `mise run test-ui-plugin-dock`
- [ ] `mise run test ruviform-plugin-host --test editor_session --test editor_session_driver`
- [ ] `mise run check-ui-driver-boundary` / `mise exec -- prek run --all-files`
