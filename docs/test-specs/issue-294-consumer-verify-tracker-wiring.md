---
id: "TSPEC-9cHJxPHaQTlIrwbuTfhkPc"
type: "test-spec"
status: "proposed"
title: "Issue #294 consumer-verify → ci-issue-tracker 配線 test spec — scope 分離・identity 照合・verdict 写像"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "docs/specs/issue-294-consumer-verify-tracker-wiring.md"
    - "docs/designs/issue-294-consumer-verify-tracker-wiring.md"
  changes:
    added:
      - type: "file"
        path: "ci/tracker/tests/issue-tracker-workflows-contract.test.mjs"
    modified:
      - type: "file"
        path: "ci/tracker/tests/tracker.test.mjs"
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-294-consumer-verify-tracker-wiring.md"
    - "docs/designs/issue-294-consumer-verify-tracker-wiring.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-294-consumer-verify-tuple-evidence.md"
    - "docs/test-specs/issue-368-ci-issue-tracker.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-294-consumer-verify-tracker-wiring.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-294-consumer-verify-tracker-wiring.md"
    - "docs/designs/issue-294-consumer-verify-tracker-wiring.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #294 consumer-verify → ci-issue-tracker 配線 test spec — scope 分離・identity 照合・verdict 写像

## Purpose

`ci-issue-tracker` の consumer-verify scope 配線が「workflow scope ごとに
独立した Issue 追跡を、既存の fail-closed 規約を一切緩めずに」行う
契約を保証する。spec/design の受入条件が変わり scope 分離・identity
照合・verdict 写像の前提が再設計された場合に本 test spec を見直す。
consumer-verify 側の証拠生成・lane 判定の正しさは別 test spec
（issue-294-consumer-verify-tuple-evidence）の管轄であり、本書は
「tracker が消費する結合面（workflow_run payload・workflow YAML・
conclusion 行列・evidence status 語彙）」の契約固定を対象とする。

## Feature

CI failure Issue Tracker の multi-workflow scope — event 入口の scope
別 job、reconcile 入口の全 scope 逐次実行、evaluator の scope 導出
title と payload workflow identity 照合、consumer verdict → workflow
conclusion → tracker kind の写像の正直性。

## Rules

- Rule: event 入口は `workflow_run.workflows: [ci, consumer-verify]`・
  `types: [completed]`・`branches: [main]` のみを持ち、他の trigger
  class（pull_request/push/schedule/workflow_dispatch）を持たない。
- Rule: scope ごとに 1 job で、`track` は `workflow_run.name == 'ci'`、
  `track-consumer-verify` は `== 'consumer-verify'` に gate され、
  consumer job は `--workflow consumer-verify --workflow-file
  consumer-verify.yml` を明示する。ci job は `--workflow` を渡さない
  （既定 scope）。
- Rule: `classifyRun` は repo・branch・event・status の既存条件に加え、
  `workflow_run.name`/`path` の存在する方が全て scope の
  `workflow`/`workflowFile` と一致することを要求し、両方欠落は skip
  する。scope 外 payload はいかなる scope の証拠にもならない。
- Rule: Issue title は scope 導出 — `ci` は `[CI failure][main] Full CI
  failed` を byte 一致で維持、`consumer-verify` は `[CI failure][main]
  consumer-verify failed`。marker scope・label・close guard・集約規約は
  変更しない。
- Rule: reconcile 入口は `schedule`+`workflow_dispatch` のみを保ち、
  1 job が `ci`（既定）→ `consumer-verify`（明示）の順に同一判定器を
  逐次実行する。片 scope の失敗は `|| rc=1` で集約し job 全体を失敗に
  倒す。TRACKER_EVENT_PATH を渡さず mode 分離を維持する。
- Rule: consumer evidence status（pass/fail/not_run/unsupported/
  skipped）は tracker の入力ではない — tracker が消費するのは
  workflow conclusion のみ。`not_run`/`unsupported`/`skipped` は pass
  にも fail にも写像されず、`gate=blocked`→`fail`・`shaMatch=false`→
  `fail`（tuple_mismatch）の sanctioned 写像と、summary job の
  fail-closed（lane 失敗/cancelled/preflight 失敗/全 not_run・
  unsupported → run 失敗）を固定する。
- Rule: tracker の conclusion 行列は不変 — `failure`/`timed_out`/
  `startup_failure` のみ failure、`success` のみ success、その他は
  operational（close も起票もしない）。
- Rule: `ci` scope の既存 contract test は無変更で pass し続ける
  （非回帰）。

## Examples

- Example: consumer-verify の `workflow_dispatch` failure 完了 →
  `track-consumer-verify` job が scope `consumer-verify` で評価し、
  `ci-failure` label + consumer-verify marker の Issue を作成/更新。
  `track` job は同じ event で skip される。
- Example: daily-ci が GITHUB_TOKEN で dispatch した consumer-verify
  の失敗 → `workflow_run` event 不発 → reconcile tick が権威履歴から
  同じ scope の Issue に収束する。
- Example: consumer-verify の `cancelled` 完了 → operational — Issue
  を作らず、open Issue があれば note 更新のみ。
- Example: `name: consumer-verify` だが `path` が別ファイルの payload
  → `classifyRun` skip（証拠にも Issue にもならない）。
- Example: `deriveStatus({gate:"blocked"})` → `fail`、
  `{gate:"not_run"}` → `not_run`、`{shaMatch:false}` → `fail`、
  lanes が `unsupported` のみ → `unsupported`、空 lanes → `not_run`。

## Guarantees

- `ci/tracker/tests/tracker.test.mjs` が consumer-verify scope の
  classifyRun 受理/拒否・cross-scope skip・scope 別 Issue 作成・
  foreign marker 非干渉・reconcile 回復・被覆 success close を
  mock client で固定する。
- `issue-tracker-workflows-contract.test.mjs` が両 workflow YAML の
  trigger/job/権限/artifact 構造と verdict 写像行列を固定する。
- `ci` scope の全既存テストが無変更で pass する。

## Non-goals

- consumer-verify.yml の lane 判定・証拠生成・summary ロジック自体の
  正しさ（別 test spec 管轄）。
- 実 GitHub API・実 Issue・実 workflow_run/reconcile 発火の E2E
  （fixture/textual のみ。実走は merge 後）。
- Issue label・marker schema・close guard・retry 規約の変更。
- consumer repo 側 workflow・credential 設定。

## Risk

- scope 分離が破れると `ci` の失敗が consumer-verify Issue を汚染
  （または逆）し、回復 close が誤 scope に波及する — marker scope・
  title・履歴フィルタの三重分離を test で固定する。
- verdict 写像が「not_run/unsupported = pass」と誤実装されると、
  未検証 tuple が verified に見做される — Issue #294 本体の中核
  契約（fail-closed）を tracker 側でも再確認する。
- reconcile の scope 追加が `ci` の reconcile を遅延・阻害すると
  既存の通知 gap 回復が後退する — `|| rc=1` 集約と逐次実行で
  両 scope の完了を保証する。
