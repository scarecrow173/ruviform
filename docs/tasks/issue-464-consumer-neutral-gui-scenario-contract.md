---
id: "TASK-034UcbptcbQ9OYtF7EZwzG"
type: "task"
status: "done"
title: "Issue #464 — Consumer-neutral GUI Scenario Contract"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/464"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-test/src/scenario.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/scenario/fake.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/scenario.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/scenario_contract.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/scenario_contract.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/observe.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-ui/src/view.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/ipc.rs"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-464-consumer-neutral-gui-scenario-contract.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-464-consumer-neutral-gui-scenario-contract.md"
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
    - "docs/test-specs/issue-464-consumer-neutral-gui-scenario-contract.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #464 — Consumer-neutral GUI Scenario Contract

## Work

`ruviform-ui-test` に driver-neutral な Scenario Contract
（`SemanticGuiTest` trait・`TargetQuery`/`ScenarioTarget`・
`ScenarioCapability`・`GuiAction`・`MintedIntent`・`ScenarioError`・
`ScenarioVerdict`/`ScenarioReport`）を追加し、Fake（`ViewHost` map）
adapter と GPUI（`GpuiObserver`）adapter が同一契約で同じ Scenario
を実行できることを示す。`ruviform-ui` には `attach_epoch()`・
`deliver_event(&Event)`・`AttachEpoch` serde derive の最小追加を行う。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-464-consumer-neutral-gui-scenario-contract.md`
- `docs/specs/issue-464-consumer-neutral-gui-scenario-contract.md`
- `docs/designs/issue-464-consumer-neutral-gui-scenario-contract.md`

## Dependencies

- `ruviform-ui` の `ViewHost`/`ControlInput`/`ComponentSnapshot`/
  `AttachEpoch`/`generation`/`WindowId`/`ViewInstanceId`/`ComponentKey`
  が main に存在すること（#390/#429/#436/#442 統合済み）。

## Affected Files

- `crates/ruviform-ui-test/src/scenario.rs`（新規）
- `crates/ruviform-ui-test/src/scenario/fake.rs`（新規）
- `crates/ruviform-ui-test/src/lib.rs`
- `crates/ruviform-ui-test/tests/scenario_contract.rs`（新規）
- `crates/ruviform-ui-gpui/src/gpui_driver/scenario.rs`（新規）
- `crates/ruviform-ui-gpui/src/gpui_driver/observe.rs`
- `crates/ruviform-ui-gpui/src/render.rs`
- `crates/ruviform-ui-gpui/src/lib.rs`
- `crates/ruviform-ui-gpui/Cargo.toml`
- `crates/ruviform-ui-gpui/tests/scenario_contract.rs`（新規）
- `crates/ruviform-ui/src/view.rs`
- `crates/ruviform-ui/src/ipc.rs`
- `mise.toml`
- `docs/development-environment.md`

## Done When

- [ ] `ruviform-ui-test` に Scenario/Case ID・Target・Capability・
  Result/Report・`SemanticGuiTest` trait が存在する。
- [ ] Fake adapter と GPUI adapter が同一契約を実装する。
- [ ] 共通 Scenario 関数が Fake と GPUI で実行できるテストがある。
- [ ] stale key/epoch/generation が明示 `ScenarioError` になる。
- [ ] 共通 API に GPUI 型が露出せず boundary guard が pass。
- [ ] `SemanticAction` 以外の capability が pre-flight で
  `unsupported` 判定される。
- [ ] verdict 語彙が `pass`/`fail`/`not_run`/`unsupported`/`blocked`。

## Verification

- [ ] `mise run fmt-check` / `check` / `lint` / `test`
- [ ] `mise run check-ui-driver-boundary`
- [ ] `mise run test-ui-driver-contract` / `test-ui-parity` /
  `test-ui-consumer-observation` / `test-ui-component-view` /
  `test-ui-gpui` / `check-ui-gpui` / `lint-ui-gpui`
- [ ] `mise run test-ui-test-msrv`
