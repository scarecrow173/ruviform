---
id: "TASK-T6nZSLe2NKHl8o4neneyTK"
type: "task"
status: "done"
title: "T4 ci-issue-tracker-reconcile.yml 新設 + README/dev-env 同期（Issue #368）"
created: "2026-11-06"
updated: "2026-11-06"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/368"
  changes:
    added:
      - type: "file"
        path: ".github/workflows/ci-issue-tracker-reconcile.yml"
    modified:
      - type: "file"
        path: "ci/tracker/README.md"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-368-tracker-reconcile.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-368-tracker-reconcile.md"
    - "docs/tasks/issue-368-t3-tracker-reconcile-mode.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-368-t3-tracker-reconcile-mode.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-368-tracker-reconcile.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# T4 ci-issue-tracker-reconcile.yml 新設 + README/dev-env 同期（Issue #368）

## Work

`.github/workflows/ci-issue-tracker-reconcile.yml` を新設し、`schedule`（`23 */6 * * *` UTC・分 != 0・daily-ci と別時刻）+ `workflow_dispatch` で `tracker.mjs --reconcile` を呼ぶ。job `reconcile` のみ `contents:read`/`actions:read`/`issues:write`・`if: github.event_name == 'schedule' || github.ref_name == github.event.repository.default_branch`（schedule payload では `github.event.repository` が欠落し得るため ref 比較は dispatch 限定）・`timeout-minutes: 15`・checkout を `${{ github.event.repository.default_branch || 'main' }}` に pin・`concurrency` は `ci-issue-tracker-<repo>` で既存 tracker と直列化・`test-ci-tracker` 自己検証を先行・report artifact を `if: always()` で uploadする。`ci/tracker/README.md`（reconcile 入口・手動運用・auto-disable 分離・Consumer 契約）と `docs/development-environment.md`（新 workflow・モード）を同期し、生成 index を再生成する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-368-tracker-reconcile.md`
- `docs/designs/issue-368-tracker-reconcile.md`

## Dependencies

- T3（`--reconcile` の実装が呼出し先）

## Affected Files

- `.github/workflows/ci-issue-tracker-reconcile.yml`（新設）
- `ci/tracker/README.md`（追記）
- `docs/development-environment.md`（追記）
- `docs/{adr,specs,designs,plans,test-specs,tasks}/README.md`（生成 index 再生成）

## Done When

- `ci-issue-tracker-reconcile.yml` が `schedule`+`workflow_dispatch` のみを持ち、`ci-issue-tracker.yml` には差分がない（`git diff` で確認）。
- `mise exec -- actionlint .github/workflows/ci-issue-tracker-reconcile.yml` が pass する。
- `ci/tracker/README.md`・`docs/development-environment.md` が新 workflow・`--reconcile`・auto-disable 分離根拠・Consumer 再利用契約を反映する。
- `mise exec -- prek run --all-files` が pass する。
- 実 cron・実 Issue・実 dispatch は起こさない（offline 検証のみ）。
