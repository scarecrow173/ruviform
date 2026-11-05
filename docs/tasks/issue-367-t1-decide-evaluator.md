---
id: "TASK-OMt1jAp3OOLOSc40FVZTD0"
type: "task"
status: "done"
title: "T1 ci/daily/decide.mjs 判定器 + fixture contract tests + test-ci-daily task（Issue #367）"
created: "2026-11-06"
updated: "2026-11-06"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/367"
  changes:
    added:
      - type: "directory"
        path: "ci/daily"
    modified:
      - type: "file"
        path: "mise.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-367-daily-scheduler.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-367-daily-scheduler.md"
  blocks:
    - "docs/tasks/issue-367-t2-daily-ci-workflow.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-367-t2-daily-ci-workflow.md"
    - "docs/tasks/issue-367-t3-consumer-contract-docs.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-367-daily-scheduler.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# T1 ci/daily/decide.mjs 判定器 + fixture contract tests + test-ci-daily task（Issue #367）

## Work

`ci/daily/decide.mjs`（stdlib-only、`ci/summary/summary.mjs` と同一構造: pure 判定コア + 注入可能 client + `main()`）と `ci/daily/tests/decide.test.mjs`（`node --test` fixture 駆動）を新設し、mise task `test-ci-daily` を追加する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-367-daily-scheduler.md`
- `docs/designs/issue-367-daily-scheduler.md`
- `docs/test-specs/issue-367-daily-scheduler.md`

## Dependencies

- なし（先行タスクなし。T2 が本タスクの成果物に依存する）

## Affected Files

- `ci/daily/decide.mjs`（新設）
- `ci/daily/tests/decide.test.mjs`（新設）
- `mise.toml`（`[tasks.test-ci-daily]` 追加）

## Done When

- `decide()` が designs の decision 表どおり `skip-verified`/`suppress-in-flight`/`hold-unknown`/`budget-exhausted`/`dispatch` を返し、`attemptsOn` が sha+branch+event allowlist で厳格同定される。
- `autoDispatches` が `event === workflow_dispatch` ∧ `triggering_actor` bot のみ計上し、上限 `DAILY_MAX_AUTO_DISPATCHES`（既定 2）で `budget-exhausted`。
- `run()` が再照合（`MAX_RECHECK`）・`POST dispatches {ref, expected_sha}`・`preDispatchIds` 差分の確認 poll（confirmed/landed-mismatch/unconfirmed）・`--dry-run`・`--fixture` を実装する。
- report JSON（`kind: "daily-ci-decision"`）+ Job Summary + exit code（0 正常 outcome / 1 error / 2 usage）が test-spec の行列で fixture 検証され、`mise run test-ci-daily` が exit 0。
