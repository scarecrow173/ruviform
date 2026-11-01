---
id: "TASK-034QfdpefNAIoQr57VQZlb"
type: "task"
status: "done"
title: "issue-189 T-2 TempoMap Beat basis + event/position/ABI migration"
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
    - "docs/plans/implement-issue-189-m3-rational-beat-variable-ppqn.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-189-m3-rational-beat-variable-ppqn.md"
    - "docs/tasks/issue-189-t-1-beat-canonical-types.md"
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

# issue-189 T-2 TempoMap Beat basis + event/position/ABI migration

## Work

plan Step 2-4: `TempoMap` を Beat 基準へ移行する。

- `TempoEvent`/`TimeSignatureEvent`: `tick: Tick` → `position: BeatPosition`
  （明示的破壊移行）。`at_tick(TickPosition, ...)` 補助コンストラクタ。
- `TempoSegment.start` → `BeatPosition`、`SignatureSpan` も Beat 基準化し
  `bar_len` を正規化有理数 beats にする。`build()` の `960`/`3840` リテラル除去。
- 公開 API: `beat_to_media`（`NearestEven` 単回）/`media_to_beat`（正確・
  分母 `u32` 超過は `TimeOverflow`）/`tick_to_media`/`media_to_tick`（絶対
  拍への `Floor`）/`musical_position`/`time_info(BeatPosition, Ppqn)`。
- `MusicalPosition.tick_offset: u16` → `beat_offset: BeatPosition`。
- `lib.rs` re-export と crate doc 更新。既存テストを新 API へ移植。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-189-m3-rational-beat-variable-ppqn.md`
- `docs/designs/issue-189-rational-beat-variable-ppqn-tempomap.md`（D4/D5/D6/D8）

## Dependencies

- T-1（`BeatPosition`/`TickPosition`/`Ppqn::new`）。

## Affected Files

- `crates/ruviform-media-core/src/music.rs`
- `crates/ruviform-media-core/src/lib.rs`

## Done When

- [x] `960`/`3840` リテラルが `music.rs` 非テストコードから除去され、
  TempoMap が Beat 基準で既存全テストを通過する。

## Verification

- [x] `mise run check`・`mise run lint`・`mise run test` が緑。
