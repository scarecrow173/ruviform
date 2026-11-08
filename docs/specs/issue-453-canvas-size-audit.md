---
id: "SPEC-7snPYyix2JlpoyXvP0zKwf"
type: "spec"
status: "approved"
title: "Issue #453 — 残 canvas 要素（Waveform/Meter/Spectrogram/TimeAxis/NativeSlot）のサイズ規則監査と是正"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/453"
    - "https://github.com/scarecrow173/ruviform/issues/450"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-453-canvas-size-audit.md"
      - type: "file"
        path: "docs/test-specs/issue-453-canvas-size-audit.md"
      - type: "file"
        path: "docs/plans/issue-453-canvas-size-audit.md"
      - type: "file"
        path: "docs/tasks/issue-453-canvas-size-audit.md"
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
    - "docs/specs/issue-450-control-canvas-size.md"
  blocks: []
  supersedes: []
  superseded-by: []
  refines:
    - "docs/specs/issue-450-control-canvas-size.md"
  related:
    - "docs/designs/issue-450-control-canvas-size.md"
    - "docs/specs/issue-421-meter-timeaxis-gpui-parity.md"
---

# Issue #453 — 残 canvas 要素のサイズ規則監査と是正

## 背景

#450 で Seek/Fader の paint canvas に `size_full()` を与え、実窓で 0 サイズになる問題を是正した。`render.rs` には他にも `canvas(` を使う要素（Waveform・Meter・Spectrogram・TimeAxis・NativeSlot projection）があり、同じ「canvas は固有サイズを持たない」問題を抱えている可能性があった。#450 の設計書は、これらを未計測の残課題として分離していた。

## 観測（出発点）

- #450 の検証で、component_showcase の実窓では Waveform bins が表示されなかった。原因は未特定だった。
- 監査前の GPUI bounds 計測（gpui-test、Panel 内・既定テストウィンドウ）では、Waveform / Meter / Spectrogram の canvas が幅 1920 px、高さ 0 px だった。TimeAxis（ruler）は高さ 18 px で、宣言された slot 高さ 24 px と一致しなかった。
- 0 サイズの原因は「幅」ではなく「高さ」だった。これらの canvas は block 配下にあり、高さ auto で内容を持たないため 0 px になる。外側 wrapper の `min_h(120)` は canvas に伝播しない。

## 要求

1. 全 paint canvas kind について、GPUI bounds 計測（実窓相当の gpui-test）で非 0 の幅と高さを確認する監査を行う。
2. 監査で 0 サイズ、または宣言された slot 寸法と一致しない kind は、#450 と同じ規則（canvas の寸法は slot に一致させ、寸法の源泉は slot のみとする）で是正する。
3. #450 の `rvf_view_seek_and_fader_canvas_bounds_nonzero` と同型の bounds 検証を他 kind にも展開する。
4. 監査で「問題なし」の kind も実測値を PR に記録する。

## 受入条件

- 全 paint canvas kind の canvas bounds 実測値を、修正前と修正後の両方について一覧にする（design と PR 本文に記録）。
- 0 サイズまたは slot 寸法と不一致の kind は修正され、回帰テストで非 0 かつ宣言寸法どおりであることを検証する。
- 実窓スクリーンショット（`component_showcase`）で、修正前は Waveform bins が表示されず、修正後は表示されることを確認する。
- 既存の GPUI / parity / consumer-observation / driver-boundary / component-view / driver-contract の各検証が、成功条件を弱めずに通過する。
- `ruviform-ui`（公開契約）と consumer 側のコードは変更しない。

## スコープ外

- Label / Table / Button / Toggle / Tabs は canvas を使わない（div のみ）ため、本 Issue の監査対象外とする。
- Fader / Knob の `h_full` が Panel の高さを占有し、兄弟要素を押し出す挙動（レイアウトの設計判断）。canvas の規則とは別の問題として残課題に記録する。
- Synvelis 側の実機スクリーンショット（consumer 環境が必要）。
- ライセンス・公開 API の変更。

## 検証の区別

- Passed: 実際に実行し、成功したもののみ。
- NotRun: 実行していないもの。理由を明記する。
- Unsupported: この環境で原理的に実行できないもの。理由を明記する。
