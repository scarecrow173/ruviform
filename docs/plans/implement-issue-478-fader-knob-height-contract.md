---
id: "PLAN-034UexVzkirHD8NES7sXZQ"
type: "plan"
status: "in-progress"
title: "Issue #478 Fader/Knob 縦寸法契約 — 設計判断と実装引き継ぎ計画"
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
        path: "docs/tasks/implement-issue-478-fader-knob-height-contract.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/issue-478-fader-knob-height-contract.md"
    - "../designs/issue-478-fader-knob-height-contract.md"
    - "../adr/issue-478-fader-knob-vertical-size-contract.md"
  implemented-by: []
  depends-on:
    - "../designs/issue-453-canvas-size-audit.md"
    - "../designs/issue-450-control-canvas-size.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../tasks/implement-issue-478-fader-knob-height-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/issue-478-fader-knob-height-contract.md"
    - "../designs/issue-478-fader-knob-height-contract.md"
    - "../adr/issue-478-fader-knob-vertical-size-contract.md"
  derived-by:
    - "../tasks/implement-issue-478-fader-knob-height-contract.md"
  verifies: []
  verified-by:
    - "../test-specs/issue-478-fader-knob-height-contract.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #478 Fader/Knob 縦寸法契約 — 設計判断と実装引き継ぎ計画

## Goal

Fader / Knob の Panel 子としての縦寸法契約を ADR で確定し
（grow + `min_h` slot）、採用契約の実装と回帰テストを後続の
実装 dispatch へ引き継ぐ。

## Scope

本 plan は 2 つの dispatch にまたがる。**本 dispatch（設計判断）**
の成果物は docs チェーンのみで、`crates/` 以下は変更しない。
**後続 dispatch（実装）** は ADR の Implementation Plan と
[test-spec](../test-specs/issue-478-fader-knob-height-contract.md)
に従い `render.rs` を変更し、回帰テストを追加する。
Issue #478 の受入条件のうち「修正 + 回帰テスト」は後続 dispatch が
担い、本 PR 本文と Issue への提案でその旨を明記する。

## Upstream Documents

- [spec](../specs/issue-478-fader-knob-height-contract.md)
- [design](../designs/issue-478-fader-knob-height-contract.md)
- [ADR](../adr/issue-478-fader-knob-vertical-size-contract.md)
- [test-spec](../test-specs/issue-478-fader-knob-height-contract.md)

## Design Inputs

- [design](../designs/issue-478-fader-knob-height-contract.md)
- [ADR](../adr/issue-478-fader-knob-vertical-size-contract.md)

## File Map

| ファイル | 変更 | dispatch |
| --- | --- | --- |
| `docs/specs|designs|adr|test-specs|plans|tasks/issue-478-*.md` `implement-issue-478-*.md` | 新設 — 本チェーン | 設計判断（本 dispatch） |
| `docs/{specs,designs,adr,test-specs,plans,tasks}/README.md` | 生成 index の再生成（`doc_maintenance.js`） | 設計判断（本 dispatch） |
| `crates/ruviform-ui-gpui/src/render.rs` | `Fader \| Knob` 分岐を grow+`min_h` slot に、`control_element` 非 Seek 経路の `h_full` を slot 充填に修正 + 回帰テスト追加 | 後続実装 dispatch |

## Implementation Sequence

設計判断 dispatch（本 dispatch・完了済みの範囲）:

- [x] D1: spec → design → ADR → test-spec → plan → task の文書
  チェーンを作成し、relations を揃える。
- [ ] D2: review-protocol のレビューを実施し、blocker が残らない
  ことを確認する。
- [x] D3: `doc_maintenance.js` で managed index を再生成する
  （merge 後に再確認する）。
- [ ] D4: `git fetch origin main && git merge origin/main` で最新
  main を取り込み、生成 index を再生成した後に `mise run fmt-check` /
  `mise run check` を実行する。
- [ ] D5: 論理単位でコミットし、`docs/478-fader-height-contract` を
  push して PR を作る（Issue #478 の受入条件 A1–A3 の記録先）。

後続実装 dispatch（未着手・別 dispatch で実施）:

- [ ] I1: `element_for` の `Fader | Knob` 分岐を `flex_grow` +
  `min_h(px(120))` の slot に改め、`control_element` 非 Seek 経路が
  slot を `size_full` で満たす形にする（`min_h` 値は viz_slot と
  一本化することを推奨）。
- [ ] I2: test-spec の Rule/Example に対応する gpui-test を追加し、
  修正前に赤・修正後に緑を確認する。
- [ ] I3: `mise run test-ui-gpui` / `test-ui-parity` /
  `test-ui-consumer-observation` / `test-ui-driver-contract` /
  `test-ui-component-view` / `check-ui-driver-boundary` と
  `component_showcase` 実窓確認を行う。

## Dependencies and Risks

- 依存: #453（PR #459）で canvas 寸法監査が main 統合済み。
  `render.rs` の対象領域は #456 系の `drain_inbox`/receipt 変更と
  競合しない（領域が異なる）。
- リスク: grow 兄弟間の均等分配により Fader の実高さが sibling 構成で
  変わる。`component_showcase` の見た目変化は実装 dispatch の実窓
  確認で担保する。
- リスク: 極端に低い Panel では `min_h` 合計超過で依然溢れる。
  Panel の overflow/scroll は別契約（design の Deferred Design
  Concerns）。

## Task Breakdown

- [task](../tasks/implement-issue-478-fader-knob-height-contract.md)
  が実行単位の追跡を担う。設計判断側のタスクは本 dispatch で完了、
  実装側タスク（I1–I3）は後続 dispatch が引き継ぐ。

## Verification

- [ ] （本 dispatch）`mise run fmt-check` / `mise run check` が
  docs のみの変更で通過する。
- [ ] （後続実装 dispatch）test-spec の各 Rule に対応する gpui-test
  が赤→緑で確認される。
- [ ] （後続実装 dispatch）既存 UI lane が成功条件を弱めずに通過する。

## Open Questions / Gaps

- `min_h` の具体値（120 px で viz_slot と一本化するか、control 用に
  別値を置くか）は実装 dispatch で確定する。ADR は「viz slot と同じ床」
  を契約とし、一本化を推奨する。
- Tabs body の `size_full`・orientation ベース sizing・Panel の
  overflow/scroll は本 Issue の範囲外（design の Deferred Design
  Concerns に記録）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: 後続の実装 dispatch（本 plan の I1–I3）。
- Independent work streams: なし（`render.rs` 単一ファイルの変更）。
- User approval required before dispatch: yes（ADR の accepted 化は
  coordinator / reviewer の判断）。
- If approved: `docs/plans/implement-issue-478-fader-knob-height-contract.md`
  と `docs/test-specs/issue-478-fader-knob-height-contract.md` を
  入力として実装 dispatch を起こす。

## Review Handoff

- レビュー観点: ADR の Option C 採用理由が #450/#453 の確立境界
  （driver-neutral・slot 規則）と矛盾しないか。`min_h` 値の一本化
  方針。残課題（Tabs `size_full`・orientation sizing・Panel
  overflow）の切り分けが妥当か。
