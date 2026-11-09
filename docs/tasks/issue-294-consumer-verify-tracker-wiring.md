---
id: "TASK-1t4MTPYFiTORT83MpcEqRB"
type: "task"
status: "done"
title: "Issue #294 consumer-verify → ci-issue-tracker 配線実装 — scope 別 job・reconcile 全 scope 化・contract test"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/294"
  changes:
    added:
      - type: "file"
        path: "ci/tracker/tests/issue-tracker-workflows-contract.test.mjs"
    modified:
      - type: "file"
        path: "ci/tracker/tracker.mjs"
      - type: "file"
        path: "ci/tracker/tests/tracker.test.mjs"
      - type: "file"
        path: "ci/tracker/README.md"
      - type: "workflow"
        path: ".github/workflows/ci-issue-tracker.yml"
      - type: "workflow"
        path: ".github/workflows/ci-issue-tracker-reconcile.yml"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-294-consumer-verify-tracker-wiring.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-294-consumer-verify-tracker-wiring.md"
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
    - "docs/test-specs/issue-294-consumer-verify-tracker-wiring.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #294 consumer-verify → ci-issue-tracker 配線実装 — scope 別 job・reconcile 全 scope 化・contract test

## Work

`ci-issue-tracker.yml` の `workflow_run.workflows` を `[ci, consumer-verify]`
に拡張し scope 別 job（`track`/`track-consumer-verify`、`workflow_run.name`
gate）を追加。`ci-issue-tracker-reconcile.yml` の reconcile を `ci` →
`consumer-verify` の順に同一判定器で逐次実行（`|| rc=1` 集約）。
`tracker.mjs` は `issueTitle(scope)`（`ci` title byte 一致維持）・
`classifyRun` の workflow `name`/`path` scope 照合・scope 対応文言を実装。
contract test（scope 分離・identity・reconcile 回復・YAML 構造・verdict
写像）を追加し、README/development-environment.md を同期する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-294-consumer-verify-tracker-wiring.md`
- `docs/designs/issue-294-consumer-verify-tracker-wiring.md`
- `docs/specs/issue-294-consumer-verify-tracker-wiring.md`
- `docs/test-specs/issue-294-consumer-verify-tracker-wiring.md`

## Dependencies

- `ci/tracker/tracker.mjs` 既存 scope 機構（`--workflow`/`--workflow-file`・
  marker scope・`classifyHistoryRecord` の path/name フィルタ — main 済み）
- `consumer-verify.yml` の workflow conclusion 契約（summary fail-closed —
  main 済み・変更なし前提）
- daily-ci の GITHUB_TOKEN dispatch（anti-recursion gap の根拠 — main 済み）

## Affected Files

- `ci/tracker/tracker.mjs`
- `ci/tracker/tests/tracker.test.mjs`
- `ci/tracker/tests/issue-tracker-workflows-contract.test.mjs`（新規）
- `.github/workflows/ci-issue-tracker.yml`
- `.github/workflows/ci-issue-tracker-reconcile.yml`
- `ci/tracker/README.md`
- `docs/development-environment.md`

## Done When

- [x] `tracker.mjs` が scope 別 `issueTitle`・payload workflow identity
  照合・scope 対応文言を持ち、`ci` scope 既存テストが無変更で pass
- [x] `ci-issue-tracker.yml` が両 workflow を監視し scope 別 2 job で
  gate される
- [x] `ci-issue-tracker-reconcile.yml` が両 scope を逐次 reconcile する
- [x] contract test（tracker.test.mjs scope 群 +
  issue-tracker-workflows-contract.test.mjs）が green
- [x] `docs/development-environment.md`・`ci/tracker/README.md` が同期
- [x] impl record（docs/impl/）が作成される

## Verification

- [x] `mise run test-ci-tracker`（108 tests pass — `node --test ci/tracker/`）
- [x] `mise exec -- actionlint .github/workflows/ci-issue-tracker.yml .github/workflows/ci-issue-tracker-reconcile.yml`（exit 0）
- [ ] `mise run fmt-check` / `mise run check` / `mise run lint` / `mise run test`
- [ ] `mise run test-ci-consumer-verify`
- [ ] `mise exec -- prek run --all-files`
