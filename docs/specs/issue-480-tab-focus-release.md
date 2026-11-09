---
id: "SPEC-034UiRf9jEzvH2FbreqPsA"
type: "spec"
status: "approved"
title: "Issue #480 — Tabs strip の FocusHandle unmount 解放"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/480"
    - "https://github.com/scarecrow173/ruviform/issues/451"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-480-tab-focus-release.md"
      - type: "file"
        path: "docs/designs/issue-480-tab-focus-release.md"
      - type: "file"
        path: "docs/plans/issue-480-tab-focus-release.md"
      - type: "file"
        path: "docs/test-specs/issue-480-tab-focus-release.md"
      - type: "file"
        path: "docs/tasks/issue-480-tab-focus-release.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/designs/issue-480-tab-focus-release.md"
    - "docs/plans/issue-480-tab-focus-release.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-451-keyboard-tab-nav.md"
    - "docs/designs/issue-479-keyboard-value-controls.md"
    - "docs/tasks/issue-451-keyboard-tab-nav.md"
  refines:
    - "docs/specs/issue-451-keyboard-tab-nav.md"
  refined-by: []
  derives-from: []
  derived-by:
    - "docs/designs/issue-480-tab-focus-release.md"
    - "docs/plans/issue-480-tab-focus-release.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-480-tab-focus-release.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/451"
    - "https://github.com/scarecrow173/ruviform/issues/479"
  defers: []
  deferred-by: []
---

# Issue #480 — Tabs strip の FocusHandle unmount 解放

## What Should Be Built

`RvfView::tab_focus`（`BTreeMap<ComponentKey, FocusHandle>`）に、strip の
key が spec から消えたとき（`ProjectionOp::Removed`）および host 全体が
unmount されたとき（`ProjectionOp::Unmounted`）に対応するエントリを解放する
経路を実装する。解放は `ViewHost` が発行する projection op を `RvfView` が
既に drain している唯一の地点（`flush_events`）で行い、描画済み key の
sweep など spec 内容への再解釈は行わない。

## Why Now

Issue #451 FU-1 で selectable Tabs strip ごとの `FocusHandle` を導入した際、
`docs/designs/issue-451-keyboard-tab-nav.md` §4 に「unmount 時に解放せず、
同じ view 内で一度でも描画された selectable Tabs の数だけ残る。数は spec に
依存して有界」と既知の制約として記録された（#480 はその FU-1d）。有界では
あるが、spec 更新で strip が消えてもハンドルが残り続けるのはリーク的であり、
解放材料（reconcile が発行する `Removed`/`Unmounted` op）は既に揃っている。

## Users and Value

- Consumer（Synvelis 等）: spec 更新で Tabs strip を出し入れする UI が、
  view の生存期間中に蓄積する dead `FocusHandle` を残さない。
- Driver 実装者: `tab_focus` のサイズが mounted tree に比例して有界に
  保たれ、unmount 後の状態が空になることが契約として明確になる。

## Scope

### In Scope

- `crates/ruviform-ui-gpui/src/render.rs` の `flush_events` に
  `ViewEvent::Op(ProjectionOp::Removed { key } | ProjectionOp::Unmounted { key })`
  を処理する分岐を追加し、`self.tab_focus.remove(&key)` を行う。
- headless（gpui-test）での解放検証テスト（`Removed`・kind-swap・
  `Unmounted`・sibling 保持・`Updated` での保持）。
- `selection: Some → None` の `Updated` ケースの扱いを design に明記する。
- `docs/` の spec → design → plan → test-spec → task チェーン。

### Out of Scope

- `control_focus`（#479 FU-c）の解放。機構は同型だが本 Issue の厳密
  スコープは Tabs strip のみ。展開可能性は design に記録する。
- `crates/ruviform-ui`（`ProjectionOp`/`ViewEvent` 契約）、`ui/`、
  `crates/ruviform-ui-test/`、`crates/ruviform-audio`、`Cargo.*`、
  `.github/` の変更。
- detach（dock 移動・window hide）時の解放。detach は tree を mounted の
  まま維持する契約であり、strip は再 attach 後も同じ key で描画される。
- spec 内容に基づく sweep（描画済み key 集合や selectable 判定による
  一括削除）。op 駆動のみとする。

## Requirements

- spec update で key が消えた selectable Tabs strip の `FocusHandle` が
  `RvfView::tab_focus` から解放される（`Removed` op 駆動。`Unmounted` も
  同様に扱う）。
- 残存 strip のハンドル・フォーカス状態は保持する（再生成しない）。
- 非アクティブ tab 配下など「mounted だが未描画」の strip のハンドルは
  保持が正しい — 描画済み key での sweep は行わない。
- 既存の reply echo・フォーカス挙動を弱めない。
- `selection: Some → None` の `Updated`（key は残るが selectable でなく
  なる）の扱いを根拠つきで design に明記する。

## Acceptance Criteria

- [ ] spec update で key が消えた selectable Tabs strip の `FocusHandle`
      が `RvfView::tab_focus` から解放される（`Removed` op 駆動。
      `Unmounted` も同様に扱う）
- [ ] headless（gpui-test）で解放を検証するテスト: strip mount・描画 →
      entry 生成 → `update_spec` で strip 除去 → entry が消え sibling
      strip のハンドルとフォーカスは保持。kind-swap / Unmounted も網羅。
      RED→GREEN を記録
- [ ] `selection: Some → None` の `Updated` ケースの扱いを design に
      明記（解放する/既知の残存ケースのどちらかを根拠つきで）
- [ ] `docs/` チェーン作成 + `control_focus`（#479 FU-c）への展開可能性
      を記録
- [ ] 全 lane green（`mise run test-ui-gpui`・`lint-ui-gpui`・
      `check-ui-driver-boundary`・`check`・`lint`・`fmt-check` 等）

## Deferred Design Concerns

- `control_focus` の unmount 解放（#479 FU-c）| 同型の有界リークだが
  別 Issue の管轄 | FU-c の Issue が起票されたとき | 対象 map が増えると
  同じ変更を複数箇所へ適用する必要が出る

## Sources and Relations

- Issue #480（FU-1d）、Issue #451（親）、Issue #479（FU-c の兄弟問題）。
- `docs/designs/issue-451-keyboard-tab-nav.md` §4 — 既知の制約として
  記録された有界リークの正本。
- `docs/designs/issue-479-keyboard-value-controls.md` §4・§8 —
  `control_focus` の同型制約の記録。
