---
id: "PLAN-r5yBO16zsFa4LSjxB9grp1"
type: "plan"
status: "approved"
title: "Issue #367 共通 Daily Scheduler 実装計画"
created: "2026-11-06"
updated: "2026-11-06"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/367"
    - "docs/specs/issue-367-daily-scheduler.md"
    - "docs/designs/issue-367-daily-scheduler.md"
  changes:
    added:
      - type: "directory"
        path: "ci/daily"
      - type: "file"
        path: ".github/workflows/daily-ci.yml"
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
    - "docs/specs/issue-367-daily-scheduler.md"
    - "docs/designs/issue-367-daily-scheduler.md"
  implemented-by:
    - "docs/tasks/issue-367-t1-decide-evaluator.md"
    - "docs/tasks/issue-367-t2-daily-ci-workflow.md"
    - "docs/tasks/issue-367-t3-consumer-contract-docs.md"
  depends-on:
    - "docs/designs/issue-367-daily-scheduler.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-367-daily-scheduler.md"
    - "docs/designs/issue-367-daily-scheduler.md"
  derived-by:
    - "docs/tasks/issue-367-t1-decide-evaluator.md"
    - "docs/tasks/issue-367-t2-daily-ci-workflow.md"
    - "docs/tasks/issue-367-t3-consumer-contract-docs.md"
    - "docs/test-specs/issue-367-daily-scheduler.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-367-daily-scheduler.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #367 共通 Daily Scheduler 実装計画 Implementation Plan

## Goal

`daily-ci.yml` + `ci/daily/decide.mjs` を実装し、「現在 main SHA に有効な Full CI がなければ `ci.yml` を `workflow_dispatch` で起動する」Daily scheduler を fixture 検証付きで導入する（docs/designs/issue-367-daily-scheduler.md の判定・dispatch・確認契約どおり）。

## Scope

1 本の実装ストリーム。T1（判定器 + tests + mise task）→ T2（daily-ci.yml）→ T3（consumer 採用契約・運用 docs 同期）の順。T2 は T1 の成果物に依存。

## Upstream Documents

- `docs/specs/issue-367-daily-scheduler.md`（SPEC-ElrrvM1k7RbACtLgfzKaxs・approved）

## Design Inputs

- `docs/designs/issue-367-daily-scheduler.md`（DESIGN-LKKyu5XTONFK84RN45ZdM2・approved）

## File Map

- `ci/daily/decide.mjs` — 判定コア（pure）+ GitHub API client + orchestration + fixture/--dry-run + report/summary 出力（新設）
- `ci/daily/tests/decide.test.mjs` — fixture 駆動 contract tests（新設）
- `.github/workflows/daily-ci.yml` — UTC cron + workflow_dispatch・単一 job・最小権限・concurrency（新設）
- `mise.toml` — `[tasks.test-ci-daily]` 追加
- `docs/development-environment.md` — daily-ci 構成・運用手順・consumer 採用契約・権限の同期

## Implementation Sequence

- [x] T1: `ci/daily/decide.mjs` + `ci/daily/tests/decide.test.mjs` + `test-ci-daily` mise task。判定行列（skip-verified / suppress-in-flight / hold-unknown / budget-exhausted / dispatch）、attemptsOn 厳格性、auto dispatch 計上（bot actor のみ）、latestVerified、dispatch 確認（confirmed/landed-mismatch/unconfirmed）、main-moved 再照合、error/exit code、report+summary 形状を `node --test` で検証
- [x] T2: `.github/workflows/daily-ci.yml` 新設 — `schedule`（UTC `"17 21 * * *"` = 06:17 JST）+ `workflow_dispatch`（`dry_run` 入力）・`permissions: {}` → job `contents: read`/`actions: write`・`concurrency`（直列・非 cancel）・`test-ci-daily` 先行・report artifact
- [x] T3: `docs/development-environment.md` に daily-ci 構成・運用手順（手動再判定/dry_run/最終成功時刻/budget-exhausted 復帰）・Consumer 採用契約・権限最小化を追記
- [ ] 検証: `mise run test-ci-daily`・`mise exec -- actionlint .github/workflows/daily-ci.yml`・`mise exec -- prek run --all-files`・残留参照 grep
- [ ] Conventional Commits で論理単位コミット（push なし）

## Dependencies and Risks

- T2 は T1 の `ci/daily/` + `test-ci-daily` 存在に依存（workflow が参照するため同 commit or 先行 commit）。
- #368（ci-issue-tracker）並行実装: 共有ファイル候補は `mise.toml`・`docs/development-environment.md`・`docs/*/README.md` index・`.github/workflows/` 新規 file（名前衝突なし — `daily-ci.yml` vs `ci-issue-tracker.yml`）。scheduler は Issue を作らない境界を維持。
- 実 dispatch 副作用ゼロ — 全検証 fixture。実 API read（run 履歴の field 実測）は read-only で実施済み。

## Verification Matrix

| 検証 | 入口 | 期待 |
|---|---|---|
| 判定器契約 | `mise run test-ci-daily` | fixture 行列全 pass |
| workflow 構文 | `mise exec -- actionlint .github/workflows/daily-ci.yml` | pass |
| yaml/toml・hook | `mise exec -- prek run --all-files` | pass |
| ci.yml 非変更 | `git diff` `.github/workflows/ci.yml` | trigger/inputs/job 無変更 |
| DB/変数禁止 | `git grep` repository variable・secret 追加 | 0 件 |
| 副作用ゼロ | fixture のみ・POST dispatch は mock | 実 dispatch 0 |

## Rollback

- 全変更は worktree 内 git commit。`daily-ci.yml` + `ci/daily/` は新規のため revert で完全撤去可能。`ci.yml`・ruleset・required context は不変。

## Task Breakdown

- T1: `docs/tasks/issue-367-t1-decide-evaluator.md`（TASK-OMt1jAp3OOLOSc40FVZTD0）
- T2: `docs/tasks/issue-367-t2-daily-ci-workflow.md`（TASK-6BTRFSnTERjkzGCwtaEgAs）
- T3: `docs/tasks/issue-367-t3-consumer-contract-docs.md`（TASK-xJy29ZBdKUklv4RDyz58A3）
