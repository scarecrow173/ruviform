---
id: "EXP-720tupleV208"
type: "impl"
status: "completed"
title: "Issue #720 consumer-verify-tuple contract v1→v2 bump — 実験記録"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/impl/ir/issue-720-consumer-tuple-v2.md"
    - "docs/impl/exp/issue-720-consumer-tuple-v2-exp.jsonl"
  changes:
    added:
      - type: "file"
        path: "docs/impl/exp/issue-720-consumer-tuple-v2.md"
      - type: "file"
        path: "docs/impl/exp/issue-720-consumer-tuple-v2-exp.jsonl"
  implements:
    - "docs/impl/ir/issue-720-consumer-tuple-v2.md"
  implemented-by: []
  depends-on:
    - "docs/impl/ir/issue-720-consumer-tuple-v2.md"
  blocks: []
  supersedes: []
  superseded-by: []
---

# Issue #720 consumer-verify-tuple contract v1→v2 bump — 実験記録

詳細な時系列イベントは companion JSONL
`docs/impl/exp/issue-720-consumer-tuple-v2-exp.jsonl`
（`experiment_event.v1` schema）を参照。

## 要約

1. main `7124f0b2` で v1 pin 箇所を棚卸し — producer/consumer/
   両 workflow/contract test/docs の 14 箇所を特定。
2. 設計論点 (a): v1 artifact の扱いを hard cutover と決定。
   `classifyRunEvidence` の contract 完全一致規則がそのまま
   non-target 分類を実装しており、受理集合の導入は不要と判断。
3. 設計論点 (b): `schemaVersion`/`lanes` 未検査の現状維持を決定 —
   consumer が必要とするのは tuple 同一性と status であり、証拠内部
   語彙は producer 側契約。contract 文字列が version 軸。
4. 一括 bump を実施し、decide test に legacy v1 → non-target と
   `resolveConfig` 既定 v2 の pin を追加。
5. `test-ci-consumer-verify` で main 既存破損（owner regex stale・
   `70b48304` 起因）を検出 — base worktree で再現確認後、test regex
   のみ修復（workflow lane 行は不変）。
6. canonical 検証一式が全て pass — AC5（post-merge 実 run）は
   NotRun として分離記録。
