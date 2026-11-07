---
id: "TASK-0Un8zJnU8BpZljyBDTCZNe"
type: "task"
status: "done"
title: "T6 RF #305 候補 scratch 検証と evidence 記録 (Issue #306)"
created: "2026-11-07"
updated: "2026-11-07"
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
    - "docs/plans/issue-306-io-composition-e2e.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-306-t4-wav-boundary.md"
    - "docs/tasks/issue-306-t5-mise-task-verification.md"
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
    - "docs/test-specs/issue-306-io-composition-e2e.md"
  references: []
  defers: []
  deferred-by: []
---

# T6 RF #305 候補 scratch 検証と evidence 記録 (Issue #306)

## Work

TEMP 配下に `git worktree add` で RF #305 候補 head `82256cf…` を
checkout（他者 worktree/main を変更しない）。旧
`StagedOutput`/`PublishOptions` 不在の grep 証跡、候補の io-wav
試験実走、`WavMuxer<&mut StagedFile>`+seal+readback+publish の
合成再演（scratch 側 members への uncommitted 追加のみ）。
SHA/command/exit code/結果を `evidence/rf306/` に記録し、
worktree を remove する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-306-io-composition-e2e.md`
- `docs/designs/issue-306-io-composition-e2e.md`（RF #305 scratch
  検証節）
