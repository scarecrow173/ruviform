---
id: "SPEC-034UetYOiYEThOr1lvUH8t"
type: "spec"
status: "implemented"
title: "Issue #478 Fader/Knob 縦寸法契約 — h_full 兄弟押し出しのレイアウト判断"
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
        path: "docs/designs/issue-478-fader-knob-height-contract.md"
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
  implements: []
  implemented-by:
    - "../designs/issue-478-fader-knob-height-contract.md"
    - "../plans/implement-issue-478-fader-knob-height-contract.md"
  depends-on:
    - "../designs/issue-453-canvas-size-audit.md"
    - "../designs/issue-450-control-canvas-size.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/issue-421-meter-timeaxis-gpui-parity.md"
    - "../adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md"
  refines:
    - "../specs/issue-450-control-canvas-size.md"
  refined-by: []
  derives-from: []
  derived-by:
    - "../designs/issue-478-fader-knob-height-contract.md"
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

## 背景

#453（PR #459）の canvas 寸法監査で、Panel 内の Fader / Knob が
`h_full`（`element_for` の `Fader | Knob` 分岐
`div().flex().h_full()` と control 側 `w(px(32)).h_full()`）
で高さを占有し、Panel の兄弟要素を溢れさせる挙動が確認された。
`docs/designs/issue-453-canvas-size-audit.md` はこれを「canvas の
寸法規則とは別の問題」「本 Issue の範囲を超えるレイアウト契約判断」
として明示的に残課題化しており、本 Issue がその判断を担う。

現状の問題は、`h_full`（高さ 100%）が flex column 内で「残り高さを
分け合う」ではなく「親の全高を基底サイズとして占有する」意味を持ち、
flex-shrink への依存で結果的に窓の高さ次第で下方の要素が押し出される
（またはコントロールが使えない細片まで縮む）点にある。縦方向
コントロールの高さ契約が未定義のままでは、kind 追加や sibling 構成の
変化のたびに同じ補正が必要になる。

## 目的

- Fader / Knob の Panel 内での縦寸法契約を、ADR または design 文書に
  記録する（不採用判断を含む）。
- 採用した契約が実装を要する場合は、実装と回帰テストを後続 dispatch
  へ引き継ぐ形で明記する（本 dispatch は設計判断のみ）。
- 既存の viz slot（`flex_grow` + `min_h(120)`）・TimeAxis
  （固定高さ + `flex_shrink_0`）の規則との一貫性、および Synvelis /
  AudioLens consumer への影響を判断材料として記録する。

## 受入条件

| # | 条件 | 結果の記録先 |
| --- | --- | --- |
| A1 | 縦寸法契約の設計判断が ADR または design 文書に記録される（不採用判断を含む） | `docs/adr/issue-478-fader-knob-vertical-size-contract.md` / `docs/designs/issue-478-fader-knob-height-contract.md` |
| A2 | （実装判断の場合）採用契約に対応する修正 + 回帰テストは後続の実装 dispatch が担う旨が、PR 本文と Issue への提案として明記される | PR 本文・`docs/plans/implement-issue-478-fader-knob-height-contract.md` |
| A3 | `mise run test-ui-gpui` 等の該当 lane が pass — コード変更なしのため既存 pass の非破壊確認のみ | PR 本文の検証欄 |

## 非目標

- `crates/` 以下のコード変更（`render.rs` を含む）。本 dispatch は
  設計判断のみであり、契約の実装は後続 dispatch で行う。
- canvas の 0 サイズ描画自体（#453 / PR #459 で解決済み）。
- `ruviform-ui` の `ViewSpec` / `ControlSpec` への寸法契約追加の
  実装（選択肢としては検討するが、driver-neutral 境界を崩すため
  非推奨 — 判断は ADR に記録）。
- Panel の overflow / scroll 方針、Tabs body の `size_full`、
  orientation に基づく control sizing など、本 Issue の直接原因でない
  隣接論点（design 文書の残課題に記録）。
- Synvelis / AudioLens consumer repo の編集（読み取り・影響記録のみ）。

## 制約

- 編集範囲: `docs/adr/`・`docs/designs/`・`docs/specs/`・`docs/plans/`・
  `docs/tasks/`・`docs/test-specs/` 配下の本 Issue 用文書チェーン
  （新規作成）と生成 index の再生成のみ。
- 検証は `mise.toml` タスクが正本。docs のみの変更のため
  `mise run fmt-check` / `mise run check` を実行し、コード lane は
  変更がないことの非破壊確認に留める。

## Sources and Relations

- Issue #478（本 Issue）、#453（canvas 寸法監査・残課題の発生源）、
  #450（control canvas サイズ規則の先行契約）。
- `docs/designs/issue-453-canvas-size-audit.md`（残課題の明記箇所）、
  `docs/designs/issue-450-control-canvas-size.md`（slot 規則の先行例）。
- 対象コード: `crates/ruviform-ui-gpui/src/render.rs`（読取のみ、
  変更しない）。
