---
id: "TASK-034SmupEbAlO5LQee4I04v"
type: "task"
status: "done"
title: "T3 consumer-impact-check PR 自動起動廃止 + living docs 同期（Issue #366 Stage 1）"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/366"
  changes:
    added: []
    modified:
      - type: "file"
        path: "docs/development-environment.md"
    deleted:
      - type: "file"
        path: ".github/workflows/consumer-impact-check.yml"
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-366-full-ci-stage1.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-366-full-ci-stage1.md"
    - "docs/tasks/issue-366-t2-ci-yml-full-ci.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-366-t2-ci-yml-full-ci.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-366-full-ci-stage1.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# T3 consumer-impact-check PR 自動起動廃止 + living docs 同期（Issue #366 Stage 1）

## Work

Issue #366 実装手順 6 に従い `.github/workflows/consumer-impact-check.yml` を削除する（PR 自動起動の廃止）。同 workflow は `pull_request` 専用で `github.event.pull_request` を読む設計のため `workflow_dispatch` 転用は不能であり、ファイル単位で除去する。Consumer 影響分類要件自体は `PULL_REQUEST_TEMPLATE.md` の表 + レビュアー human gate（`docs/development-environment.md` の運用記述）に残る — required check でないため merge デッドロックは生じない。`docs/development-environment.md` の該当記述を同期する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-366-full-ci-stage1.md`
- `docs/designs/issue-366-full-ci-stage1.md`

## Dependencies

- `docs/tasks/issue-366-t2-ci-yml-full-ci.md`（docs 同期の重複を避けるため後続）

## Affected Files

- `.github/workflows/consumer-impact-check.yml`（削除）
- `docs/development-environment.md`

## Done When

- `.github/workflows/consumer-impact-check.yml` が存在せず PR 起動しない。
- `PULL_REQUEST_TEMPLATE.md` の Consumer 影響分類表は不変で残る（要件の所在）。
- `docs/development-environment.md` が workflow 撤去と human gate 維持を正確に記述する。
- `ci/metrics/collect-ci-metrics.mjs` の workflow 名マッピングは残す（過去 run の分類用途・削除しない）。
