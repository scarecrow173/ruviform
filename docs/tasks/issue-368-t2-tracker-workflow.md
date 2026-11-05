---
id: "TASK-Peb0C7F4qBHb9sZP8wXZnR"
type: "task"
status: "done"
title: "T2 ci-issue-tracker.yml workflow + 運用 README + dev-env 同期（Issue #368）"
created: "2026-11-06"
updated: "2026-11-06"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/368"
  changes:
    added:
      - type: "file"
        path: ".github/workflows/ci-issue-tracker.yml"
      - type: "file"
        path: "ci/tracker/README.md"
    modified:
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-368-ci-issue-tracker.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-368-ci-issue-tracker.md"
    - "docs/tasks/issue-368-t1-tracker-evaluator.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-368-t1-tracker-evaluator.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-368-ci-issue-tracker.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# T2 ci-issue-tracker.yml workflow + 運用 README + dev-env 同期（Issue #368）

## Work

`.github/workflows/ci-issue-tracker.yml` を新設し、`ci/tracker/README.md` に運用契約を、`docs/development-environment.md` に tracker workflow・task・制約を同期する。T1 の判定器を workflow から `test-ci-tracker` 自己検証付きで呼ぶ。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-368-ci-issue-tracker.md`
- `docs/designs/issue-368-ci-issue-tracker.md`
- `docs/tasks/issue-368-t1-tracker-evaluator.md`

## Dependencies

- T1（`ci/tracker/` の存在に依存）

## Affected Files

- `.github/workflows/ci-issue-tracker.yml`（新設）
- `ci/tracker/README.md`（新設）
- `docs/development-environment.md`（同期）

## Done When

- `ci-issue-tracker.yml` が `workflow_run: {workflows: [ci], types: [completed], branches: [main]}` で起動し、`pull_request`/`push`/`schedule`/`workflow_dispatch` trigger を持たない。
- workflow 直下 `permissions: contents: read`、job `track` のみ `contents: read`/`actions: read`/`issues: write`。`concurrency` が repo 単位直列・`cancel-in-progress: false`。
- job steps が default branch checkout → mise（node のみ）→ `mise run test-ci-tracker` → `node ci/tracker/tracker.mjs`（`GH_TOKEN`・`TRACKER_EVENT_PATH`）の順で、失敗 run の artifact/code を触れない構造。
- `actionlint` pass・`ci.yml`/`consumer-verify.yml` 無変更。
- README が label `ci-failure`・権限不足・GitHub Actions 障害・manual close 方針・diverged 手動対応・Consumer 再利用契約をカバーする。
- `docs/development-environment.md` が tracker workflow・`test-ci-tracker`・運用制約を反映する。
