---
id: "SPEC-ko4Wl9fIvU1e2cmiVYFmwp"
type: "spec"
status: "proposed"
title: "Issue #294 consumer-verify → ci-issue-tracker 配線 — 失敗の Issue 追跡を既存 tracker scope で実現"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "docs/specs/issue-294-cross-consumer-candidate-gate.md"
    - "docs/specs/issue-368-ci-issue-tracker.md"
    - "docs/specs/issue-368-tracker-reconcile.md"
    - "docs/specs/issue-294-daily-tuple-trigger.md"
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
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-294-cross-consumer-candidate-gate.md"
    - "docs/specs/issue-368-ci-issue-tracker.md"
    - "docs/specs/issue-368-tracker-reconcile.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-368-ci-issue-tracker.md"
    - "docs/designs/issue-368-tracker-reconcile.md"
    - "docs/designs/issue-294-cross-consumer-candidate-gate.md"
    - "docs/designs/issue-294-daily-tuple-trigger.md"
    - "docs/test-specs/issue-294-consumer-verify-tuple-evidence.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "docs/specs/issue-294-cross-consumer-candidate-gate.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/368"
    - "https://github.com/scarecrow173/ruviform/issues/367"
  defers: []
  deferred-by: []
---

# Issue #294 consumer-verify → ci-issue-tracker 配線 — 失敗の Issue 追跡を既存 tracker scope で実現

## What Should Be Built

Issue #294 の残存 ruviform 側 slice: `consumer-verify` workflow の失敗を
既存 `ci-issue-tracker` 機構（Issue #368）で追跡する配線。`ci/tracker/README.md`
が「`consumer-verify` 追跡は #294 の別ステップ」と明示していた部分を実装する。

1. **event 入口の scope 拡張**: `ci-issue-tracker.yml` の
   `workflow_run.workflows` を `[ci, consumer-verify]` に拡張し、
   scope ごとの job（`track` / `track-consumer-verify`）が
   `workflow_run.name` で gate されて自 scope のみを評価する。
2. **reconcile 入口の scope 拡張**: `ci-issue-tracker-reconcile.yml` の
   1 job が `ci` → `consumer-verify` の順に同一判定器
   （`tracker.mjs --reconcile`）を逐次実行する。daily-ci は
   `consumer-verify` を `GITHUB_TOKEN` で dispatch するため完了時に
   `workflow_run` event が発火しない（anti-recursion）— この gap は
   reconcile 側でしか埋まらない。
3. **scope 対応の evaluator 整備**: `tracker.mjs` の Issue title を
   scope 導出（`issueTitle(scope)`）に変え、event payload 側にも
   workflow `name`/`path` の scope 照合を追加する（#368 で記録されて
   いた「同名 workflow 混入」P2 既知制限の解消を兼ねる）。
4. **contract tests**: `ci/tracker/` に consumer-verify scope の
   分類・Issue 分離・reconcile 回復のテストと、両 workflow YAML +
   verdict 写像（evidence status → workflow conclusion → tracker kind）
   の構造契約テストを追加する。

## Why Now

- `consumer-verify.yml`（tuple 証拠契約）と daily tuple scheduler
  （`decide-consumer-verify` job）は main に存在するが、consumer-verify
  の**失敗が Issue として可視化・追跡される経路がない**。手動
  `workflow_dispatch` の失敗も daily dispatch の失敗も、run を人間が
  見に行かない限り放置される。
- tracker は `--workflow`/`--workflow-file` scope 引数を既に持ち、
  marker scope・履歴フィルタも workflow 対応済み — 正しい拡張点は
  既存 scope 機構の再利用であり、新 framework は不要。
- Issue #294 は umbrella のため本 slice 完了後も Open 継続が前提
  （PR 本文は `Refs #294`）。

## Users and Value

- **Ruviform 運用者**: consumer-verify の失敗（tuple_mismatch・lane
  fail・preflight 失敗を含む workflow failure）が `ci-failure` label +
  scope marker の Issue に自動集約され、回復時は既存の多重 guard 付き
  auto-close で閉じる。`ci` scope の Issue とは完全に分離される。
- **Consumer Owner（AL/SV）**: consumer 側起因の検証失敗が Ruviform
  側の追跡 Issue として観測でき、対応状況が Issue 履歴に残る。

## Scope

### In Scope

- `.github/workflows/ci-issue-tracker.yml`: `workflows` に
  `consumer-verify` 追加、scope 別 job 分割（`if` gate・個別 report
  artifact）。
- `.github/workflows/ci-issue-tracker-reconcile.yml`: reconcile を
  全 scope 逐次実行に拡張（`|| rc=1` 集約・失敗伝播維持）。
- `ci/tracker/tracker.mjs`: `issueTitle(scope)`（`ci` は従来 title
  verbatim 維持）、`classifyRun` の workflow `name`/`path` scope 照合、
  body/comment の scope 対応文言。
- `ci/tracker/tests/tracker.test.mjs`: payload helper の workflow
  identity 既定 + consumer-verify scope テスト群。
- `ci/tracker/tests/issue-tracker-workflows-contract.test.mjs`（新規）:
  両 workflow YAML の構造契約 + verdict 写像の固定。
- `ci/tracker/README.md`・`docs/development-environment.md` の同期。
- 対象 chain docs（spec/design/plan/task/test-spec/impl-record）。

### Out of Scope

- `consumer-verify.yml` 自体の lane・証拠契約の変更（既存契約を
  そのまま消費する）。
- `daily-ci.yml` の変更（dispatch 経路は既存のまま）。
- 実 daily dispatch の実走証跡・both-consumer-pass 実測・
  `CROSS_REPO_DEPLOY_KEY` 設定・GUI-VISUAL/OS lanes・
  PLUGIN-NATIVE-DOCK（全て人間/実機 gate — NotRun）。
- umbrella Issue #294 自体の close。
- 新しい追跡 framework・別 label・retry 規約の変更（既存規約を流用）。

## Requirements

- **R1 event scope**: `workflow_run.workflows: [ci, consumer-verify]`、
  `types: [completed]`、`branches: [main]` を維持し、job を scope 別に
  分ける。`track` は `if: workflow_run.name == 'ci'`、
  `track-consumer-verify` は `if: workflow_run.name ==
  'consumer-verify'` + `--workflow consumer-verify --workflow-file
  consumer-verify.yml`。
- **R2 reconcile scope**: reconcile job が `ci`（既定）と
  `consumer-verify`（明示 scope）を逐次 reconcile し、片方の失敗が
  他方を妨げないが job 全体は失敗に倒す。concurrency group・権限・
  checkout pin・self-test は不変。
- **R3 payload identity**: `classifyRun` が `workflow_run.name`/
  `path` を scope の `workflow`/`workflowFile` と照合し、不一致・
  両欠落を skip する（event は trigger であり証拠ではない — reconcile
  が権威履歴で回復する）。
- **R4 Issue 分離**: Issue title は `issueTitle(scope)` で scope 導出
  （`ci` → 従来 `[CI failure][main] Full CI failed` verbatim、
  `consumer-verify` → `[CI failure][main] consumer-verify failed`）。
  marker scope・label・close guard・retry/集約規約は既存のまま。
- **R5 verdict の正直性**: tracker が消費するのは workflow conclusion
  のみ。consumer evidence の `not_run`/`unsupported`/`skipped` は
  pass にも fail にも写像されず、`gate=blocked`→`fail`・
  `shaMatch=false`→`fail`（`tuple_mismatch`）の sanctioned 写像と
  workflow summary の fail-closed（全 not_run/unsupported・lane 失敗
  で run 失敗）を contract test で固定する。
- **R6 非破壊**: `ci` scope の既存挙動・ Issue title・marker 形式・
  既存テストは全て不変。`consumer-verify` は dispatch-only のため
  実質 `workflow_dispatch` event のみが届くが、共有
  `ALLOWED_EVENTS` allowlist は変更しない。
- **R7 fail-closed 維持**: event 入口に `schedule`/`workflow_dispatch`/
  `pull_request`/`push` trigger を追加しない。reconcile の mode 分離
  （event payload 不読）・共有 concurrency group・schedule auto-disable
  分離は不変。

## Acceptance Criteria

- [ ] (構造) `ci-issue-tracker.yml` が `workflows: [ci, consumer-verify]`
  を監視し、scope 別 2 job が `workflow_run.name` で gate され、
  consumer-verify job が `--workflow consumer-verify --workflow-file
  consumer-verify.yml` で評価する。
- [ ] (構造) `ci-issue-tracker-reconcile.yml` が `ci` + `consumer-verify`
  の両 scope を逐次 reconcile し、schedule+workflow_dispatch のみ・
  共有 concurrency group・mode 分離を維持する。
- [ ] (構造) `classifyRun` の workflow identity 照合・scope 別
  `issueTitle`・scope 対応 body/comment が実装され、`ci` scope の
  既存テストが無変更で pass する。
- [ ] (構造) verdict 写像 contract test が green:
  `not_run`/`unsupported`/`skipped` を pass/fail と誤認しない、
  `gate=blocked`→`fail`・`shaMatch=false`→`fail` を維持、
  operational conclusion が failure/ success に化けない。
- [ ] (構造) `actionlint`（変更 workflow）・`mise run fmt-check` /
  `check` / `lint` / `test`・`mise run test-ci-tracker` /
  `test-ci-consumer-verify` が pass。
- [ ] (実走・後続) merge 後の consumer-verify 完了（手動または daily
  dispatch）で `ci-failure` label の scope 別 Issue が起票/更新/close
  されること、および GITHUB_TOKEN dispatch 分が reconcile tick で
  回復することを実測する（本 PR では NotRun — workflow_run/reconcile
  の実走は merge 後）。

## Deferred Design Concerns

| 後送り項目 | 理由 | 追跡先 |
| --- | --- | --- |
| 実 daily dispatch・実 consumer-verify run での tracker 発火実測 | run API/運用制約・merge 後の実走事項 | Issue #294 本体 |
| both-consumer-pass の実測・`CROSS_REPO_DEPLOY_KEY` 設定 | audio-lens export portability 修正待ち・人手 credential 作業 | Issue #294 本体 |
| GUI-VISUAL/OS lanes・PLUGIN-NATIVE-DOCK | 実機/OS gate が owner | Issue #294 本体 |
| umbrella Issue #294 の close | 残 AC があるため Open 継続 | Issue #294 本体 |
