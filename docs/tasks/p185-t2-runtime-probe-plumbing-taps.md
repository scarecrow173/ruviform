---
id: "TASK-034QidbvzKgQyKTfniAhWG"
type: "task"
status: "done"
title: "T-2 RtRuntime probe plumbing: frame_cursor, published_plan, command drains, tap sites, deck channels"
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
    - "docs/tasks/p185-t1-level-extract-and-binding-centralization.md"
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

# T-2 RtRuntime probe plumbing: frame_cursor, published_plan, command drains, tap sites, deck channels

## Work

plan Step 3: `RtState` に `frame_cursor`（block 先頭位置）と `probes: ProbeDeck` を追加。`GraphRuntime::published_plan`（publish 時 retain の `Arc` + 非 RT accessor）を追加。probe command を 2 系統で drain（gen-scoped `Install`/`Rebind` は `activate_generation` 内で `plan.generation` 照合・stale 計数、lifetime `SetMode`/`Remove` は毎 callback event drain 前に適用）。schedule loop に 4 tap 点（graph input・node input post-materialize・node output post-stamp・graph output）を配線し、`free`/`chunks_out`/`telemetry` を deck に配線。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-185-generic-probe-poc.md`
- `docs/designs/issue-185-generic-probe-typed-runtime-observation-rt-capture.md`
- `docs/specs/issue-185-generic-probe-rt-capture.md`

## Dependencies

- `docs/tasks/p185-t1-level-extract-and-binding-centralization.md`

## Affected Files

- `crates/ruviform-audio/src/typed/runtime.rs`、`typed/mod.rs`（re-export）

## Done When

- [ ] `frame_cursor` が可変 frames で単調増加する
- [ ] probe command 2 系統が設計どおりの drain 位置・照合規則で動く
- [ ] 4 tap 点が `st`/`&arena` の disjoint borrow で `try_borrow`/`stamp==seq` 規則を持つ
- [ ] `published_plan` が publish 成功時の plan を返す

## Verification

- [ ] `mise run check`/`lint` exit 0
- [ ] `cargo test -p ruviform-audio` exit 0（既存挙動不変）
