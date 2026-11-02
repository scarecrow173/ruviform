---
id: "TASK-mO3RtjrUoyUadajAWyCadp"
type: "task"
status: "done"
title: "Issue #196 T-3: ruviform-audio-plugin-bridge crate"
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
    - "docs/tasks/issue-196-t-2-host-media-bus-enumeration.md"
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

# T-3: ruviform-audio-plugin-bridge crate

## Deliverable

新規 crate `crates/ruviform-audio-plugin-bridge`（利用側層）:

- `cap.rs`: `CapabilityLevel`/`AdapterCapability`/`capability_for(Backend)`。
- `bus.rs`: `BusDecl`/`bus_decl_to_port_def`/`validate_bus_decls`/`BusError`。
- `control.rs`: `PluginNodeControl` + `NodeControl`/`NodeReport` + SPSC。
- `node.rs`: `PluginNode`（`GraphProcessor`）— endpoint wrap + scheduler +
  windowed views + silence policies + report push。
- `factory.rs`: `PluginNodeFactory`（New/Reuse/Boundary・構成変更検証）。
- `attach.rs`: `AttachConfig`/`attach()`/`AttachError`。
- `vault.rs`: `PlanVault`（keep-alive + retire + VaultExceeded）。
- `status.rs`: `NodeStatus` atomics。
- `lib.rs` + `Cargo.toml` + `README.md` + workspace `members`/`Cargo.lock`/
  `crates/README.md` index 更新。

## Acceptance

- `cargo test -p ruviform-audio-plugin-bridge` 緑。
- `mise run check`/`lint`/`test` 緑。

## Notes

- no alloc/no unsafe on RT path。`MAX_BUSES=MAX_CHANNELS=16`。
- `process_audio` Rollback 経路維持。
