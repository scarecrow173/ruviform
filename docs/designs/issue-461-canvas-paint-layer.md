---
id: "DESIGN-034TXijlXpfTBgjbwGLo16"
type: "design"
status: "approved"
title: "Issue #461 — canvas painter の paint batch（paint_layer）適用と kind 別監査"
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
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-461-canvas-paint-layer.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-453-canvas-size-audit.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-434-spectrogram-kind.md"
  refines:
    - "docs/designs/issue-453-canvas-size-audit.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-461-canvas-paint-layer.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #461 — canvas painter の paint batch（paint_layer）適用と kind 別監査

## 概要

`ruviform-ui-gpui` の spec サイズ比例 canvas painter は、要素ごとに `window.paint_quad` を発行し、各 quad が `Scene::insert_primitive` → `BoundsTree::insert` を通る。`BoundsTree::insert` の `find_max_ordering` は intersect する subtree を走査するため、同一 canvas 内の quad 同士が交差しなくても内部 node の union bounds が常に交差し、insert あたり O(n) → 全体で O(n²) となる。`Window::paint_layer` で囲むと内部 primitive は layer の order を共有して tree を経由しないため O(n) になる。

本設計は、spec サイズに比例する全 canvas painter（Spectrogram / Waveform / Meter / TimeAxis）に `paint_layer` を適用し、契約上限 payload での repaint コストを計測して監査結果を記録する。quad 数が spec サイズに比例しない painter（Fader / Knob / Seek / NativeSlot）は対象外とする。

## 監査方法

- headless `TestAppContext`（`#[gpui::test]`）に、各 kind の契約上限 payload を持つ `ViewSpec` を Panel 直下にマウントし、`notify` → `flush_effects` の同期 draw で 1 repaint を計測する。`TestWindow::draw` は完成した `Scene` を受け取るだけで、`Scene` 構築（`BoundsTree::insert` を含む paint 経路）は headless でも実経路を通る。
- 計測は debug build（`cargo test` 既定プロファイル）、Windows x64、ワークスペース `rf461-paint-layer`、gpui 0.2.2 で行った。値は 1 サンプルの wall-clock であり、絶対値ではなく before/after のオーダー比較に使う。
- 上限 payload: Spectrogram 512 columns × 128 rows = `MAX_SPECTROGRAM_CELLS` (65,536) cells、Waveform 2ch × 8,192 bins = `MAX_WAVEFORM_BINS` (16,384) bins（全 bin が peak+RMS で最大約 32k quad）、Meter 2ch × 4,096 bars = `MAX_METER_BINS` (8,192) bars（最大約 16k quad）、TimeAxis `MAX_TIME_AXIS_TICKS` (1,024) ticks。

## 監査結果（repaint 1 回あたり、debug build）

| kind | 1 canvas あたりの quad 数（上限） | 修正前 | 修正後 | 判定 |
| --- | --- | --- | --- | --- |
| Spectrogram | 1（背景）+ 最大 65,536 | 6.37 s | 53.3 ms | **O(n²) → layer 化** |
| Waveform | 1（背景）+ ch 本の baseline + bin ごと最大 2 | 45.19 s | 27.3 ms | **O(n²) → layer 化** |
| Meter | 1（背景）+ bar ごと最大 2 | 13.06 s | 11.5 ms | **O(n²) → layer 化** |
| TimeAxis | tick ごと 1（上限 1,024） | 906 ms | 806 ms | **layer 化（下記の残コスト注記）** |
| Fader / Knob / Seek | 3（固定） | — | — | 対象外（spec サイズに比例しない） |
| NativeSlot | 1（固定） | — | — | 対象外（同上） |
| Label / Table / Button / Toggle / Tabs | canvas 不使用 | — | — | 対象外（canvas painter ではない） |

注記:

- Waveform が Spectrogram より遅いのは、peak quad と RMS overlay quad が同一 x 範囲で重なり、隣接 bin の 1 px clamp も重なりを生むため、`find_max_ordering` の走査がより深くなるためと考えられる（構造推定。計測値のみが確定事実）。
- TimeAxis は layer 化で tick quad 分（約 100 ms 相当）が改善したが、repaint の大部分は tick ごとの label `div`（最大 1,024 要素）の layout・text 描画に支配される。これは canvas quad ではなく element tree 規模の問題であり、本 Issue の対象外として残課題に記録する。tick quad は spec サイズに比例するため、規則の一貫性と将来の tick 数増加への備えとして layer 化を維持する。
- Fader / Knob / Seek は `control_element` が track + fill + thumb の 3 quad のみ、NativeSlot は placeholder 1 quad のみで、spec サイズに比例しない。3〜1 回の `BoundsTree::insert` は問題にならないため layer 化しない。

## 原因の位置

- 破られている不変条件: 「1 paint あたりの scene 挿入コストは primitive 数に対して準線形である」。所有する層は `ruviform-ui-gpui` の renderer（`render.rs` の各 `*_element` paint closure）である。
- `BoundsTree::insert` は挿入 bounds と intersect する既存 bounds の最大 ordering + 1 を返す。同一 canvas 内の quad は互いに交差しないが、canvas 内の全 quad が同じ領域を占めるため内部 node の union bounds が常に intersect し、`find_max_ordering` が広い subtree を走査する。これが O(n²) の源である。
- `Window::paint_layer(bounds, f)` は layer bounds を 1 回だけ `primitive_bounds` に挿入し、`f` 内の `insert_primitive` は `layer_stack.last()` の order を共有して tree を経由しない。`Scene::finish` の stable sort が同一 order 内の挿入順を維持するため、layer 内の描画順は paint 順どおりに保存される。cell の 1 px clamp による端の重なりも、同一 order 内では挿入順で正しく重なる。

## 是正の形

- 各 spec サイズ比例 painter の paint closure で、背景 quad（1 枚・固定）は layer の外に描き、要素ごとの quad 列を `window.paint_layer(bounds, |window| { ... })` で囲む。
  - Spectrogram: 背景 → layer 内で cell 列。
  - Waveform: 背景 → layer 内で channel ごとの baseline + bin の peak/RMS quad。
  - Meter: 背景 → layer 内で bar の peak/RMS quad。
  - TimeAxis: 背景 quad を持たないため、tick mark 列全体を layer 化。
- `paint_layer` は paint phase でのみ呼べる（`debug_assert_paint`）が、canvas の paint callback は paint phase で実行されるため適合する。layer bounds は `bounds ∩ content_mask` に clip され、空なら layer を push しない（その場合も `f` は実行され、primitive は通常経路に落ちる）。
- `paint_layer` は primitive の content_mask を変更しない。各 quad の clip は従来どおり ambient content_mask で行われ、cell の端の 1 px clamp は描画結果を変えない。
- layer の order は「layer bounds と intersect する既存 primitive の最大 order + 1」であり、canvas 内に閉じた描画では従来の個別 insert と同じ重なり順を与える。canvas 外の要素との前後関係も、後続に描かれる intersect する primitive が layer order より大きい order を得るため保存される。

## 採らなかった案

- quad を 1 枚の `Path` / 単一 mesh にまとめる案。色・alpha が cell ごとに異なり（`palette.viz_waveform.opacity(cell.intensity)`）、結合すると頂点色付き mesh 経路が必要になる。GPUI の公開 API では `paint_quad` の batch が `paint_layer` であり、追加の描画プリミティブを導入する根拠がない。
- `ruviform-ui` の spec 側で payload を間引く案。契約上限は正当な利用であり、renderer が上限 payload を実用的に描けることが本 Issue の要求である。間引きは consumer の責務であり、契約を変えない。
- Fader / Knob / Seek / NativeSlot の layer 化。quad 数が固定で小さく、layer push/pop のオーバーヘッドに見合わない。
- TimeAxis の label `div` 削減（例: 表示範囲内のみ label を出す等）。element tree 規模の別問題であり、本 Issue の canvas quad 監査の範囲を超える。残課題に記録する。

## 残課題（本設計の範囲外）

- TimeAxis の repaint 約 0.8 s は tick ごとの label `div` + text 描画が支配的。上限 1,024 tick で実用上ブロックし得るため、label 描画戦略（間引き・canvas 化・件数上限の再検討）は別 Issue で扱う。
- Table の行×列 `div` 生成（`MAX_TABLE_ROWS` × `MAX_TABLE_COLUMNS`）も element tree 規模のコストであり、本 Issue の canvas painter 監査の対象外。必要なら別途監査する。
- audio-lens 側の pin 更新は本 Issue merge 後に Coordinator が管理する。

## 検証計画

- 回帰ゲート `rvf_view_max_payload_canvas_repaint_budget`: Spectrogram / Waveform / Meter を契約上限 payload でマウントし、warmup repaint の後の repaint が 500 ms 未満であることを検証する（修正前の実測 6.4 s / 45.2 s / 13.1 s に対し、修正後の実測は約 55 ms 以下。budget は回帰検知用でありベンチマークではない）。
- 既存の drawn plan / canvas bounds parity テスト（`spectrogram_plan_skips_gaps_and_clamps`、`parity_new_kinds_plans_match_shared_fixture`、`rvf_view_canvas_bounds_nonzero_*` 等）が維持されること。
- 既存の GPUI / parity / consumer-observation / driver-boundary / component-view / driver-contract の各 lane が通過すること。
