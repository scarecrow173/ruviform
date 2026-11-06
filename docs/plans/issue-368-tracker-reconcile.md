---
id: "PLAN-WuOx4rrntClDKCSVPDAM8d"
type: "plan"
status: "approved"
title: "Issue #368 Tracker reconcile 実装計画"
created: "2026-11-06"
updated: "2026-11-06"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/368"
    - "docs/specs/issue-368-tracker-reconcile.md"
    - "docs/designs/issue-368-tracker-reconcile.md"
  changes:
    added:
      - type: "file"
        path: ".github/workflows/ci-issue-tracker-reconcile.yml"
    modified:
      - type: "file"
        path: "ci/tracker/tracker.mjs"
      - type: "file"
        path: "ci/tracker/tests/tracker.test.mjs"
      - type: "file"
        path: "ci/tracker/README.md"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-368-tracker-reconcile.md"
    - "docs/designs/issue-368-tracker-reconcile.md"
  implemented-by:
    - "docs/tasks/issue-368-t3-tracker-reconcile-mode.md"
    - "docs/tasks/issue-368-t4-reconcile-workflow.md"
  depends-on:
    - "docs/designs/issue-368-tracker-reconcile.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-368-tracker-reconcile.md"
    - "docs/designs/issue-368-tracker-reconcile.md"
  derived-by:
    - "docs/tasks/issue-368-t3-tracker-reconcile-mode.md"
    - "docs/tasks/issue-368-t4-reconcile-workflow.md"
    - "docs/test-specs/issue-368-tracker-reconcile.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-368-tracker-reconcile.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #368 Tracker reconcile 実装計画 Implementation Plan

## Goal

`GITHUB_TOKEN` dispatch の完了が `workflow_run` を発火させない gap を、event 非依存の `--reconcile` モード（同一判定器・権威履歴 + marker のみ）と別 workflow の `schedule`/`workflow_dispatch` 入口で埋める。`ci-issue-tracker.yml` には触れず、既存 fail-closed 契約・最小権限・既存 secret のみで実装する。

## Scope

1 本の実装ストリーム。T3（`tracker.mjs` reconcile モード + contract tests）→ T4（reconcile workflow + README/dev-env 同期）の順。docs チェーン（spec/ADR/design/plan/test-spec/task）と生成 index の更新を含む。

## Upstream Documents

- `docs/specs/issue-368-tracker-reconcile.md`（SPEC-9R6tNeU4aiEuUseUczer9o・approved）
- `docs/adr/issue-368-tracker-reconcile-schedule-entry.md`（ADR-vJvyZNtrHi2xnycJxbnSPv・accepted — ADR-034SraR8CrkZ0SSa7tQEwh trigger 禁止の補足）

## Design Inputs

- `docs/designs/issue-368-tracker-reconcile.md`（DESIGN-StMgstUBs2Df2UPm1oy2GF・approved）
- `docs/designs/issue-368-ci-issue-tracker.md`（本体設計・変更しない共通経路の正本）

## File Map

- `ci/tracker/tracker.mjs` — `parseTrackerArgs` 分離 export・`--reconcile` フラグ・`run({reconcile})` で `classified={kind:"reconcile"}`・report/summary に `mode` 追加・usage/header 追記
- `ci/tracker/tests/tracker.test.mjs` — reconcile 契約 test + `parseTrackerArgs` mode 分岐 test（新規追加・`node --test`）
- `.github/workflows/ci-issue-tracker-reconcile.yml` — `schedule`+`workflow_dispatch`・同一 concurrency group・最小権限・default branch pin・self-test 先行（新設）
- `ci/tracker/README.md` — reconcile 入口・手動運用・auto-disable 分離根拠・Consumer 契約の追記
- `docs/development-environment.md` — reconcile workflow・reconcile モードの同期
- `docs/adr/README.md`・`docs/specs/README.md`・`docs/designs/README.md`・`docs/plans/README.md`・`docs/test-specs/README.md`・`docs/tasks/README.md` — 生成 index の再生成

## Implementation Sequence

- [ ] T3: `tracker.mjs` の `parseTrackerArgs` 分離・`--reconcile`・`run({reconcile})`・`mode` 出力 + `tests/tracker.test.mjs` へ reconcile 契約 test 追加。**先に RED（reconcile test が fail）を作り、実装で GREEN にする**。
- [ ] T4: `ci-issue-tracker-reconcile.yml` 新設 + `ci/tracker/README.md`・`docs/development-environment.md` 同期 + 生成 index 再生成。
- [ ] 検証: `mise run test-ci-tracker`・`mise run test-ci-daily`・`mise run test-ci-summary`・`mise exec -- actionlint .github/workflows/ci-issue-tracker-reconcile.yml`（+ 変更無し確認として `ci-issue-tracker.yml` も lint）・`mise exec -- prek run --all-files`・`git diff` で既存 workflow 4 本の無変更確認。
- [ ] Conventional Commits（日本語）で論理単位コミット → branch push → 日本語 PR 作成。

## Dependencies and Risks

- T4 は T3 の `--reconcile` 実装に依存（workflow が呼ぶため）。同一 PR 内で整合。
- `docs/adr/README.md` 他の生成 index は doc-maintenance の `rebuild-index` で再生成する（手編集しない）。
- `ci-issue-tracker.yml` を含む既存 workflow 4 本は無変更 — `git diff` で不変を確認する（trigger・権限・job 構成）。
- 実 cron・実 Issue・実 dispatch は行わない — 全て mock client + actionlint + `git diff` の offline 検証。実 cron 受入は merge 後の別工程。
- `TRACKER_EVENT_PATH`/`GITHUB_EVENT_PATH` の reconcile 無視を test で pin する — ambient env の存在を error にしない規則を誤って厳しくしない。

## Verification Matrix

| 検証 | 入口 | 期待 |
|---|---|---|
| tracker 判定器契約（event + reconcile） | `mise run test-ci-tracker` | 全 contract test pass（reconcile 契約含む） |
| daily scheduler 契約（無変更の回帰確認） | `mise run test-ci-daily` | pass |
| ci-summary 契約（無変更の回帰確認） | `mise run test-ci-summary` | pass |
| workflow 構文 | `mise exec -- actionlint .github/workflows/ci-issue-tracker-reconcile.yml` | pass |
| 既存 workflow 不変 | `git diff` で 4 本 | 差分 0（新規ファイル追加のみ） |
| yaml/toml/hook | `mise exec -- prek run --all-files` | pass |
| 副作用なし | test が mock client のみ使用・外部通信なし | コードレビュー+fixture で確認 |

## Rollback

- 全変更は worktree 内の git commit。問題時は当該 commit の revert で復帰可能。`ci-issue-tracker-reconcile.yml` の revert は reconcile 入口のみを止め、event 入口・既存 CI には無影響。

## Task Breakdown

- T3: `docs/tasks/issue-368-t3-tracker-reconcile-mode.md`（TASK-6LaGohoePjG92Eb5u6lUv7）
- T4: `docs/tasks/issue-368-t4-reconcile-workflow.md`（TASK-T6nZSLe2NKHl8o4neneyTK）
