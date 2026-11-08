---
id: "PLAN-034U0ARdtPkZR0btUirc6D"
type: "plan"
status: "completed"
title: "Issue #463 — Spectrogram cell intensity 修正計画"
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
  implements:
    - "docs/specs/issue-463-spectrogram-intensity.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-463-spectrogram-intensity.md"
    - "docs/designs/issue-463-spectrogram-intensity.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #463 — Spectrogram cell intensity 修正計画 Implementation Plan

## Goal

Spectrogram cell の描画色を `viz.waveform.opacity(intensity)` の
`surface.bg` 上への不透明事前合成に変更し、列 frame 区間の重なりで
alpha が累積飽和する Issue #463 を解消する。

## Scope

単一実装ストリーム。`crates/ruviform-ui-gpui/src/render.rs` の
Spectrogram paint 経路のみ。plan 層・他 painter・公開契約は不変。

## Upstream Documents

- docs/specs/issue-463-spectrogram-intensity.md
- docs/designs/issue-463-spectrogram-intensity.md

## Design Inputs

- docs/designs/issue-463-spectrogram-intensity.md

## File Map

- `crates/ruviform-ui-gpui/src/render.rs` - `spectrogram_element` の cell
  色を `cell_paint_color`（不透明事前合成）へ変更 + gpui-test 回帰
  テスト追加
- `crates/ruviform-ui-gpui/examples/`（既存 example 構成に従う） -
  実窓検証用 spectrogram intensity gradient spec の描画
- `evidence/rf463-spectrogram-intensity/` - 実窓 capture と pixel 解析
  結果の記録

## Implementation Sequence

- [x] 原因確定（paint_layer 棄却・列 frame 共有による alpha 累積飽和を
  gpui 0.2.2 ソースと `Tile::columns` 契約で確認）
- [x] `cell_paint_color` helper（Rgba source-over → 不透明 Rgba）実装
- [x] `spectrogram_element` の cell 色を helper 経由に変更
- [x] gpui-test 回帰テスト追加（端点・単調・不透明・重なり決定性）
- [x] 実窓 example で中間 intensity 中間色を確認し evidence 記録
- [x] mise 正規検証 + review-protocol

## Dependencies and Risks

- `surface_bg` は palette 由来で cell 毎に一定。cell 色計算は O(1)。
- 既知リスク: 不透明化により「cell 重なり時の blend」が「後勝ち上書き」
  に変わる — これは意図した修正（期待色と一致）。
- `surface_bg` quad は gap cell 用に維持（全面背景として必要）。

## Task Breakdown

- TASK-034U0zT7iVYoM99rNxQgBT（本 issue の単一 task）

## Verification

- [x] `mise run fmt-check` / `check` / `lint` / `build` / `test`
- [x] `mise run lint-ui-gpui` / `check-ui-gpui` / `test-ui-gpui`
- [x] `mise run test-ui-parity` / `test-ui-consumer-observation`
      （`check-cli-headless` は mise task 非存在のため NotRun）
- [x] 実窓 capture で中間色 pixel を確認（evidence/）

## Open Questions / Gaps

- 実窓 capture 手段: gpui-test は headless で pixel を返さないため、
  専用 example の実行 + OS 画面 capture（PowerShell 等）で代替する。
  実行環境が window を開けない場合は NotRun として記録する。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（worker dispatch 内で直接実装）
- Independent work streams: none
- User approval required before dispatch: yes（review-protocol で代替）
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- reviewer focus: `cell_paint_color` の source-over 数式が GPU の
  `SRC_ALPHA/INV_SRC_ALPHA` と一致するか、`Hsla`→`Rgba` 変換での
  alpha/色空間の扱い、既存 gap/cull/1px clamp への非干渉。
