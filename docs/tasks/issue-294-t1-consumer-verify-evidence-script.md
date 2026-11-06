---
id: "TASK-034Sa3F7jL2nQ5uY8cE1hK"
type: "task"
status: "done"
title: "T1 ci/consumer-verify evidence.mjs + fixture tests + test-ci-consumer-verify task（Issue #294）"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/294"
  changes:
    added:
      - type: "directory"
        path: "ci/consumer-verify"
    modified:
      - type: "file"
        path: "mise.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-294-cross-consumer-candidate-gate.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-294-cross-consumer-candidate-gate.md"
  blocks:
    - "docs/tasks/issue-294-t2-consumer-verify-workflow-tuple-ui-lanes.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-294-t2-consumer-verify-workflow-tuple-ui-lanes.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-294-cross-consumer-candidate-gate.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# T1 ci/consumer-verify evidence.mjs + fixture tests + test-ci-consumer-verify task（Issue #294）

## Work

`ci/consumer-verify/evidence.mjs`（stdlib-only、pure 判定コア + `main()`、`ci/summary`・`ci/daily` と同一構造）を新設する。コマンドは `classify-ref` / `lane` / `finalize` の 3 種（design §1）。`ci/consumer-verify/tests/evidence.test.mjs` で fixture 駆動の contract test を用意し、`mise.toml` に `test-ci-consumer-verify` を追加する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-294-cross-consumer-candidate-gate.md`
- `docs/designs/issue-294-cross-consumer-candidate-gate.md`
- `docs/test-specs/issue-294-consumer-verify-tuple-evidence.md`

## Dependencies

- なし（T2 の前提）

## Affected Files

- `ci/consumer-verify/evidence.mjs`（新規）
- `ci/consumer-verify/tests/evidence.test.mjs`（新規）
- `mise.toml`（`test-ci-consumer-verify` 追加）

## Done When

- [ ] `mise run test-ci-consumer-verify` が pass（classify-ref 行列・status 導出全枝・証拠 JSON schema・`shaMatch` 三値・summary 形状・秘密非転写）
- [ ] status 導出が design の優先順どおりで、`not_run`/`unsupported`/`skipped` が pass に計上されない
- [ ] `docs/development-environment.md` 対応表の追記は T2 でまとめて行う
