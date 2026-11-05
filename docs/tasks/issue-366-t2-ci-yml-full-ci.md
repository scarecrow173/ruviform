---
id: "TASK-034SmuoocFyRqTmasB818H"
type: "task"
status: "done"
title: "T2 ci.yml Full CI 化 + io-structured-verify lane + 選択実行機構撤去（Issue #366/#364 Stage 1）"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/366"
    - "https://github.com/scarecrow173/ruviform/issues/364"
  changes:
    added: []
    modified:
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/development-environment.md"
    deleted:
      - type: "directory"
        path: "ci/impact"
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-366-full-ci-stage1.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-366-full-ci-stage1.md"
    - "docs/tasks/issue-366-t1-ci-summary-evaluator.md"
  blocks:
    - "docs/tasks/issue-366-t3-consumer-impact-removal.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-366-t1-ci-summary-evaluator.md"
    - "docs/tasks/issue-366-t3-consumer-impact-removal.md"
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

# T2 ci.yml Full CI 化 + io-structured-verify lane + 選択実行機構撤去（Issue #366/#364 Stage 1）

## Work

`ci.yml` を選択実行機構なしの Full CI へ変換する。`impact` job・全 `needs: [impact]`・全 lane `if:`・`CI_IMPACT_DISABLE` env を撤去し、`io-structured-verify` job を新設、`macos-report`・`ci-gate` を `ci-summary`（needs 11 件・`if: always()`）へ置換、`workflow_dispatch` に `expected_sha` 入力を追加する。`ci/impact/` と mise task `test-ci-impact` を削除し、`docs/development-environment.md` を同期する。required context を産む `verify`/`l1-real-verify`/`ui-verify` の job id・matrix os 配列・`name:` は不変。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-366-full-ci-stage1.md`
- `docs/designs/issue-366-full-ci-stage1.md`

## Dependencies

- `docs/tasks/issue-366-t1-ci-summary-evaluator.md`（workflow が `ci/summary/` を参照するため先行）

## Affected Files

- `.github/workflows/ci.yml`
- `ci/impact/`（削除。`ci/metrics/`・`ci/verify-linux/` は残す）
- `mise.toml`（`test-ci-impact` 削除・stale comment 更新）
- `docs/development-environment.md`

## Done When

- `ci.yml` に `impact` job・`needs.impact`・lane `if:`・`CI_IMPACT_DISABLE` が存在せず、全 11 lane が無条件実行される。
- `io-structured-verify` job が `mise run check-io-structured`/`test-io-structured`/`lint-io-structured` を実行する。
- `ci-summary` job が 11 needs + `if: always()` + `test-ci-summary` 自己検証 + `summary.mjs` 評価 + `ci-summary-report` artifact を持つ。
- `workflow_dispatch.inputs.expected_sha` が存在する。
- `verify`/`l1-real-verify`/`ui-verify` の check 名（job id・matrix・`name:`）が不変。
- `mise exec -- actionlint .github/workflows/ci.yml` pass・残留参照 grep 0 件（履歴 docs 除く）・`docs/development-environment.md` 同期済み。
