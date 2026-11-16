---
id: "TASK-034YqGYxiVHh080aoR6oju"
type: "task"
status: "in-progress"
title: "Issue #631 T1 — real-row 拡充（AC1 catalog + AC3 非画素 + headless fail-closed）"
created: "2026-11-16"
updated: "2026-11-16"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/631"
    - "docs/plans/issue-631-real-row.md"
  changes:
    added:
      - type: "doc"
        path: "docs/tasks/issue-631-real-row.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-631-real-row.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-631-real-row.md"
    - "docs/specs/issue-631-real-row.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-631-real-row.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-631-real-row.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-631-real-row.md"
  verified-by: []
  references:
    - "evidence/rf631/verification.md"
  defers: []
  deferred-by: []
---

# Issue #631 T1 — real-row 拡充

## タスク

1. `real_catalog.rs` に 7 行追加（AC1: multiwindow-focus /
   os-input / window-dpi / window-ime / window-close、AC3:
   editor-resize / editor-redock）。
2. `real_dock.rs`: `real_entries()` 12 行化、`RealDockContext` に
   `window_b`、2 window mount、`observe_open` に `lease_raw`、
   新規 case 7 本、event 系列 check、test 更新。
3. `plugin-dock-real-scenario.rs` に `--blocked-probe`。
4. `mise.toml` に `test-ui-plugin-dock-real-headless`、コメント整合。
5. `docs/development-environment.md`・crate README 整合、
   `evidence/rf631/plugin-dock-real-headless.jsonl` 記録。
6. 検証: check/lint/test（feature 有無）、probe lane、非回帰 lane、
   boundary guard、prek。

## 完了条件

- spec の Acceptance Criteria 全項目。
- PR 本文に実行検証・未検証（NotRun/Unsupported 理由）を明記。
