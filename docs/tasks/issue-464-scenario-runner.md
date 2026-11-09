---
id: "TASK-j0UrquC3p37Fqdl53IyKUK"
type: "task"
status: "done"
title: "Issue #464 slice — Scenario Runner・Report 組み立て契約"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/464"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-test/src/scenario/runner.rs"
      - type: "file"
        path: "docs/specs/issue-464-scenario-runner.md"
      - type: "file"
        path: "docs/designs/issue-464-scenario-runner.md"
      - type: "file"
        path: "docs/plans/issue-464-scenario-runner.md"
      - type: "file"
        path: "docs/tasks/issue-464-scenario-runner.md"
      - type: "file"
        path: "docs/test-specs/issue-464-scenario-runner.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/src/scenario.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/lib.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-464-scenario-runner.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-464-scenario-runner.md"
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
    - "docs/test-specs/issue-464-scenario-runner.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #464 slice — Scenario Runner・Report 組み立て契約

## Work

`ruviform-ui-test::scenario` に runner 層を additive に追加する。
`ScenarioSpec`（run/scenario/case ID・required capability・
environment・timeout 予算）・`ScenarioOutcome`（verdict・
expected/observed・targets・receipts・evidence・detail）・
`ScenarioTimeout`（budget/elapsed/timed_out）・`run_scenario`
（capability pre-flight → 本体実行 → verdict 写像 → report
組み立て）・`settle_until`（bounded settle + read-only ready
述語）を実装し、`ScenarioReport` に `timeout`/`cleanup` field を
追加する。adapter・他 crate・`tests/`・`Cargo.toml` は変更しない。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-464-scenario-runner.md`
- `docs/specs/issue-464-scenario-runner.md`
- `docs/designs/issue-464-scenario-runner.md`

## Dependencies

- `SemanticGuiTest`/`ScenarioError`/`ScenarioVerdict`/
  `ScenarioReport`/`ScenarioCapability` が main に存在すること
  （PR #477 + #483 統合済み）。

## Affected Files

- `crates/ruviform-ui-test/src/scenario/runner.rs`（新規）
- `crates/ruviform-ui-test/src/scenario.rs`
- `crates/ruviform-ui-test/src/lib.rs`
- `docs/{specs,designs,plans,tasks,test-specs}/issue-464-scenario-runner.md`

## Done When

- [ ] `ScenarioSpec`/`ScenarioOutcome`/`ScenarioTimeout`/
  `run_scenario`/`settle_until` が存在する。
- [ ] capability 不足で本体を実行せず `Unsupported` report を返す。
- [ ] `ScenarioError` が `Unsupported`→`unsupported`・その他→
  `fail`+detail に写像される。
- [ ] timeout 予算超過の `Pass` が `Fail` に落ちる。
- [ ] `settle_until` が `max_settles` 上限の bounded 待機として
  動作する。
- [ ] `ScenarioReport.timeout`/`cleanup` が記録できる。
- [ ] 公開 API に GPUI/adapter 型が露出しない。

## Verification

- [ ] `mise run fmt-check` / `check` / `lint` / `test`
- [ ] `mise run test-ui-scenario-contract`
- [ ] `mise run check-ui-driver-boundary`
