---
id: "TASK-r4Ta2OILbKEFuFRjAfBMAM"
type: "task"
status: "done"
title: "T1 ci/tracker/tracker.mjs 判定器 + contract tests（Issue #368）"
created: "2026-11-06"
updated: "2026-11-06"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/368"
  changes:
    added:
      - type: "directory"
        path: "ci/tracker"
    modified:
      - type: "file"
        path: "mise.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-368-ci-issue-tracker.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-368-ci-issue-tracker.md"
  blocks:
    - "docs/tasks/issue-368-t2-tracker-workflow.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-368-t2-tracker-workflow.md"
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

# T1 ci/tracker/tracker.mjs 判定器 + contract tests（Issue #368）

## Work

`ci/tracker/tracker.mjs`（stdlib-only・client 注入型）と `ci/tracker/tests/tracker.test.mjs`（`node --test`・fixture/mock のみ・外部通信なし）を新設し、mise task `test-ci-tracker` を追加する。`ci/summary/summary.mjs` の入力規約（env + `--flag`・`--report`/`--summary` 出力）を踏襲する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-368-ci-issue-tracker.md`
- `docs/designs/issue-368-ci-issue-tracker.md`
- `docs/test-specs/issue-368-ci-issue-tracker.md`

## Dependencies

- なし（先行タスクなし。T2 が本タスクの成果物に依存する）

## Affected Files

- `ci/tracker/tracker.mjs`（新設）
- `ci/tracker/tests/tracker.test.mjs`（新設）
- `mise.toml`（`test-ci-tracker` 追加）

## Done When

- `classifyRun` が有効 run フィルタ（repo・branch・event allowlist・fork 除外・`action: completed`）と conclusion 分類（`success`→success・`failure`/`timed_out`/`startup_failure`→failure・`cancelled`/`skipped`/`neutral`/`stale`/`action_required`/null/未知→operational）を返す。
- `plan` が Issue 一覧+compare+run メタから action list（`create`/`update`/`close`/`noop`）を返し、完了順逆転（同 run は `run_attempt`・run 間は `updated_at` の順序付けで stale 判定・同 id 失敗 a1→成功 a2 rerun は正当な回復）・複数 open canonical 集約（最小番号・duplicate close）・close guard（最新証拠が記録最新失敗より曖昧なく新しい成功 ∧ `compare(失敗 sha→成功 sha)`・`compare(現在 main head→成功 sha)` がともに `{identical,ahead}` ∧ pending/unknown 履歴なし）・foreign Issue 非接触を判定する。
- `renderBody`/`parseMarker` が scope marker（`<!-- ci-issue-tracker:{json} -->`）を往復し、sha/URL/job 名/conclusion を sanitize する。
- `run(event, client)` orchestrator が ensureLabel→list→jobs→compare（close 判定時のみ）→apply の順で注入 client を呼び、API 失敗がそのまま伝播する。
- `makeGhClient` が `gh api` で label/list/jobs/compare/create/update/comment/close を実装する（実呼出しは workflow のみ。テストは mock client）。
- `mise run test-ci-tracker`（`node --test ci/tracker/`）が exit 0。
