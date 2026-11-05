---
id: "TASK-6BTRFSnTERjkzGCwtaEgAs"
type: "task"
status: "done"
title: "T2 `.github/workflows/daily-ci.yml` 新設（Issue #367）"
created: "2026-11-06"
updated: "2026-11-06"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/367"
  changes:
    added:
      - type: "file"
        path: ".github/workflows/daily-ci.yml"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-367-daily-scheduler.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-367-daily-scheduler.md"
    - "docs/tasks/issue-367-t1-decide-evaluator.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-367-t1-decide-evaluator.md"
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

# T2 `.github/workflows/daily-ci.yml` 新設（Issue #367）

## Work

`daily-ci.yml` を新設する。`schedule`（UTC cron・日本時間 Daily 相当・分 != 0）+ `workflow_dispatch`（`dry_run` 入力）、workflow 全体 `permissions: {}`、単一 `decide` job（ubuntu-latest・`contents: read` + `actions: write`・timeout 15 分）、`concurrency` 直列化（`cancel-in-progress: false`）、`mise run test-ci-daily` 自己検証の後に `ci/daily/decide.mjs` を実行し、`daily-ci-report-attempt-<n>` artifact を upload する。`ci.yml` の trigger・inputs・job は一切変更しない。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-367-daily-scheduler.md`
- `docs/designs/issue-367-daily-scheduler.md`

## Dependencies

- T1（workflow が `ci/daily/decide.mjs`・`test-ci-daily` を参照）

## Affected Files

- `.github/workflows/daily-ci.yml`（新設）

## Done When

- `mise exec -- actionlint .github/workflows/daily-ci.yml` が pass。
- cron が UTC・分 != 0・日本時間 Daily 相当（06:17 JST）である。
- `actions: write` が単一 job に限定され、workflow 全体は `permissions: {}`。
- `concurrency` が scheduler 自身を直列化し `cancel-in-progress: false`。
- `workflow_dispatch` に `dry_run` 入力があり、`decide.mjs` へ `--dry-run` で伝播する。
