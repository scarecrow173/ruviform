---
id: "TSPEC-Vm4QrJc7LsBk2NwYg9DeHa"
type: "test-spec"
status: "proposed"
title: "Issue #429 — Consumer 観測 seam の検証契約"
created: "2026-11-08"
updated: "2026-11-08"
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

## Feature: 内部 test hook の撤去と共有 lifecycle 経路

### Rule: lifecycle trace は seam 経由で Fake と同一になる

- Example: `GpuiObserver` 経由で shared lifecycle fixture を実行すると、trace は `expected_trace()` と一致し、driver は exit する。
  - 検査: `gpui_driver_matches_shared_lifecycle_trace`（`mise run test-ui-gpui`）。

## Feature: 公開面は gpui 型を露出しない

### Rule: boundary guard が pass する

- Example: `gui-boundary-guard` の `dep-graph` / `signature` / `allowlist` が pass する。`GpuiObserver` と観測 DTO の公開 signature に `gpui::*` は現れない。
  - 検査: `mise run check-ui-driver-boundary`。

## Feature: 通常ビルドの挙動は変わらない

### Rule: raw parent 捕捉は gpui-test 以外で従来どおり行われる

- Example: `gpui-test` を有効にしない build では `capture_raw_parent` は `raw_parent_of` を呼ぶ（従来どおり）。`gpui-test` の build では `None` を返し、gpui の test platform の `window_handle` を呼ばない。
  - 検査: `mise run test-ui-gpui`（gpui-test）と `mise run lint-ui-gpui`（gpui のみ）の両方が通ること。

## 未検証（NotRun）

- 実 OS window 上での観測（manual verification。gpui test platform の範囲外）。
- Issue #429 の残 DoD（NumericInput/Transport・長尺/RT/GPU release・UI-DVR-03/07/08 GPUI・story 2×2）。PR 本文の対応表を参照。
