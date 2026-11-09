---
id: "IMPL-bBcD5tUpOGVjPEiV79ah2o"
type: "impl"
status: "completed"
title: "Issue #294 consumer-verify → ci-issue-tracker 配線（scope 別 job・reconcile 全 scope 化・contract test）"
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
      - type: "file"
        path: "docs/impl/ir/issue-294-consumer-verify-tracker-wiring.md"
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
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-294-consumer-verify-tracker-wiring.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-294-consumer-verify-tracker-wiring.md"
    - "docs/plans/issue-294-consumer-verify-tracker-wiring.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-294-consumer-verify-tracker-wiring.md"
    - "docs/test-specs/issue-294-consumer-verify-tracker-wiring.md"
    - "docs/impl/ir/issue-294-consumer-verify-tuple-evidence.md"
    - "docs/impl/ir/issue-294-daily-candidate-tuple-trigger-decide-mjs-tuple-mode-daily-ci-decide-co.md"
    - "docs/impl/ir/issue-368-t2-tracker-workflow-implementation.md"
    - "docs/designs/issue-367-daily-scheduler.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-294-consumer-verify-tracker-wiring.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #294 consumer-verify → ci-issue-tracker 配線（scope 別 job・reconcile 全 scope 化・contract test）

## Summary

consumer-verify の失敗 run を既存 ci-issue-tracker 機構で Issue 追跡できるように配線した。`ci-issue-tracker.yml` の `workflow_run` trigger を `workflows: [ci, consumer-verify]` に拡張し、`workflow_run.name` でゲートされた scope 別 job（`track` / `track-consumer-verify`）がそれぞれ `--workflow` / `--workflow-file` で scope を明示して同一 evaluator を実行する。`ci-issue-tracker-reconcile.yml` は GITHUB_TOKEN dispatch の anti-recursion 補完として `ci` → `consumer-verify` の 2 scope を逐次実行する。`tracker.mjs` は scope 由来の issue title（`issueTitle(scope)`）と `classifyRun` の workflow identity 照合（name/path）を追加し、Issue 本文・triage/recovery コメントを scope 対応にした。新規 framework は追加していない。

## Context

- 実施タスク: `docs/tasks/issue-294-consumer-verify-tracker-wiring.md`（`docs/plans/issue-294-consumer-verify-tracker-wiring.md` 配下）
- 上流: `docs/specs/issue-294-consumer-verify-tracker-wiring.md`・`docs/designs/issue-294-consumer-verify-tracker-wiring.md`・`docs/test-specs/issue-294-consumer-verify-tracker-wiring.md`
- 先行実装: `docs/impl/ir/issue-294-consumer-verify-tuple-evidence.md`（証拠契約）・`docs/impl/ir/issue-294-daily-candidate-tuple-trigger-decide-mjs-tuple-mode-daily-ci-decide-co.md`（daily dispatch・GITHUB_TOKEN anti-recursion の発生源）・`docs/impl/ir/issue-368-t2-tracker-workflow-implementation.md`（tracker workflow 本体）
- tracker 評価器は `tracker.mjs` 1 本・scope `(repo, workflow, branch)`・marker tag `ci-issue-tracker`・label `ci-failure`・history limit 10 を共有し、`ci` scope の既存挙動は不変。
- consumer-verify の verdict（`pass`/`fail`/`not_run`/`unsupported`/`skipped`）は workflow run の conclusion として tracker に届くため、tracker 側は conclusion のみを評価する。`not_run`/`unsupported` は workflow run conclusion の `failure`/`timed_out`/`startup_failure` に写像されず、Issue 起票対象にならない（contract test で固定）。`gate=blocked → fail` の sanctioned 写像は consumer-verify evidence 側の既存挙動であり、本 slice では変更していない。

## Implementation

### Changes

- `.github/workflows/ci-issue-tracker.yml` — `workflows: [ci]` → `[ci, consumer-verify]`（`types: [completed]`、`branches: [main]` は不変）。既存 `track` job に `if: github.event.workflow_run.name == 'ci'` を追加し、新規 `track-consumer-verify` job を追加（同条件の `== 'consumer-verify'`）。consumer job は `contents: read` / `actions: read` / `issues: write` で `mise exec -- node ci/tracker/tracker.mjs --workflow consumer-verify --workflow-file consumer-verify.yml --report ci-issue-tracker-report-consumer-verify.json` を実行し、同一 self-test `mise run test-ci-tracker` を通し、`ci-issue-tracker-report-consumer-verify-attempt-<run_attempt>` artifact を upload する。
- `.github/workflows/ci-issue-tracker-reconcile.yml` — コメントを 2 scope 構成に更新。reconcile step は既定 `ci` scope を実行後、同一 step 内で `--workflow consumer-verify --workflow-file consumer-verify.yml` の 2 回目を実行する。`set -uo pipefail` + `rc` 蓄積で 1 scope の失敗が他 scope を starve せず、両 report を artifact upload する。concurrency group・permissions は不変。
- `ci/tracker/tracker.mjs` — `WORKFLOW_TITLES`（`ci` → `Full CI`、`consumer-verify` → `consumer-verify`）と export 関数 `issueTitle(scope)` を追加し、Issue create/update の title を scope 対応にした（`ci` の既存 title `[CI failure][main] Full CI failed` は不変）。`classifyRun` に workflow identity 照合を追加: `workflow_run.path` または `workflow_run.name` の少なくとも一方が必須で、存在する path は `scope.workflowFile`（末尾一致も可）・name は `scope.workflow` と一致しない場合 skip。Issue 本文・triage/recovery コメントの workflow 参照を `scope.workflow` 駆動に変更。`parseTrackerArgs` は既存の `--workflow`/`--workflow-file` をそのまま利用（新規引数なし）。
- `ci/tracker/tests/tracker.test.mjs` — consumer-verify scope の contract test を追加: scope 解決、workflow name/path 照合（path 末尾一致・name 一致・不一致 skip）、history 分類の scope 分離、consumer-verify failure run → Issue 起票 plan、success run → close 判定、Issue title/marker scope 分離。
- `ci/tracker/tests/issue-tracker-workflows-contract.test.mjs` — 新規。両 workflow YAML の trigger/job/permission/report 契約を検査（`workflows: [ci, consumer-verify]`、2 job の `if` ゲート、consumer job の tracker 引数、reconcile の 2 scope 実行と `rc` 蓄積、artifact 名）。
- `ci/tracker/README.md` — 2 scope 構成・scope 別 Issue title・workflow identity 照合・両 workflow の anti-recursion 補完・verdict/conclusion の対応を記述。
- `docs/development-environment.md` — tracker task 表と workflow 節を 2 scope 構成に同期。
- docs チェーン — `docs/specs|designs|plans|test-specs|tasks/issue-294-consumer-verify-tracker-wiring.md` を新規作成し、doc-maintenance で管理索引 5 本（specs/designs/plans/test-specs/tasks README）を再生成。

### Design Decisions

- **拡張点は `workflow_run` trigger + reconcile の 2 面**: daily-ci.yml は GITHUB_TOKEN で consumer-verify を dispatch するため anti-recursion で `workflow_run` completed が発火しない。event 経路だけでは daily dispatch 起因の失敗を取りこぼすため、reconcile が権威ある run history を走査する既存境界をそのまま 2 scope 化した（新 framework なし）。
- **scope 分離は job レベルの `if` ゲート + evaluator 引数**: 1 job 内で両 workflow を処理すると `github.event.workflow_run` がどちらの run か曖昧になるため、workflow 名で job を分け、それぞれが自 scope の `--workflow`/`--workflow-file` を明示する。`classifyRun` の name/path 照合は defense-in-depth として scope 外 run の混入を fail-closed で skip する。
- **Issue title は scope 由来**: `ci` と `consumer-verify` が同一 Issue に混ざらないよう `[CI failure][<branch>] <workflow> failed` を `issueTitle(scope)` で生成。marker tag/label/history 上限は scope 内で共有する既存規約を維持。
- **verdict → tracker 入力の写像は conclusion のみ**: tracker は evidence JSON の verdict を読まず workflow run の conclusion を見る。`not_run`/`unsupported` は consumer-verify.yml が failure conclusion を出さない限り tracker の失敗集合（`failure`/`timed_out`/`startup_failure`）に入らず、success でもない（close は既存の「新しい一意な success が失敗 revision と main tip を cover」規約のみ）。contract test でこの境界を固定した。

## Validation

| Check | Result |
| --- | --- |
| `mise run test-ci-tracker` | pass — 108 tests（tracker.test.mjs + issue-tracker-workflows-contract.test.mjs） |
| `mise run test-ci-consumer-verify` | pass — 64 tests |
| `mise run fmt-check` | pass |
| `mise run check` | pass（cargo check --workspace --all-targets） |
| `mise run lint` | pass（cargo clippy --workspace --all-targets -- -D warnings） |
| `mise run test` | pass（cargo test --workspace、全クレート + doctest） |
| `mise run test-ci-daily` | pass — 111 tests |
| `mise exec -- prek run --all-files` | pass |
| `mise exec -- actionlint .github/workflows/ci-issue-tracker.yml .github/workflows/ci-issue-tracker-reconcile.yml` | pass（exit 0） |
| `node .claude/skills/doc-maintenance/scripts/doc_maintenance.js apply --json` | applied — 5 managed index 再生成（findings 650 → 635、残件は既存の manual/migration 分類） |

## NotRun / 残件

- 実 daily dispatch の実走証跡: NotRun（run API 制限・ops 事項）。reconcile の consumer-verify scope は次回 scheduled/manual 実行で初めて実データを処理する。
- both-consumer-pass の実測: NotRun（audio-lens 側 export portability 修正待ち、#294 umbrella の別 slice）。
- `CROSS_REPO_DEPLOY_KEY` 等 credential 設定: NotRun（人手の credential 作業）。
- GUI-VISUAL / OS lanes・`PLUGIN-NATIVE-DOCK`: NotRun（実機 gate、#313/#314 系）。
- umbrella Issue #294 の close: 対象外（`Refs #294` のみ）。
- doc-driven-dev-graph router は既存の repository-wide broken relation（Issue #243/#313/#410 系文書）で `blocked`/`focus-required` を返す。本 slice の新規文書は索引・relations とも整合済みで、既存 graph 欠陥は本タスクの範囲外として未修復。
