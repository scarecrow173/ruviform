---
id: "TASK-034QdVap6IcvMjOYnaWzCR"
type: "task"
status: "done"
title: "T3 AGENTS.md の ci.yml 記述 drift 修正"
created: "2026-11-01"
updated: "2026-11-01"
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
    - "docs/plans/issue-219-consumer-cross-repo-ci-gate.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-219-consumer-cross-repo-ci-gate.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# T3 AGENTS.md の ci.yml 記述 drift 修正

## Work

`AGENTS.md` L146 前後の `.github/workflows/ci.yml` 記述（「`workflow_dispatch` 専用の手動検証ワークフロー（push / pull_request / schedule 等の自動トリガーは設けない）」）を現行実態に修正する。現行: `pull_request`/`push`（main）/`workflow_dispatch` trigger、required checks 12 件（verify×3・l1-real-verify×3・ui-verify×3・rt-verify・icc-verify・bench）。job 構成の列挙も現行 ci.yml に合わせる。Consumer 影響分類の運用責務（`unknown` の merge 時点 human gate）も合わせて記述する。

## Classification

- Type: implementation
- Size: XS

## Upstream Documents

- `docs/plans/issue-219-consumer-cross-repo-ci-gate.md`
- `docs/designs/issue-219-cross-repo-ci-gate-credential-runbook.md`

## Dependencies

なし

## Affected Files

- `AGENTS.md`

## Done When

- stale 記述が現行 trigger・job・required checks と一致
- Consumer 影響分類の merge 時点 human gate 運用責務が記述される（AC10・design §PR template 注記と整合）

## Verification

- [ ] `.github/workflows/ci.yml` の `on:`・job 名と照合
- [ ] `gh api` の required contexts 12 件と照合
