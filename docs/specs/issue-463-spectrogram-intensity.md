---
id: "SPEC-034U0yFY8Tr771Gyy5U4yy"
type: "spec"
status: "approved"
title: "Issue #463 — Spectrogram cell の intensity(alpha)が実窓で反映されない"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/463"
    - "https://github.com/scarecrow173/audio-lens/issues/179"
    - "https://github.com/scarecrow173/audio-lens/issues/180"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-463-spectrogram-intensity.md"
      - type: "file"
        path: "docs/test-specs/issue-463-spectrogram-intensity.md"
      - type: "file"
        path: "docs/plans/issue-463-spectrogram-intensity.md"
      - type: "file"
        path: "docs/tasks/issue-463-spectrogram-intensity.md"
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
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-434-spectrogram-kind.md"
    - "docs/specs/issue-443-tile-column-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-461-canvas-paint-layer.md"
    - "docs/specs/issue-461-canvas-paint-layer.md"
  refines:
    - "docs/specs/issue-434-spectrogram-kind.md"
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #463 — Spectrogram cell の intensity(alpha)が実窓で反映されない

## What Should Be Built

`ruviform-ui-gpui` の Spectrogram canvas painter が、cell 毎の
`intensity` を pixel 色へ正しく反映する。具体的には、各 cell の描画色が
`viz.waveform.opacity(intensity)` を `surface.bg` に合成した期待色と
一致し、intensity が 0..1 に散る spec で中間 intensity が中間色として
観測できる。列の frame 区間が重なり合う spec（`ruviform-viz`
`Tile::columns` が `cols > frames` のときに保証する「neighbouring
columns share frames」のケースを含む）でも、重なった cell の alpha が
累積して飽和しない。

## Why Now

AudioLens AL#179 の実窓検証で、`viewport=0..7`・512 columns・128 rows
の `SpectrogramSpec`（65,536 cells、intensity 分布 0.2〜1.0）を描画した
ところ、canvas 全域がほぼ不透明の `viz.waveform`（#4A90D9）で塗り潰され、
cell 毎の intensity 差が 1 pixel も現れなかった（占有色は #4A90D9 と
#498FD8 の 2 色のみ）。RF#461 の `paint_layer` batch 化（pin `863a7a9d`）
以降の経路で観測された regression であり、Spectrogram の heatmap としての
価値が実窓で失われている。

原因は `paint_layer` による alpha 喪失ではない（gpui 0.2.2 の Scene /
shader / blend state をソース確認済み — layer は draw order の batch で
あり、per-quad alpha は `SRC_ALPHA/INV_SRC_ALPHA` で正しく合成される）。
真の原因は、`SpectrogramColumn.start/end` の frame 区間が重なる spec で、
半透明 quad が同一 pixel に重なり alpha が `1-(1-a)^n` で累積・飽和する
ことである。`ruviform-viz` `Tile::columns` は `cols > frames.len()` の
とき各列が最低 1 frame をカバーし隣接列が frame を共有すると明示しており、
これは契約上合法な spec である。AL#179 の `viewport=0..7`・512 columns
では約 73 列が同一 frame slot を共有し、実効 alpha がほぼ 1.0 に飽和した。

## Users and Value

- AudioLens 等の consumer: Spectrogram tab で magnitude の強弱が
  視覚的に区別でき、解析結果を実窓で読める。
- ruviform consumer 全般: `SpectrogramSpec` の契約（列 frame 区間は
  共有されうる）を consumer 側 workaround なしに正しく描画できる。

## Scope

### In Scope

- `crates/ruviform-ui-gpui` の Spectrogram canvas paint 経路
  （`spectrogram_element` とその補助関数）。
- intensity → pixel 色の対応を検証する回帰テスト。
- 実窓での中間 intensity 中間色の確認と証拠の記録。
- 文書チェーン（spec/design/test-spec/plan/task）。

### Out of Scope

- audio-lens / synvelis 側の spec 生成・projection 変更（consumer 側
  workaround で隠さない方針のため、そもそも変更しない）。
- Waveform / Meter / TimeAxis 等他 painter の描画色変更（不透明色の
  ため同種の alpha 累積は起きない。監査結果は design に記録する）。
- `ruviform-ui` の公開契約（`SpectrogramSpec` / `SpectrogramColumn` /
  `Tile::columns` の意味論は変更しない）。
- gpui 本体（0.2.2）の変更 — layer / blend semantics は正しく、
  重なりは consumer データが生む正当な入力である。

## Requirements

- Spectrogram の各 cell は、`viz.waveform.opacity(intensity)` を
  `surface.bg` に source-over 合成した色で描画される。
- 合成結果の quad は不透明（alpha = 1）であり、cell 同士が重なっても
  alpha が累積しない（後に描かれた cell が前の cell を上書きする
  painter's algorithm で決定的に解決される）。
- intensity = 0 の cell は `surface.bg` 相当、intensity = 1 の cell は
  `viz.waveform` 相当の色になる。
- cell の x/y ジオメトリ、gap cell（`count == 0`・非有限 magnitude）の
  非描画、viewport culling、`paint_layer` による batch 化（RF#461 の
  性能契約）は維持する。

## Acceptance Criteria

- [ ] intensity が 0..1 に散る `SpectrogramSpec` を実窓で描画し、
  pixel 色が `viz.waveform.opacity(intensity)` を `surface.bg` に
  合成した期待色と一致すること（中間 intensity で中間色が出ること）。
  Issue #463 の検証条件と同一。
- [ ] 列 frame 区間が重なる spec（例: `viewport` より多い columns）でも
  cell の intensity 差が pixel に現れること。
- [ ] intensity 勾配が paint 結果に現れることを検証する回帰テストが
  `rvf_view_max_payload_canvas_repaint_budget` と同系の配置
  （`render.rs` の gpui-test モジュール）に追加されていること。

## Deferred Design Concerns

- STFT の hop < window による本来の列重なり（実音声で cols <= frames
  でも列 frame 区間が共有されうるか）は `ruviform-viz` の tile 規則依存。
  現行 `column_offsets` は `cols <= span` で厳密な分割を保証するため、
  本 Issue の対象は `cols > span` の共有ケースに限定される。将来 tile
  規則が真の部分重なりを許した場合も、本修正の「cell 色の不透明事前
  合成」は同じく正しく機能する。

## Sources and Relations

- Issue #463（観測事実・仮説・検証条件の正本）。
- audio-lens#179（AL 側 projection 正常性の実測ダンプ）・
  audio-lens#180（hang 解消と pin 追従の経緯）。
- `ruviform-viz` `Tile::columns` / `column_offsets`
  （`crates/ruviform-viz/src/spectrogram.rs`）— 列 frame 区間の共有を
  保証する上流契約（Issue #443 で公開）。
- gpui 0.2.2 `Window::paint_layer` / `Scene::push_layer` /
  `insert_primitive`（draw order batch であり合成 semantics を変えない
  ことの確認）、`shaders.hlsl` `quad_fragment`（per-quad alpha 保持）、
  `create_blend_state`（`SRC_ALPHA/INV_SRC_ALPHA`）。
- RF#461（`paint_layer` batch 化 — 今回の regression を含む pin
  `863a7a9d`）、RF#434（Spectrogram kind）、RF#443（Tile 列区間契約）、
  RF#442/#436（DrawnPlan observation — plan 層の intensity は既に
  consumer から観測可能）。
