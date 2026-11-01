---
id: "TASK-034QfdkD1MggWcolouiYsy"
type: "task"
status: "done"
title: "issue-189 T-1 BeatPosition/Ppqn/TickPosition canonical types"
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

# issue-189 T-1 BeatPosition/Ppqn/TickPosition canonical types

## Work

plan Step 1: `crates/ruviform-media-core/src/music.rs` に新正本型を実装する。

- `Ppqn`: `pub const fn new(value: u16) -> Result<Self>` を追加し `1..=u16::MAX`
  を許可（`0` は `Error::InvalidValue("ppqn")`）。`pub const PPQN = Ppqn(960)` は
  legacy resolution 定数として残す。
- `BeatPosition { num: i64, den: u32 }`: `new`（`den==0` は
  `InvalidValue("beat_denominator")`、必ず gcd 正規化）、`from_int`、
  `num`/`den`、`checked_add`/`checked_sub`/`checked_neg`（`TimeOverflow` 明示
  拒否）、`Ord`（`i128` 中間値・全域で全関数）、`Display`、`to_f64`。
- `TickPosition { tick: Tick, ppqn: Ppqn }`: `new`/`tick`/`ppqn`/`to_beat`/
  `from_beat`/`rescale`。`PartialEq` は `(tick,ppqn)` 表現等値。`Ord`/`Add`
  は実装しない（音楽的比較・演算は `to_beat()` 経由）。
- `lib.rs` re-export 追加。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-189-m3-rational-beat-variable-ppqn.md`
- `docs/designs/issue-189-rational-beat-variable-ppqn-tempomap.md`（D1/D2/D3）

## Dependencies

- なし（最初の slice）。T-2 は本タスクの型に依存。

## Affected Files

- `crates/ruviform-media-core/src/music.rs`
- `crates/ruviform-media-core/src/lib.rs`

## Done When

- [x] 3 型が spec R1〜R3・design D1〜D3 どおりにコンパイル・単体テスト合格。

## Verification

- [x] `mise run check`・`mise run test`（型単体テスト含む）が緑。
