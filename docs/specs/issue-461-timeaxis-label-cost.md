---
id: "SPEC-034U1Mw5EZAfhDMxQUUpbi"
type: "spec"
status: "approved"
title: "Issue #461 残課題 — TimeAxis label の O(n²) text paint を canvas 内の共有 layer で解消"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/461"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-461-timeaxis-label-cost.md"
      - type: "file"
        path: "docs/test-specs/issue-461-timeaxis-label-cost.md"
      - type: "file"
        path: "docs/plans/issue-461-timeaxis-label-cost.md"
      - type: "file"
        path: "docs/tasks/issue-461-timeaxis-label-cost.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "docs/designs/issue-461-canvas-paint-layer.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-461-canvas-paint-layer.md"
  blocks: []
  supersedes: []
  superseded-by: []
  refines:
    - "docs/specs/issue-461-canvas-paint-layer.md"
  related:
    - "docs/designs/issue-461-canvas-paint-layer.md"
    - "docs/specs/issue-453-canvas-size-audit.md"
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #461 残課題 — TimeAxis label の O(n²) text paint を canvas 内の共有 layer で解消

## 背景

Issue #461 / PR #462 で spec サイズ比例の canvas painter（Spectrogram / Waveform / Meter / TimeAxis の tick quad）を `Window::paint_layer` で batch 化し、quad ごとの `BoundsTree::insert` が生む O(n²) を解消した。しかし TimeAxis の repaint は約 906 ms → 806 ms に留まり、支配的な残コストは tick ごとの label `div`（最大 `MAX_TIME_AXIS_TICKS` = 1,024 要素）の element tree 構築と text 描画である。design `docs/designs/issue-461-canvas-paint-layer.md` の監査表・残課題で「別 Issue で扱う」と記録された残課題を、本 spec で引き受ける。

## 観測（出発点）

headless `TestAppContext`（debug build、gpui 0.2.2、`cargo test` 既定プロファイル）での診断計測（`render.rs` の一時 probe、修正前コード）:

- label あり 1,024 ticks: 約 797 ms / 1.72 s（2 サンプル）。
- label 空文字 1,024 ticks: 約 124〜128 ms（text 描画が消えると激減）。
- 1 文字 label / 全 tick 同一 label 1,024 ticks: 約 746〜895 ms（label の内容・長さはほぼ無関係）。
- label あり 256 ticks: 約 55〜57 ms（tick 数に対しほぼ 2 乗で増加）。
- viewport 外に全 tick を cull した場合: 約 262 µs（`axis_mark_plan` の cull は既に効いている）。

機構: 各 label `div` の text 要素は `WrappedLine::paint` → gpui `paint_line` を通り、`paint_line` は行ごとに `window.paint_layer(line_bounds)` を開く。label bounds は互いに交差するため、各 layer の `BoundsTree::insert` が広い subtree を走査し、1,024 label で O(n²) となる。PR #462 が quad に対して直したのと同じ構造問題が text 経路に残っていた。gpui の `LineLayoutCache` は text・font size・run・wrap で keyed されフレーム間で再利用されるため、アプリ側の shaping キャッシュ追加は主因ではない。

## 要求

1. `TimeAxisSpec` の公開契約（tick の `at` / `label` / `major`、viewport cull、表示される tick mark と label テキスト）を変えずに、契約上限 1,024 ticks の repaint が debug build で他 painter と同程度のオーダー（目安: 数十 ms）に収まること。
2. 表示される tick mark（major は全高・minor は半高）と label（major・minor 双方の consumer 解決済み表示文字列、位置・色）の視覚結果を退行させないこと。
3. 回帰ゲート: 既存 `rvf_view_max_payload_canvas_repaint_budget` と同系の budget テストで、label 付き上限 ticks の repaint が閾値内に収まることを検証する。

## 受入条件

- `TimeAxisSpec` が契約上限（`MAX_TIME_AXIS_TICKS` = 1,024 ticks、全 tick に label 付き）を使っても repaint が debug build で有意に高速化されること（before/after を実測して PR 本文に記録する）。
- tick mark・label の描画結果（表示位置・テキスト・色）が視覚的に退行しないこと。
- `rvf_view_max_payload_canvas_repaint_budget`（または同等の回帰検出手段）に TimeAxis の budget ケースを追加する。
- Fake/GPUI parity・consumer observation の既存契約を維持する。

## スコープ外

- `ruviform-ui`（公開契約）の変更。`TimeAxisSpec` / `AxisTick` の形・値域・意味は変えない。
- consumer 側（audio-lens / synvelis）のコードと pin 更新。
- Table の行×列 `div` 生成など、TimeAxis 以外の element tree 規模コスト（design の残課題として引き続き記録）。
- 実機（実 GPUI 窓）での計測。headless `TestAppContext` の scene 構築で計測する。
- ライセンス・公開 API の変更。

## 検証の区別

- Passed: 実際に実行し、成功したもののみ。
- NotRun: 実行していないもの。理由を明記する。
- Unsupported: この環境で原理的に実行できないもの。理由を明記する。

## Sources and Relations

- Issue #461 本文（PR #462 merge 後の残課題として TimeAxis label コストが記録済み）。
- `docs/designs/issue-461-canvas-paint-layer.md`（監査表・残課題・計測方法の正本）。
- `docs/specs/issue-461-canvas-paint-layer.md` / `docs/specs/issue-453-canvas-size-audit.md`。
- gpui 0.2.2 `text_system/line.rs` の `paint_line`（行ごとの `paint_layer`）と `text_system.rs` の `LineLayoutCache`。
