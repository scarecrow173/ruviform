---
id: "TASK-Wn3PdK7RyLb5QvHs9ZcGe8"
type: "task"
status: "proposed"
title: "Issue #436 — 描画 plan 公開の実装タスク"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/436"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-436-observer-drawn.md"
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
    - "docs/plans/issue-436-observer-drawn.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-436-observer-drawn.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-436-observer-drawn.md"
---

# Issue #436 — 描画 plan 公開の実装タスク

## T1: Waveform gap 判定の抽出（挙動不変）

- 対象: `render.rs` の `waveform_element` paint closure。
- 作業: gap 判定を `fn waveform_bin_drawn(bin: &WaveformBin) -> bool` へ抽出し、paint から呼ぶ。
- 完了条件: `mise run test-ui-gpui` が通る。

## T2: 描画走査 `drawn_plans` と内部 DrawPlan

- 対象: `render.rs`。
- 作業: `DrawPlan { Meter, TimeAxis, Waveform }`、`drawn_plans(&ComponentSpec) -> Vec<(ComponentKey, DrawPlan)>`。走査は `element_for` と同じ（Panel 全子・Tabs active のみ）。`MeterBarPlan` / `AxisMarkPlan` の field を `pub(crate)` 化。
- 完了条件: `drawn_plans_follow_paint_tree` が通る（Tabs の非 active 子が現れない、gap・clamp・culling が plan に反映される）。

## T3: 公開 DTO と `GpuiObserver::drawn`

- 対象: `observe.rs`、`lib.rs`。
- 作業: `ViewDrawn` / `ComponentDrawn` / `DrawnPlan` / `DrawnBar` / `DrawnMark` / `DrawnWaveformChannel`（`#[non_exhaustive]`、rustdoc 付き）。`drawn(&self, window) -> Option<ViewDrawn>`。`lib.rs` の再公開に追加。
- 完了条件: `cargo check -p ruviform-ui-gpui --features gpui-test --all-targets` が通る。

## T4: consumer 観測テスト

- 対象: `tests/consumer_observation.rs`。
- 作業:
  - `observes_drawn_meter_and_axis_plans_through_seam`: `expect.meter.drawnBars` / `expect.timeAxis.drawnMarks` と一致（許容誤差 `1e-3`）。
  - `observes_drawn_waveform_bins_through_seam`: inline JSON の ViewSpec（gap の count 0 を含む）で drawn bin 範囲を確認。
  - `unknown_window_has_no_view_to_observe_or_drive` に `drawn(...)` が `None` である assertion を追加。
- 完了条件: `mise run test-ui-consumer-observation` が通る。

## T5: mise と docs

- 対象: `mise.toml`（`test-ui-consumer-observation` の description）。
- 完了条件: description が drawn 観測を含む。task 名・コマンドは不変。

## T6: 検証と PR

- 作業: `docs/plans/issue-436-observer-drawn.md` の検証マトリクス 1〜7 を実行し、結果を `docs/test-specs/issue-436-observer-drawn.md` に記録。review-protocol（独立レビュー）を実施し、指摘を反映。
- 完了条件: 検証結果と NotRun / Unsupported の理由を PR 本文に記載。PR は `Refs #436`（Issue は close しない）。
