---
id: "TSPEC-EcUAfWYYG6G0Npqen48ITk"
type: "test-spec"
status: "approved"
title: "Issue #450 — Seek/Fader canvas bounds の検証契約"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/450"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  verifies:
    - "docs/specs/issue-450-control-canvas-size.md"
    - "docs/designs/issue-450-control-canvas-size.md"
  depends-on:
    - "docs/specs/issue-450-control-canvas-size.md"
    - "docs/designs/issue-450-control-canvas-size.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-421-meter-timeaxis-gpui-parity.md"
---

# Issue #450 — Seek/Fader canvas bounds の検証契約

## Feature: Seek/Fader の canvas は親 slot と同じ非 0 の bounds を持つ

### Rule: paint canvas の bounds は幅と高さの両方が非 0 である

- Example: Seek（横）と Fader（縦）を含む Panel をマウントし、`RvfView.bounds` に記録された Seek / Fader の canvas bounds を読むと、幅と高さがともに 0 より大きい。
  - 検証手段: `render.rs` の gpui-test `rvf_view_seek_and_fader_canvas_bounds_nonzero`。
  - 修正前の結果: Seek は `0px × 24px`（幅 0）で失敗する。
  - 修正後の結果: 成功する（実行済み、後述）。

### Rule: 既存の契約と境界を弱めない

- Example: GPUI 側の描画・gesture・parity・consumer-observation の既存テストが、成功条件を緩めずに通過する。
  - 検証手段: `mise run test-ui-gpui`、`mise run test-ui-consumer-observation`、`mise run test-ui-parity`、`mise run test-ui-driver-contract`、`mise run test-ui-component-view`、`mise run check-ui-driver-boundary`。

### Rule: 静的検証を通過する

- Example: fmt と clippy（`-D warnings`）が通過する。
  - 検証手段: `mise run fmt-check`、`mise run lint-ui-gpui`。

## 検証の区別

- Passed: 実際に実行し、成功したもの。
- NotRun: 実行していないもの。理由を明記する。
- Unsupported: この環境で原理的に実行できないもの。理由を明記する。

## 実窓の確認（ruviform `component_showcase`）

- Passed（目視）: `component_showcase` を実 GPUI 窓で起動し、Ch 1 の Fader と、窓下部の Seek が track / fill / thumb 付きで描画されることを確認した。修正前に `size_full` を一時的に外して撮影すると、Fader と Seek は消え、左端の細い縦線だけが残った（Issue 本文の症状と一致）。
- 証拠は一時ファイルとして保持（リポジトリには含めない）: `%TEMP%\issue450-showcase-before-crop.png`（修正前）、`%TEMP%\issue450-showcase-after-crop.png`（修正後）。
- 注意: 撮影はこの環境の Windows デスクトップ上の窓に対するものである。Ch 2 タブへの切替をクリックで試みたが反応せず、タブ header に pointer handler が無いことをコードで確認した。そのため Ch 2 側の描画は未確認である。

## NotRun / Unsupported

- NotRun: `synvelis-native` 実窓で Seek が視認できることのスクリーンショット証拠。consumer 側のチェックアウトと実機での撮影が必要で、本作業では実行していない。
- NotRun: Waveform / Meter / TimeAxis / Spectrogram の canvas bounds。本 Issue のスコープ外であり、計測していない。
