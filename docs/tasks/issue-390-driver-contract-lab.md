---
id: "TASK-034TaJ4jQd92g4mWa81cxX"
type: "task"
status: "todo"
title: "Issue #390 Driver Contract/Lab 実装"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-390-driver-contract-lab.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-390-driver-contract-lab.md"
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
    - "docs/test-specs/issue-390-driver-boundary-lab.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #390 Driver Contract/Lab 実装

## Work

Driver Boundary Guard（ruleset + dep-graph + signature 検査 +
CLI）、Driver Lab registry（cases/stories manifest + 検証 +
evidence）、UI-DVR-01〜08 registry + UI-DVR-01/05/07/08 実行
fixture、正負 mini-workspace fixtures、canonical mise task の実装。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/issue-390-driver-contract-lab.md
- docs/designs/issue-390-driver-contract-lab.md
- docs/specs/issue-390-driver-contract-lab.md

## Dependencies

- 公開契約未着（#385/#386/#387/#388）は blocked binding で記録 —
  この task 自体はそれらに依存しない（stub 代替なし）。

## Affected Files

- crates/ruviform-ui-test/{Cargo.toml,src/lib.rs,
  src/driver_boundary.rs,src/driver_boundary/sig.rs,
  src/driver_lab.rs,src/bin/gui_boundary_guard.rs,
  tests/driver_boundary.rs,tests/driver_contract.rs,
  fixtures/driver-lab/**,README.md}
- mise.toml（check-ui-driver-boundary / test-ui-driver-contract）
- docs/development-environment.md（task 表 2 行）
- docs/{specs,designs,plans,test-specs,tasks}/issue-390-*
- docs/discovery/issue-390-driver-contract-lab-inventory.md

## Done When

- [x] `mise run check-ui-driver-boundary` が live workspace で
  exit 0 / verdict pass。
- [x] `mise run test-ui-driver-contract` が manifest 整合 + 実行
  fixture 5 件 pass。
- [x] `cargo test -p ruviform-ui-test` 全件 pass・
  `cargo clippy -p ruviform-ui-test --all-targets -- -D warnings`
  クリア。
- [x] 7 negative + 1 positive fixture が期待 verdict で pin。
- [x] 判定語彙・layer・requiredCases が manifest で宣言・検証済み。

## Verification

- [x] `mise run check-ui-driver-boundary` → verdict: pass。
- [x] `mise run test-ui-driver-contract` → 5 tests ok。
- [x] `cargo test -p ruviform-ui-test` → 62 tests ok。
- [x] `cargo clippy -p ruviform-ui-test --all-targets -- -D warnings`
  → clean。
- [x] `RUVIFORM_DRIVER_LAB_EVIDENCE=target/driver-lab/evidence.jsonl`
  で 4 実行 case の evidence emission 確認。
