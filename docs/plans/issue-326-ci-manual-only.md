---
id: "PLAN-lb9Tb7cIysOfXayRz4GJY8"
type: "plan"
status: "approved"
title: "Issue #326 ci.yml dispatch 専用化 実装計画"
created: "2026-11-06"
updated: "2026-11-06"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/326"
    - "docs/specs/issue-326-ci-manual-only.md"
    - "docs/designs/issue-326-ci-manual-only.md"
  changes:
    added:
      - type: "file"
        path: "ci/daily/tests/ci-yml-contract.test.mjs"
    modified:
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "docs/development-environment.md"
      - type: "file"
        path: "docs/adr/issue-328-ci-concurrency-grouping.md"
      - type: "file"
        path: "mise.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-326-ci-manual-only.md"
    - "docs/designs/issue-326-ci-manual-only.md"
  implemented-by:
    - "docs/tasks/issue-326-t1-ci-yml-dispatch-only.md"
    - "docs/tasks/issue-326-t2-trigger-contract-test.md"
  depends-on:
    - "docs/designs/issue-326-ci-manual-only.md"
    - "docs/adr/issue-326-ci-yml-dispatch-only.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-366-full-ci-stage1.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-326-ci-manual-only.md"
    - "docs/designs/issue-326-ci-manual-only.md"
  derived-by:
    - "docs/tasks/issue-326-t1-ci-yml-dispatch-only.md"
    - "docs/tasks/issue-326-t2-trigger-contract-test.md"
    - "docs/test-specs/issue-326-ci-manual-only.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-326-ci-manual-only.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #326 ci.yml dispatch 専用化 実装計画

## Summary

`ci.yml` を `workflow_dispatch` 専用にし、PR 条件 concurrency 機構を簡素化し、workflow 横断の trigger 不存在を contract test で固定する。変更対象は `ci.yml`（trigger + concurrency + コメント）・`ci/daily/tests/ci-yml-contract.test.mjs`（新規）・`docs/development-environment.md`（同期）のみ — 実行系・他 4 workflow・ruleset は不変。

## Tasks

- [ ] T1: `ci.yml` dispatch 専用化 + concurrency 簡素化 + コメント同期 → `docs/tasks/issue-326-t1-ci-yml-dispatch-only.md`
- [ ] T2: trigger contract test + dev-env 同期 → `docs/tasks/issue-326-t2-trigger-contract-test.md`

## Steps

1. `ci.yml`: `on:` から `pull_request`/`push` 除去 → `workflow_dispatch`（`expected_sha` 宣言不変）+ header/expected_sha/pin/transition-note コメント同期 + concurrency を `ci-<run_id>`・`cancel-in-progress:false` に簡素化。
2. `ci/daily/tests/ci-yml-contract.test.mjs`: `.github/workflows/*.yml` を stdlib scan — (a) 全ファイルに `pull_request:`/`pull_request_target:`/`push:` trigger 0 件、(b) `ci.yml` に `workflow_dispatch:`+`expected_sha:`、(c) `ci-issue-tracker.yml` に `workflow_run:`。
3. `docs/development-environment.md` の `ci.yml` trigger/concurrency 記述を同期（checkstatus disabled・dispatch 専用・簡素 concurrency・ADR-SYd9kRt5mEssMKoPG8hnSG 参照）。
4. 検証: `actionlint .github/workflows/*.yml` + `mise run test-ci-daily`（新 test 同梱）+ `test-ci-tracker`/`test-ci-summary` 無回帰 + `prek run --all-files` + 4 workflow `git diff`=0。
5. push → 日本語 PR → `gh pr checks`/`gh api .../check-suites`/`gh run list --event pull_request` で pull_request 起動 0 件を記録（実挙動を正直に — head/base どちらの file で評価されるかも記録）→ head SHA・時刻・実証結果を worker_done。

## Dependencies and Ordering

- 前提（充足済み・実測）: `main-checkstatus` disabled（Ruleset 26547674）・`main-protect` active（26529830）・Classic 404・reconcile main 成功 run 42178261480・`ci-issue-tracker-reconcile` workflow active（id 409034138）。
- T1 → T2 は独立して実装可だが同一 branch で順次（T1 が先 — test が guard する契約を先に定義）。
- review-protocol 独立 A/B は実装前に完了していること（ADR+spec+design 対象）。

## Risks and Mitigations

| Risk | Mitigation |
| --- | --- |
| PR 自身で `pull_request` 起動が止まらない（base 側 file 評価） | 実挙動を記録 — merge 後 trigger 不存在は不変。check suite/run 実測を報告し、起動した場合は base 評価と記録して受入を満たす |
| contract test が brittle（yaml 非 parse・字句 scan） | trigger 名は `on:` block の字句一致 + コメント除去 — 既存 stdlib-only 方針と同じ |
| ci-summary transition note の記述が ruleset 実態と乖離 | disabled 実測値を文面化し、Owner 移行先事項として残す |

## Notes

- コミット分割: `docs(ci)`（文書 chain）→ `ci`（workflow + contract test + dev-env）の 2 commit を目安。日本語 Conventional Commits。
- worker_done 必須記録: PR URL・head SHA・PR 作成時刻・pull_request 起動実測・reconcile manual 成功 vs cron 未観測の区別・ruleset 実測値・検証結果。
