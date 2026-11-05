---
id: "TSPEC-GfyGGakuj069mKFQrVtF3e"
type: "test-spec"
status: "approved"
title: "Issue #367 テスト仕様 — daily scheduler 判定・dispatch 確認・fail-closed 契約"
created: "2026-11-06"
updated: "2026-11-06"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/367"
  changes:
    added:
      - type: "directory"
        path: "ci/daily"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-367-daily-scheduler.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-367-daily-scheduler.md"
    - "docs/test-specs/ci-profile-build-behavior.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-367-daily-scheduler.md"
    - "docs/plans/issue-367-daily-scheduler.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-367-daily-scheduler.md"
    - "docs/designs/issue-367-daily-scheduler.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #367 テスト仕様 — daily scheduler 判定・dispatch 確認・fail-closed 契約

## Purpose

`ci/daily/decide.mjs` が「現在 main SHA に有効な Full CI があるか」を GitHub Actions 履歴から厳格に判定し、誤った skip/誤った dispatch/誤った budget 消費を fixture で再現可能な形で禁止することを保証する。全検証は offline fixture で行い、実 Actions dispatch 等の副作用を発生させない。

## Feature

daily scheduler の decision/dispatch/確認/報告契約 — `attemptsOn` 厳格同定・decision 優先度・auto dispatch budget・再照合・確認 poll・report/summary/exit code。

## Rules

- Rule: `attemptsOn(S)` は `head_sha === S` ∧ `head_branch === main` ∧ `event ∈ {push, workflow_dispatch, schedule}` のみを含む。他 SHA・他 branch・`pull_request`・scheduler 自身の run・他 workflow の run はいかなる decision にも寄与しない。
- Rule: in-flight（`queued`/`waiting`/`in_progress`/`pending`/`requested`）が 1 件でもあれば `suppress-in-flight` — 成功履歴の有無・新しさに関わらず dispatch しない。
- Rule: 未知 `status`（in-flight 集合にも `completed` にも属さない値）があれば `hold-unknown` で dispatch しない。
- Rule: 最新 attempt が `completed`+`success` なら `skip-verified`。非 success（`failure`/`cancelled`/`timed_out`/`action_required`/`startup_failure`/null/未知 conclusion）または履歴空なら dispatch 候補。
- Rule: `autoDispatches(S)` は `event === workflow_dispatch` ∧ `triggering_actor.login ∈ autoActors`（既定 `github-actions[bot]`）で数え、`>= maxAutoDispatches`（2）なら `budget-exhausted`。ユーザ actor の手動 dispatch・`push` run は計上しない。
- Rule: API 失敗・履歴未取得・ref 解読不能・fixture 破損は `error` outcome で exit 1。decision を完遂した outcome（budget/hold/main-moved/unconfirmed/landed-mismatch 含む）は exit 0 で `warnings` に記録。
- Rule: `dispatch` 経路は直前 `getRefSha` 再照合を行い、SHA が動いていたら新 SHA で再判定する（`MAX_RECHECK` 超過 → `main-moved` で dispatch しない）。
- Rule: dispatch 後の確認 poll は `preDispatchIds` 差分で新規 `workflow_dispatch` run を同定し、着地 `head_sha` を記録する。`landed !== expected` → `dispatch-landed-mismatch`、poll timeout → `dispatched-unconfirmed`。いずれも dispatch 要求自体の成功を否定しないが、誤 SHA を「検証済み」には記録しない。
- Rule: report JSON は `kind: "daily-ci-decision"`・decisionSha・outcome・reason・latestVerified・attempts・autoDispatchCount・dispatch 詳細・warnings・generatedAt を持ち、Job Summary に同じ判定理由が表形式で残る。token は一切出力しない。
- Rule: `--dry-run` は dispatch POST を実行せず `dry-run` outcome を返す。`--fixture` は実 API を呼ばない。

## Examples

- `latest success on S → skip-verified`: runs=[{head_sha:S, branch:main, event:push, status:completed, conclusion:success}] → outcome `skip-verified`・dispatch なし・exit 0。
- `no history → dispatch`: runs=[] → decision `dispatch` → POST 記録（fixture）→ postDispatchRuns に新規 `workflow_dispatch` run(head_sha:S) → `dispatch-confirmed`。
- `in_progress same sha → suppress`: runs=[{head_sha:S, branch:main, event:workflow_dispatch, status:in_progress}] → `suppress-in-flight`・dispatch 0 件。
- `2 auto dispatches → budget-exhausted`: runs=[2 件の {event:workflow_dispatch, triggering_actor:github-actions[bot], head_sha:S, status:completed, conclusion:failure}] → `budget-exhausted`・warning・dispatch なし。manual actor の 3 件目を混ぜても計上は bot のみ。
- `manual actor dispatch does not consume budget`: bot dispatch 0・manual dispatch 2 → dispatch decision（budget 未消費）。
- `unknown status → hold-unknown`: status:`"mystery"` → `hold-unknown`・dispatch なし。
- `main moved → re-decide`: refReads=[S1, S2]、runs に S2 向け success 履歴 → `skip-verified`（S1 には dispatch しない）。
- `dispatch lands on other sha`: postDispatchRuns の新規 run head_sha=S' → `dispatch-landed-mismatch`・warning。
- `dispatch unconfirmed`: postDispatchRuns 空 → `dispatched-unconfirmed`・warning。
- `API error → error outcome`: listRuns が throw → `error`・exit 1・report なし or error 記録。
- `pull_request/other-branch runs ignored`: {event:pull_request, head_sha:S}・{head_branch:feature, head_sha:S} のみ → 履歴なし扱いで `dispatch`。
- `latestVerified`: 古い成功 run（他 SHA）→ latestVerified.head_sha=その SHA・updated_at 記録。

## Guarantees

- fixture 行列が spec/design の decision 表を網羅し、誤成功判定・誤 dispatch・budget 超過・誤 SHA の検証済み記録が構造的に起きないことを pin する。
- `node --test ci/daily/`（= `mise run test-ci-daily`）が CI daily-ci job の self-test として先に実行され、壊れた判定器が verdict を出さない。

## Non-goals

- 実 GitHub Actions への dispatch・実 run 履歴の取得（禁止事項 — fixture/mock のみ）。
- `ci-summary` 側の `expected_sha` 照合（`ci/summary/` テストが既にカバー — ここでは入力を正しく渡すことのみ）。
- `ci-issue-tracker.yml`（#368）の Issue 連携動作。
- workflow YAML の実行 semantics（actionlint で構文のみ検証 — schedule が実際に火を吹くことは検証しない）。

## Risk

- 本 spec が外れると、(a) 検証済みでない main を「済み」と誤判定して回帰を見逃す、(b) 同一 SHA へ dispatch を連発して runner と履歴を汚染する、(c) race で誤 SHA を検証済み記録する、のいずれかが静かに起きる。
