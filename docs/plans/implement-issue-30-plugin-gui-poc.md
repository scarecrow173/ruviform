---
id: "PLAN-034JZPGUI0000000000003"
type: "plan"
status: "approved"
title: "Issue #30 実施計画 — プラグイン GUI PoC"
created: "2026-10-21"
updated: "2026-10-21"
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
    - "docs/designs/plugin-gui-poc.md"
  derived-by:
    - "docs/tasks/plugin-gui-poc-harness.md"
    - "docs/tasks/plugin-gui-poc-closeout.md"
  verifies: []
  verified-by:
    - "docs/test-specs/plugin-gui-poc.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #30 実施計画

## 手順

1. `poc/plugin-gui/` 独立ワークスペース（wry 0.57・windows・raw-window-handle 依存）
2. main.rs: 素 Win32 親窓 + メッセージループ + `build_as_child` attach + E1/E2 実験 + jsonl 出力
3. 実行して `poc/plugin-gui/RESULTS.md` と `docs/impl/exp/plugin-gui-*.jsonl` に記録
4. 文書評価（AC3/AC4）を RESULTS.md に集約
5. 採否 ADR 作成・チェーンクローズ

## Verification

- [x] E1/E2 の実機結果が記録済み
- [x] 採否 ADR が存在し全条項が confirm/revise 判定
- [x] ルートワークスペース無変更
- [x] doc 監査で本チェーン起因 findings 0
