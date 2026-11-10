---
id: "TASK-034Vgd9bXcARNIc2rWG7tW"
type: "task"
status: "done"
title: "Issue #314 slice 4 follow-up — real-scenario lane + matrix tasks"
created: "2026-11-10"
updated: "2026-11-11"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-314-real-scenario-lane.md"
  implements:
    - "docs/plans/issue-314-real-scenario-lane.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-314-real-scenario-lane.md"
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
    - "docs/test-specs/issue-314-real-scenario-lane.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #314 slice 4 follow-up — real-scenario lane + matrix tasks

## Work

`real-scenario` canonical mise lane の追加と、real-os story matrix
残 cell（scale 2.0・各 story 第 2 theme）の実走・証拠記録。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-314-real-scenario-lane.md`
- `docs/specs/issue-314-real-scenario-lane.md`
- `docs/designs/issue-314-real-scenario-lane.md`

## Dependencies

- PR #554（`RealOsGuiTest`・`real-scenario` feature・
  `tests/real_window.rs`）merge 済みであること

## Affected Files

- `mise.toml` — `[tasks.test-ui-real-scenario]` 追加のみ
- `crates/ruviform-ui-gpui/tests/real_window.rs` — matrix 化・
  env-blocked 区分・PNG dir 解決
- `crates/ruviform-ui-test/fixtures/driver-lab/stories.json` —
  `drivers.real-os` 実績更新
- `evidence/issue-314-real-scenario/` — lane 実行証拠
- `docs/{specs,designs,plans,tasks,test-specs,impl/ir}/
  issue-314-real-scenario-lane.md` — 文書チェーン + IR

## Done When

- [x] `mise run test-ui-real-scenario` が定義され、対話 desktop
  session で `tests/real_window.rs`（`real-scenario` feature）を
  実行する
- [x] story ごとに 2 theme × 2 scale の 4 cell が実 window で
  walk + pixel capture + golden loop を通る
- [x] env 不通時に `environment_blocked` が GUI Fail と区別して
  evidence JSONL に記録される
- [x] `stories.json` `drivers.real-os` が実行実績と一致する
- [x] `evidence/issue-314-real-scenario/` に JSONL + PNG/meta が
  保存される

## Verification

- [x] `mise run fmt-check` / `check` / `lint` / `test`
- [x] `mise run test-ui-golden` / `test-ui-scenario-contract` /
  `test-ui-parity` / `test-ui-component-view` / `test-ui-gpui` /
  `check-ui-driver-boundary`
- [x] `mise run test-ui-real-scenario`（対話 desktop session）
