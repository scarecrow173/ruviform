---
id: "TSPEC-Qm7Lw3Rt9Xc2Hb5Vd8Ns4Ke"
type: "test-spec"
status: "proposed"
title: "Issue #446 — 空 frame 範囲の Tile 検証契約"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/446"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/test-specs/issue-443-tile-column-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-443-tile-column-contract.md"
---

# Issue #446 — 検証契約

## Feature: 空 frame 範囲の Tile 生成

### Rule: frame 範囲が空なら、bin 範囲が非空でも panic せず全セル NaN の空 Tile を返す

- Example: 入力 frame 数を超える範囲（`5..9`、実効 `2..2`）と、空の範囲（`0..0`）のいずれも、`Spectrogram::tile` は `Ok` を返す。`Tile::frames` は空、`values` は `rows * cols` 個の NaN で、`Aggregate::Max` / `Aggregate::Mean` の両方で成立する。
  - 検査: `tile_empty_frame_range_with_bins_is_all_nan`（`ruviform-viz` の lib テスト）。
  - 再現: 修正前は `spectrogram.rs` の `mag` 参照で `index out of bounds: the len is 0 but the index is 0` の panic になる（同テストが修正前に失敗することを確認済み）。

## Feature: 既存挙動の非退行

### Rule: 非空 frame 範囲の集約・クランプ・列区間は変わらない

- Example: 既存の集約（Max / Mean）・frame / bin のクランプ・列区間規則・gap 列の count が従来どおり。
  - 検査: `tile_aggregates_frames_and_bins`、`tile_clamps_and_marks_empty`、`tile_columns_split_indivisible_span_and_mark_nan_gap`、`empty_tile_columns_have_empty_frame_ranges`（無変更で pass）。
