---
id: "DESIGN-034UiRsPAtNVImhGaTw2TY"
type: "design"
status: "approved"
title: "Issue #480 設計 — tab_focus の op 駆動解放"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-480-tab-focus-release.md"
    - "https://github.com/scarecrow173/ruviform/issues/480"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-480-tab-focus-release.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-480-tab-focus-release.md"
  implemented-by:
    - "docs/plans/issue-480-tab-focus-release.md"
  depends-on:
    - "docs/specs/issue-480-tab-focus-release.md"
    - "docs/designs/issue-451-keyboard-tab-nav.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-479-keyboard-value-controls.md"
    - "docs/designs/issue-451-focus-next-wiring.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-480-tab-focus-release.md"
  derived-by:
    - "docs/plans/issue-480-tab-focus-release.md"
    - "docs/test-specs/issue-480-tab-focus-release.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-480-tab-focus-release.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/451"
    - "https://github.com/scarecrow173/ruviform/issues/479"
  defers: []
  deferred-by: []
---

# Issue #480 設計 — tab_focus の op 駆動解放

## Context

`RvfView::tab_focus`（`BTreeMap<ComponentKey, FocusHandle>`）は selectable
Tabs strip を初めて描画したときに `tab_focus()` が entry を作る
（`render.rs` `element_for` の `ComponentSpec::Tabs` 分岐）。FU-1
（#451）の設計は「unmount 時に解放しない・spec 依存で有界」を既知の制約
として記録したが、解放材料は既に存在する:

- `ViewHost::update`（`view.rs`）は reconcile 結果を `ProjectionOp`
  （`Mounted`/`Updated`/`Removed`/`Unmounted`）として返し、各 op を
  `ViewEvent::Op` として host の event キューへ push する。
- `ViewHost::unmount` は全 mounted key に `Unmounted` op を発行する。
- `RvfView::flush_events` は host event の唯一の drain point であり、
  `update_spec`・`input`・`select_row`/`select_tab`・`invoke`・
  `detach_view` seam の全 mutation path が必ず経由する。現状は
  `ViewEvent::Intent` のみ処理し、Op は破棄していた。

## Scope

### In Scope

- `flush_events` への `ViewEvent::Op(ProjectionOp::Removed { key } |
  ProjectionOp::Unmounted { key })` 処理の追加（`tab_focus.remove(&key)`）。
- `tab_focus` フィールドの doc comment の実態への整合。
- gpui-test による解放・保持の恒久テスト。

### Out of Scope

- `control_focus`（#479 FU-c）の解放 — 同型機構だが別 Issue の管轄
  （§Deferred Design Concerns）。
- `ruviform-ui` 側の契約変更（`ProjectionOp`/`ViewEvent` は既存のまま）。
- detach 時の解放 — detach は tree を mounted のまま維持する契約。
- 描画 sweep・spec 再解釈による一括削除。

## Components and Boundaries

- **解放の所有者は `RvfView`**: `tab_focus` を持つのは view であり、
  host は key の存否を op で通知するだけ。op を解釈して map を更新する
  責務は map の所有者に置くのが正しい境界。
- **`flush_events` が唯一の drain point**: op を発行しうる全経路
  （`update_spec` 経由の `host.update`、将来の `host.unmount` 呼出、
  test seam の `host_mut()`）は flush を伴う。ここに置けば経路追加時に
  個別対応が不要になる。
- **gpui 側の安全性**: `FocusHandle::drop` は `FocusMap` の `ref_count`
  を減算するだけで、最後の clone が落ちると `App::
  release_dropped_focus_handles`（`flush_effects` 内）が entry を除去し、
  その handle に focus していた window を `blur()` する。明示的な
  blur 呼出は不要で、「focus 中の strip が消えた」場合はフォーカス無しに
  自然に倒れる。

## Data and Control Flow

```
update_spec(spec) / host.unmount()
  -> ViewHost::update / unmount        [ruviform-ui: view.rs]
       -> reconcile -> ProjectionOp::{Removed,Unmounted}{key}
       -> push ViewEvent::Op(...)      # host event queue
  -> RvfView::flush_events             [ruviform-ui-gpui: render.rs]
       -> Intent        -> intent_handler (existing)
       -> Op(Removed|Unmounted{key}) -> tab_focus.remove(&key)  # new
       -> 他            -> drop (existing)
  -> cx.notify() -> 次 frame の element_for が残存 strip を再描画
```

- kind-swap（同一 key で kind が変わる）は reconcile が `Removed` +
  `Mounted` の対を同一 update 内でこの順に発行する。`Removed` で旧
  handle を解放し、`Mounted` は無視 — 新 kind が再び selectable Tabs
  になることはない（同一 kind なら `Updated`）ため、必要なら次回描画で
  新しい handle が作られる。
- 同一 key の remove→再 add（別 update）が起きた場合も、再 mount は
  新しい component identity であり、次回描画で新しい handle が作られる。
  旧 handle のフォーカスを引き継がないのは remount の意味論と一致する。

## Non-Functional Constraints

- RT 制約なし（非 RT renderer）。`BTreeMap::remove` は対数コストで、
  op 数は spec 差分に比例する。
- reply echo・receipt・selection undo・detach policy など契約層の挙動は
  一切変更しない。`flush_events` の Intent 経路は不変。
- `#[non_exhaustive]` な `ViewEvent`/`ProjectionOp` への match は
  wildcard arm を維持する（`Mounted`/`Updated`/`Gesture`/`Unsupported`
  は従来どおり drop）。

## Risks and Trade-offs

| 論点 | 決定 | 根拠 |
| --- | --- | --- |
| `selection: Some → None` の `Updated` | **handle を保持する（解放しない）** | `ProjectionOp::Updated` は props 差分を持たず、flush 時点で「selectable でなくなったか」を op だけからは判別できない。key は mounted のままで、selection が戻れば同じ handle でフォーカスも維持できる。spec 依存で有界な残存ケースとして許容する（テストで固定）。 |
| `Updated` 一般（title 変更等） | 保持 | 同上。op は identity の存続を意味し、handle の寿命は key の存続に揃えるのが一貫する。 |
| detach（dock 移動・hide） | 保持 | `detach` は tree を mounted のまま維持し generation のみ進める契約。strip は同じ key で再描画され、フォーカス維持が期待される。 |
| 解放タイミング | flush 時点（op 受信時） | 描画タイミングに依存しない。op が queue に残る経路は flush まで遅延するが、production の全 mutation path は同期的に flush する。 |
| `control_focus` も同時に解放するか | **しない（別 Issue）** | Issue #480 の厳密スコープは Tabs strip。同じ match arm に `control_focus.remove` を足すだけの同型変更だが、FU-c は別途追跡されるべき挙動変更であり、本 PR で黙って吸収すると Issue 管理と検証記録が不明瞭になる。 |

## 検討して退けた案

| 案 | 退けた理由 |
| --- | --- |
| 描画済み key の sweep（`element_for` で集めた key 集合と map を照合） | 「mounted だが未描画」の strip（非アクティブ tab 配下・一時的に spec にあるが描画されない）の handle まで消し、タブ復帰時にフォーカスを失う。ユーザー指定の要件に反する。 |
| spec の selectable 判定による sweep | `flush_events` が spec を再走査する責務を負い、`element_for` が既に持つ「selection があるか」の知識を二重化する。`Updated` の扱いを変えても gain が小さい。 |
| `update_spec` 内でのみ処理 | `host.unmount` 等 `update_spec` を通らない op 発行経路をカバーできない。drain point に集約する方が境界として正しい。 |
| `control_focus` も同じ sweep で解放 | FU-c は別 Issue。同梱するとスコープが曖昧になり、#479 系の検証記録と混ざる。 |

## Deferred Design Concerns

- `control_focus`（#479 FU-c）の unmount 解放 | 同型の有界リーク。
  `flush_events` の同じ match arm に `self.control_focus.remove(&key)`
  を足すだけで実現できる | FU-c の Issue 起票・着手 | map が増えるほど同じ
  変更を個別適用するコストが増える |

## Verification Notes

- [x] gpui-test（`render.rs` `mod tests`）: `Removed`（`update_spec` で
      strip 除去）・kind-swap（`Removed`+`Mounted` 対）・`Unmounted`
      （`host.unmount`）で entry が消え、sibling の handle・フォーカス・
      key 応答が保持されること。`Updated`（`selection: Some → None`）
      で保持されること。RED→GREEN を確認済み。
- [ ] `mise run test-ui-gpui` / `lint-ui-gpui` /
      `check-ui-driver-boundary` / `check` / `lint` / `fmt-check` green。
