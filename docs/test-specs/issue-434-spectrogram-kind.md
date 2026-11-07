---
id: "TSPEC-Bw6Qk3RnZtLp9VcSj4MdGa"
type: "test-spec"
status: "proposed"
title: "Issue #434 — Spectrogram kind の検証契約"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/434"
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
    - "docs/specs/issue-434-spectrogram-kind.md"
    - "docs/designs/issue-434-spectrogram-kind.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-421-meter-timeaxis-gpui-parity.md"
---

# Issue #434 — 検証契約

## Feature: Spectrogram payload の契約

### Rule: NaN セルと count == 0 の列は payload に保持される

- Example: `magnitudes[0]` が NaN、`count == 0` の列を含む spec は mount を受理し、mount 後の spec でも NaN と `count == 0` が保持される。
  - 検査: `spectrogram_keeps_gap_cells_and_rejects_unbounded_payloads`（`ruviform-ui` の lib テスト。`mise exec -- cargo test -p ruviform-ui --lib`）。

### Rule: 不正な payload は mount で拒否される

- Example: 列の `magnitudes.len() != rows`、`rows == 0`、`columns × rows > MAX_SPECTROGRAM_CELLS`、`magnitude_range` の `start >= end` は `ViewError::InvalidSpec`。
  - 検査: `spectrogram_keeps_gap_cells_and_rejects_unbounded_payloads`。

## Feature: Fake/GPUI parity（同一 fixture）

### Rule: Spectrogram の mount key と projection は両 driver で一致する

- Example: `driver-lab/parity/new-kinds.json` の `mountedKeys` に `spectrogram` が含まれ、Fake は `Mounted { key: spectrogram }` を発行し observe 可能。
  - 検査（Fake）: `parity_spectrogram_payload_retained_fake`（`mise run test-ui-parity`）。
  - 検査（既存）: `ui_dvr_02_parity_new_kinds_mount_reconcile` は fixture の全 `mountedKeys` を検査する。
  - 検査（GPUI）: `consumer_observation` の `observes_mounted_projection_of_parity_view`（`mise run test-ui-consumer-observation`）と `rvf_view_parity_new_kinds_mount_reconcile`（`mise run test-ui-gpui`）。

### Rule: GPUI の描画 plan は shared fixture の drawn 期待値と一致する

- Example: `expect.spectrogram.drawnCells`（4 セル、x/y 区間と intensity）と `spectrogram_cell_plan` の出力が一致する。row 0 は下端、row 1 は上端。
  - 検査（GPUI）: `parity_new_kinds_plans_match_shared_fixture`（`mise run test-ui-gpui`）。

### Rule: GPUI は gap を描画せず、強度を範囲内にクランプする

- Example: `count == 0` の列、非有限 magnitude のセル、viewport と重ならない列は plan に出ない。`magnitude_range` の上下は intensity `1.0` / `0.0` にクランプされる。
  - 検査（GPUI）: `spectrogram_plan_skips_gaps_and_clamps`（`mise run test-ui-gpui`）。

## Feature: 既存 kind の不変

### Rule: Waveform / Meter / TimeAxis の挙動は変わらない

- Example: 既存の `component_view`（17 件）、`driver_parity` の既存 3 件、GPUI の既存 gpui-test（Meter/TimeAxis plan・gesture・reconcile を含む）が pass する。
  - 検査: `mise run test-ui-component-view`、`mise run test-ui-parity`、`mise run test-ui-gpui`。

## Feature: 境界

### Rule: 共通層に GPUI 型を出さない

- Example: `ruviform-ui` は GPUI に依存せず、`check-ui-driver-boundary` が pass する。
  - 検査: `mise run check-ui-driver-boundary`。
