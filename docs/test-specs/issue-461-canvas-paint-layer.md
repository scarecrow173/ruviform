---
id: "TSPEC-034TXoqpr4Ij3WOJxlAjic"
type: "test-spec"
status: "approved"
title: "Issue #461 — canvas painter paint batch の検証契約"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/461"
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
  depends-on:
    - "docs/specs/issue-461-canvas-paint-layer.md"
    - "docs/designs/issue-461-canvas-paint-layer.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-453-canvas-size-audit.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/issue-461-canvas-paint-layer.md"
    - "docs/designs/issue-461-canvas-paint-layer.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #461 — canvas painter paint batch の検証契約

## Feature: spec サイズ比例の canvas painter は、契約上限 payload の repaint が実用的な時間に収まる

### Rule: 契約上限 payload の repaint が budget 内に収まる

- Example: `MAX_SPECTROGRAM_CELLS`（65,536 cells = 512 columns × 128 rows）の Spectrogram を Panel にマウントし、warmup repaint の後の repaint が 500 ms 未満で完了する。
  - 検証手段: gpui-test `rvf_view_max_payload_canvas_repaint_budget`（`timed_repaint` + `assert_budget`）。
  - 修正前の実測: 6.37 s（debug build、headless `TestAppContext`）。修正後の実測: 約 34 ms。
- Example: `MAX_WAVEFORM_BINS`（16,384 bins = 2ch × 8,192、全 bin が peak+RMS）と `MAX_METER_BINS`（8,192 bars = 2ch × 4,096、peak+RMS）の repaint も同じ budget を満たす。
  - 検証手段: 同テスト内の `waveform 16384 bins` / `meter 8192 bins` ケース。
  - 修正前の実測: 45.19 s / 13.06 s。修正後の実測: 約 16 ms / 約 8 ms。
- budget の根拠: 修正後実測（最大約 55 ms）の約 10 倍。O(n²) への回帰（修正前 6〜45 s）を確実に検知しつつ、低速な CI 環境での偽陽性を避ける幅を持たせた回帰ゲートであり、描画性能のベンチマークではない。

### Rule: paint 順・描画内容・既存契約を変えない

- Example: layer 内の quad は挿入順（= 従来の paint 順）で描かれ、cell の 1 px clamp による端の重なりも同じ順序で重なる。
  - 検証手段: 既存の drawn plan / parity テストが成功条件を緩めずに通過すること（`spectrogram_plan_skips_gaps_and_clamps`、`parity_new_kinds_plans_match_shared_fixture`、`waveform_plan_keeps_only_painted_bins`、`meter_and_axis_plans_skip_gaps_and_clamp`、`drawn_plans_follow_paint_tree`）。`paint_layer` は primitive の content_mask を変更せず、layer の order 割当は intersect する既存 primitive に対して従来の個別 insert と同じ前後関係を与える（design の「是正の形」参照）。
- Example: canvas bounds の記録・pointer 写像・slot 投影は変更されない。
  - 検証手段: `rvf_view_canvas_bounds_nonzero_*`、`rvf_view_native_slot_*`、consumer_observation テストの通過。

### Rule: 既存の検証 lane を弱めない

- Example: GPUI / parity / consumer-observation / driver-boundary / component-view / driver-contract の既存テストが通過する。
  - 検証手段: `mise run test-ui-gpui`、`mise run test-ui-consumer-observation`、`mise run test-ui-parity`、`mise run test-ui-driver-contract`、`mise run test-ui-component-view`、`mise run check-ui-driver-boundary`。

### Rule: 静的検証を通過する

- Example: fmt と clippy（`-D warnings`）が通過する。
  - 検証手段: `mise run fmt-check`、`mise run lint-ui-gpui`。

## 監査の記録（design への委譲）

kind 別の quad 数・修正前後の計測値・layer 化判定の一覧は `docs/designs/issue-461-canvas-paint-layer.md` の監査結果表を正本とし、PR 本文にも転記する。

## 検証の区別

- Passed: 実際に実行し、成功したもの。
- NotRun: 実行していないもの。理由を明記する。
- Unsupported: この環境で原理的に実行できないもの。理由を明記する。

## NotRun / Unsupported

- NotRun: 実 GPUI 窓（`component_showcase` 等の example）での実機 paint 計測。headless `TestAppContext` が `Scene` 構築の全経路（`BoundsTree::insert` を含む）を通すため、O(n²) の回帰検知は headless で完結する。実機での体感確認は consumer 側（audio-lens）の pin 更新後の確認に委ねる。
- NotRun: TimeAxis の label `div` コストの改善検証。本 Issue の対象外（design の残課題）。
