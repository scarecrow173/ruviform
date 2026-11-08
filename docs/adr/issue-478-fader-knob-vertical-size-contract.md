---
id: "ADR-034UevIyecjzQ9iBQ9W2SG"
type: "adr"
status: "proposed"
title: "Issue #478 Fader/Knob の縦寸法契約 — grow+min_h slot"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/478"
    - "https://github.com/scarecrow173/ruviform/issues/453"
    - "https://github.com/scarecrow173/ruviform/issues/450"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-478-fader-knob-height-contract.md"
      - type: "doc"
        path: "docs/designs/issue-478-fader-knob-height-contract.md"
      - type: "doc"
        path: "docs/test-specs/issue-478-fader-knob-height-contract.md"
      - type: "doc"
        path: "docs/plans/implement-issue-478-fader-knob-height-contract.md"
      - type: "doc"
        path: "docs/tasks/implement-issue-478-fader-knob-height-contract.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../plans/implement-issue-478-fader-knob-height-contract.md"
  depends-on:
    - "../specs/issue-478-fader-knob-height-contract.md"
    - "../designs/issue-478-fader-knob-height-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/issue-478-fader-knob-height-contract.md"
    - "../designs/issue-478-fader-knob-height-contract.md"
    - "../designs/issue-453-canvas-size-audit.md"
    - "../designs/issue-450-control-canvas-size.md"
    - "driver-neutral-gui-gpui-driver-webview-first-issue-384.md"
  refines:
    - "../designs/issue-450-control-canvas-size.md"
  refined-by: []
  derives-from:
    - "../specs/issue-478-fader-knob-height-contract.md"
    - "../designs/issue-478-fader-knob-height-contract.md"
  derived-by:
    - "../test-specs/issue-478-fader-knob-height-contract.md"
    - "../plans/implement-issue-478-fader-knob-height-contract.md"
  verifies: []
  verified-by:
    - "../test-specs/issue-478-fader-knob-height-contract.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# Issue #478 Fader/Knob の縦寸法契約 — grow+min_h slot

## Context and Problem Statement

`ruviform-ui-gpui` の Panel は `flex_col` で子を縦に積む。#450/#453 で
「paint canvas の寸法は slot に一致させ、寸法の源泉は slot のみ」という
規則が確立されたが、Fader / Knob の slot 高さだけが未確定のまま残った。
現状は `element_for` の `Fader | Knob` 分岐が `div().flex().h_full()`、
`control_element` の非 Seek 経路が `w(px(32)).h_full()` を使い、
高さ 100% を flex 基底サイズとして占有する。

#453 の監査（`docs/designs/issue-453-canvas-size-audit.md`）で、この
`h_full` が兄弟要素を Panel の表示領域から押し出し得ることが確認され、
「レイアウト契約として別途判断する」残課題として本 Issue に引き継がれた。
縦方向コントロールが「残り高さをどう分け合うか」の契約がないため、
kind 追加や sibling 構成の変化のたびに同じ補正が必要になる。

## Decision Drivers

* 単独配置（Synvelis 型 `Panel > Tabs > Fader`）で従来どおり残り全高を
  使えること — channel strip の fader は高いほど操作しやすい。
* 兄弟と同居する場合に、兄弟の `min_h` / 固定高さを守り、押し出しを
  起こさないこと。
* viz slot（`flex_grow` + `min_h(px(120))`）・TimeAxis
  （`h(px(24))` + `flex_shrink_0`）・NativeSlot（`flex_grow`）で
  確立済みの縦寸法規則との一貫性。
* `ViewSpec` / `ControlSpec` は driver-neutral でピクセル寸法を持たない
  という境界を維持すること（#450/#453 の確立事項）。
* Synvelis / AudioLens consumer の `ViewSpec` 記述を変更せずに済むこと。

## Considered Options

* Option A: 現状維持（`h_full` を flex share として意図的に扱う）。
* Option B: 固定高さ（例: `h(px(120))` + `flex_shrink_0`）。
* Option C: viz_slot 型の `min_h` + `flex_grow`（slot が残り高さを分け合い、
  下限を保つ）。
* Option D: `ViewSpec` / `ControlSpec` に寸法契約を追加する。

## Decision Outcome

Chosen option: "Option C", because 縦に伸縮可能な slot の既存規則
（viz_slot・NativeSlot）と同じ「`flex_grow` で残り高さを分け合い、
`min_h` で下限を保つ」契約に揃えるのが、単独配置での全高利用と
兄弟同居での押し出し防止を両立する最小の整合的変更である。`h_full`
は「残りを取る」ではなく「全高を基底に占有する」意味であり、
flex-shrink への暗黙依存を明示契約に置き換える。

具体的な契約: Fader / Knob の Panel 子 slot は `flex_grow` +
`min_h(px(120))`（viz_slot と同じ床。値の一本化は実装 dispatch で
確定）とし、子の control / paint canvas は `size_full` で slot を満たす。
幅 32 px（`w(px(32))`）など横方向の規則は変更しない。

### Consequences

* Good, because: 単独配置では従来どおり残り全高まで伸び、channel
  strip の視覚結果を維持する。兄弟同居では grow 分配になり、viz
  slot・Seek・TimeAxis の最小/固定高さが守られ、押し出しが解消する。
  Tabs body 内の Fader でも「header を差し引いた残り」を明示的に取る
  ため、shrink 依存の暗黙挙動が契約になる。
* Bad, because: Fader の実高さが兄弟構成で変わり得るため、
  `component_showcase` 等の見た目が変化し得る（bounds テストと
  実窓確認で担保）。複数の grow 兄弟とは残り高さを均等に分け合うため、
  特定の兄弟を優遇する重み付けは表現できない（必要になれば別契約）。

### Confirmation

- gpui-test の bounds 計測で、Fader/Knob canvas が slot どおりの
  非 0 寸法を持ち、兄弟同居時に押し出しが起きないことを検証する
  （`docs/test-specs/issue-478-fader-knob-height-contract.md`）。
- `mise run test-ui-gpui` 等の既存 lane が成功条件を弱めずに通過する
  こと。

## Implementation Plan

* Affected paths: `crates/ruviform-ui-gpui/src/render.rs` のみ
  （`element_for` の `Fader | Knob` 分岐と `control_element` の
  非 Seek 経路）。`ruviform-ui`・`ruviform-ui-test`・consumer 側・
  `Cargo.lock` は変更しない。
* Patterns to follow: `viz_slot`（`flex_col` + `flex_grow` +
  `min_h(px(120))` + canvas `size_full`）と #453 の「寸法の源泉は
  slot のみ」規則。`min_h` の値は viz_slot と一本化することを推奨
  （別値にする場合は根拠を PR に記録）。
* Patterns to avoid: `h_full` / `size_full` による Panel 子の全高占有、
  `ControlSpec` へのピクセル寸法追加、呼び出し側ごとの個別補正。
* Dependencies or configuration: なし（依存追加なし）。
* 本 dispatch は設計判断のみ。実装と回帰テストは後続 dispatch が
  `docs/plans/implement-issue-478-fader-knob-height-contract.md` に
  従って行う。

## Verification

* [ ] （後続実装 dispatch）`render.rs` の Fader/Knob slot が
  `flex_grow` + `min_h` になり、control/canvas が `size_full` で
  slot を満たす。
* [ ] （後続実装 dispatch）test-spec の各 Rule に対応する gpui-test
  が追加され、修正前後で赤緑を確認する。
* [ ] （後続実装 dispatch）`mise run test-ui-gpui` /
  `test-ui-parity` / `test-ui-consumer-observation` /
  `test-ui-driver-contract` / `test-ui-component-view` /
  `check-ui-driver-boundary` が通過する。
* [ ] （後続実装 dispatch）`component_showcase` 実窓で Fader の表示を
  確認する。
* [x] （本 dispatch）docs のみの変更で `mise run fmt-check` /
  `mise run check` / `mise run test-ui-gpui` が pass（非破壊確認）。

## Pros and Cons of the Options

### Option A — 現状維持（`h_full`）

* Good, because: 変更不要。単独配置では全高を使えており、見た目の
  回帰がない。
* Bad, because: 「全高を基底に占有」は「残りを分け合う」意味ではなく、
  兄弟同居時の振る舞いが flex-shrink への暗黙依存になる。低い窓で
  下方要素が押し出される既知の欠陥を温存し、将来の sibling 追加で
  fader が使えない細片まで縮み得る。

### Option B — 固定高さ

* Good, because: 最も単純で予測可能。TimeAxis の `h(px(24))` と同型。
* Bad, because: 高い窓で fader が低いまま固定され、channel strip と
  しての使い勝手を損なう。固定高の合計が Panel 高を超える低い窓では
  同じ押し出しが残る。「全高を使いたい」需要に応えられない。

### Option C — viz_slot 型 `min_h` + `flex_grow`（採用）

* Good, because: viz slot・NativeSlot と同じ「grow で残りを分け合い、
  min で床を保つ」契約に揃う。単独配置で全高を維持しつつ、兄弟同居で
  押し出しを解消する。`ViewSpec` を変更しない。
* Bad, because: grow 兄弟間の重み付けはできず均等分配になる。
  `min_h` の具体値は viz slot との一本化を要検討。

### Option D — `ViewSpec` への寸法契約追加

* Good, because: consumer が寸法意図を明示できる。
* Bad, because: 契約は driver-neutral であり、ピクセル/レイアウト寸法を
  持ち込むと Fake/GPUI/consumer の境界が崩れる（#450/#453 で同趣旨の
  案を棄却済み）。Seek=24・TimeAxis=24・viz=120 など既存の全 kind の
  寸法が renderer 側規則であり、1 kind の問題のために契約層を変える
  必要性の根拠がない。

## More Information

- 設計詳細: [design](../designs/issue-478-fader-knob-height-contract.md)
- 検証契約: [test-spec](../test-specs/issue-478-fader-knob-height-contract.md)
- 実装計画: [plan](../plans/implement-issue-478-fader-knob-height-contract.md)
- 先行契約: [#450 canvas サイズ規則](../designs/issue-450-control-canvas-size.md)、
  [#453 canvas 寸法監査](../designs/issue-453-canvas-size-audit.md)
- driver-neutral 境界: [ADR #384](driver-neutral-gui-gpui-driver-webview-first-issue-384.md)
- 対象コード（読取のみ）: `crates/ruviform-ui-gpui/src/render.rs`
  （`viz_slot`・`control_element`・`element_for` の各分岐）
- 関連 Issue: #450, #453, #421, #387
