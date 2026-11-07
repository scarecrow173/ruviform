---
id: "SPEC-Qm7Lr2TxVb9KdWn4HcYe8Sa"
type: "spec"
status: "proposed"
title: "Issue #443 — Tile から SpectrogramColumn への写し替え規則の上流契約"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/443"
    - "https://github.com/scarecrow173/ruviform/issues/434"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-443-tile-column-contract.md"
    modified:
      - type: "file"
        path: "crates/ruviform-viz/src/spectrogram.rs"
      - type: "file"
        path: "crates/ruviform-viz/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-viz/README.md"
      - type: "file"
        path: "crates/ruviform-ui/src/view.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-434-spectrogram-kind.md"
    - "docs/designs/issue-434-spectrogram-kind.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "crates/ruviform-viz/src/spectrogram.rs"
---

# Issue #443 — Tile から SpectrogramColumn への写し替え規則の上流契約

## 背景

#434 は `ComponentSpec::Spectrogram` の payload を列ごとの `SpectrogramColumn { start, end, count, magnitudes }` と定めた。`ruviform-viz` の `Tile { frames, cols, rows, values }` はこの列区間と count を持たない。列区間は `Spectrogram::tile` の内部の等分割式でのみ定まり、公開された規則が無い。AudioLens は同じ式を `crates/al-gui/src/view/screen.rs` の `spectrogram_columns` に複製しており、上流の式が変わるとテストが通ったまま列区間がずれうる。

## 目的

- Tile の列区間規則を単一の上流実装として公開する。consumer が式を複製しなくて済むこと。
- 列の count の意味を契約として確定する。Tile から得られる量に限定して定義する。
- `ruviform-ui` の契約（`SpectrogramColumn` の形）は変えない。`count` の説明文だけを実装が返す量に合わせる。

## 受入条件（Issue #443 本文の検証条件に対応）

| # | Issue #443 検証条件 | 本 PR での扱い |
| --- | --- | --- |
| A1 | 列区間が Tile の frame 範囲を隙間なく分割する（割り切れない span を含む） | 達成。`columns_partition_frames_for_every_span`（span 1..=24 の全組合せで隙間・重なり・過不足なし）、`tile_columns_split_indivisible_span_and_mark_nan_gap`（10 frame / 3 列 → 0..3, 3..6, 6..10）。`cols > span` のときは重なりを許す（D3） |
| A2 | NaN のみの列が count 0 として識別できる | 達成。`tile_columns_split_indivisible_span_and_mark_nan_gap`（NaN 列 count 0）、`column_count_counts_finite_cells`（部分 NaN の列は有限セル数） |
| A3 | consumer 側が上流の関数だけで列区間を得られ、式の複製が無い | 上流側は達成（`Tile::columns`）。AudioLens 側の `spectrogram_columns` 複製の除去は consumer 側の後続作業であり、本 PR では行わない（NotRun：audio-lens 側の変更は範囲外） |
| A4 | 既存の Tile / Spectrogram 挙動に変化が無い | 達成。`tile` は同じ列区間式を `column_offsets` 経由で使う。既存の `tile_aggregates_frames_and_bins` / `tile_clamps_and_marks_empty` / `stft_*` が無変更で pass |

## スコープ（本 PR）

- `ruviform-viz`: `TileColumn`、`Tile::columns`、単一の列区間規則 `column_offsets`、単体テスト、export（`lib.rs`）、README。
- `ruviform-ui`: `SpectrogramColumn::count` の doc を「有限セル数（`0..=rows`、0 = gap 列）」へ修正（doc のみ。型・validation は不変）。
- 文書 chain（spec → design → plan → task → test-spec）。

## 非スコープ（follow-up として明示）

- AudioLens 側 `spectrogram_columns` の複製除去（audio-lens の consumer 作業）。
- `Tile` への per-column の有限 sample 数の追加。Tile は Max/Mean で集約済みのため、元の有限 sample 数は Tile から復元できない（設計 D2）。
- `ruviform-ui` 側で `count <= rows` を validation すること。本 PR は契約の doc 修正に留め、受理条件は変えない。
- 空 frame 範囲の tile で `Spectrogram::tile` が `mag` を参照しうる件（既存挙動。別途報告。本 PR で挙動は変えない）。

## 不変条件

- 列区間は `Tile::frames` の範囲内に収まる。空 tile では全列が空の frame 範囲を持つ。
- 列区間の規則は `ruviform-viz` 内の単一関数で定義され、`tile` と `Tile::columns` が共有する。
- `count` は `magnitudes` の有限セル数に等しい（`0..=rows`）。
- `ruviform-ui` は `ruviform-viz` に依存しない（依存方向は不変）。公開 enum 追加なし、MSRV 1.89、workspace lint 準拠。
