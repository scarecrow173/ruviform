---
id: "TASK-034Qfs3QuOMXdVC8ZnuKz6"
type: "task"
status: "todo"
title: "T-2 typed RT runtime: GraphProcessor, GraphRuntime/RtRuntime, activation, faults"
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
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-195-m2-typed-graph-audio-runtime.md"
    - "docs/tasks/m2-t1-typed-compiler-buffer-io.md"
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

# T-2 typed RT runtime: GraphProcessor, GraphRuntime/RtRuntime, activation, faults

## Work

plan Step 3: `runtime.rs`（GraphProcessor trait・GraphRuntime/RtRuntime・`ruviform_rt::{slot,channel,retire_channel}` による publish/install-remove/generation・NodeEvent・SharedStatus fault・RtRuntime::drop の retire）を実装する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-195-m2-typed-graph-audio-runtime.md`
- `docs/designs/issue-195-m2-typed-graph-compiler-buffer-pdc-rt-runtime-ownership-activation.md`
- `docs/adr/m2-typed-graph-audio-runtime-placement-stateful-rt-ownership-and-activation-reti.md`

## Dependencies

- `docs/tasks/m2-t1-typed-compiler-buffer-io.md`

## Affected Files

- `crates/ruviform-audio/src/typed/runtime.rs`・`typed/mod.rs`（再輸出）

## Done When

- [ ] `RtRuntime::process` が `&RtContext`・`frames<=max_frames`・zero-frame no-op で動く
- [ ] generation 変更時に install/remove command が drain され activate/deactivate が 1 回ずつ呼ばれる
- [ ] 旧 generation event が遮断され stale counter が増える
- [ ] `Drop for RtRuntime` が processor Box を `Retirer` へ送り RT 側 destructor を避ける

## Verification

- [ ] `mise exec -- cargo check -p ruviform-audio` exit 0
- [ ] `mise run lint` exit 0
