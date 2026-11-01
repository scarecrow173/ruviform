---
id: "TASK-034QeftbDcuNOqRCo3KycI"
type: "task"
status: "completed"
title: "T-1 ruviform-graph crate skeleton + ids/port/graph types + builder"
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
    - "docs/plans/implement-issue-184-m1-ruviform-graph-crate-typed-ir-validator-poc.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
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

# T-1 ruviform-graph crate skeleton + ids/port/graph types + builder

## Work

plan Step 1-3: root `Cargo.toml` members 追記、`crates/ruviform-graph/{Cargo.toml,README.md}` 骨格、`src/lib.rs` crate doc、`src/ids.rs`（stable ID + revision + runtime index 型）、`src/port.rs`（orthogonal port 属性 + `PortFormat::domain()` + `Multiplicity` + `ChannelMap`）、`src/graph.rs`（`NodeDef`/`PortDef`/`ConnDef` → `GraphBuilder` 採番 → private `GraphSpec` + `BuildError`）を実装する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-184-m1-ruviform-graph-crate-typed-ir-validator-poc.md`
- `docs/designs/issue-184-typed-graph-ir-ports-buses-connections-validation-revisions-m1.md`

## Dependencies

- なし（最初の slice）。T-2/T-3 は本タスクの型に依存。

## Affected Files

- `Cargo.toml`（members）
- `crates/ruviform-graph/Cargo.toml`、`README.md`
- `crates/ruviform-graph/src/{lib,ids,port,graph}.rs`

## Done When

- [ ] `mise exec -- cargo check -p ruviform-graph` で型が compile する
- [ ] builder が Node/Port/Connection ID を u64=1 起点で決定的に採番し、完了 `GraphSpec` の fields が private である

## Verification

- [ ] `mise exec -- cargo check -p ruviform-graph` exit 0
