---
id: "TASK-034XEbSIxHaIKblAZevx7R"
type: "task"
status: "done"
title: "Issue #631 T1 — Plugin-dock scenario skeleton（ruviform-plugin-dock-test）"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/631"
    - "docs/specs/issue-631-plugin-dock-scenario.md"
  changes:
    added:
      - type: "doc"
        path: "docs/tasks/issue-631-plugin-dock-scenario.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-631-plugin-dock-scenario.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-631-plugin-dock-scenario.md"
    - "docs/specs/issue-631-plugin-dock-scenario.md"
    - "docs/designs/issue-631-plugin-dock-scenario.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-631-plugin-dock-scenario.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-631-plugin-dock-scenario.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-631-plugin-dock-scenario.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 T1 — Plugin-dock scenario skeleton（ruviform-plugin-dock-test）

## Work

`docs/plans/issue-631-plugin-dock-scenario.md` の File Map / Implementation Sequence を実施する: `EnvironmentKind` 拡張、ui-test scenario spec 追加、新規 test-only crate、mise task、lane 実走と evidence。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-631-plugin-dock-scenario.md`
- `docs/specs/issue-631-plugin-dock-scenario.md`
- `docs/designs/issue-631-plugin-dock-scenario.md`
- `docs/test-specs/issue-631-plugin-dock-scenario.md`

## Dependencies

- `ruviform-plugin-test-vst3` の cdylib（`mise run build ruviform-plugin-test-vst3` で生成）。

## Affected Files

- `crates/ruviform-plugin-dock-test/`（新規）
- `crates/ruviform-audio-test/src/{metadata.rs,lib.rs}`
- `crates/ruviform-ui-test/src/{lib.rs,scenario.rs,scenario/plugin_dock.rs}`
- `Cargo.toml` / `Cargo.lock` / `mise.toml`（additive）
- `evidence/rf631/`、`docs/*/README.md`（再生成）

## Done When

- [ ] `mise run test-ui-plugin-dock` が実走し、`evidence/rf631/` に JSONL 証拠が保存される
- [ ] load/discover/hash = `pass`、editor attach/dock/pixel = `unsupported`/`not_run` が report に記録される
- [ ] spec の Acceptance Criteria を全て満たす

## Verification

- [ ] `mise run fmt-check` / `prek run --all-files`
- [ ] `mise run check|lint|test`（plugin-dock-test / audio-test / ui-test / plugin-test-vst3）
- [ ] scenario contract・native-slot・golden・driver-contract・ui-gpui・driver-boundary 各 lane 非回帰
