---
id: "TASK-Xk2Dv9LqNb7RtPm5WcFs8H"
type: "task"
status: "done"
title: "Issue #443 — Tile 列区間と count 契約の実装タスク"
created: "2026-11-08"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/443"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-443-tile-column-contract.md"
      - type: "file"
        path: "docs/designs/issue-443-tile-column-contract.md"
      - type: "file"
        path: "docs/plans/issue-443-tile-column-contract.md"
      - type: "file"
        path: "docs/test-specs/issue-443-tile-column-contract.md"
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
  implements:
    - "docs/plans/issue-443-tile-column-contract.md"
---

# Issue #443 — Tile 列区間と count 契約の実装タスク

## T1: 列区間規則の単一実装（`ruviform-viz`）

- [x] `column_offsets` を追加し、`Spectrogram::tile` の列区間式を置き換える。
- [x] `TileColumn` と `Tile::columns` を追加する。
- [x] `lib.rs` で `TileColumn` を re-export する。

## T2: 契約テスト（`ruviform-viz`）

- [x] `columns_partition_frames_for_every_span`（隙間・重なり・過不足なし）
- [x] `columns_cover_frames_when_cols_exceed_span`（`cols > span` の重なり）
- [x] `tile_columns_split_indivisible_span_and_mark_nan_gap`（割り切れない span と NaN 列の count 0）
- [x] `column_count_counts_finite_cells`（有限セル数）

## T3: 契約の doc（`ruviform-ui` / README）

- [x] `SpectrogramColumn::count` の doc を有限セル数（`0..=rows`）へ修正（型・validation は不変）。
- [x] `crates/ruviform-viz/README.md` に `Tile::columns` を記載。

## T4: 検証

- [x] `mise exec -- cargo test -p ruviform-viz --lib`（PR #445 時点 40 件 pass）
- [x] `mise run fmt-check` / `lint` / `check` / `test`（merge 後 HEAD で実施。2026-11-09 に `chore/verify-442-443-435`（= `origin/main` `4e81b055` + docs のみの差分）で再実行し全件 pass。`docs/impl/ir/issue-442-443-435-verification-closeout.md` 参照）
- [x] `mise run check-ui-driver-boundary` / `test-ui-driver-contract` / `test-ui-parity` / `test-ui-component-view` / `test-ui-test-msrv`（同上）
- [x] `mise run test-ui-gpui` / `lint-ui-gpui` / `check-ui-gpui`（同上）

## T5: レビューと PR

- [x] review-protocol（独立サブエージェントによる diff レビュー）で blocker なしを確認する（PR #445 本文に記録済み）。
- [x] PR を日本語で作成する（`Refs #443`、Issue は close しない）。PR 本文に受入対応表・検証欄・NotRun/Unsupported・review-protocol 結果を記載する（PR #445、merge 済み `ccee0cb1`）。

## Follow-up（本 PR の範囲外）

- ~~AudioLens 側 `spectrogram_columns` の複製を `Tile::columns` へ置き換える（consumer 作業）。~~ → 完了。audio-lens `view/screen.rs` は `tile.columns()` をそのまま `SpectrogramColumn` へ写しており、式の複製は無い（2026-11-09 に audio-lens main で確認）。
- ~~空 frame 範囲での `Spectrogram::tile` の添字参照（既存挙動）の調査と修正。~~ → 完了。Issue #446 / PR #446（`d3be1ad9`）で修正済み。
- `ruviform-ui` の `count <= rows` validation の要否検討（未対応。follow-up Issue 候補）。
