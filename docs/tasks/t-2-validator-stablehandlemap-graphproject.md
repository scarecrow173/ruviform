---
id: "TASK-034Qefu64sxmR4DaHbCFGf"
type: "task"
status: "completed"
title: "T-2 Validator + StableHandleMap + GraphProject"
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
  implemented-by: []
  depends-on:
    - "docs/tasks/t-1-ruviform-graph-crate-skeleton-ids-port-graph-types-builder.md"
  blocks:
    - "docs/tasks/t-3-poc-ex-001-006-index-lockfile-full-verification-evidence.md"
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

# T-2 Validator + StableHandleMap + GraphProject

## Work

plan Step 4-5: `src/validate.rs`（`ValidationError`/`ValidationErrors`/`Validator`/`ValidatedGraph`/`LatencyStatus`、規則 1-7: 方向・domain・format 階層・layout+ChannelMap・capacity・required・全グラフ SCC cycle 判定）と `src/mapping.rs`（Kahn+BTreeMap tie-break 双方向 `StableHandleMap`）、`src/project.rs`（`GraphProject::prepare` / `PrepareError` / `PublishedGraph`、requested/published revision 区別・失敗時旧 published 維持）を実装する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-184-m1-ruviform-graph-crate-typed-ir-validator-poc.md`
- `docs/designs/issue-184-typed-graph-ir-ports-buses-connections-validation-revisions-m1.md`

## Dependencies

- T-1（`docs/tasks/t-1-ruviform-graph-crate-skeleton-ids-port-graph-types-builder.md`）

## Affected Files

- `crates/ruviform-graph/src/{validate,mapping,project}.rs`

## Done When

- [ ] 規則 1-7 全ての rejection variant を `ValidationError` が持ち `ValidationErrors` が全件収集する
- [ ] 全グラフ SCC 検査で zero-only cycle→`ZeroDelayCycle`、declared 含む cycle→`DelayedCycleUnsupported`
- [ ] `StableHandleMap` が Kahn+BTreeMap tie-break で deterministic な runtime index を双方向に引ける
- [ ] `GraphProject::prepare` が requested/published revision を区別し失敗時旧 published を維持

## Verification

- [ ] `mise exec -- cargo check -p ruviform-graph` exit 0
- [ ] `rg "ruviform_rt|RtContext|unsafe" crates/ruviform-graph/src` → 0 match
