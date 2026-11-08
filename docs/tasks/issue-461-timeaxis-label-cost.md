---
id: "TASK-034U1MMRtRon6XPgSgK7gD"
type: "task"
status: "in-progress"
title: "Issue #461 残課題 — TimeAxis label canvas 描画の実装タスク"
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
    - "docs/plans/issue-461-timeaxis-label-cost.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-461-timeaxis-label-cost.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-461-timeaxis-label-cost.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #461 残課題 — TimeAxis label canvas 描画の実装タスク

## Work

`time_axis_element` の label `div` を廃止し、canvas prepaint の `shape_axis_labels`（`WindowTextSystem::shape_text`、style は `window.text_style()` + `text_dim`）と canvas paint の共有 `paint_layer` 内 `paint_axis_label_line`（`paint_glyph` / `paint_emoji`、gpui `paint_line` の単一 style 経路と同じ配置式）で label を描画する。tick mark quad の `paint_layer` は従来どおり。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-461-timeaxis-label-cost.md`
- `docs/specs/issue-461-timeaxis-label-cost.md`
- `docs/designs/issue-461-timeaxis-label-cost.md`
- `docs/test-specs/issue-461-timeaxis-label-cost.md`

## Dependencies

- `docs/plans/issue-461-canvas-paint-layer.md`（前段 Issue #461 の paint_layer 適用済みが前提）

## Affected Files

- `crates/ruviform-ui-gpui/src/render.rs`
- `docs/specs/issue-461-timeaxis-label-cost.md`
- `docs/designs/issue-461-timeaxis-label-cost.md`
- `docs/test-specs/issue-461-timeaxis-label-cost.md`
- `docs/plans/issue-461-timeaxis-label-cost.md`
- `docs/tasks/issue-461-timeaxis-label-cost.md`
- `docs/designs/issue-461-canvas-paint-layer.md`
- `docs/{specs,designs,plans,tasks,test-specs}/README.md`（managed index、生成物）

## Done When

- [x] T1: headless `TestAppContext` の実 paint 経路で label 条件別・tick 数別の repaint を計測し、per-line `paint_layer` → `BoundsTree::insert` が支配的であることを確認する（label 空 ~125 ms / 1,024 labelled ~0.8〜1.7 s / 全 cull ~262 µs）。
- [x] T2: gpui 0.2.2 の `paint_line` / `shape_text` / `LineLayoutCache` / `Window::text_style` を確認し、再現すべき style・glyph 配置・cache 経路を確定する。
- [x] T3: `time_axis_element` の label `div` を廃止し、canvas prepaint + 共有 label `paint_layer` で label を描画する。
- [x] T4: `rvf_view_max_payload_canvas_repaint_budget` に `MAX_TIME_AXIS_TICKS` の labelled TimeAxis ケースを追加する。
- [x] T5: 修正後 repaint を計測して design・test-spec・PR 本文に記録する（約 11 ms）。
- [x] T6: 文書チェーン（spec / design / test-spec / plan / task）を作成し、managed index を再生成、`docs/designs/issue-461-canvas-paint-layer.md` の残課題を更新する。

## Verification

- [x] V1: `rvf_view_max_payload_canvas_repaint_budget` が TimeAxis ケースを含めて通過する（debug、実測 約 11 ms、budget 500 ms）。
- [x] V2: `mise run fmt-check` / `lint-ui-gpui` / `check-ui-gpui` が通過する。
- [x] V3: `mise run test-ui-gpui` / `test-ui-consumer-observation` / `test-ui-parity` / `test-ui-driver-contract` / `test-ui-component-view` / `check-ui-driver-boundary` が通過する。
- [x] V4: `mise run check` / `lint` / `test` / `build` が通過する。
- [x] V5: review-protocol のレビューで blocker が残らないこと（1 サイクル、判定 ACCEPT WITH NON-BLOCKING NOTES。指摘対応: `window.text_style()` による旧 div と同じ style 解決、clippy `assert_is_empty`、wrap boundaries の debug_assert。非ブロッキング: pixel 比較は headless 非対象、label 複製の String alloc は無視できるコスト）。
- [ ] V6: `git fetch origin main && git merge origin/main` 後に最終検証を通し、論理単位でコミット・push・PR 作成。
