---
id: "DESIGN-034U0zRvA4RGErmyI4Tcad"
type: "design"
status: "approved"
title: "Issue #463 — Spectrogram cell 色の不透明事前合成"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/463"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-463-spectrogram-intensity.md"
      - type: "file"
        path: "crates/ruviform-ui-gpui/examples/spectrogram_intensity.rs"
      - type: "file"
        path: "evidence/rf463-spectrogram-intensity/report.md"
      - type: "file"
        path: "evidence/rf463-spectrogram-intensity/analyze.py"
      - type: "file"
        path: "evidence/rf463-spectrogram-intensity/before.png"
      - type: "file"
        path: "evidence/rf463-spectrogram-intensity/after.png"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/Cargo.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-463-spectrogram-intensity.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-461-canvas-paint-layer.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-443-tile-column-contract.md"
  refines:
    - "docs/designs/issue-461-canvas-paint-layer.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-463-spectrogram-intensity.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #463 — Spectrogram cell 色の不透明事前合成

## 概要

Spectrogram painter は cell 毎に `viz.waveform.opacity(intensity)` の
半透明 quad を `paint_layer` 内へ発行する。列の frame 区間が重なる
spec（`ruviform-viz` `Tile::columns` が `cols > frames` のとき保証する
frame 共有を含む）では、同一 pixel を覆う半透明 quad が複数枚になり、
alpha blending が `1-(1-a)^n` で累積してほぼ不透明へ飽和する。これが
Issue #463 の「intensity が pixel に現れない」の root cause である。

修正は painter が cell を塗る色を **`surface.bg` 上に source-over
合成済みの不透明色** に変えること。painter's algorithm により、重なった
cell は後勝ちで上書きされ、各 pixel は必ず「その位置を覆う最後の
cell の期待色」になる。cell が重ならない通常ケースでは、従来の
「半透明 quad 1 枚を bg quad の上に blend」と数値的に同じ結果を
1 段の blend で得る（丸め誤差の差のみ）。

`paint_layer` 自体は正しい（gpui 0.2.2 で layer は draw order の
batch であり、per-quad alpha・`SRC_ALPHA/INV_SRC_ALPHA` blend を
変えないことをソース確認済み）。RF#461 の batch 化・O(n²) 解消・
clip・1px clamp は全て維持する。

## Context

- `spectrogram_element`（`crates/ruviform-ui-gpui/src/render.rs`）は
  `spectrogram_cell_plan` が返す `SpectrogramCellPlan { x0,x1,y0,y1,
  intensity }` を canvas paint closure 内で quad 化する。
- plan 層（`spectrogram_cell_plan`）は spec → 正規化座標 + intensity
  `[0,1]` への写像のみを担い、色解釈は持たない。`DrawnPlan::
  Spectrogram` observation もこの plan を公開しており、intensity の
  観測契約は既に consumer test で固定済み（`consumer_observation.rs`）。
- `SpectrogramColumn.start/end` は frame 単位の `Range<u64>` で、
  `viewport_fraction` で x 位置へ写る。列区間の重なりは spec 契約上
  合法（上流 `Tile::columns` doc 明記）。
- 他の canvas painter（Waveform/Meter/TimeAxis）は `palette.*` の
  不透明色のみを使い、cell/quad 単位の alpha を持たないため同種の
  累積は起きない（kind 別監査: render.rs 内の `.opacity(` 使用は
  Spectrogram の 1 箇所のみ）。

## Scope

### In Scope

- `spectrogram_element` の cell 色計算を「`viz.waveform` の alpha =
  intensity を `surface.bg` に source-over 合成した不透明 `Rgba`」へ
  変更する補助関数とその適用。
- intensity → paint 色の単調性・不透明性・端点一致を検証する
  gpui-test 回帰テスト。
- 実窓（専用 example）での中間 intensity 中間色の pixel 証拠。

### Out of Scope

- plan 層（`spectrogram_cell_plan` / `SpectrogramCellPlan` /
  `DrawnPlan`）の変更 — intensity の意味論は正しく、観測契約も維持。
- `paint_layer` / `paint_quad` / clip / bounds の変更。
- Waveform/Meter/TimeAxis/Fader/Knob/Seek/NativeSlot の paint 経路。
- `ruviform-ui` / `ruviform-viz` の公開契約・tile 規則。
- gpui への patch / fork。

## Components and Boundaries

- `spectrogram_element`（render.rs）: 唯一の変更点。paint closure 内で
  cell quad の色を `cell_paint_color(viz_waveform, surface_bg,
  cell.intensity)` に置き換える。`surface_bg` の全面 quad は gap cell
  用に維持する。
- 新規 private helper（render.rs）: `Rgba` 同士の source-over 合成。
  GPU の `SRC_ALPHA/INV_SRC_ALPHA` blend と同じく非乗算 alpha の
  「over」を sRGB 空間で計算し、非乗算 `Rgba` を返す。
- `spectrogram_cell_plan` / `drawn_plans`: 不変。

## Data and Control Flow

1. spec → `spectrogram_cell_plan` → `Vec<SpectrogramCellPlan>`
   （従来どおり。intensity ∈ [0,1] clamp 済み）。
2. paint closure: `surface_bg` 全面 quad → `paint_layer(bounds)` 内で
   各 cell について `cell_paint_color` で不透明色を求め `paint_quad`。
3. GPU: 不透明 quad は `SRC_ALPHA=1` で bg を完全に置き換える。
   重なった cell は挿入順（= spec の column→row 順）の後勝ちで、
   各 pixel の最終色は「その pixel を覆う最後の cell の
   `viz.waveform.opacity(intensity)` over `surface.bg`」になる。

## 代替案と却下理由

- **plan 層で列区間を正規化（非重複化）**: 列 frame 区間の共有は
  契約上合法であり、どの列が x を代表するかは一意に決まらない。
  plan の意味論を変えると `DrawnPlan` observation と乖離する。
- **alpha のまま描画し consumer 側で重なりを潰す**: user 指定の
  「ruviform 側の責務は ruviform で直す」に反し、consumer workaround
  は禁止。
- **intensity を色相/明度へ写像（alpha を使わない）**: 受入条件が
  「`viz.waveform.opacity(intensity)` を `surface.bg` に合成した期待色」
  を要求しており、alpha 合成 semantics を維持する必要がある。
- **`paint_layer` 撤去で per-quad insert に戻す**: alpha 累積は
  layer の有無に関係なく起きるため無効。RF#461 の O(n²) 修正も
  巻き戻る。

## Non-Functional Constraints

- RF#461 の性能契約を維持: `paint_layer` batch・cell 数は spec 比例の
  まま。cell 色計算は cell 毎に O(1) の浮動小数演算のみ。
- 既存の描画順・clip・1px clamp・gap/cull 規則は不変。
- `unsafe` なし、新規依存なし、公開 API 変更なし。

## 監査: kind 別 alpha 使用状況

| painter | cell/quad 色 | 単位 alpha | 重なり時の影響 |
| --- | --- | --- | --- |
| Spectrogram | `viz_waveform.opacity(intensity)` | あり | **累積飽和（本 Issue）** → 不透明事前合成で解消 |
| Waveform | `viz_waveform` / `viz_waveform_rms` / `viz_baseline` | なし（a=1） | 後勝ち上書きのみ・無害 |
| Meter | `viz_waveform` / `viz_waveform_rms` | なし | 同上 |
| TimeAxis | `viz_baseline` | なし | 同上 |
| Fader/Knob/Seek | `control_*` / `accent` 等 | なし | quad 数固定・対象外（RF#461 と同じ判断） |

## 検証

- headless: `render.rs` gpui-test モジュールに、intensity → paint 色が
  期待合成色と一致・不透明・単調であることを検証する回帰テストを追加
  （`rvf_view_max_payload_canvas_repaint_budget` と同系配置）。
- 実窓: 専用 example で intensity 勾配 + 列 frame 共有の spec を描画し、
  画面 capture の pixel 解析で中間色を確認（`evidence/` に記録）。
