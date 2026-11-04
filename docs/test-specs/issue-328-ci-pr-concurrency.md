---
id: "TSPEC-oHAlZzKO4OjFLCg0SpIiws"
type: "test-spec"
status: "approved"
title: "Issue #328 テスト仕様 — CI concurrency の不変条件と証拠"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/328"
  changes:
    added: []
    modified:
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: ".github/workflows/consumer-impact-check.yml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-328-ci-pr-concurrency.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-328-ci-pr-concurrency.md"
    - "docs/specs/issue-328-ci-pr-concurrency.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-328-ci-pr-concurrency.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-328-ci-pr-concurrency.md"
  verified-by:
    - "docs/impl/ir/issue-328-ci-pr-concurrency-implementation.md"
  references:
    - "https://docs.github.com/en/actions/how-tos/writing-workflows/choosing-what-your-workflow-does/control-the-concurrency-of-workflows-and-jobs"
  defers: []
  deferred-by: []
---

# Issue #328 テスト仕様 — CI concurrency の不変条件と証拠

## Scope

`.github/workflows/ci.yml`・`consumer-impact-check.yml` に追加する workflow-level `concurrency` block の意味論のみを対象とする。job 構成・step・check 名は変更対象外(非変更であることを検証する側)。

## Invariants

- INV-1(PR group 一意性): `pull_request` run の group は `<workflow>-refs/pull/<N>/merge` に解決され、同じ PR の run のみが共有する。別 PR・別 workflow と衝突しない。
- INV-2(非 PR の非干渉): `push`/`workflow_dispatch`/将来の `merge_group` run の group は `<workflow>-<run_id>` で run ごと一意。どの run も他 run を cancel しない。
- INV-3(最新優先): 同一 PR group では新規 run が既存 pending run を置換し(既定動作)、`cancel-in-progress: true` により running run も cancel する。連続 push で完走し得るのは最後の run のみ。
- INV-4(偽装なし): cancel された run の check run conclusion は `cancelled` で、その run の head SHA に紐づく。required checks は最新 head SHA の check を評価するため、旧 SHA の cancelled 結果は合格証拠にならない。`cancelled` は合格条件(success/skipped)に含まれない。
- INV-5(非破壊性): `concurrency` block 追加以外の diff が存在しない — job 名・job 数・trigger・permissions・step は同一。12 required checks の check 名は不変。

## Cases

| Case | 検証 | 実施者 |
| --- | --- | --- |
| C1: 静的妥当性 | `mise exec -- actionlint .github/workflows/ci.yml .github/workflows/consumer-impact-check.yml` が exit 0 | worker |
| C2: 式の分岐評価 | design の group 解決表に対し、`pull_request`/`push`/`workflow_dispatch`/`merge_group` の各 event で group・cancel-in-progress が表の通り解決されること(机上評価 + `github` context property が公式の許可リスト内) | worker |
| C3: 実測 — 同一 PR supersession | 本 PR で連続 2 push → 旧 run が `cancelled`、新 run が完走。両方の Run URL を IR/PR に記録 | coordinator(worker は NotRun 記録) |
| C4: 非破壊 diff | `git diff` が両 workflow で `concurrency:` 追加 + コメントのみを示す | worker |
| C5: rollback | `git revert` で block 除去 → 従来の全並列実行へ戻る(差分が追加のみのため自明) | 記録のみ(実行は不要) |

## Evidence

- worker: actionlint 出力・`git diff` の要約・group 解決表の評価結果を IR に記録。
- coordinator: 2 push 実験の両 Run URL(旧=cancelled / 新=completed)を IR または PR へ記録。取得不能なら IR に NotRun + 理由を明記する。
