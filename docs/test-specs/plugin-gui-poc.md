---
id: "TSPEC-034JZPGUI0000000000004"
type: "test-spec"
status: "approved"
title: "プラグイン GUI PoC 判定規則"
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
  derived-by: []
  verifies:
    - "docs/designs/plugin-gui-poc.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# プラグイン GUI PoC 判定規則

- `passed`/`failed`/`skipped`/`doc-eval` の4値。実機未実施は `skipped`、文書のみは `doc-eval`
- 各結果は `poc-metric.v1` 形式 jsonl + RESULTS.md 集約表
- AC1 は attach 成功 + 描画 + リサイズ追従 + IPC roundtrip が全て成立で `passed`
