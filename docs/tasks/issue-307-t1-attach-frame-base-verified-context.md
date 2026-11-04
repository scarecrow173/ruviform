---
id: "TASK-QNzC22ZqitcqDhco1hHMKj"
type: "task"
status: "done"
title: "Issue #307 T1: attach 時 frame_base 検証 context（accessors + typed mismatch 拒否）"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/307"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/tests/frame_base.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/attach.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/control.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/node.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/lib.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-307-attach-frame-base-verified-context.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-307-attach-frame-base-verified-context.md"
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
    - "docs/test-specs/issue-307-attach-frame-base-verified-context.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #307 T1: attach 時 frame_base 検証 context

## Work

design 確定 API を実装する（`docs/designs/issue-307-attach-frame-base-verified-context.md`）。
(a) `control.rs`: `PluginNodeControl` に `frame_base: TimeBase` フィールド、
`frame_base()`/`check_frame_base()`/`plan_builder()`/`install_plan` 新戻り値
（`InstallPlanError`）、新規型 `FrameBaseMismatch`・`InstallPlanError`（
`#[non_exhaustive]`、`QueueFull(QueueFull)`/`FrameBaseMismatch(FrameBaseMismatch)`）。
(b) `attach.rs`: `PluginNodeControl::new` へ `cfg.frame_base` 供給（node と単一起源）。
(c) `node.rs`: `PluginNode::frame_base()` を scheduler 委譲で追加。
(d) `lib.rs`: 新型を re-export。
(e) `tests/frame_base.rs` 新設: test-spec Rules/Examples の oracle 実装
（accessor 往復一致、check ok/err、plan_builder 束縛、異値 install 拒否の
非破壊性、device_restart 後不変、verified base plan の絶対 frame 配送）。
(f) 全 `install_plan` 呼出箇所を新戻り値で確認（`.unwrap()` 互換）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-307-attach-frame-base-verified-context.md`
- `docs/specs/issue-307-attach-frame-base-verified-context.md`（AC-1..6）
- `docs/designs/issue-307-attach-frame-base-verified-context.md`（API 形態・境界・棄却案）

## Dependencies

- なし（plugin-bridge 内で完結）

## Affected Files

- `crates/ruviform-audio-plugin-bridge/src/{control,attach,node,lib}.rs`
- `crates/ruviform-audio-plugin-bridge/tests/frame_base.rs`（新規）

## Verification

- `mise exec -- cargo test -p ruviform-audio-plugin-bridge`（反復）
- `mise run fmt-check` / `check` / `lint` / `test` / `test-rt`（最終）
