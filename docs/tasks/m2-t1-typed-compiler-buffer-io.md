---
id: "TASK-034Qfs0cxlLQ9xk1dN3NJM"
type: "task"
status: "done"
title: "T-1 typed buffer arena + io views + compiler (schedule/slots/PDC)"
created: "2026-11-01"
updated: "2026-11-01"
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
    - "docs/plans/implement-issue-195-m2-typed-graph-audio-runtime.md"
  implemented-by:
    - "docs/impl/ir/issue-195-m2-typed-graph-compiler-runtime.md"
  depends-on:
    - "docs/plans/implement-issue-195-m2-typed-graph-audio-runtime.md"
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

# T-1 typed buffer arena + io views + compiler (schedule/slots/PDC)

## Work

plan Step 1-2: `ruviform-audio` に `ruviform-graph` 依存と `typed` module 骨格、`buffer.rs`（AlignedSamples/Slot/DelayLine）、`io.rs`（NodeIo/view）、`compile.rs`（deterministic schedule・slot 割当・fan-in merge/PDC・定義済み失敗・lifetime 監査）を実装する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-195-m2-typed-graph-audio-runtime.md`
- `docs/designs/issue-195-m2-typed-graph-compiler-buffer-pdc-rt-runtime-ownership-activation.md`

## Dependencies

- なし（最初の slice）。T-2 は本タスクの CompiledPlan/NodeIo に依存。

## Affected Files

- `crates/ruviform-audio/Cargo.toml`、`src/lib.rs`
- `crates/ruviform-audio/src/typed/{mod,buffer,io,compile}.rs`

## Done When

- [ ] `mise exec -- cargo check -p ruviform-audio` で型が compile する
- [ ] `Compiler::compile` が deterministic schedule・bounded slot・PDC・定義済み失敗（UnsupportedEdgeDelay/BufferAlias/SlotOverflow/DelayOverflow 等）を返す
- [ ] slot data が alignment≥64B（over-allocate+offset・unsafe なし）で確保される

## Verification

- [ ] `mise exec -- cargo check -p ruviform-audio` exit 0
- [ ] `mise run lint` exit 0（新コードが -D warnings を通る）
