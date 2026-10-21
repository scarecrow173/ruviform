---
id: "TASK-034JZLISO0000000000005"
type: "task"
status: "done"
title: "plugin-host API 整合検証 + Issue #31 クローズ"
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
  implements:
    - "docs/plans/implement-issue-31-plugin-isolation.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-31-plugin-isolation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 整合検証 + クローズ

## Done When

- [x] ADR の境界契約（ハンドル + 直列化コマンド）が設計の L0/L1 分岐と整合することを確認（`ruviform-plugin-host` は未実装 — 新設時の境界として規定）
- [x] 検証列・doc 監査・graph complete
