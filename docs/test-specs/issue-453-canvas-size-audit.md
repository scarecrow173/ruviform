---
id: "TSPEC-WsywxRxhFuHBznTRP361Ie"
type: "test-spec"
status: "approved"
title: "Issue #453 — paint canvas bounds の検証契約（kind 別監査）"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/453"
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
    - "docs/specs/issue-453-canvas-size-audit.md"
    - "docs/designs/issue-453-canvas-size-audit.md"
  depends-on:
    - "docs/specs/issue-453-canvas-size-audit.md"
    - "docs/designs/issue-453-canvas-size-audit.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-450-control-canvas-size.md"
---

# Issue #453 — paint canvas bounds の検証契約

## Feature: 全 paint canvas kind は、宣言された slot どおりの非 0 の bounds を持つ

### Rule: paint canvas の bounds は幅と高さの両方が非 0 である

- Example: Fader・Seek・Waveform を含む Panel をマウントし、`RvfView.bounds` に記録された各 canvas の幅と高さがともに 0 より大きい。
  - 検証手段: gpui-test `rvf_view_canvas_bounds_nonzero_for_controls_and_waveform`（`assert_canvas_bounds_nonzero`）。
  - 修正前: Waveform が `1920 × 0`（失敗）。修正後: `1920 × 120`（成功）。
- Example: Knob・Meter・TimeAxis・Spectrogram を、shared parity fixture の Panel でマウントすると、各 canvas の幅と高さがともに 0 より大きい。
  - 検証手段: gpui-test `rvf_view_canvas_bounds_nonzero_for_parity_kinds`。
  - 修正前: Meter と Spectrogram が高さ 0（失敗）。修正後: 両者とも `1920 × 120`（成功）。
- Example: bound された NativeSlot の canvas が、非 0 の bounds を持つ。
  - 検証手段: gpui-test `rvf_view_native_slot_projects_layout` 内の `assert_canvas_bounds_nonzero(v, &["dock.editor"])`。
  - 修正後: `1918 × 1041`（成功）。

### Rule: slot の宣言寸法を保つ

- Example: TimeAxis の canvas は、宣言された高さ 24 px を、Panel が溢れる場合でも保つ。
  - 検証手段: gpui-test `rvf_view_canvas_bounds_nonzero_for_parity_kinds` の `ruler` 高さの `assert_eq!(…, 24.0)`。
  - 修正前: 18 px（失敗）。修正後: 24 px（成功）。`flex_shrink_0` を外すと同テストが失敗することを確認済み。

### Rule: 既存の契約と境界を弱めない

- Example: GPUI 描画・gesture・parity・consumer-observation・driver-boundary・component-view・driver-contract の既存テストが、成功条件を緩めずに通過する。
  - 検証手段: `mise run test-ui-gpui`、`mise run test-ui-consumer-observation`、`mise run test-ui-parity`、`mise run test-ui-driver-contract`、`mise run test-ui-component-view`、`mise run check-ui-driver-boundary`。

### Rule: 静的検証を通過する

- Example: fmt と clippy（`-D warnings`）が通過する。
  - 検証手段: `mise run fmt-check`、`mise run lint-ui-gpui`。

## 実窓の確認（component_showcase）

- Passed（目視）: `component_showcase` を実 GPUI 窓で起動し、デスクトップを撮影した。修正前は Waveform の領域が空白で、bins は表示されなかった。修正後は Ch 1 の Fader と窓下部の Seek に加え、Waveform の bins（青い波形）が表示された。
- 証拠は一時ファイルとして保持（リポジトリには含めない）: `%TEMP%\issue453-showcase-before.png`（修正前）、`%TEMP%\issue453-showcase-after.png`（修正後）。
- 撮影は Windows 1920×1080 の全画面キャプチャである。窓は 640×480 の論理サイズ（表示は拡大縮小されている）。

## 検証の区別

- Passed: 実際に実行し、成功したもの。
- NotRun: 実行していないもの。理由を明記する。
- Unsupported: この環境で原理的に実行できないもの。理由を明記する。

## NotRun / Unsupported

- NotRun: `synvelis-native` 実窓での canvas 表示の確認。consumer 側のチェックアウトと実機撮影が必要で、本作業では実行していない。
- NotRun: 高 DPI・別解像度での実窓計測。本作業は 1 つのディスプレイで行った。
- NotRun: Ch 2 タブ切替後の実窓の撮影。撮影は Ch 1 の表示のみで、Ch 2 の Fader は未確認。Tabs の切替は #451 で扱った範囲である。
