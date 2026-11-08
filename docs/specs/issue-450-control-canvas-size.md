---
id: "SPEC-LOaygozld2Yp16xXhm1Yx5"
type: "spec"
status: "approved"
title: "Issue #450 — Seek/Fader の canvas が実窓で 0 サイズになり描画されない問題の是正"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/450"
    - "https://github.com/scarecrow173/ruviform/issues/421"
    - "https://github.com/scarecrow173/ruviform/issues/387"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-450-control-canvas-size.md"
      - type: "file"
        path: "docs/test-specs/issue-450-control-canvas-size.md"
      - type: "file"
        path: "docs/plans/issue-450-control-canvas-size.md"
      - type: "file"
        path: "docs/tasks/issue-450-control-canvas-size.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
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
  refines:
    - "docs/specs/issue-387-component-view-specs.md"
  related:
    - "docs/specs/issue-421-meter-timeaxis-gpui-parity.md"
    - "docs/designs/issue-421-meter-timeaxis-gpui-parity.md"
---

# Issue #450 — Seek/Fader の canvas が実窓で 0 サイズになり描画されない問題の是正

## 背景

Synvelis PR#183（Issue #175）の実機受入で、`synvelis-native` の実 GPUI 窓において Seek が描画されないことが観測された（Transport タブに track/fill/thumb が出ず、左端の細い縦線のみ）。headless（FakeDriver）の projection→display 経路は Pass しており、consumer 側の mount/投影は原因ではない。

## 観測された原因（計測済み）

`crates/ruviform-ui-gpui/src/render.rs` の `control_element` は Seek/Fader の描画を `canvas` と `paint_quad` で行う。`canvas` は固有サイズを持たないため、明示的な幅を与えないと親の flex 行の中で幅 0 になる。GPUI の描画用 bounds を `RvfView.bounds` で計測した結果、修正前の Seek canvas は `0px × 24px`（高さのみ親の 24px に stretch し、幅は 0）、Fader canvas は `0px × 1056px`（幅 0）であった。両者とも幅 0 で、track / fill / thumb は描画されない。

## 要求

1. Seek/Fader の paint canvas は、親レイアウト内で非 0 の幅と高さを持ち、実窓で track / fill / thumb が描画される。
2. canvas サイズ決定の責務と規則を明文化する。RF387 系の残 DoD（#421 は部分達成で Close 済み）の追跡先とする。
3. GPUI 側の回帰テストで、Seek/Fader の canvas bounds が非 0 であることを検証する。

## 受入条件

- Seek/Fader の canvas bounds が、GPUI の gpui-test 上で幅と高さの両方とも非 0 である（回帰テストで検証）。
- 修正は `ruviform-ui-gpui`（driver の layout 責務）で行い、Synvelis 側に workaround を入れない。
- `ruviform-ui` の公開契約（`ComponentSpec` / `ControlSpec`）は変更しない。サイズは driver 側の layout 責務であり、契約にピクセル寸法を持ち込まない。
- `synvelis-native` 実窓で Seek が視認できること（スクリーンショット証拠）。この環境で実機検証が不可能な場合は NotRun と明記する。
- 既存の GPUI/parity/consumer-observation/driver-boundary の検証が、成功条件を弱めずに通過すること。

## スコープ外

- Waveform / Meter / TimeAxis / Spectrogram の canvas サイズ規則。同じ構造の canvas を使うが、本 Issue の受入範囲は Seek/Fader に限る。計測していないため、残課題として別途追跡する（設計書 参照）。
- Synvelis 側の修正（本 Issue は upstream 側の修正である）。
- ライセンス・公開 API の変更。

## 検証の区別

- 実行して成功したものだけを Passed と記録する。
- 実機の実窓スクリーンショットは、実行できた場合のみ証拠として扱う。実行できなかった場合は NotRun とする。
