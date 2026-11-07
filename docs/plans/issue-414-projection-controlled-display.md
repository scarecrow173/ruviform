---
id: "PLAN-dS9xUtZcP4wLvYnK6nRt3F"
type: "plan"
status: "completed"
title: "Issue #414 — projection→controlled display 実装計画"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/414"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-414-projection-controlled-display.md"
    - "docs/designs/issue-414-projection-controlled-display.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-414-projection-controlled-display.md"
    - "docs/designs/issue-414-projection-controlled-display.md"
  derived-by:
    - "docs/tasks/issue-414-projection-controlled-display.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-414-projection-controlled-display.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #414 — projection→controlled display 実装計画

## スコープ

`crates/ruviform-ui`（behavior.rs / view.rs）、
`crates/ruviform-ui-gpui`（render.rs の test）、
`crates/ruviform-ui-test`（component_view.rs test +
driver-lab cases.json）、`docs/`。ledger の retention /
correlation / epoch 機構（Issue #410 領域）には触れない。

## 変更点

1. `behavior.rs`
   - `projection_value_input(&serde_json::Value) -> Option<ValueInput>`
     — payload decode の正規契約（finite number→Plain、
     `{plain}`→Plain、`{normalized}`→Normalized、それ以外→None）。
   - `InteractionControl::apply_external(ValueInput) -> Option<ControlValue>`
     — 絶対値のみ受理、mapper 解決、gesture 中は stage / idle は
     即時 commit、event 非発行。
   - `pending_external: Option<ControlValue>` field +
     `end_gesture` での flush。
   - unit tests：idle commit / gesture stage / commit・cancel flush /
     range policy / decode shapes。

2. `view.rs`
   - `ViewHost::apply_projection`：accepted 時に decode →
     `dispatcher.control_mut(control_id).apply_external(input)`。
   - `register_control`：生存 binding の `last_observed` を新規
     control へ復元適用。
   - `apply_projection` doc に三状態区別と正規経路を明記。
   - unit tests：idle 非ゼロ / preview 優先 / commit・cancel /
     stale revision / undecodable payload / detach-reattach /
     re-register 復元。

3. `ruviform-ui-gpui/src/render.rs`
   - inbox → `apply_projection` → `observe().display.normalized`
     （`control_element` が読む値そのもの）を検証する
     `#[gpui::test]` 2 件追加。renderer コード自体の変更は不要
     （display が正しくなれば既存 paint 経路が追従する）。

4. `ruviform-ui-test`
   - `tests/component_view.rs` に `ui_dvr_09_*` 3 件追加
     （idle/stale/undecodable、gesture commit+cancel、detach-reattach）。
   - `fixtures/driver-lab/cases.json` に `UI-DVR-09` case 追加
     （fake=component_view cargo-test、gpui=test-ui-gpui
     mise-task、real-os=not_run）。

5. `docs/` — 本 spec/design/plan/task/test-spec/impl chain。

## 検証

`mise run` 系：fmt-check / check / lint / test /
check-ui-driver-boundary / test-ui-driver-contract /
test-ui-component-view / check-ui-headless / test-ui-headless /
check-ui-components / test-ui-components / test-ui-binding /
test-ui-guitest / test-ui-test-msrv / test-ui-gpui。
merge `origin/main` 後に最終実行。

## リスク

- `pending_external` の flush を `end_gesture` に集約したため、
  gesture 終了の全経路（commit/cancel/detach/focus-lost/
  availability-cancel/atomic-set/unregister）で一貫して適用される。
  pending ⟹ gesture active の不変条件は `end_gesture` が唯一の
  終了点であることで保たれる。
- `register_control` での last_observed 復元は descriptor 変更時に
  display を default に留めないためのもので、値 domain が変わる
  contract change でも新 mapper 経由で解決される。
