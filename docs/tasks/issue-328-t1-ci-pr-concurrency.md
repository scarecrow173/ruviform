---
id: "TASK-uRQkJW5851dqMXehrKNZkI"
type: "task"
status: "done"
title: "Issue #328 T1 — ci.yml / consumer-impact-check.yml へ PR concurrency block 適用"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/328"
  changes:
    added: []
    modified:
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: ".github/workflows/consumer-impact-check.yml"
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-328-ci-pr-concurrency.md"
  implemented-by:
    - "docs/impl/ir/issue-328-ci-pr-concurrency-implementation.md"
  depends-on:
    - "docs/plans/issue-328-ci-pr-concurrency.md"
    - "docs/specs/issue-328-ci-pr-concurrency.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-328-ci-pr-concurrency.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-328-ci-pr-concurrency.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #328 T1 — ci.yml / consumer-impact-check.yml へ PR concurrency block 適用

## Objective

design(`docs/designs/issue-328-ci-pr-concurrency.md`)の snippet を両 workflow の `permissions:` 直後に追加し、静的検証と AGENTS.md 同期まで行う。実測(AC3/C3)は coordinator が本 PR の CI で実施。

## Checklist

- [x] `ci.yml` `permissions:` 直後に `concurrency:` block 追加(設計書 snippet 通り)
- [x] `consumer-impact-check.yml` `permissions:` 直後に同 block 追加
- [x] `git diff` で job 名・job 数・trigger・permissions・step が非変更であること(INV-5)
- [x] `mise exec -- actionlint .github/workflows/ci.yml .github/workflows/consumer-impact-check.yml` が exit 0
- [x] AGENTS.md の ci.yml 記述 bullet に concurrency の一文を追加
- [x] doc index 再生成(`doc_maintenance.js apply`)→ 新規 blocking finding なし
- [x] IR(`docs/impl/ir/issue-328-ci-pr-concurrency-implementation.md`)に検証記録 + 実測引継ぎ(NotRun 明記)
