---
id: "TSPEC-034U1MLtHiia8IiRjMgkW9"
type: "test-spec"
status: "approved"
title: "Issue #461 残課題 — TimeAxis label canvas 描画の検証契約"
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
    - "docs/specs/issue-461-timeaxis-label-cost.md"
    - "docs/designs/issue-461-timeaxis-label-cost.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-461-canvas-paint-layer.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/issue-461-timeaxis-label-cost.md"
    - "docs/designs/issue-461-timeaxis-label-cost.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #461 残課題 — TimeAxis label canvas 描画の検証契約

## Purpose

TimeAxis label の per-line `paint_layer` → `BoundsTree::insert` による O(n²) repaint 回帰を、契約上限 payload の budget ゲートで恒久的に検知する。併せて、label を canvas 内の共有 layer で描いても tick mark・label の描画契約（位置・テキスト・色・major/minor 描き分け・viewport cull）が維持されることを既存テストで保証する。

## Feature

TimeAxis は、契約上限 `MAX_TIME_AXIS_TICKS`（1,024）の label 付き tick を debug build の repaint で実用的な時間に描き、表示内容は従来の label `div` 実装と同じである。

## Rules

- Rule: 契約上限 payload の repaint が budget 内に収まる
  - Example: `MAX_TIME_AXIS_TICKS`（1,024 ticks、全 tick に label 付き、major/minor 混在）を viewport 全域に持つ TimeAxis を Panel にマウントし、warmup repaint の後の repaint が 500 ms 未満で完了する。
    - 検証手段: gpui-test `rvf_view_max_payload_canvas_repaint_budget` の `time axis 1024 labelled ticks` ケース（`timed_repaint` + `assert_budget`）。
    - 修正前の実測: 約 0.8〜1.7 s（debug build、headless `TestAppContext`、条件別 probe の最大値）。修正後の実測: 約 11 ms。
  - budget の根拠: 既存の Spectrogram / Waveform / Meter と同じ 500 ms。修正後実測（約 11 ms）の約 45 倍であり、O(n²) への回帰（秒オーダー）を確実に検知しつつ低速環境の偽陽性を避ける。ベンチマークではなく回帰ゲート。
- Rule: 描画契約（tick mark・label の内容と位置）を変えない
  - Example: viewport 内の各 tick が `axis_mark_plan` どおりの x 位置・major/minor で描かれ、label は consumer 解決済み文字列を旧 label `div` と同じ原点（`left = bounds.width * m.x`、`top = space_sm`、dim text 色）で描く。
    - 検証手段: `axis_mark_plan` と spec 由来の drawn plan（`collect_drawn` の `DrawnPlan::TimeAxis`）は変更していないため、`meter_and_axis_plans_skip_gaps_and_clamp`・consumer_observation の `observes_drawn_meter_and_axis_plans_through_seam`（minor tick の label `"250"` を含む）が成功条件を緩めずに通過する。glyph 配置は gpui `paint_line` の単一 style・wrap 無し経路と同じ式で、style は `window.text_style()` + `text_dim` で旧 `.text_color` と等価。
- Rule: 既存の検証 lane を弱めない
  - Example: GPUI / parity / consumer-observation / driver-boundary / component-view / driver-contract の既存テストが通過する。
    - 検証手段: `mise run test-ui-gpui`、`mise run test-ui-consumer-observation`、`mise run test-ui-parity`、`mise run test-ui-driver-contract`、`mise run test-ui-component-view`、`mise run check-ui-driver-boundary`。
- Rule: 静的検証を通過する
  - Example: fmt と clippy（`-D warnings`）が通過する。
    - 検証手段: `mise run fmt-check`、`mise run lint-ui-gpui`。

## Guarantees

- label は major・minor 双方の tick に描かれ、viewport 外の tick は描かれない（cull は `axis_mark_plan` が担い不変）。
- label の shaping は gpui `WindowTextSystem::shape_text` を通るため、`LineLayoutCache` によるフレーム間再利用が効く。
- label glyph は tick mark layer とは別の 1 つの共有 `paint_layer`（bounds = 全 label ink の union）に描かれ、per-line の layer insert は発生しない。

## Non-goals

- label の文字列内容・配置・色の「新しい」仕様の規定（consumer 解決済み文字列をそのまま描く従来契約の維持のみ）。
- pixel 単位のスクリーンショット比較。headless では glyph atlas の実ラスタライズ結果までは比較しない。
- Table など TimeAxis 以外の element tree コストの budget 化。

## Risk

- budget テストが時間依存であり、極端に低速な環境では偽陽性を出し得る。修正前の O(n²) 回帰は秒オーダーであり 500 ms budget との識別幅は十分大きい。
- glyph 配置の再実装が gpui `paint_line` と将来の gpui 更新でずれる可能性。dependency 更新時に label の見た目を確認する運用に委ねる（gpui は 0.2.2 pin）。
