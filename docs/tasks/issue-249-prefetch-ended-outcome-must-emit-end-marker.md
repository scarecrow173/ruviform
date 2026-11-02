---
id: "TASK-034QAsO7suXcts3TkIr3Un"
type: "task"
status: "done"
title: "Issue 249 prefetch Ended outcome must emit End marker"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/249"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/impl/ir/issue-249-prefetch-detection-ended-strands-end-marker.md"
  depends-on: []
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

# Issue 249 prefetch Ended outcome must emit End marker

## Work

`PrefetchProducer::pump` は `fill_block` の EOF arm が `PumpOutcome::Ended` を返すが、`End` marker は次 pump の step 2 で合成される。`Idle`/`Ended` まで反復する documented manual driver が検出時点 `Ended` で停止すると `End{gen}` が ring に出ず consumer は `eof` を観測できない（#244 review で記録された残件 wart）。`Ended` を「`End` marker がこの call で ring に載った」という outcome semantics に整合させ、EOF 検出と marker emit を同一 step に畳み込む。fault / dead-generation 経路（`Boundary` → `Fault` → `End`）は `pending` 経由の sequencing が必須なため step 2 lazy emission を維持し、共有 `emit_end` で commit-once を一元化する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/impl/ir/issue-244-media-reader-prefetch-contract-residual-warts.md`（Follow-ups に本 wart を P2 残件として記録）
- `docs/specs/issue-244-media-reader-prefetch-contract-warts.md`
- `docs/adr/issue-244-prefetch-contract-warts-boundary-first-fault-emission-dead-generation-.md`

## Dependencies

- #197 / #233 / #244 の prefetch contract（boundary-first emission・dead-generation sequence・`End` commit-once）を維持すること

## Affected Files

- `crates/ruviform-media-reader/src/prefetch.rs`
- `crates/ruviform-media-reader/tests/prefetch.rs`
- `crates/ruviform-media-reader/README.md`

## Done When

- [x] detection `Ended` を返した pump で `End{gen}` が ring（または backpressure 時は `pending` commit）済み — driver が即停止しても terminal marker が strand しない
- [x] `End` は generation あたり正確に 1 回（duplicate なし）
- [x] regression test が red → green を確認

## Verification

- [x] `cargo test -p ruviform-media-reader`（prefetch 25 + 新規 + rt_fill 3 → 27 + 11 + 3 全量 green）
- [x] `mise run fmt-check` / `check` / `lint` / `test` green
- [x] consumer 影響調査（synvelis manual driver・audio-lens）を impl record に記録
