---
id: "TASK-034Uey3QmAAfhAZo6ln7H4"
type: "task"
status: "in-progress"
title: "Issue #314 slice 2 — Golden × Scenario 能力契約接続の実装タスク"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/314"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-314-golden-scenario.md"
      - type: "file"
        path: "crates/ruviform-ui-test/src/scenario/visual.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/src/scenario.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/scenario/fake.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/scenario.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/scenario_contract.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/scenario_contract.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/README.md"
      - type: "file"
        path: "crates/ruviform-ui-gpui/README.md"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-314-golden-scenario.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-314-golden-scenario.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-314-golden-scenario.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #314 slice 2 — Golden × Scenario 能力契約接続の実装タスク

## Work

`docs/plans/issue-314-golden-scenario.md` の T1〜T4 を実施する。

- T1: `scenario/visual.rs` 新設（`CaptureOutcome`・`VisualGuiTest`・
  `golden_check` 既定実装・`record_golden_step`・
  `From<GoldenVerdict> for ScenarioVerdict`）+ `scenario.rs` に
  `pub mod visual` と `ScenarioVerdict::lane_status`。
- T2: `FakeGuiTest`・`GpuiGuiTest` に `impl VisualGuiTest`
  （`capture_frame` = pin 再検査 + honest `Unsupported` outcome）。
- T3: 両 crate の `tests/scenario_contract.rs` に golden step の
  contract/parity tests。
- T4: README・`mise.toml` description・`development-environment.md`
  更新、検証、review-protocol。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-314-golden-scenario.md`
- `docs/specs/issue-314-golden-scenario.md`
- `docs/designs/issue-314-golden-scenario.md`

## Dependencies

- slice 1 Golden Utility（merge `2230018e`）・#464 Scenario
  Contract（merge `57e4e8ce`）— 共に main 入り済み。

## Affected Files

- `crates/ruviform-ui-test/src/scenario/visual.rs`（新規）
- `crates/ruviform-ui-test/src/scenario.rs`
- `crates/ruviform-ui-test/src/scenario/fake.rs`
- `crates/ruviform-ui-test/src/lib.rs`
- `crates/ruviform-ui-gpui/src/gpui_driver/scenario.rs`
- `crates/ruviform-ui-test/tests/scenario_contract.rs`
- `crates/ruviform-ui-gpui/tests/scenario_contract.rs`
- `crates/ruviform-ui-test/README.md`・
  `crates/ruviform-ui-gpui/README.md`
- `mise.toml`・`docs/development-environment.md`・`docs/**`

## Done When

- [ ] `VisualGuiTest` が `SemanticGuiTest` を継承し、
  `capture_frame`/`golden_check`/`record_golden_step` が
  driver-neutral な型のみで公開される。
- [ ] Fake/GPUI の `capture_frame` が pin 再検査後に honest な
  `Unsupported` outcome を返し、`golden_check` が `unsupported`
  verdict の `GoldenComparison` + evidence emit を行う。
- [ ] `GoldenVerdict`→`ScenarioVerdict`・`lane_status`
  （`blocked`→`not_run`）の写像が固定される。
- [ ] 両側 contract tests が capability 宣言・preflight・verdict
  語彙・evidence 記録の一致を検証する。
- [ ] `mise run test-ui-scenario-contract`・`test-ui-golden`・
  `check-ui-driver-boundary`・`check`・`lint`・`fmt-check` が通る。

## Verification

- [ ] `mise run test-ui-scenario-contract`
- [ ] `mise run test-ui-golden`
- [ ] `mise run check-ui-driver-boundary`
- [ ] `mise run check` / `mise run lint` / `mise run fmt-check`
