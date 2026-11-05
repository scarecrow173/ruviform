---
id: "TASK-xJy29ZBdKUklv4RDyz58A3"
type: "task"
status: "done"
title: "T3 Consumer 採用契約・運用手順の docs 同期（Issue #367）"
created: "2026-11-06"
updated: "2026-11-06"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/367"
  changes:
    added: []
    modified:
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-367-daily-scheduler.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-367-daily-scheduler.md"
    - "docs/tasks/issue-367-t2-daily-ci-workflow.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-367-t1-decide-evaluator.md"
    - "docs/tasks/issue-367-t2-daily-ci-workflow.md"
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

# T3 Consumer 採用契約・運用手順の docs 同期（Issue #367）

## Work

`docs/development-environment.md` に `daily-ci.yml` の構成（trigger・permissions・concurrency・report artifact）、運用手順（手動再判定・`dry_run`・最終成功検証時刻・`budget-exhausted` からの復帰・schedule 欠落の固有制約）、Consumer 採用契約（vendoring 単位・env 設定差分・`expected_sha` 前提・権限最小化）を追記する。`test-ci-daily` の task 表エントリも追加する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-367-daily-scheduler.md`
- `docs/designs/issue-367-daily-scheduler.md`

## Dependencies

- T2（documented workflow が存在すること）

## Affected Files

- `docs/development-environment.md`（更新）

## Done When

- task 表に `test-ci-daily` が載り、workflow 構成節に `daily-ci` の項目がある。
- 運用手順（手動再判定・dry_run・最終成功時刻・budget 復帰・schedule 欠落制約）と Consumer 採用契約（vendoring・env・権限最小化・Ruviform 非依存）が記載される。
- #368 tracker との責務境界（scheduler は Issue を作らない）が明記される。
