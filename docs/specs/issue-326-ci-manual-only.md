---
id: "SPEC-LVITTCzVKzuRSkMzcYwUmc"
type: "spec"
status: "approved"
title: "Issue #326 ci.yml を workflow_dispatch 専用化 — PR/push trigger 撤去（Daily Full CI 移行・RF Stage 4）"
created: "2026-11-06"
updated: "2026-11-06"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/326"
    - "https://github.com/scarecrow173/ruviform/issues/365"
    - "https://github.com/scarecrow173/ruviform/issues/366"
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
  implements: []
  implemented-by:
    - "docs/adr/issue-326-ci-yml-dispatch-only.md"
    - "docs/designs/issue-326-ci-manual-only.md"
    - "docs/plans/issue-326-ci-manual-only.md"
  depends-on:
    - "docs/specs/issue-366-full-ci-stage1.md"
    - "docs/specs/issue-367-daily-scheduler.md"
    - "docs/adr/issue-328-ci-concurrency-grouping.md"
    - "docs/adr/issue-368-tracker-reconcile-schedule-entry.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-328-ci-pr-concurrency.md"
    - "docs/specs/issue-368-ci-issue-tracker.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-366-full-ci-stage1.md"
  derived-by:
    - "docs/adr/issue-326-ci-yml-dispatch-only.md"
    - "docs/designs/issue-326-ci-manual-only.md"
    - "docs/plans/issue-326-ci-manual-only.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-326-ci-manual-only.md"
  references:
    - "https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/trigger-a-workflow"
    - "https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/about-rulesets"
  defers: []
  deferred-by: []
---

# Issue #326 ci.yml を workflow_dispatch 専用化 — PR/push trigger 撤去

## Background / Context

Issue #365 決定 1 は「PR/push 通常 CI を廃止し、各 repo の `ci.yml` は手動 `workflow_dispatch` でのみ Full CI を実行する。`daily-ci.yml` が毎日『現在 main の未検証』を判定して必要時だけ dispatch する」を全 repo 共通契約として定めた。Issue #326 は同移行の Stage 4（Required 解除・trigger 移行）の Ruviform 実施 Issue で、「Owner が Required checks を解除したことを確認してから PR/push の重い CI trigger を取り除く。PR CI トリガーだけ先に止めて Required を残すデッドロックは禁止」を段階条件とする。

2026-11-06 時点の前提条件は全て充足済み:

- Full CI は #366 Stage 1 で全 lane 無条件実行 + `ci-summary` fail-closed 集約に移行済み。
- Daily Scheduler（#367）が main HEAD 検証済み判定 + `expected_sha` 契約付き dispatch を実装済み。
- CI failure Issue Tracker（#368）+ reconcile 入口（PR #380 / ADR-vJvyZNtrHi2xnycJxbnSPv）が GITHUB_TOKEN dispatch の通知 gap を埋め、bot Full CI の失敗起票・回復 close が自動化された。
- `main-checkstatus` ruleset（required_status_checks: 9 lane context）は **enforcement: disabled**（実 API 確認済み 2026-11-06 — required checks は実効ゼロ）。`main-protect`（PR 必須・non-fast-forward・deletion 禁止）は active で維持する。Classic branch protection は未設定（404 確認）。
- 撤去条件の実証: reconcile 入口は main で manual dispatch run 42178261480 が成功し、report で `mode: reconcile`・権威履歴参照・fail-closed noop（pending run 保留）を確認した（実 cron 走行は未観測 — 区別して記録する）。

## Problem Statement

`ci.yml` の `pull_request`/`push` trigger が残ると、(a) 通常 PR/push でも重い Full CI が起動して外部 runner 待機がクリティカルパスを塞ぎ、(b) Daily Scheduler の「未検証判定 → dispatch」モデルと二重の検証経路が併存し、どちらが検証証跡の正本か曖昧になる。#365 受入条件「PR 作成/更新および main push だけでは重い CI/補助 Actions が動かない。手動 Full CI は常時利用可能」を Ruviform で実現する段階に至った。

## Requirements

- R1: `.github/workflows/ci.yml` の `on:` から `pull_request`（全 types）と `push` を除去し、`workflow_dispatch` のみとする。`inputs.expected_sha`（Daily dispatch 契約）の名前・型・意味は一切変更しない。
- R2: `.github/workflows/` 配下の**全** workflow に `pull_request`/`pull_request_target`/`push` trigger を持つファイルが存在しないことを恒久的な契約として回帰テストで固定する（「PR 作成/更新/push で CI が走り始めない」を機械的に guard）。`workflow_run` は event 反応であり該当しない。
- R3: Full CI の lane 構成・fail-closed 集約は不変 — 全 lane job 無条件実行・`ci-summary` の `if: always()` 集約・`expected_sha` 照合・checkout SHA 証跡を維持する。
- R4: Daily Scheduler（`daily-ci.yml`）・Event Tracker（`ci-issue-tracker.yml`）・Reconcile（`ci-issue-tracker-reconcile.yml`）・Consumer verify（`consumer-verify.yml`）の正当入口を一切変更しない。
- R5: concurrency は trigger 撤去後の正当解に簡素化する — pull_request 条件分岐は撤去し `ci-<run_id>` 一意 group・`cancel-in-progress: false` とする（ADR-ieVahO2wRjuTcKd43fT59I の不変条件「非 PR run は cancel されない」を変更なく満たす）。
- R6: ruleset/保護設定は変更しない — `main-protect` active・`main-checkstatus` disabled の現状を前提条件として維持し、本変更はコード側のみ（GitHub Settings への書き込みは本タスクの範囲外・管理者/Owner 所有）。
- R7: 新規 secret・権限増量・外部サービスを追加しない。
- R8: `ci.yml` の header comment・`expected_sha` コメント・各 lane の checkout pin コメント・`ci-summary` transition note を、trigger 撤去後の実態と矛盾しない文面に更新する（PR CI を前提とした記述を残さない）。
- R9: reconcile の実 cron 走行と manual dispatch 成功を区別する — cron 未観測を成功として記録しない。
- R10: docs/development-environment.md の `ci.yml` trigger 記述を実態に同期する（historical docs は書き換えない — 当時の決定記録として保持）。

## Acceptance Criteria

- [ ] `ci.yml` が `on: workflow_dispatch`（`expected_sha` input 維持）のみを持ち、actionlint clean。
- [ ] `.github/workflows/` 全ファイルに `pull_request`/`pull_request_target`/`push` trigger が存在しないことを contract test（`ci/daily/tests/ci-yml-contract.test.mjs` 経由 `mise run test-ci-daily`）で回帰固定。
- [ ] `ci-issue-tracker.yml`/`ci-issue-tracker-reconcile.yml`/`daily-ci.yml`/`consumer-verify.yml` が byte-identical 不変（`git diff` 0 行）。
- [ ] `mise run test-ci-tracker`/`test-ci-daily`/`test-ci-summary`・`actionlint`（全 workflow）・`prek run --all-files` が pass。
- [ ] 新関連 ADR が ADR-ieVahO2wRjuTcKd43fT59I（concurrency）との関係（補足 — event 条件式の撤廃。不変条件維持）を記録し、独立レビュー済み。
- [ ] PR 作成後、当該 PR 自体の `pull_request` 起動 run が 0 件であることを Actions API / check suites で記録（merge 前 PR で「CI が走り始めない」ことの実証 — 起動された場合は実挙動を正直に記録し、trigger 評価が base/head どちらの workflow file を使うかも記録する）。
- [ ] head SHA と PR 作成時刻を報告する。実 cron 起動は未観測として明示。

## Non-Goals

- `main-checkstatus` の ruleset 本体の廃止・required checks の再設定（GitHub Settings 書き込み権限を持つ管理者の操作 — 本変更は前提条件の確認のみ）。
- AudioLens #147・Synvelis #145 の trigger 撤去（別 worker が各 repo で実施）。
- Issue #326/#365 等の close（coordinator 権限）。
- 人為的な重い Full CI 失敗の追加起動（既存 bot 成功履歴・#375 負例・実 reconcile ログで代替 — 必要性を再検討した上で不要と判断）。
- tracker `ALLOWED_EVENTS` の `push`/`schedule` 削除（履歴証跡として正当 — 過去の push run 成功は従来どおり evidence として有効であり、allowlist 狭化は不要な契約変更）。

## Dependencies

- 前提充足: #366 Stage 1（Full CI 全 lane + ci-summary）、#367（Daily Scheduler）、#368 + PR #380（Tracker + reconcile 入口・main で manual 成功実証済み）、`main-checkstatus` disabled（2026-11-06 実 API 確認）、`main-protect` active。
- 文書: `docs/specs/issue-366-full-ci-stage1.md`、`docs/specs/issue-367-daily-scheduler.md`、`docs/adr/issue-328-ci-concurrency-grouping.md`、`docs/adr/issue-368-tracker-reconcile-schedule-entry.md`。

## Risks / Open Questions

- **PR 評価時の trigger 起点**: `pull_request` event で実行される workflow は head ref のファイルを使うが、trigger マッチング自体が base/head どちらの `on:` を参照するかは GitHub 実装依存 — 本 PR 自身で「CI 起動 0 件」が確認できれば最良の実証となるが、base 側ファイルで評価される場合は起動し得る。その場合も「merge 後は trigger が存在しない」ことを受入条件とし、実挙動を記録する。
- **reconcile の schedule auto-disable**: 別入口の運用制約は #368 側で記録済み（60 日無活動 disable・全 trigger 不応答）。本変更はその制約を増減させない。
- **旧 push run の evidence 価値**: trigger 撤去後は新規 push run が発生しないため、履歴の `push`/`pull_request` record は時間とともに減衰する。tracker/daily の allowlist は履歴互換のため維持する（R10/Non-Goals）。
- **PR 上で status が一切生成されない運用**: `main-protect` の PR 必須ルールは status を要求しないため deadlock にならない（required checks は disabled 確認済み）。
