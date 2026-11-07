---
id: "TASK-Xb2Qm5Ty7WzHf3Jn6RsPd1"
type: "task"
status: "proposed"
title: "Issue #442 — 描画 plan への Spectrogram / Label / Table 追加の実装タスク"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/442"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-442-observer-drawn-spectrogram-label-table.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/observe.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/consumer_observation.rs"
      - type: "file"
        path: "mise.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-442-observer-drawn-spectrogram-label-table.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-442-observer-drawn-spectrogram-label-table.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-442-observer-drawn-spectrogram-label-table.md"
---

# Issue #442 — 描画 plan への Spectrogram / Label / Table 追加の実装タスク

## T1: 期待値テストの追加（consumer）

- 対象: `tests/consumer_observation.rs`。
- 作業:
  - `fixture(name)` で parity fixture を名前指定で読む。
  - `observes_drawn_spectrogram_cells_through_seam`: `new-kinds.json` の `expect.spectrogram.drawnCells` と一致。
  - `observes_drawn_label_and_table_plans_through_seam`: `label-table.json` の `expect.label.text` / `expect.table.drawnHeader` / `drawnRows` と一致。
  - `observes_drawn_meter_and_axis_plans_through_seam` の drawn 一覧を `[meter, ruler, spectrogram]` へ更新。
- 完了条件: テストが期待値を比較している（件数と各値）。

## T2: render.rs の描画走査

- 対象: `render.rs`。
- 作業:
  - `DrawPlan` に `Spectrogram(Vec<SpectrogramCellPlan>)`・`Label(String)`・`Table(TablePlan)`。
  - `collect_drawn` に `ComponentSpec::Spectrogram` / `Label` / `Table` の arm を追加（既存の `spectrogram_cell_plan` / `table_plan` を呼ぶ）。
  - `SpectrogramCellPlan` / `TablePlan` と各 field を `pub(crate)` 化。
  - `drawn_plans` の doc を更新。
- 完了条件: `mise run test-ui-gpui` が通る。

## T3: 公開 DTO と写像

- 対象: `observe.rs`、`lib.rs`。
- 作業:
  - `DrawnPlan` に `Spectrogram { cells }`・`Label { text }`・`Table { header, rows }`（`#[non_exhaustive]` の enum の variant。rustdoc 付き）。
  - `DrawnCell { x0, x1, y0, y1, intensity }`（`#[non_exhaustive]`、`f64`）。
  - `to_drawn_plan` に 3 variant の写像を追加。
  - `ViewDrawn` / `drawn` の doc を更新。
  - `lib.rs` の再公開に `DrawnCell` を追加。
- 完了条件: `mise run test-ui-consumer-observation` と `mise run check-ui-gpui` が通る。

## T4: mise と docs

- 対象: `mise.toml`（`test-ui-consumer-observation` の description）。
- 完了条件: description が Spectrogram / Label / Table を含む。task 名・コマンドは不変。

## T5: 検証と PR

- 作業: `docs/plans/issue-442-observer-drawn-spectrogram-label-table.md` の検証マトリクス 1〜7 を実行し、結果を `docs/test-specs/issue-442-observer-drawn-spectrogram-label-table.md` に記録。review-protocol（独立レビュー）を実施し、指摘を反映。
- 完了条件: 検証結果と NotRun / Unsupported の理由を PR 本文に記載。PR は `Refs #442`（Issue は close しない）。
