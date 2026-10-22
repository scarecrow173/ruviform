---
id: "PLAN-FcYqNdQHMlWhhFuRcYDyWi"
type: "plan"
status: "approved"
title: "Issue #74 実施計画 — ruviform-plugin-host（L0）新設"
created: "2026-11-03"
updated: "2026-11-03"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/plugin-host-l0.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  derived-by:
    - "docs/tasks/plugin-host-l0-crate.md"
    - "docs/tasks/plugin-host-l0-closeout.md"
  verifies: []
  verified-by:
    - "docs/test-specs/plugin-host-l0-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #74 実施計画

## 手順

1. `crates/ruviform-plugin-host` 新設 — `command.rs`（`HostCommand`/`HostReply`/`HostError`）・`plugin.rs`（`Plugin`/`PluginFactory`/`GuiHandle`）・`host.rs`（`PluginHandle`/`InProcessHost`）+ workspace メンバー追加
2. 単体テスト: モック factory/plugin で scan→instantiate→ライフサイクル→state→gui→destroy のコマンド駆動、違法遷移 reject、generation ハンドル検出
3. RT テスト `rt_process_no_alloc`（assert_no_alloc 窓・Processing 状態のみ到達）
4. 文書クローズ + 検証列

## Verification

- [x] コマンド境界が全ライフサイクル/state/gui オペレーションをカバーし純データ型のみ
- [x] RT `process` 0 alloc 実測・非 Processing reject
- [x] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt`/`bench-smoke`・prek パス
- [x] doc 監査で本チェーン起因 findings 0
