---
id: "PLAN-VM42GIDhiqjgeTP4U1EbJM"
type: "plan"
status: "completed"
title: "Issue #464 slice — Scenario Runner・Report 組み立て契約の実装計画"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/464"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-464-scenario-runner.md"
      - type: "file"
        path: "docs/tasks/issue-464-scenario-runner.md"
      - type: "file"
        path: "docs/test-specs/issue-464-scenario-runner.md"
    modified:
      - type: "file"
        path: "docs/specs/issue-464-scenario-runner.md"
      - type: "file"
        path: "docs/designs/issue-464-scenario-runner.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/designs/issue-464-scenario-runner.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-464-scenario-runner.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-464-consumer-neutral-gui-scenario-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-464-scenario-runner.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-464-scenario-runner.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #464 slice — Scenario Runner・Report 組み立て契約の実装計画

## Goal

`ruviform-ui-test` に scenario 実行の宣言（`ScenarioSpec`）・結果
（`ScenarioOutcome`）・組み立て関数（`run_scenario`）・bounded
待機（`settle_until`）を additive に追加し、`ScenarioReport` が
Issue #464 詳細 6 の timeout/cleanup 記録を保持できるようにする。

## Scope

`crates/ruviform-ui-test/src/` 内で完結。adapter・他 crate・
`tests/`・`Cargo.toml` には触れない（依存追加が不要な限り
`Cargo.toml` 変更なし）。

## Upstream Documents

- `docs/specs/issue-464-scenario-runner.md`
- `docs/designs/issue-464-scenario-runner.md`
- `docs/specs/issue-464-consumer-neutral-gui-scenario-contract.md`
- `docs/designs/issue-464-consumer-neutral-gui-scenario-contract.md`

## Design Inputs

- `docs/designs/issue-464-scenario-runner.md`

## File Map

- `crates/ruviform-ui-test/src/scenario/runner.rs` - 新規。
  `ScenarioSpec`/`ScenarioOutcome`/`ScenarioTimeout`/
  `run_scenario`/`settle_until` + unit test。
- `crates/ruviform-ui-test/src/scenario.rs` - `pub mod runner;`、
  `ScenarioReport` に `timeout`/`cleanup` field 追加。
- `crates/ruviform-ui-test/src/lib.rs` - runner 型の re-export。
- `docs/{specs,designs,plans,tasks,test-specs}/issue-464-scenario-runner.md`
  - docs chain。

## Implementation Sequence

- [x] T1: `scenario.rs` に `pub mod runner;` と `ScenarioReport`
  の `timeout`/`cleanup` field を追加。
- [x] T2: `scenario/runner.rs` に `ScenarioSpec`/`ScenarioOutcome`/
  `ScenarioTimeout`/`run_scenario`/`settle_until` を実装。
- [x] T3: `runner.rs` 内 unit test（pre-flight・verdict 写像・
  timeout 矯正・settle_until）。
- [x] T4: `lib.rs` re-export・fmt/lint/test 検証・docs chain 確定。

## Dependencies and Risks

- `run_scenario` は既存 `SemanticGuiTest`/`ScenarioError`/
  `ScenarioVerdict`/`ScenarioReport` のみに依存し、adapter 変更を
  必要としない（additive）。
- `ScenarioReport` への field 追加は pub field 構造体の変更だが、
  repo 内の構築は全て `ScenarioReport::new` 経由（grep 確認済み）。
- `settle_until` の `ready` 述語は `&G` 経由の read-only 観測のみ
  — 状態を書き換える待機条件は契約上書けない。

## Task Breakdown

- T1: report field + module 公開。
- T2: runner 型・関数実装。
- T3: unit test。
- T4: export・検証・docs。

## Verification

- [ ] `runner.rs` unit test が pass。
- [ ] `mise run fmt-check` / `check` / `lint` / `test`。
- [ ] `mise run test-ui-scenario-contract`。
- [ ] `mise run check-ui-driver-boundary`。

## Open Questions / Gaps

- なし（「source revision」は design の解釈節で確定 — domain
  `Revision` の観測経路は既存で十分）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution
- Independent work streams: none
- User approval required before dispatch: no（review-protocol で代替）

## Review Handoff

- `run_scenario` が executor 化していないこと（body は `FnOnce`、
  scenario は普通の Rust 関数のまま）を reviewer が確認する。
- timeout 矯正が Pass→Fail のみに限定され、他 verdict を
  書き換えないことを確認する。
