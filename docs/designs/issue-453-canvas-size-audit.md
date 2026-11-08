---
id: "DESIGN-60vvc47vxcVx8vlRAGVd3v"
type: "design"
status: "approved"
title: "Issue #453 — paint canvas のサイズ決定契約と kind 別監査"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/453"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-453-canvas-size-audit.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-453-canvas-size-audit.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-450-control-canvas-size.md"
  blocks: []
  supersedes: []
  superseded-by: []
  refines:
    - "docs/designs/issue-450-control-canvas-size.md"
  related:
    - "docs/designs/issue-421-meter-timeaxis-gpui-parity.md"
---

# Issue #453 — paint canvas のサイズ決定契約と kind 別監査

## 概要

`ruviform-ui-gpui` の paint canvas は、固有サイズを持たない。canvas の寸法は、その親（slot）の寸法から決まる。#450 は Seek/Fader について「canvas は `size_full` で slot に一致させ、寸法の源泉は slot のみ」と定めた。本設計は同じ規則を全 paint canvas kind に適用し、kind ごとの監査結果と是正を記録する。

## 監査方法

- 各 paint canvas の prepaint hook で、laid-out bounds を `RvfView.bounds`（canvas bounds の記録先）に保存する。Seek/Fader で #450 が使った経路と同じである。
- Knob / Meter / TimeAxis / Spectrogram は shared parity fixture（`driver-lab/parity/new-kinds.json`）の Panel 内でマウントし、Fader / Seek / Waveform は Panel 内でマウントして計測する。NativeSlot は既存の bound slot テストの Tab 内で計測する。
- 計測は gpui-test の既定テストウィンドウ（1920×960 論理 px）で行う。Panel は flex column であり、component_showcase（640×480）と同じ構造である。
- 実窓の証拠は `component_showcase` を実 GPUI 窓で起動し、デスクトップを撮影して得る。

## 監査結果（canvas bounds 実測値, 論理 px）

| kind（key） | 修正前 | 修正後 | 判定 |
| --- | --- | --- | --- |
| Fader（`fader`） | 32 × 936 | 32 × 936 | 問題なし（#450 で修正済み） |
| Seek（`seek`） | 1920 × 24 | 1920 × 24 | 問題なし（#450 で修正済み） |
| Knob（`knob`） | 32 × 804 | 32 × 798.5 | 問題なし（兄弟の ruler 高さ増により縮小） |
| Waveform（`wave`） | 1920 × **0** | 1920 × 120 | **0 サイズ → 修正** |
| Meter（`meter`） | 1920 × **0** | 1920 × 120 | **0 サイズ → 修正** |
| Spectrogram（`spectrogram`） | 1920 × **0** | 1920 × 120 | **0 サイズ → 修正** |
| TimeAxis（`ruler`） | 1920 × 18（宣言 24） | 1920 × 24 | **slot 寸法と不一致 → 修正** |
| NativeSlot（`dock.editor`） | 未計測（canvas の寸法規則は変更なし） | 1918 × 1041 | 問題なし |

Label / Table / Button / Toggle / Tabs は canvas を使わないため、監査対象外とする。

## 原因の位置

kind 別の原因は次のとおり。

- **Waveform / Meter / Spectrogram（0 サイズ）**: `canvas` は block 配下で高さ auto となり、内容を持たないため 0 px になる。外側 wrapper の `min_h(120)` は canvas に伝播しない。破られている不変条件は「paint canvas の bounds は slot の bounds と一致する」である。所有する層は `ruviform-ui-gpui`（driver の layout 責務）である。
- **Waveform の追加 wrapper**: `element_for` の Waveform 分岐だけが `div().flex_grow()` で包んでいた。Meter / Spectrogram / TimeAxis はこの包みを持たない。Panel の flex 子として、Waveform の slot 自体が `min_h` を持つべきである。
- **TimeAxis（slot 不一致）**: `h(24)` の wrapper は flex 子として縮小できる（min-height auto が内容の 0 に縮むため）。Panel が溢れると 18 px になり、宣言された slot 高さと一致しない。
- **Fader / Knob の `h_full`**: Panel の高さを占有し、兄弟を溢れさせる。これはレイアウトの設計判断であり、canvas の寸法規則とは別の問題である。本設計では変更しない（残課題）。

## サイズ決定の規則

| 要素 | 外側（slot） | paint canvas |
| --- | --- | --- |
| Seek（横） | 親の全幅 × 高さ 24 px | `size_full`（#450） |
| Fader / Knob（縦） | 幅 32 px × 親の全高 | `size_full`（#450） |
| Waveform / Meter / Spectrogram | `viz_slot`: flex column、`flex_grow`、`min_h(120)` | `size_full`（本 Issue） |
| TimeAxis | `relative`、`w_full`、`h(24)`、`flex_shrink_0` | `size_full`（本 Issue で縮小禁止を追加） |
| NativeSlot | `flex_grow`（slot 要素） | `size_full`（変更なし） |

規則: canvas の寸法は slot に一致させ、寸法の源泉は slot（外側 wrapper）のみとする。canvas 自身は寸法を決めない。slot は、固定高さ（TimeAxis）か、最小高さを持つ flex column（viz slot）のいずれかで、自身の高さを確定させる。

## 是正の形

- `RvfView::record_bounds(key)` を追加し、全 paint canvas の prepaint hook で bounds を記録する。control_element の既存の記録を置き換える。pointer 写像と監査は同じ値を読む。
- `viz_slot(paint)` をモジュール関数として追加し、Waveform / Meter / Spectrogram の共通 slot とする。三つの同一の wrapper を一つの規則に集約する。
- Waveform の `element_for` 分岐から不要な wrapper を外し、Meter / Spectrogram と同じ形（`waveform_element` を直接返す）にする。
- TimeAxis の slot に `flex_shrink_0` を付け、宣言された 24 px を保つ。

## 採らなかった案

- 呼び出し側（`element_for`）で Waveform / Meter / Spectrogram に固定幅・固定高さを足す案。寸法の決定が二箇所に分散し、kind を追加するたびに同じ補正が増える。
- `h(120)` の固定高さに変更する案。現行の `flex_grow` による伸長を失い、Panel の高さに追従しなくなる。`min_h` を保ち、伸長は残す。
- `ruviform-ui` の `ViewSpec` にピクセル寸法を追加する案。契約は driver-neutral であり、寸法を持ち込むと境界が崩れる。必要性の根拠もない。
- Fader / Knob の `h_full` を変更する案。Panel 内の他要素の配置に影響する設計変更であり、本 Issue の範囲を超える。

## 残課題（本設計の範囲外）

- Fader / Knob の `h_full` による兄弟の押し出し（上記）。showcase では Fader の下でも Waveform は `min_h` で 120 px を保ち、Seek も窓下部に表示された（実窓撮影で確認）。ただし兄弟が Panel の残り高さを分け合うため、窓の高さによっては下方の要素が押し出される。レイアウト契約として別途判断する。
- NativeSlot の修正前の canvas bounds は計測していない。canvas の寸法規則には変更を加えていない。
- Synvelis 側の実機確認は consumer 環境が必要であり、本作業では実行していない（NotRun）。

## 検証計画

- 回帰テスト `rvf_view_canvas_bounds_nonzero_for_controls_and_waveform`（Fader / Seek / Waveform）と `rvf_view_canvas_bounds_nonzero_for_parity_kinds`（Knob / Meter / TimeAxis / Spectrogram、TimeAxis は 24 px も検証）を追加する。`rvf_view_native_slot_projects_layout` に NativeSlot の canvas bounds の非 0 検証を追加する。
- 既存の GPUI / parity / consumer-observation / driver-boundary / component-view / driver-contract の各 lane が通過すること。
- 実窓スクリーンショット（`component_showcase`）で、修正前後の Waveform の表示を比較する。
