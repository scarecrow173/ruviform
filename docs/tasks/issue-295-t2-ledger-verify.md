---
id: "TASK-034RoWAHTxefOCVrz2FVL4"
type: "task"
status: "done"
title: "T2 共有台帳更新・main 取込・最終検証・IR (Issue #295)"
created: "2026-11-04"
updated: "2026-11-04"
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
    - "docs/plans/issue-295-io-file-staged-publish.md"
  implemented-by:
    - "docs/impl/ir/issue-295-ruviform-io-file-staged-publish-contract.md"
  depends-on:
    - "docs/plans/issue-295-io-file-staged-publish.md"
    - "docs/tasks/issue-295-t1-crate-and-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-295-io-file-staged-publish.md"
  references: []
  defers: []
  deferred-by: []
---

# T2 共有台帳更新・main 取込・最終検証・IR (Issue #295)

## Work

`crates/README.md`（汎用 I/O 基盤層の追加・クレート数）・AGENTS.md
（クレート数記載）・`Cargo.lock` の再生成を行い、`git fetch origin` →
`git merge origin/main` で台帳を再整合してから最終検証（fmt-check・check・
lint・test）を通し、IR（implementation record）を記録する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-295-io-file-staged-publish.md`
- `docs/designs/issue-295-io-file-staged-publish.md`

## Dependencies

- `docs/tasks/issue-295-t1-crate-and-contract.md`

## Affected Files

- `crates/README.md`・`AGENTS.md`・`Cargo.lock`・`docs/impl/ir/issue-295-*.md`

## Done When

- [x] 台帳 3 点（crates/README.md・AGENTS.md・Cargo.lock）が新クレートを反映
      （計33クレートで整合）
- [x] `origin/main` 取込後に最終検証が緑・コミット済み
      （merge は Already up to date — 分岐点 = main tip）

## Verification

- [x] `mise run fmt-check` / `mise run check` / `mise run lint` / `mise run test`
      （全 PASS — IR の Validation 表参照）
