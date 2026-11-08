---
id: "SPEC-034TXb9sMUqVVNvWbTUXkU"
type: "spec"
status: "approved"
title: "Issue #461 — spec サイズ比例の canvas painter の O(n²) paint を paint_layer で解消"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/461"
    - "https://github.com/scarecrow173/audio-lens/issues/180"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-461-canvas-paint-layer.md"
      - type: "file"
        path: "docs/test-specs/issue-461-canvas-paint-layer.md"
      - type: "file"
        path: "docs/plans/issue-461-canvas-paint-layer.md"
      - type: "file"
        path: "docs/tasks/issue-461-canvas-paint-layer.md"
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
    - "docs/specs/issue-453-canvas-size-audit.md"
  blocks: []
  supersedes: []
  superseded-by: []
  refines:
    - "docs/specs/issue-453-canvas-size-audit.md"
  related:
    - "docs/designs/issue-453-canvas-size-audit.md"
    - "docs/specs/issue-434-spectrogram-kind.md"
---

# Issue #461 — spec サイズ比例の canvas painter の O(n²) paint を paint_layer で解消

## 背景

AudioLens #180 の調査で、Dataset workspace の Spectrogram tab 選択時の UI thread hard hang の root cause が 2 段階あることが判明した。前段（canvas 0 サイズによる全面 overlap）は #453 / PR #459 で解消済みだが、後段が残っている: `spectrogram_element` が cell ごとに `window.paint_quad` を発行し、最大 65,536 primitive/フレームがそれぞれ `Scene::insert_primitive` → `BoundsTree::insert` を通る。cell 同士は交差しないが、内部 node の union bounds が常に交差するため `find_max_ordering` が sibling subtree を広く走査し、insert あたり O(n) → 全体で O(n²) となる。debug build で 1 paint あたり約 11.7 秒（headless GPUI / VisualTestContext、`129786ba` 計測値）。

`Window::paint_layer` は「non-overlapping かつ同一 draw order の geometry を batch するための機構」と gpui doc に明記されており、内部 primitive は layer の order を共有して `BoundsTree::insert` を経由しない。同じ計測で `paint_layer` で囲んだ場合 43.5 ms（約 270 倍高速化）を確認済みである。

## 観測（出発点）

- `129786ba`（#453 後）、canvas bounds `1280px × 737px`、65,536 cells で paint に 11.74 秒。
- 同条件 + `window.paint_layer(bounds, ...)` で cell 描画をラップしたローカル検証で 43.5 ms。
- Waveform（bin ごとに最大 2 quad）・Meter（bar ごとに最大 2 quad）・TimeAxis（tick ごとに 1 quad）も primitive 数が spec サイズに比例し、同じリスクを持つ。契約上限は `MAX_WAVEFORM_BINS = 16384`、`MAX_METER_BINS = 8192`、`MAX_TIME_AXIS_TICKS = 1024`、`MAX_SPECTROGRAM_CELLS = 65536`。
- Fader / Knob / Seek は 1 canvas あたり 3 quad、NativeSlot は 1 quad で、spec サイズに比例しない。

## 要求

1. Spectrogram の cell 描画を `Window::paint_layer`（または同等の batch 経路）で囲み、契約上限 65,536 cells でも paint が UI thread を実用上ブロックしないこと（目安: debug build で数十〜数百 ms オーダー）。
2. Waveform・Meter・TimeAxis など spec サイズに比例する canvas painter を監査し、paint コストが閾値を超えるものは同じ batch 経路で是正する。監査結果（対象 kind・quad 数・判定）は design と PR 本文に一覧化する。
3. 上限 cell で paint が閾値内に収まる回帰テスト、または `paint_layer` 経路の構造的検証を追加する。既存の drawn plan / canvas bounds parity テストを維持する。

## 受入条件

- `SpectrogramSpec` が契約上限（65,536 cells）を使っても paint が UI thread を実用上ブロックしないこと（目安: debug build で数十〜数百 ms オーダー）。
- Waveform・Meter など primitive 数が spec サイズに比例する canvas painter を併せて監査し、閾値超過のものは是正する。監査結果を一覧化する。
- spectrogram の paint 性能回帰テスト（上限 cell で paint が閾値内に収まるか、または paint_layer 経路の構造的検証）、Fake/GPUI parity 維持。

## スコープ外

- `ruviform-ui`（公開契約）の変更。`SpectrogramSpec`・`WaveformSpec`・`MeterSpec`・`TimeAxisSpec` の形は変えない。
- consumer 側（audio-lens / synvelis）のコードと pin 更新。AL 側の pin 更新は本 Issue merge 後に Coordinator が管理する。
- canvas を使わない要素（Label / Table / Button / Toggle / Tabs）の element tree 規模。Table の行×列 div 生成は別のコスト系であり、本 Issue の canvas painter 監査の対象外とする。
- 実機（実 GPUI 窓）での計測。headless `TestAppContext` の scene 構築で計測する（`TestWindow::draw` は scene を受け取るのみで、`Scene` 構築＝`BoundsTree::insert` のコストは headless でも実経路を通る）。
- ライセンス・公開 API の変更。

## 検証の区別

- Passed: 実際に実行し、成功したもののみ。
- NotRun: 実行していないもの。理由を明記する。
- Unsupported: この環境で原理的に実行できないもの。理由を明記する。

## Sources and Relations

- Issue #461 本文（計測済み root cause と検証条件）。
- AudioLens Issue #180（実機 hang の元報告）。
- #453 / PR #459（canvas 0 サイズ是正・前段）: `docs/specs/issue-453-canvas-size-audit.md`。
- #434 / #442（Spectrogram kind 追加）: `docs/specs/issue-434-spectrogram-kind.md`。
