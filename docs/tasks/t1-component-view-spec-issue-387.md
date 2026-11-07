---
id: "TASK-034T6RWUx1Zc8Qf6mE4rP9"
type: "task"
status: "proposed"
title: "T1 Component/View Spec schema + ViewHost(ruviform-ui::view)"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "../plans/implement-issue-387-component-renderer.md"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui/src/view.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui/Cargo.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-issue-387-component-renderer.md"
  implemented-by: []
  depends-on:
    - "../tasks/t2-headless-behavior-issue-387.md"
  blocks:
    - "../tasks/t3-semantic-fixtures-issue-387.md"
    - "../tasks/t4-gpui-renderer-issue-387.md"
  supersedes: []
  superseded-by: []
  related:
    - "../test-specs/issue-387-component-renderer.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/issue-387-component-view-specs.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# T1: Component/View Spec + ViewHost

## Work

`ruviform-ui::view` 新設。`ComponentKey`/`ComponentSpec`(Panel/Tabs/
Fader/Seek/Waveform)/`ViewSpec`/`A11ySpec`/`ComponentVisualState`/
`ProjectionOp`/`ViewEvent`/`ViewError`/`ViewHost`。reconcile は key
map diff、generation は `ViewInstanceId` 毎に採番し stale を拒否。
`Waveform` payload は `ruviform-viz::ViewBin`/`WaveformInfo` を
bounded で保持(`MAX_WAVEFORM_BINS`)。

## Acceptance

- `cargo check -p ruviform-ui` pass(中立型のみ)。
- view.rs 内 unit test: spec validate、reconcile op 列、
  stale generation 拒否が pass。
