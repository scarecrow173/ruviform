---
id: "PLAN-s8XS22tWJoekwN7HhRH9hj"
type: "plan"
status: "approved"
title: "Issue #368 CI failure Issue Tracker 実装計画"
created: "2026-11-06"
updated: "2026-11-06"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/368"
    - "docs/specs/issue-368-ci-issue-tracker.md"
    - "docs/designs/issue-368-ci-issue-tracker.md"
  changes:
    added:
      - type: "file"
        path: ".github/workflows/ci-issue-tracker.yml"
      - type: "directory"
        path: "ci/tracker"
    modified:
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-368-ci-issue-tracker.md"
    - "docs/designs/issue-368-ci-issue-tracker.md"
  implemented-by:
    - "docs/tasks/issue-368-t1-tracker-evaluator.md"
    - "docs/tasks/issue-368-t2-tracker-workflow.md"
  depends-on:
    - "docs/designs/issue-368-ci-issue-tracker.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-368-ci-issue-tracker.md"
    - "docs/designs/issue-368-ci-issue-tracker.md"
  derived-by:
    - "docs/tasks/issue-368-t1-tracker-evaluator.md"
    - "docs/tasks/issue-368-t2-tracker-workflow.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #368 CI failure Issue Tracker 実装計画 Implementation Plan

## Goal

`ci.yml` の完了を `workflow_run` で監視し、main の Full CI 失敗を 1 Issue に集約・coverage 条件で自動 close する tracker を、mock client で完全 offline 検証できる形で実装する。

## Scope

1 本の実装ストリーム。T1（判定器 + contract tests + mise task）→ T2（workflow + README + dev-env 同期）の順。2 つの独立したコミットに分けられるが同一作業単位。

## Upstream Documents

- `docs/specs/issue-368-ci-issue-tracker.md`（SPEC-4KMssIOpNuvh9jGgKKfp7R・approved）

## Design Inputs

- `docs/designs/issue-368-ci-issue-tracker.md`（DESIGN-UGm4KjCwx7pEetENeTzIwB・approved）

## File Map

- `ci/tracker/tracker.mjs` — `classifyRun`（有効 run + conclusion 分類）・`plan`（action list）・`renderBody`/`parseMarker`（marker+sanitize）・`apply`（client 実行）・`makeGhClient`（`gh api`）・`main`（新設）
- `ci/tracker/tests/tracker.test.mjs` — 全判定経路の contract tests（新設・`node --test`）
- `ci/tracker/README.md` — 運用契約（権限不足・label・障害時・手動対応・Consumer 再利用）（新設）
- `.github/workflows/ci-issue-tracker.yml` — `workflow_run` trigger・job-level 最小権限・concurrency 直列・自己検証 step（新設）
- `mise.toml` — `test-ci-tracker` task 追加（`node --test ci/tracker/`）
- `docs/development-environment.md` — tracker workflow・task・運用制約の同期

## Implementation Sequence

- [ ] T1: `ci/tracker/tracker.mjs` + `ci/tracker/tests/tracker.test.mjs` + mise task `test-ci-tracker`。classify 行列（skip/failure/success/operational）・plan 全経路（create/update/stale/dup-canonical/close-guard 行列/operational）・marker roundtrip・sanitize・mock client API 呼出列・API 失敗伝播を `node --test` で検証
- [ ] T2: `ci-issue-tracker.yml` 新設（`workflow_run: {workflows:[ci],types:[completed],branches:[main]}`・job `track` のみ `contents:read`/`actions:read`/`issues:write`・`concurrency` 直列・`test-ci-tracker` 自己検証 step → `tracker.mjs` apply）+ `ci/tracker/README.md` + `docs/development-environment.md` 同期
- [ ] 検証: `mise run test-ci-tracker`・`mise exec -- actionlint .github/workflows/ci-issue-tracker.yml`・`mise exec -- prek run --all-files`・`git diff` で `ci.yml`/`consumer-verify.yml` 無変更確認
- [ ] Conventional Commits で論理単位コミット（push なし）

## Dependencies and Risks

- T2 は T1 の `ci/tracker/` 存在に依存（workflow が参照するため同 commit or 先行 commit）。
- Issue #367（daily-ci scheduler）と並行: `mise.toml`・`docs/development-environment.md` が共有候補ファイル（#367 は `daily-ci.yml` 新設と同 2 ファイルを触る可能性）。`ci/tracker/`・`ci-issue-tracker.yml` は #368 専有。共有ファイルの変更は merge 時に再ベース前提とし、coordinator に報告する。
- GitHub API の実呼出しは行わない（mock client のみ）。`gh` CLI への依存は GitHub-hosted runner 標準プリインストールに限定。
- marker を壊す Issue 手編集の扱い: parse 不可なら foreign 扱い（触れない）で fail-closed。

## Verification Matrix

| 検証 | 入口 | 期待 |
|---|---|---|
| tracker 判定器契約 | `mise run test-ci-tracker` | 全 contract test pass |
| workflow 構文 | `mise exec -- actionlint .github/workflows/ci-issue-tracker.yml` | pass |
| yaml/toml/hook | `mise exec -- prek run --all-files` | pass |
| 既存 trigger 不変 | `git diff` で `ci.yml`/`consumer-verify.yml` | 差分 0 |
| 副作用なし | test が mock client のみ使用・外部通信無し | コードレビュー+fixture で確認 |

## Rollback

- 全変更は worktree 内の git commit。問題時は当該 commit の revert で復帰可能。`ci-issue-tracker.yml` は `workflow_run` 専用の独立ファイルで、revert すれば tracker は完全停止する（既存 CI へ無影響）。

## Task Breakdown

- T1: `docs/tasks/issue-368-t1-tracker-evaluator.md`（TASK-r4Ta2OILbKEFuFRjAfBMAM）
- T2: `docs/tasks/issue-368-t2-tracker-workflow.md`（TASK-Peb0C7F4qBHb9sZP8wXZnR）
