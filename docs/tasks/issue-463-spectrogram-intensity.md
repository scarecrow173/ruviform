---
id: "TASK-034U0zT7iVYoM99rNxQgBT"
type: "task"
status: "completed"
title: "Issue #463 — Spectrogram cell intensity 修正タスク"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #463 — Spectrogram cell intensity 修正タスク

## Work

`crates/ruviform-ui-gpui/src/render.rs` の Spectrogram painter を修正:
cell 色を `viz.waveform.opacity(intensity)` の半透明 quad から、
`surface.bg` 上に source-over 合成済みの不透明色へ変更する。
列 frame 区間の重なりによる alpha 累積飽和（Issue #463 の root
cause）を解消し、intensity 勾配が実窓 pixel に現れることを保証する。
併せて gpui-test 回帰テストと実窓 evidence を追加する。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/issue-463-spectrogram-intensity.md
- docs/designs/issue-463-spectrogram-intensity.md
- docs/specs/issue-463-spectrogram-intensity.md
- docs/test-specs/issue-463-spectrogram-intensity.md

## Dependencies

- RF#461 `paint_layer` batch（pin `863a7a9d`）— 性能契約として維持
- RF#443 `Tile::columns` 列 frame 区間共有契約 — 重なり入力の正当性
- gpui 0.2.2（blend/shader は alpha を正しく保持 — 変更不要）

## Affected Files

- `crates/ruviform-ui-gpui/src/render.rs` — `spectrogram_element` +
  新規 `cell_paint_color` helper + gpui-test 回帰テスト
- `crates/ruviform-ui-gpui/examples/` — 実窓検証 example（既存構成に従う）
- `evidence/rf463-spectrogram-intensity/` — 実窓 capture + pixel 解析
- `docs/{specs,designs,test-specs,plans,tasks}/issue-463-spectrogram-intensity.md`
  + 各 README index

## Done When

- [x] cell paint 色が `over(viz_waveform.opacity(i), surface_bg)` の
  不透明色になり、intensity 0/中間/1 で期待色・単調性を満たす
- [x] 列 frame 区間が重なる spec でも intensity 差が pixel に現れる
  （実窓 evidence で確認）
- [x] 回帰テストが `rvf_view_max_payload_canvas_repaint_budget` と
  同系配置で追加されている
- [x] RF#461 の `paint_layer` batch・性能契約が維持されている

## Verification

- [x] `mise run fmt-check` `check` `lint` `build` `test`
- [x] `mise run lint-ui-gpui` `check-ui-gpui` `test-ui-gpui`
- [x] `mise run test-ui-parity` `test-ui-consumer-observation`
      （`check-cli-headless` は task 非存在 → NotRun）
- [x] 実窓 capture の pixel 解析（中間色の存在）
