---
id: "TASK-034UexWbrpLoijqKctQjFB"
type: "task"
status: "in-progress"
title: "Issue #478 Fader/Knob 縦寸法契約 — 設計判断と実装タスク"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/478"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-478-fader-knob-height-contract.md"
      - type: "doc"
        path: "docs/designs/issue-478-fader-knob-height-contract.md"
      - type: "doc"
        path: "docs/adr/issue-478-fader-knob-vertical-size-contract.md"
      - type: "doc"
        path: "docs/test-specs/issue-478-fader-knob-height-contract.md"
      - type: "doc"
        path: "docs/plans/implement-issue-478-fader-knob-height-contract.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-issue-478-fader-knob-height-contract.md"
  implemented-by: []
  depends-on:
    - "../plans/implement-issue-478-fader-knob-height-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/issue-478-fader-knob-height-contract.md"
    - "../designs/issue-478-fader-knob-height-contract.md"
    - "../adr/issue-478-fader-knob-vertical-size-contract.md"
    - "../test-specs/issue-478-fader-knob-height-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-issue-478-fader-knob-height-contract.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/issue-478-fader-knob-height-contract.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #478 Fader/Knob 縦寸法契約 — 設計判断と実装タスク

関連文書: [plan](implement-issue-478-fader-knob-height-contract.md) /
[spec](../specs/issue-478-fader-knob-height-contract.md) /
[design](../designs/issue-478-fader-knob-height-contract.md) /
[ADR](../adr/issue-478-fader-knob-vertical-size-contract.md) /
[test-spec](../test-specs/issue-478-fader-knob-height-contract.md)

## Work

Fader / Knob の Panel 子としての縦寸法契約を決定し、docs チェーンに
記録する。採用契約（ADR Option C: `flex_grow` + `min_h` slot）の
実装と回帰テストは後続の実装 dispatch が担う。

## Classification

- `normal-plan-task`

## Upstream Documents

- [plan](implement-issue-478-fader-knob-height-contract.md)
- [spec](../specs/issue-478-fader-knob-height-contract.md)
- [design](../designs/issue-478-fader-knob-height-contract.md)
- [ADR](../adr/issue-478-fader-knob-vertical-size-contract.md)
- [test-spec](../test-specs/issue-478-fader-knob-height-contract.md)

## Dependencies

- #453（PR #459）の canvas 寸法監査が main 統合済みであること
  （本 worktree は `25648a06` 起点）。
- 実装側タスクは ADR の accepted 化を待たずに着手可能だが、
  coordinator の dispatch 判断に従う。

## Affected Files

- 設計判断 dispatch: `docs/{specs,designs,adr,test-specs,plans,tasks}/`
  の本チェーン新設 + 生成 index の再生成のみ。`crates/` 以下は
  変更しない。
- 後続実装 dispatch: `crates/ruviform-ui-gpui/src/render.rs` のみ。

## Tasks

設計判断 dispatch（本 dispatch）:

- [x] T1: spec / design / ADR / test-spec / plan / task の文書
  チェーンを作成し、relations を揃える。
- [x] T2: `docs/designs/issue-453-canvas-size-audit.md`・
  `issue-450-control-canvas-size.md`・`render.rs` の対象箇所を読み、
  選択肢（現状維持 / 固定高さ / viz_slot 型 min_h+grow /
  ViewSpec 契約追加）を評価する。
- [x] T3: review-protocol のレビューを実施し、blocker が残らない
  ことを確認する（2 サイクル。順序づけの指摘を修正済み）。
- [x] T4: `doc_maintenance.js` で managed index を再生成し、
  doc audit に新規 broken relation がないことを確認する
  （merge 後に再確認する）。
- [x] T5: `git fetch origin main && git merge origin/main`（`7ae916b3`
  取り込み済み）で生成 index を再生成した後に `mise run fmt-check` /
  `mise run check` / `mise run test-ui-gpui` を実行し、全て pass。
- [ ] T6: 論理単位でコミットし、`docs/478-fader-height-contract` を
  push して PR を作る（`Refs #478`）。

後続実装 dispatch（未着手）:

- [ ] T7: `element_for` の `Fader | Knob` 分岐と `control_element`
  非 Seek 経路を grow + `min_h` slot 契約に改める。
- [ ] T8: test-spec の Rule/Example に対応する gpui-test を追加し、
  赤→緑を確認する。
- [ ] T9: 既存 UI lane（`test-ui-gpui` 等）と `component_showcase`
  実窓確認を行う。

## Done When

- [ ] 縦寸法契約の設計判断が ADR（不採用判断を含む）に記録され、
  PR が作成されている。
- [ ] 実装判断の場合の「修正 + 回帰テスト」は後続 dispatch に
  引き継がれる旨が PR 本文・plan・task に明記されている。
- [x] `mise run fmt-check` / `mise run check` が通過している。

## Verification

- [x] `mise run fmt-check`（docs のみの変更での非破壊確認・pass）
- [x] `mise run check`（同上・pass）
- [x] `mise run test-ui-gpui`（A3 非破壊確認・85 tests pass）
- [ ] （後続実装 dispatch）`mise run test-ui-gpui` 等の該当 lane
