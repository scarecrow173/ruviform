---
id: "TSPEC-Nd4Gq8VzMr2TkLb6XjPw1E"
type: "test-spec"
status: "approved"
title: "Issue #443 — Tile 列区間と count の検証契約"
created: "2026-11-08"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/443"
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
    - "docs/specs/issue-443-tile-column-contract.md"
    - "docs/designs/issue-443-tile-column-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-434-spectrogram-kind.md"
  verifies:
    - "docs/specs/issue-443-tile-column-contract.md"
    - "docs/designs/issue-443-tile-column-contract.md"
---

# Issue #443 — 検証契約

## Feature: Tile の列区間規則

### Rule: 列区間は Tile の frame 範囲を隙間なく覆う

- Example: `Tile::columns` が返す列区間は、先頭が `Tile::frames.start`、末尾が `Tile::frames.end` に一致し、隣接列が隙間なく連続する（span 1..=24、cols 1..=span の全組合せ）。
  - 検査: `columns_partition_frames_for_every_span`（`ruviform-viz` の lib テスト）。
- Example: 割り切れない span（10 frame / 3 列）は `0..3, 3..6, 6..10` に分割される。`Spectrogram::tile` の出力も同じ列区間を持つ。
  - 検査: `tile_columns_split_indivisible_span_and_mark_nan_gap`。
- Example: 空の tile（`frames` が空）は、全列が空の frame 範囲（`start..start`）と `count == 0` を持つ。
  - 検査: `empty_tile_columns_have_empty_frame_ranges`。

### Rule: cols が span を超える場合は各列が最低 1 frame を覆う

- Example: 2 frame / 4 列の列区間は `0..1, 0..1, 1..2, 1..2` で、全体を隙間なく覆う。
  - 検査: `columns_cover_frames_when_cols_exceed_span`。

## Feature: 列の count（有限セル数）

### Rule: count は magnitudes の有限セル数であり、0 は gap 列を表す

- Example: NaN のみの列は `count == 0` として識別できる。
  - 検査: `tile_columns_split_indivisible_span_and_mark_nan_gap`（NaN 列の count 0、magnitudes が全 NaN）。
- Example: 部分的に NaN の列は、有限セルの数を count とする（例: 2 行の列 `[NaN, 4]` は count 1）。
  - 検査: `column_count_counts_finite_cells`。

## Feature: 既存挙動の非退行

### Rule: `Spectrogram::tile` の集約・クランプ・STFT 結果は変わらない

- Example: 既存の集約（Max / Mean）・frame / bin のクランプ・NaN 入力の伝播が従来どおり。
  - 検査: `tile_aggregates_frames_and_bins`、`tile_clamps_and_marks_empty`、`stft_nan_input_makes_nan_column`、`stft_finds_a_sine_bin`（無変更で pass）。
