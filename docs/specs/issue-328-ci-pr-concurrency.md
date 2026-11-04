---
id: "SPEC-BbANBkZi3v9ZHthJOmnDCy"
type: "spec"
status: "approved"
title: "Issue #328 CI-OPT-01 PR Concurrency 制御・旧 revision CI キャンセル"
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
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/adr/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
  implements: []
  implemented-by:
    - "docs/plans/issue-328-ci-pr-concurrency.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/issue-328-ci-concurrency-grouping.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-328-ci-pr-concurrency.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/326"
    - "https://github.com/scarecrow173/ruviform/issues/327"
  defers: []
  deferred-by: []
---

# Issue #328 CI-OPT-01 PR Concurrency 制御・旧 revision CI キャンセル

## Problem

`.github/workflows/ci.yml` に `concurrency` 制御がなく、同一 PR への連続 push で旧 revision の重い matrix(verify / ui-verify / l1-real-verify の 3 OS + rt-verify / icc-verify / bench = 12 required checks)が走り続ける。runner 時間の浪費と最新 revision のシグナル遅延を招く。

## Diagnosis(変更前に診断する)

- `.github/workflows/` 配下 3 workflow いずれにも `concurrency:` キーが存在しない。GitHub Actions の既定は無制限並列で、旧 run を止める機構が何もない。
- `ci.yml` の trigger は `pull_request`(branches: main)・`push`(branches: main)・`workflow_dispatch` の 3 種。PR 連続 push は Orca multi-agent 運用で日常的であり、無駄は構造的。
- `consumer-impact-check.yml` も `pull_request` trigger(`opened`/`edited`/`synchronize`/`reopened`/`ready_for_review`)。軽量だが `edited` は同一 SHA で再実行されるため、古い実行が新しい実行より後に完了すると stale な本文検査結果が「最新結果」として残り得る。同じ方針を適用する。
- `consumer-verify.yml` は `workflow_dispatch` 専用で、各 dispatch は意図的な診断実行。相互キャンセルさせてはならず、本 issue の対象外。

## Requirements

1. `ci.yml` に同一 PR 単位の concurrency group を導入し、新 run 開始時に同 PR の pending/in-progress run を cancel する。
2. `push` to main と `workflow_dispatch` は絶対に巻き添え cancel しない — 非 PR event は run ごと一意の group にする。
3. 別 PR・他 workflow との group 衝突を構造的に排除する(`github.workflow` 接頭辞 + PR ごと一意の `github.ref`)。
4. cancel された旧 revision の check 結果が、最新 head SHA の required checks 合格証拠として扱われないことを確認する(check run は SHA 単位・cancelled conclusion は合格とみなされない)。
5. 既存 12 required checks の job 名・構成を一切変更しない(Issue #326 の Ruleset 移行を壊さない)。
6. `consumer-impact-check.yml` にも同じ方針を適用し、同一 PR の stale 実行による最新結果の上書きを防ぐ。

## Acceptance Criteria

- AC1: `ci.yml` トップレベルに `concurrency` block があり、`pull_request` では group が `${{ github.workflow }}-refs/pull/<N>/merge`、それ以外では `${{ github.workflow }}-<run_id>` に解決される。`cancel-in-progress` は `pull_request` のみ `true`。
- AC2: `consumer-impact-check.yml` に同等の `concurrency` block がある。
- AC3: 同一 PR 連続 push で旧 revision run が cancel され最新 run が完了する実測証拠(実 Run URL)が記録される — 2 回 push を伴う実験は coordinator 実施。worker 環境で実測不可の場合は IR で NotRun と理由を明記する。
- AC4: main push・workflow_dispatch・別 PR の run が cancel されないことが式の静的評価で示される。
- AC5: 旧 SHA 上の cancelled run が最新 head の required checks を偽装しないことが GitHub check run の意味論(per-SHA check suite・cancelled ≠ success/skipped)で示される。
- AC6: `mise exec -- actionlint` が変更した両 workflow で green。job 名・job 数・step 構成の diff がない(`concurrency` block + コメントのみの差分)。

## Non-goals

- merge queue(`merge_group`)対応 — 未導入。式の構造上、将来導入しても run ごと一意 group となり queue 実行を潰さないことを設計書で確認する。
- `consumer-verify.yml` への concurrency 追加 — dispatch-only のため同一式でも run ごと一意 group となり no-op。意図的に追加しない。
- `queue: max`・job-level concurrency・deploy 系の直列化など、本 Issue の要求を超える同時実行制御。
- runner キャパシティ/課金の調整自体。
