---
id: "TASK-034QidaAEbNmHe5Q6M5sRI"
type: "task"
status: "todo"
title: "T-1 ChannelLevel::measure extraction + deps + compile binding centralization"
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
    - "docs/plans/implement-issue-185-generic-probe-poc.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-185-generic-probe-poc.md"
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

# T-1 ChannelLevel::measure extraction + deps + compile binding centralization

## Work

plan Step 1-2: `ruviform-audio-analysis::level` に channel 単位 `ChannelLevel::measure(&[f32])` を抽出し `LevelMeter::process` から呼ぶ（挙動不変）。`ruviform-audio/Cargo.toml` に `ruviform-audio-analysis`/`ruviform-observability` 依存を追加。`compile.rs` で `ScheduledNode::outputs` を `Vec<(PortId, usize)>` 化、`InputBinding::effective_slot`（`scratch ?? 単一 edge`）を compile 確定、`ScheduledNode::{latency_accum, own_latency}` を `apply_pdc` で正本化。`io.rs::input`・`materialize`・`collect_boundaries`・既存テストを新規則に合わせる。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-185-generic-probe-poc.md`
- `docs/designs/issue-185-generic-probe-typed-runtime-observation-rt-capture.md`
- `docs/specs/issue-185-generic-probe-rt-capture.md`

## Dependencies

- なし（plan のみ）

## Affected Files

- `crates/ruviform-audio-analysis/src/level.rs`、`crates/ruviform-audio/Cargo.toml`、`crates/ruviform-audio/src/typed/{compile,io,runtime}.rs`、既存 typed テストの outputs/binding 参照

## Done When

- [ ] `ChannelLevel::measure` が追加され `LevelMeter` の挙動テストが不変で緑
- [ ] `cargo check` で `outputs`/`effective_slot`/latency フィールドが型付きに通る
- [ ] 既存 typed テストが新 binding 規則で緑

## Verification

- [ ] `mise run check` exit 0
- [ ] `cargo test -p ruviform-audio-analysis` exit 0
- [ ] `cargo test -p ruviform-audio` exit 0
