---
id: "DESIGN-5mzN1XENwHq9qvfxBhzqdc"
type: "design"
status: "approved"
title: "Issue #450 — Seek/Fader canvas のサイズ決定契約"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/450"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-450-control-canvas-size.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-450-control-canvas-size.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-421-meter-timeaxis-gpui-parity.md"
    - "docs/designs/issue-387-component-view-specs.md"
---

# Issue #450 — Seek/Fader canvas のサイズ決定契約

## 概要

Seek/Fader の描画は `canvas` 要素と `paint_quad` で行われる。`canvas` は固有サイズを持たず、親の flex 行に幅を与えないと 0 px になる。本設計は、その canvas のサイズを誰が決めるかを明文化し、修正箇所を一つに絞る。

## 原因の位置

- 破られている不変条件: 「paint canvas の bounds は、その control の layout slot と一致する」。現状は canvas が slot を満たさず、幅 0 になる。
- 所有する層: サイズは driver 側の layout 責務である（`ruviform-ui-gpui`）。`ComponentSpec` / `ControlSpec` はピクセル寸法を持たず、契約側でサイズを指定する必要はない。
- 修正場所: `control_element` が作る canvas 1 箇所。Seek と Fader の両方がこの関数を通るため、呼び出し側（`element_for` の `Seek` / `Fader` 分岐）に補正を重複させない。

## サイズ決定の規則

| 要素 | 外側（slot） | paint canvas |
| --- | --- | --- |
| Seek（横） | 親の全幅 × 高さ 24 px（`control_element` の `h(24)`） | `size_full`（外側の幅・高さに一致） |
| Fader（縦） | 幅 32 px（`w(32)`） × 親の全高 | `size_full`（外側の幅・高さに一致） |

規則: control の paint canvas は `size_full()` で親の slot に一致させる。canvas 自身は寸法を決めず、寸法の源泉は slot（外側の wrapper）のみとする。これは既存の `native_slot_element` が採る方式と同じである。

## 採らなかった案

- 呼び出し側（`element_for`）の `Seek` / `Fader` 分岐に `flex_grow` や固定幅を足す案。サイズの決定が二箇所に分散し、将来の control 追加で同じ補正が増える。
- `ruviform-ui` の `ControlSpec` にピクセル寸法を追加する案。契約は driver-neutral であり、寸法を持ち込むと Fake driver と GPUI driver の境界が崩れる。現時点で必要性の根拠もない。

## 残課題（本設計の範囲外）

残りの canvas 描画は本件の対象外とし、計測していない。TimeAxis は `size_full` を付けた canvas を `w_full` / `h(24)` の wrapper に置いている。Waveform / Spectrogram / Meter は `size_full` を持たない canvas を使っている。実窓の確認（後述）では、修正後も Waveform の bin 波形は視認できず、baseline のみが描画された。この原因は特定していない。同じ規則（slot に一致する size）で揃えるか、各 control の slot 寸法を決めるかは、#421 / RF387 系の残 DoD として別途判断する。

## 検証計画

- 回帰テスト `rvf_view_seek_and_fader_canvas_bounds_nonzero`（gpui-test）: Seek と Fader を含む Panel をマウントし、`RvfView.bounds`（canvas prepaint が記録する bounds）の幅と高さが非 0 であることを検証する。修正前の計測は Seek が `0px × 24px`、Fader が `0px × 1056px`（いずれも幅 0）。
- 既存の GPUI / parity / consumer-observation / driver-boundary の各 lane が通過すること。
- 実機の `synvelis-native` 実窓スクリーンショットは、この環境で実行できた場合のみ証拠とする。
