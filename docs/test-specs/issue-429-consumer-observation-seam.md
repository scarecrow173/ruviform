---
id: "TSPEC-Vm4QrJc7LsBk2NwYg9DeHa"
type: "test-spec"
status: "proposed"
title: "Issue #429 — Consumer 観測 seam の検証契約"
created: "2026-11-08"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/429"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/consumer_observation.rs"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-429-consumer-observation-seam.md"
    - "docs/designs/issue-429-consumer-observation-seam.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-421-meter-timeaxis-gpui-parity.md"
---

# Issue #429 — 検証契約

## Feature: consumer が GPUI driver の projection を観測できる

### Rule: mount 直後の projection は host の mounted 状態と一致する

- Example: parity fixture を `create_view_window` で mount し settle すると、`observe` の component 集合は fixture の `expect.mountedKeys` と一致する。
  - 検査: `observes_mounted_projection_of_parity_view`（`consumer_observation.rs`）。
- Example: toggle の committed は、fixture の descriptor の `default_value` で始まる（gesture 前は既定値）。
  - 検査: 同上。
- Example: window inbox は未消費 0 件である。
  - 検査: 同上（`inbox_len`）。

### Rule: gesture の intent は settle の後に観測でき、順序は fixture と一致する

- Example: knob の begin / preview / commit を `dispatch_input` で与え settle すると、`intents` の command kind 列は `expect.knob.intents`（begin, preview, commit）と一致する。committed は preview 値 0.8 になる。
  - 検査: `observes_knob_gesture_intents_through_seam`。
- Example: toggle の atomic set は `expect.toggle.intents`（begin, commit）を発行し、committed は `committedAfterClick`（1.0）になる。
  - 検査: `observes_toggle_atomic_set_through_seam`。

### Rule: 観測できない window は観測も駆動もできない

- Example: 未知の window に対して `observe` は `None`、`dispatch_input` は `UiError::Host` を返す。
  - 検査: `unknown_window_has_no_view_to_observe_or_drive`。

## Feature: 内部 test hook の一本化と共有 lifecycle 経路

### Rule: lifecycle trace は seam 経由で Fake と同一になる

- Example: `GpuiObserver` 経由で shared lifecycle fixture を実行すると、trace は `expected_trace()` と一致し、driver は exit する。
  - 検査: `gpui_driver_matches_shared_lifecycle_trace`（`mise run test-ui-gpui`）。

## Feature: 公開面は gpui 型を露出しない

### Rule: boundary guard が pass する

- Example: `gui-boundary-guard` の `dep-graph` / `signature` / `allowlist` が pass する。`GpuiObserver` と観測 DTO の公開 signature に `gpui::*` は現れない。
  - 検査: `mise run check-ui-driver-boundary`。

## Feature: 長尺・bound 近傍 payload を seam 経由で観測できる

### Rule: payload 上限ちょうどの描画 plan が欠落なく観測できる

- Example: Waveform が `MAX_WAVEFORM_BINS`（16_384 = 8ch × 2_048 bins）で、frame が `u32::MAX` を超える長尺領域にあるとき、`drawn` の各 channel は宣言順の全 bin の frame range（`start..end`）を bit-exact に返す。
  - 検査: `observes_waveform_at_bin_bound_with_long_frame_range`。
- Example: Spectrogram が `MAX_SPECTROGRAM_CELLS`（65_536 = 512 列 × 128 行）で、viewport が列の一部だけを含むとき、viewport 外の列は描画されず、残り 256 列 × 128 行 = 32_768 cell が column-major 順で、f32 幾何・intensity を f64 へ正確に拡幅して返す。
  - 検査: `observes_spectrogram_at_cell_bound_with_viewport_culling`。
- Example: Meter が `MAX_METER_BINS`（8_192 = 16ch × 512 bins）で、viewport 外の bin が channel ごとに cull され、残り 251 bin × 16ch の bar が viewport 分数・clamp 済み peak/RMS を返す。
  - 検査: `observes_meter_at_bin_bound_with_viewport_culling`。

### Rule: gesture 中の authoritative projection は observed に記録され preview が display を駆動する

- Example: knob の gesture 中に `Notify`（revision 1→2）を inbox 経由で届けると、`observed.revision` は新しい側を追い、`display` は preview 値・`committed` は据え置き・`phase` は `Preview` のままである。commit 後は最後に stage された projection が committed/display を上書きする。
  - 検査: `observes_projection_during_gesture_through_seam`。

### Rule: 契約の f64 値は観測経路で f32 に縮退しない

- Example: `min=-60, max=0` の knob に `Plain(-33.333333333333336)` を `Set` すると `committed.plain`/`display.plain` が bit-exact（`to_bits` 一致）で、`normalized` は mapper と同じ f64 演算順の値になる。`Normalized` 入力でも `plain = min + n*(max-min)` が bit-exact で返る。
  - 検査: `observes_committed_value_at_f64_precision`。

### Rule: spec 差し替え後の描画 plan が seam に反映される

- Example: `update_view` で waveform spec を差し替えると、mount 世代（generation guard）は維持されたまま `drawn` が新しい bin range を返す。
  - 検査: `observes_updated_spec_reprojection_through_seam`。

## Feature: 通常ビルドの挙動は変わらない

### Rule: raw parent 捕捉は gpui-test 以外で従来どおり行われる

- Example: `gpui-test` を有効にしない build では `capture_raw_parent` は `raw_parent_of` を呼ぶ（従来どおり）。`gpui-test` の build では `None` を返し、gpui の test platform の `window_handle` を呼ばない。
  - 検査: `mise run test-ui-gpui`（gpui-test）と `mise run lint-ui-gpui`（gpui のみ）の両方が通ること。

## 未検証（NotRun）

- 実 OS window 上での観測（manual verification。gpui test platform の範囲外）。
- 実 GPU release（platform 制約。#313 帰属）。
- Issue #429 の残 DoD のうち他 Issue 帰属のもの: story 2×2 GPUI（#426）・real-os lane（#313）・headless-ts（#294）・pixel golden（#314）・共通 Scenario 契約（#464）。
- `NumericInput` / `Transport` kind: 再評価の結果 composition で充足と判断し新 kind は未実装（task doc の再評価記録参照）。
