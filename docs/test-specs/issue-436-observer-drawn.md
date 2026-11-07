---
id: "TEST-Cz6GhM2TwPq9RkLd4VnXa7"
type: "test-spec"
status: "proposed"
title: "Issue #436 — 描画 plan 公開のテスト仕様"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/436"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-436-observer-drawn.md"
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
    - "docs/specs/issue-436-observer-drawn.md"
    - "docs/designs/issue-436-observer-drawn.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-436-observer-drawn.md"
---

# Issue #436 — 描画 plan 公開のテスト仕様

## Feature: consumer が GpuiObserver から描画 plan を観測できる

### Rule: Meter の drawnBars は fixture の期待値と一致する

- Example: 共有 fixture `parity/new-kinds.json` を mount し、`drawn(window)` の meter component の bars（件数・順序・channel・x0・x1・peak・rms）が `expect.meter.drawnBars` と一致する（許容誤差 `1e-3`）。
- 検証: `observes_drawn_meter_and_axis_plans_through_seam`（`consumer_observation.rs`）。

### Rule: TimeAxis の drawnMarks は viewport 内のみ返す

- Example: `expect.timeAxis.drawnMarks`（4 tick のうち viewport 外の 1 件を除く 3 件）と、件数・x・major・label が一致する。
- 検証: 同上。

### Rule: Waveform は描画される bin の frame 範囲を返す

- Example: channel 0 は `[0,100)` の bin（count > 0）と `[100,200)` の gap bin（count == 0）を持つ → 描画 bin は `0..100` のみ。channel 1 は `0..300` の 1 bin → `0..300`。
- 検証: `observes_drawn_waveform_bins_through_seam`（inline JSON）。

### Rule: 描画対象外の component は plan に現れない

- Example: `drawn` の `components` は meter と ruler のみ（toggle / knob / root panel を含まない）。
- 検証: `observes_drawn_meter_and_axis_plans_through_seam` 内の key 検査。

### Rule: 未知 window では drawn は None

- Example: `WindowId` 9999 で `drawn` が `None`。
- 検証: `unknown_window_has_no_view_to_observe_or_drive`。

## Feature: 描画 plan の走査と paint の規則は一致する（render.rs）

### Rule: Tabs は active の子だけを描画 plan に含める

- Example: Tabs（tabs 2、active 1）の両方に meter がある spec で、`drawn_plans` は active 側の 1 件のみを返す。
- 検証: `drawn_plans_follow_paint_tree`。

### Rule: gap・非有限 peak・viewport 外は plan に含めない（既存規則の維持）

- Example: `meter_and_axis_plans_skip_gaps_and_clamp`（既存）が引き続き通る。
- 検証: `mise run test-ui-gpui`。

### Rule: Waveform の gap bin（count == 0・非有限 min / max）は描画 plan に含めない

- Example: channel 0 の `[200,300)`（min / max が NaN、count 100）と `[100,200)`（count 0）は plan に現れず、`0..100` のみ残る。
- 検証: `waveform_plan_keeps_only_painted_bins`（render.rs）。paint と plan は `waveform_bin_drawn` を共有する。

## Feature: 公開面の境界

### Rule: 公開 signature に gpui 型が現れない

- 検証: `mise run check-ui-driver-boundary`（exit 0）。
