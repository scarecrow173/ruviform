---
id: "TSPEC-034U0AQAAk59m51vJU5F4G"
type: "test-spec"
status: "approved"
title: "Issue #463 — Spectrogram cell intensity の paint 色検証契約"
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
  verifies:
    - "docs/specs/issue-463-spectrogram-intensity.md"
    - "docs/designs/issue-463-spectrogram-intensity.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #463 — Spectrogram cell intensity の paint 色検証契約

## Purpose

Spectrogram cell の intensity が paint 結果（GPU へ渡る quad 色）へ
反映されることを保証する検証契約。plan 層の intensity 観測は
`consumer_observation` で既に固定済みであり、本 spec は「plan の
intensity が painter の出力色へ正しく写る」境界を対象とする。
実窓での pixel 検証が困難な headless CI では、本契約が唯一の
自動回帰ゲートとなる。

## Feature

Spectrogram cell intensity paint color — Spectrogram painter は各 cell を
`viz.waveform.opacity(intensity)` を `surface.bg` に source-over 合成した
**不透明**色で塗り、列 frame 区間が重なる spec でも alpha 累積飽和を
起こさない。

## Rules

- 各 cell の paint 色は `over(viz_waveform.opacity(i), surface_bg)` で
  得られる不透明（alpha = 1.0）色である。
- intensity = 0 → `surface_bg` 相当、intensity = 1 → `viz_waveform`
  相当。中間 intensity → RGB が単調に中間へ写る。
- 列 frame 区間の重なり（spec 契約上合法）は描画色に影響しない
  （painter's algorithm の後勝ちで決定的）。
- gap cell（`count == 0`・非有限 magnitude）は描画されない。

## Examples

- Given intensity 0.0/0.5/1.0 の 3 cell、When `cell_paint_color` を
  適用、Then それぞれ surface_bg・中間色・viz_waveform の不透明色。
- Given 列 frame 区間が完全に重なる spec、When 同一 x を覆う cell が
  複数ある、Then 各 pixel の色は最後に描かれた cell の期待色と一致し、
  alpha は常に 1.0。

## Guarantees

- `spectrogram_element` が発行する全 cell quad の alpha は 1.0。
- paint 色関数は intensity に対して RGB 単調（intensity 増で viz_waveform
  側へ単調に近づく）。
- 既存のジオメトリ・culling・gap 規則は変更されない。

## Non-goals

- 実 GPU での pixel 値の厳密一致（実窓 evidence で別途確認）。
- Waveform/Meter/TimeAxis の色検証（単位 alpha を持たず対象外）。
- `DrawnPlan` observation の値（plan 層は変更しない）。

## Risk

- paint 色計算を誤ると intensity が再び潰れる、または hue が
  palette 意図とずれる。端点（0/1）と単調性のテストで検出する。
