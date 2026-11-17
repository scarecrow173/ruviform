---
id: "EXP-630presetDiscoveryReeval08"
type: "impl"
status: "completed"
title: "Issue #630 preset-discovery 外部検証再評価 slice — 実験記録"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/impl/ir/issue-630-preset-discovery-reeval.md"
    - "docs/impl/exp/issue-630-preset-discovery-reeval-exp.jsonl"
  changes:
    added:
      - type: "file"
        path: "docs/impl/exp/issue-630-preset-discovery-reeval.md"
      - type: "file"
        path: "docs/impl/exp/issue-630-preset-discovery-reeval-exp.jsonl"
  implements:
    - "docs/impl/ir/issue-630-preset-discovery-reeval.md"
  implemented-by: []
  depends-on:
    - "docs/impl/ir/issue-630-preset-discovery-reeval.md"
  blocks: []
  supersedes: []
  superseded-by: []
---

# Issue #630 preset-discovery 外部検証再評価 slice — 実験記録

詳細な時系列イベントは companion JSONL
`docs/impl/exp/issue-630-preset-discovery-reeval-exp.jsonl`
（`experiment_event.v1`）を参照。

## 要約

1. clap-validator を upstream commit `47e2424`（PR #41 merge）から
   build し `pin-47e2424/clap-validator.exe` に pin。SHA-256
   `b4bd645877bba3d5abe4300a84c5785312ce16f7d1179a2fd3177eedc45afe26`。
2. `CLAP_FIXTURE_LIBRARY_EXCLUDES` を削除し fixture target を
   `excludes: []` に変更。contract test を更新し 32 pass。
3. canonical lane を pin validator で実走し verdict Passed / exit 0。
   preset-discovery crawl/load が raw output で success として観測
   され、preset-discovery exclusion は run.json に残らない。
