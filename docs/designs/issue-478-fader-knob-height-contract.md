---
id: "DESIGN-034UeuINTqCil2VtMHJxGx"
type: "design"
status: "approved"
title: "Issue #478 Fader/Knob 縦寸法契約 — h_full 兄弟押し出しのレイアウト判断"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/478"
    - "https://github.com/scarecrow173/ruviform/issues/453"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-478-fader-knob-height-contract.md"
      - type: "doc"
        path: "docs/adr/issue-478-fader-knob-vertical-size-contract.md"
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
  implements:
    - "../specs/issue-478-fader-knob-height-contract.md"
  implemented-by: []
  depends-on:
    - "../designs/issue-453-canvas-size-audit.md"
    - "../designs/issue-450-control-canvas-size.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/issue-421-meter-timeaxis-gpui-parity.md"
    - "../adr/issue-478-fader-knob-vertical-size-contract.md"
    - "../adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md"
  refines:
    - "../designs/issue-450-control-canvas-size.md"
    - "../designs/issue-453-canvas-size-audit.md"
  refined-by: []
  derives-from:
    - "../specs/issue-478-fader-knob-height-contract.md"
  derived-by:
    - "../adr/issue-478-fader-knob-vertical-size-contract.md"
    - "../test-specs/issue-478-fader-knob-height-contract.md"
    - "../plans/implement-issue-478-fader-knob-height-contract.md"
  verifies: []
  verified-by: []
  references: []
  defers:
    - "../plans/implement-issue-478-fader-knob-height-contract.md"
  deferred-by: []
---

# Issue #478 Fader/Knob 縦寸法契約 — h_full 兄弟押し出しのレイアウト判断

[spec](../specs/issue-478-fader-knob-height-contract.md) が確定した範囲の
設計判断。決定の正本は
[ADR: Fader/Knob の縦寸法契約](../adr/issue-478-fader-knob-vertical-size-contract.md)。
本 dispatch は設計判断のみであり、コード変更は行わない。

## Context

### 現状の構造（`crates/ruviform-ui-gpui/src/render.rs`、読取のみ）

Panel は `size_full().flex().flex_col()` であり、子要素は縦に積まれる
（`element_for` の `Panel` 分岐）。kind ごとの縦寸法の決め方は現在
ばらばらである。

| kind | slot の縦寸法（Panel 子として） | 根拠 |
| --- | --- | --- |
| Fader / Knob | `div().flex().h_full()`（高さ 100% を基底に占有）+ control 側 `w(px(32)).h_full()` | `element_for` `Fader \| Knob` 分岐・`control_element`（`is_seek == false` 経路） |
| Seek | 実効 24 px（`h(px(24))`） | `control_element`（`is_seek == true` 経路） |
| Toggle | 固定 32 × 24 px | `toggle_element` |
| Waveform / Meter / Spectrogram | `viz_slot`: `flex_col` + `flex_grow` + `min_h(px(120))`、canvas は `size_full` | `viz_slot` |
| TimeAxis | `relative` + `w_full` + `h(px(24))` + `flex_shrink_0` | `time_axis_element` |
| NativeSlot | `flex_grow`（+ 任意 `min_w`/`min_h`） | `native_slot_element` |
| Tabs body | `size_full` + `flex_col`（header + active child） | `element_for` `Tabs` 分岐 |
| Label / Table / Button | 内容サイズ | 各 `*_element` |

`h_full` は「残り高さを分け合う」意味ではなく「親の全高を基底サイズ
として占有する」意味を持つ。flex column 内で複数の子がこれを持つと
合計が親高を超え、flex-shrink が結果を決める。#453 の監査では、
Fader がほぼ全高（936 / 960 px）を占め、兄弟（Seek 24 px・
Waveform `min_h` 120 px）が残りを分け合う形になり、窓が低い場合は
下方の要素が Panel の表示領域から押し出されることが確認された
（`issue-453-canvas-size-audit.md` 監査結果・残課題）。

### 破られている不変条件と所有層

- 不変条件: 「Panel 子要素の縦寸法は、slot が確定し canvas/control は
  slot を満たす」という #450/#453 で確立した規則に対し、Fader/Knob
  だけ slot の高さが `h_full` という未確定の占有指定になっており、
  兄弟との共存ルールが存在しない。
- 所有層: `ruviform-ui-gpui` の renderer layout 責務。`ViewSpec` /
  `ControlSpec` はピクセル寸法を持たず（#450 で確認済み）、寸法契約は
  driver 側の規則として置くのが既存の境界である。

## Scope

### In Scope

- Fader / Knob の Panel 子としての縦寸法契約の決定と記録
  （ADR + 本設計）。
- 採用契約の render.rs 上の具体形と、後続実装 dispatch への引き継ぎ
  （file map・回帰テスト方針・検証計画）。
- 既存規則（viz_slot・TimeAxis・NativeSlot）との一貫性と consumer
  影響の評価。

### Out of Scope

- コード変更（本 dispatch は設計判断のみ）。
- Panel 自体の overflow / scroll 方針。
- Tabs body の `size_full`（同型の占有パターン。残課題に記録）。
- orientation（Horizontal/Vertical）に基づく control sizing の分岐
  （残課題に記録）。
- `ruviform-ui` 契約層（`ViewSpec`/`ControlSpec`）の変更。

## Components and Boundaries

- 決定の正本: `docs/adr/issue-478-fader-knob-vertical-size-contract.md`
  （採用案と不採用案の記録）。
- 実装境界: `crates/ruviform-ui-gpui/src/render.rs` のみ
  （`element_for` の `Fader | Knob` 分岐と `control_element` の
  非 Seek 経路）。`ruviform-ui`・`ruviform-ui-test`・consumer 側は
  変更しない。
- 検証境界: gpui-test の bounds 計測（`RvfView.bounds`）と既存 lane
  （`test-ui-gpui` 等）の非破壊確認。

## Data and Control Flow

- ViewSpec → `ViewHost` → `element_for` → kind 別 element → taffy
  layout → prepaint hook が `RvfView.bounds` に canvas bounds を記録
  → pointer 写像と監査が同じ値を読む、という既存経路は変更しない。
  変わるのは Fader/Knob の slot が要求する縦寸法のみである。

## 決定の要約（詳細は ADR）

Fader / Knob の slot を **viz_slot と同じ規則 — `flex_grow` +
`min_h(px(120))`、子の control/canvas は `size_full` で slot を満たす**
に改める。すなわち「縦に伸縮可能な slot は、残り高さを flex-grow で
分け合い、下限 `min_h` を保つ」という契約を、viz kind（Waveform /
Meter / Spectrogram）・NativeSlot（`flex_grow`）に続く縦方向
コントロールの規則として明示する。

- 単独配置（Synvelis 型 `Panel > Tabs > Fader`）では従来どおり残り
  全高まで伸び、視覚結果は維持される。
- 兄弟と同居する場合は `h_full` の全高占有ではなく grow 分配になり、
  兄弟の `min_h` / 固定高さが守られ、押し出しが解消される。
- Tabs body 内の Fader でも、header を差し引いた残り高さを grow で
  取るため、現行の「body 全高を基底に shrink で帳尻を合わせる」
  暗黙挙動が明示契約になる。

## Non-Functional Constraints

- `unsafe` 追加なし、公開 API（`ruviform-ui`）変更なし、consumer
  コード変更なし。
- 既存の `min_h(120)` を viz_slot と共有するか、control 用に別値を
  置くかは実装 dispatch で確定する（本設計は「viz slot と同じ床」を
  契約とし、値の一本化を推奨）。

## Risks and Trade-offs

- **複数の grow 兄弟での分配**: Fader 複数 + viz slot が同居すると
  残り高さを均等に分け合う。現行の「全高占有→shrink」より予測可能で
  あり、mixer 的な均等配分として妥当と判断。
- **極端に低い Panel**: 兄弟の `min_h` 合計が Panel 高を超える場合は
  依然として溢れる。これは Panel に scroll がないことに起因する
  別問題であり、本契約の範囲外（残課題）。
- **回帰の見え方**: `component_showcase` 等で Fader の実高さが変わり
  得る（兄弟構成依存）。bounds テストと実窓確認で担保する。
- **Knob の将来描画**: 現行は縦バー描画で Fader と同一経路。将来
  rotary 描画が入っても slot 契約（grow + min_h）は維持できる。

## Deferred Design Concerns

| 懸念 | 理由 | 再開条件 | 無視した場合のリスク |
| --- | --- | --- | --- |
| Tabs body の `size_full`（同型の全高占有） | 本 Issue の対象 kind 外。Panel 子としての占有規則の一般化は別判断 | Panel 子の縦寸法契約を kind 横断で棚卸しする Issue | Tabs + 兄弟構成で同じ押し出しが残る |
| orientation に基づく control sizing（`is_seek` が kind を orientation と混同） | 水平 Fader/Knob の需要が現行 consumer にない。別の潜在不整合 | Horizontal orientation の control を使う consumer/spec が出た時 | 水平 control が縦 slot で描画される |
| Panel の overflow / scroll 方針 | レイアウト契約の別層。min 合計超過時の振る舞いを決める必要がある | 実窓で押し出しが実害になった報告 | 低い窓で下方要素が見えないまま |

## Verification Notes

- [ ] 後続実装 dispatch: `render.rs` の `Fader | Knob` 分岐と
  `control_element` 非 Seek 経路を grow + min_h slot に改め、
  gpui-test で縦寸法契約（test-spec 参照）を回帰検証する。
- [ ] 本 dispatch: `mise run fmt-check` / `mise run check` が
  docs のみの変更で通過すること。
- [ ] `component_showcase` 実窓で Fader の表示が従来どおり
  （単独/tab 内で全高）であることを実装 dispatch が確認する。
