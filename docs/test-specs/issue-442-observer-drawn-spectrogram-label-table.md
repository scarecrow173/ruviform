---
id: "TSPEC-Ek8Nr3Gu6LvBx4Cq9TjMaZ"
type: "test-spec"
status: "approved"
title: "Issue #442 — 描画 plan への Spectrogram / Label / Table 追加のテスト仕様"
created: "2026-11-08"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/442"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-442-observer-drawn-spectrogram-label-table.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/consumer_observation.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-442-observer-drawn-spectrogram-label-table.md"
    - "docs/designs/issue-442-observer-drawn-spectrogram-label-table.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-442-observer-drawn-spectrogram-label-table.md"
  verifies:
    - "docs/specs/issue-442-observer-drawn-spectrogram-label-table.md"
    - "docs/designs/issue-442-observer-drawn-spectrogram-label-table.md"
---

# Issue #442 — 描画 plan への Spectrogram / Label / Table 追加のテスト仕様

## Feature: consumer が GpuiObserver から Spectrogram / Label / Table の描画 plan を観測できる

### Rule: Spectrogram の drawnCells は fixture の期待値と一致し、gap 列を含まない

- Example: 共有 fixture `parity/new-kinds.json` を mount し、`drawn(window)` の spectrogram component の cells（件数・順序・x0・x1・y0・y1・intensity）が `expect.spectrogram.drawnCells`（4 cell）と一致する（許容誤差 `1e-3`）。gap 列（`count == 0`）と viewport 外の cell は現れない。
- 検証: `observes_drawn_spectrogram_cells_through_seam`（`consumer_observation.rs`）。

### Rule: Label の text は fixture の期待値と一致する

- Example: 共有 fixture `parity/label-table.json` の `status` を `drawn` で観測すると、`Label { text }` の text が `expect.label.text`（`Running`）と一致する。
- 検証: `observes_drawn_label_and_table_plans_through_seam`。

### Rule: Table の header と rows は fixture の期待値と一致し、欠損セルは宣言位置に残る

- Example: `events` の `Table { header, rows }` が `expect.table.drawnHeader` と `drawnRows` に一致する。`[null, "running", "3"]` と `["export", null, null]` の `None` は位置を保つ。
- 検証: 同上。

### Rule: drawn に含まれる component は paint 順の描画 kind のみ

- Example: `new-kinds.json` の `drawn` の component 一覧は `[meter, ruler, spectrogram]`（toggle・knob・root を含まない）。`label-table.json` の一覧は `[status, events]`。
- 検証: `observes_drawn_meter_and_axis_plans_through_seam`（拡張）、`observes_drawn_label_and_table_plans_through_seam`。

### Rule: 未知 window では drawn は None（既存規則の維持）

- 検証: `unknown_window_has_no_view_to_observe_or_drive`（既存）。

## Feature: 描画値は paint と同じ純関数の出力に一致する

### Rule: 非有限 magnitude・gap 列・viewport 外の列は Spectrogram の描画 plan に含まれない

- Example: NaN magnitude、`count == 0` 列、viewport 外列を含む spectrogram で、`spectrogram_cell_plan` の出力数と順序が期待どおり。JSON fixture では NaN を表現できないため crate 内で検証する。
- 検証: `spectrogram_plan_skips_gaps_and_clamps`（render.rs・既存）。

### Rule: Label / Table の描画 plan は fixture の drawn grid と a11y grid が一致する

- Example: `parity_label_table_plans_match_shared_fixture`（既存）で `table_plan` の header・rows が fixture と一致し、drawn grid が a11y tree の grid と一致する。
- 検証: `mise run test-ui-gpui`（既存）。

## Feature: 公開面と描画挙動の境界

### Rule: 公開 signature に gpui 型が現れない

- 検証: `mise run check-ui-driver-boundary`（exit 0）。

### Rule: 描画挙動に変化がない

- Example: 既存の render unit test（`rvf_view_parity_*`・`drawn_plans_follow_paint_tree`・`waveform_plan_keeps_only_painted_bins` 等）と parity fixture の Fake 側テストが引き続き通る。
- 検証: `mise run test-ui-gpui` / `mise run test-ui-parity`。
