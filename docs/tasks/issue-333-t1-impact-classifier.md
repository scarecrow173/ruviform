---
id: "TASK-69XeSXbqn9bazeVEgD0LzH"
type: "task"
status: "done"
title: "T1 ci/impact classifier + gate 実装（Issue #333）"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/333"
  changes:
    added:
      - type: "directory"
        path: "ci/impact"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-333-impact-based-ci.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-333-impact-based-ci.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-333-impact-based-ci.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-333-impact-based-ci.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  task:
    lane: "implementation"
---

# T1 ci/impact classifier + gate 実装（Issue #333）

## Goal

design の分類契約どおりに `ci/impact/map.mjs`（規則データ）・
`classify.mjs`（diff→decision JSON）・`gate.mjs`（expected×actual→判定）
を stdlib-only Node `.mjs` で実装し、`node --test` fixture 検証を通す。

## Acceptance

- `classify.mjs` が test-spec の全分類 Rule を満たす decision JSON を出力する。
- `gate.mjs` が expected×actual 全組合せを design 判定表どおりに処理し、
  fail 時 exit 1 を返す。
- `node --test ci/impact/` が全件 pass。

## 関連文書

- 実装計画: [docs/plans/issue-333-impact-based-ci.md](../plans/issue-333-impact-based-ci.md)
- 検証仕様: [docs/test-specs/issue-333-impact-based-ci.md](../test-specs/issue-333-impact-based-ci.md)
