---
id: "PLAN-034U1MMbt0YsJ3ObAlnx0z"
type: "plan"
status: "in-progress"
title: "Issue #461 残課題 — TimeAxis label canvas 描画の実装計画"
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
  implements:
    - "docs/specs/issue-461-timeaxis-label-cost.md"
    - "docs/designs/issue-461-timeaxis-label-cost.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-461-canvas-paint-layer.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines:
    - "docs/plans/issue-461-canvas-paint-layer.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-461-timeaxis-label-cost.md"
    - "docs/designs/issue-461-timeaxis-label-cost.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-461-timeaxis-label-cost.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #461 残課題 — TimeAxis label canvas 描画の実装計画

## Goal

`time_axis_element` の label `div`（最大 1,024 要素）を廃し、canvas prepaint で `WindowTextSystem::shape_text` により shaping した label を、tick mark とは別の 1 つの共有 `paint_layer` 内で glyph 描画することで、契約上限 payload の repaint を debug build で他 painter と同程度のオーダーにする。

## Scope

単一の実装ストリーム。変更対象は `crates/ruviform-ui-gpui/src/render.rs` と docs のみ。`ruviform-ui`・consumer 側・`Cargo.lock` は変更しない。公開契約は不変。

## Upstream Documents

- `docs/specs/issue-461-timeaxis-label-cost.md`
- `docs/designs/issue-461-timeaxis-label-cost.md`
- `docs/specs/issue-461-canvas-paint-layer.md` / `docs/designs/issue-461-canvas-paint-layer.md`（前段 Issue・残課題の正本）

## Design Inputs

- `docs/designs/issue-461-timeaxis-label-cost.md`（approved）

## File Map

- `crates/ruviform-ui-gpui/src/render.rs` — `time_axis_element` の label `div` 廃止、`shape_axis_labels` / `AxisLabelPaint` / `AxisLabelLines` / `paint_axis_label_line` の追加、`rvf_view_max_payload_canvas_repaint_budget` への TimeAxis ケース追加
- `docs/specs/issue-461-timeaxis-label-cost.md` — 追加（受入条件と非スコープ）
- `docs/designs/issue-461-timeaxis-label-cost.md` — 追加（計測・原因・是正の形）
- `docs/test-specs/issue-461-timeaxis-label-cost.md` — 追加（検証契約）
- `docs/plans/issue-461-timeaxis-label-cost.md` — 本書
- `docs/tasks/issue-461-timeaxis-label-cost.md` — 実行単位の追跡
- `docs/designs/issue-461-canvas-paint-layer.md` — 残課題に follow-up の結果を追記
- `docs/*/README.md` の managed index — doc tooling による生成物

## Implementation Sequence

- [x] 診断: 修正前コードの一時 probe で label 条件別・tick 数別の repaint を計測し、per-line `paint_layer` → `BoundsTree::insert` が支配的であることを確認（label 空で ~125 ms、1,024 labelled で ~0.8〜1.7 s、全 cull で ~262 µs）。
- [x] gpui 0.2.2 の `paint_line` / `shape_text` / `LineLayoutCache` / `text_style` を確認し、再現すべき style・glyph 配置・cache 経路を確定。
- [x] 実装: `time_axis_element` の label `div` を廃止し、canvas prepaint で `shape_axis_labels`（`window.text_style()` + `text_dim` 色、`shape_text`）、canvas paint で tick mark layer + label ink union の共有 `paint_layer` 内に `paint_axis_label_line` で glyph を描画。
- [x] 回帰ゲート: `rvf_view_max_payload_canvas_repaint_budget` に `time axis 1024 labelled ticks`（`MAX_TIME_AXIS_TICKS`）を追加。
- [x] 計測: 修正後 repaint 約 11 ms（budget 500 ms）を確認。
- [x] 文書: spec → design → test-spec → plan → task を作成し、managed index を再生成。`docs/designs/issue-461-canvas-paint-layer.md` の残課題を更新。
- [x] review-protocol のレビュー（blocker が残らないこと。ACCEPT WITH NON-BLOCKING NOTES）。
- [x] `git fetch origin main && git merge origin/main` で最新 main を取り込み、生成物を再生成（Already up to date）。
- [x] 最終検証（fmt-check / check / lint / build / test / lint-ui-gpui / check-ui-gpui / test-ui-gpui / test-ui-consumer-observation / test-ui-parity / test-ui-driver-contract / test-ui-component-view / check-ui-driver-boundary）。
- [ ] 論理単位でコミットし、自分のブランチを push。PR を作る（`Refs #461`）。

## Dependencies and Risks

- 依存: gpui 0.2.2 の公開 API（`WindowTextSystem::shape_text`、`WrappedLine` / `LineLayout`、`Window::paint_glyph` / `paint_emoji` / `paint_layer` / `text_style` / `content_mask`）のみ。新規依存なし。
- リスクと対策は design の「Risks and Trade-offs」を参照。

## Task Breakdown

- T1 診断計測と gpui text pipeline の確認
- T2 canvas label 描画の実装
- T3 repaint budget 回帰ゲートへの TimeAxis 追加
- T4 文書チェーンと index 再生成、design 残課題の更新
- T5 review-protocol、main 取り込み、最終検証、コミット・push・PR

## Verification

- [x] `rvf_view_max_payload_canvas_repaint_budget` が TimeAxis ケースを含めて通過する（実測 約 11 ms / budget 500 ms、debug）。
- [x] `mise run fmt-check` / `lint-ui-gpui` / `check-ui-gpui` / `test-ui-gpui` が通過する。
- [x] `mise run test-ui-consumer-observation` / `test-ui-parity` / `test-ui-driver-contract` / `test-ui-component-view` / `check-ui-driver-boundary` が通過する。
- [x] `mise run check` / `lint` / `test` / `build` が通過する。
- [ ] 実 GPUI 窓での実機 paint 計測は NotRun（headless で `Scene` 構築の全経路を通すため O(n²) 回帰は検知可能。実機の体感確認は consumer 側の pin 更新後に委ねる）。

## Open Questions / Gaps

- なし。公開契約変更は不要と判断した（`ask` 不要）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（単一ファイルの renderer 変更と文書のみ）
- Independent work streams: none
- User approval required before dispatch: review-protocol で代替（Worker 共通契約）

## Review Handoff

- glyph 配置が gpui `paint_line` の単一 style 経路と一致するか、label ink union の layer bounds が sibling との描画順を保存するかが reviewer の焦点。
