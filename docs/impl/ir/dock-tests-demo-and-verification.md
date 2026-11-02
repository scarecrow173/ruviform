---
id: "IMPL-034QqITVZVkBse5tdUFJVQ"
type: "impl"
status: "completed"
title: "Dock tests, demo, and verification"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/182"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui/tests/dock.rs"
      - type: "file"
        path: "crates/ruviform-ui/examples/demo_dock.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-182-04-tests-demo-verification.md"
  implemented-by: []
  depends-on:
    - "dock-model-layer.md"
    - "dock-workspace-and-two-phase-move.md"
    - "layout-persistence-and-window-position.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../plans/issue-182-dock-tree.md"
    - "../../designs/issue-182-dock-tree.md"
    - "../../specs/issue-182-dock-tree.md"
    - "../../adr/issue-182-dock-tree-placement-move-contract-and-persistence.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../../test-specs/issue-182-dock-tree.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Dock tests, demo, and verification

## Summary

Issue #182 Task 4 — integration test(`tests/dock.rs`, public API
のみ)、headless demo(`examples/demo_dock.rs`)、crate README 更新、
検証スイート全実行を行った。fake host 駆動で multi-window
detach→reattach・2 相 move・失敗 rollback・close disposition・
snapshot round-trip を end-to-end 実証。実 GUI 検証は未実施 —
capability flag として正直に記録する。

## Context

docs/tasks/issue-182-04-tests-demo-verification.md —
test-spec の必須カバレッジを満たす。

## Implementation

### Changes

- `crates/ruviform-ui/tests/dock.rs`: `WindowHandle` を実装した
  `FakeHost`(`fail_creates` で window 生成失敗を注入)経由で
  7 つの integration test。
  - `two_phase_move_prepares_destination_then_switches_ownership`
  - `window_creation_failure_rolls_back_to_source_dock`
  - `commit_after_destination_destroyed_rolls_back`
  - `detach_reattach_preserves_logical_view_id`
  - `menu_move_fallback_matches_drop_move`
  - `window_close_applies_consumer_chosen_disposition`
  - `layout_snapshot_roundtrips_with_monitor_correction`
- `crates/ruviform-ui/examples/demo_dock.rs`: dock→split→2 相
  move→rollback→detach→close→snapshot/restore のナレーション
  付き headless run。
- `crates/ruviform-ui/README.md`: `dock` モジュール節・
  demo_dock 起動例・issue-182 参照を追記。

### Decisions During Implementation

- view state 保存は consumer 側 `HashMap<ViewInstanceId,String>`
  で表現 — 同一論理 id が dest へ到達すれば state がそのまま
  適用されることを assert(DOM 転送非保証の実証)。
- menu-move fallback は `move_view`(1-shot)を使い、zone drop と
  同一 end-state になることを比較で証明。
- 実 GUI 検証は headless 環境では非実行 — demo は
  `DockWorkspace` 純粋 model で駆動(capability flag:
  `real-gui-verified=false`)。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

実行済み(全て pass):
- `mise run fmt-check`
- `mise run check`
- `mise run lint`
- `mise run test`(workspace 全体 — dock unit 23 件 + integration 7 件)
- `mise run check-ui-webview`/`lint-ui-webview`/`test-ui-webview`
- `cargo run -p ruviform-ui --example demo_dock` — 全節が期待
  出力で完走。

未実行(正直な capability flag):
- real GUI(実 WebView window)での dock/move/restore の
  実機確認 — headless のみで検証。GUI 経路は
  `ruviform-ui-webview` の `demo_app` 相当の手動検証 lane が
  必要(Synvelis 側統合時に実施想定)。

## Risks

DropTarget/drag preview の platform 制約(webview 間 native
drag の可否)は adapter 実機で未確認 — 契約上は menu-move
fallback が常に存在するため阻害しない。

## Follow-ups

- Synvelis 側で view registry/menu 配線 + 実 GUI 検証
  (Issue #192/#193 系に続く)。
- `close/drop が project job を cancel しない`は
  `remove_window` が workspace のみに作用する構造で担保
  (job は `UiRuntime` 所有 — 本実装は触らない)。
