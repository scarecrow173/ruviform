---
id: "TASK-Awm44usEpigGUWYXNxfbWd"
type: "task"
status: "done"
title: "Issue #196 T-1: plugin-core bus role + enumeration contract"
created: "2026-11-02"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/196"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-196-typed-graph-plugin-adapter.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-196-typed-graph-plugin-adapter.md"
    - "docs/designs/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-196-typed-graph-plugin-adapter.md"
    - "docs/designs/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# T-1: plugin-core bus role + enumeration contract

## Deliverable

- `crates/ruviform-plugin-core/src/bus.rs`: `BusRole`（`#[non_exhaustive]`
  Main/Aux/Sidechain/Send/Return）追加、`BusDescriptor.main: bool` →
  `role: BusRole` 置換、`is_main()` helper。
- `src/plugin.rs`: `Plugin::buses(&self) -> &[BusDescriptor]`（default
  `&[]` = 未宣言）、`ErasedPlugin::buses`、`Erased<P>` forward、`lib.rs`
  re-export。

## Acceptance

- `cargo test -p ruviform-plugin-core` 緑。`BusDescriptor` doctest 更新済み。
- `mise run check`/`lint` 緑。

## Notes

- `role` 置換は公開 API 破壊（0.1.0）。doc 例を `role: BusRole::Main` に更新。
