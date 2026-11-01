---
id: "TASK-034QefuVxwLiU0xGxi496X"
type: "task"
status: "completed"
title: "T-3 PoC ex_001-006 + index/lockfile + full verification evidence"
created: "2026-11-01"
updated: "2026-11-02"
owners: ["scarecrow173"]
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
    - "docs/plans/implement-issue-184-m1-ruviform-graph-crate-typed-ir-validator-poc.md"
  implemented-by:
    - "docs/impl/ir/issue-184-typed-graph-ir-poc.md"
  depends-on:
    - "docs/tasks/t-2-validator-stablehandlemap-graphproject.md"
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
    - "docs/test-specs/issue-184-m1-typed-graph-ir-representation-multi-bus-topology-validation-failure.md"
  references: []
  defers: []
  deferred-by: []
---

# T-3 PoC ex_001-006 + index/lockfile + full verification evidence

## Work

plan Step 6-9: `tests/poc_ex.rs` に ex_001（全属性）/ex_002（Main+Send+Sidechain+fan-in）/ex_003（各 rejection）/ex_004（参照不整合+cycle+capacity+required）/ex_005（revision/mapping）/ex_006（失敗時 published 維持 + Main-only fixture）を実装し、`crates/README.md` 索引更新、`Cargo.lock` 再生成、全検証 lane（check/lint/test/test-rt/cargo tree/rg）を実行して evidence を記録する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-184-m1-ruviform-graph-crate-typed-ir-validator-poc.md`
- `docs/test-specs/issue-184-m1-typed-graph-ir-representation-multi-bus-topology-validation-failure.md`

## Dependencies

- T-2（`docs/tasks/t-2-validator-stablehandlemap-graphproject.md`）

## Affected Files

- `crates/ruviform-graph/tests/poc_ex.rs`
- `crates/README.md`、`Cargo.lock`

## Done When

- [ ] `mise exec -- cargo test -p ruviform-graph --test poc_ex` exit 0（ex_001〜006 全緑）
- [ ] `mise run check`/`lint`/`test`/`test-rt` 実測結果と `cargo tree` 依存確認が記録される
- [ ] `crates/README.md` が graph crate を索引化

## Verification

- [ ] `mise exec -- cargo test -p ruviform-graph --test poc_ex` exit 0
- [ ] `mise exec -- cargo tree --locked -p ruviform-graph --edges normal` → audio-core+media-core のみ
- [ ] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` 実測記録
