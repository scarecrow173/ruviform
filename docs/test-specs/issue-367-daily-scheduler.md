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
- Rule: 未知 `status`（in-flight 集合にも `completed` にも属さない値）があれば `hold-unknown` で dispatch しない。`completed` で conclusion が既知集合外・null の場合も同様（未知 conclusion は retry 候補にしない）。
- Rule: 最新 attempt が `completed`+`success` かつ「最新」の一意性が立証できる場合のみ `skip-verified`。"最新" は `attemptTime = max(created_at, run_started_at)` で順序付け、tie の非 success・unorderable・`updated_at` が最新 attemptTime より新しい非 success run は「最新」を曖昧にし `hold-unknown` とする。既知の非 success（`failure`/`cancelled`/`timed_out`/`action_required`/`startup_failure`/`stale`/`skipped`/`neutral`）または履歴空なら dispatch 候補。
- Rule: `autoDispatches(S)` は `event === workflow_dispatch` ∧ ¬（`actor.login` と `triggering_actor.login` が両方とも識別可能な非 auto actor）で数え、`>= maxAutoDispatches`（2）なら `budget-exhausted`。片側が bot・null・空 login・非 object の variant は自動発火を否定できないため全て計上する（例: `actor=null` + `triggering_actor`=human + `run_attempt=2`）。両側識別可能なユーザ actor の手動 dispatch・`push` run は計上しない。bot 発火 run の人力 rerun（`actor`=bot・`triggering_actor`=human）は計上を維持する。
- Rule: 履歴 record の対象/非対象同定は positive proof 制 — 有効 `head_sha` ≠ S・非空 `head_branch` ≠ main・非空 `event` ∉ allowlist のいずれも立証できない record は unidentifiable として全 verdict を `hold-unknown` に倒す（dispatch・skip-verified いずれも禁止、可視 success の影にも隠れない）。`id`/`head_sha`/`head_branch`/`event`/`status` を欠く record は wire schema 違反として listRuns が `error`。
- Rule: API 失敗・malformed response（`workflow_runs` 欠落・record schema 違反等）・ref 解読不能・fixture 破損は `error` outcome で exit 1。`truncated` は `history-truncated` で保留（dispatch しない）。decision を完遂した outcome（budget/hold/history-truncated/main-moved/unconfirmed/landed-mismatch 含む）は exit 0 で `warnings` に記録。
- Rule: `dispatch` 経路は直前 `getRefSha` 再照合を行い、SHA が動いていたら新 SHA で再判定する（`MAX_RECHECK` 超過 → `main-moved` で dispatch しない）。
- Rule: dispatch 後の確認 poll は `preDispatchIds` 差分で新規 `workflow_dispatch` run を同定し、着地 `head_sha` を記録する。`landed !== expected` → `dispatch-landed-mismatch`、poll timeout → `dispatched-unconfirmed`。いずれも dispatch 要求自体の成功を否定しないが、誤 SHA を「検証済み」には記録しない。
- Rule: report JSON は `kind: "daily-ci-decision"`・decisionSha・outcome・reason・latestVerified・attempts・autoDispatchCount・unidentifiableCount・dispatch 詳細・warnings・generatedAt を持ち、Job Summary に同じ判定理由が表形式で残る。token は一切出力しない。
- Rule: `--dry-run` は dispatch POST を実行せず `dry-run` outcome を返す。`--fixture` は実 API を呼ばない。

## Examples

- `latest success on S → skip-verified`: runs=[{head_sha:S, branch:main, event:push, status:completed, conclusion:success}] → outcome `skip-verified`・dispatch なし・exit 0。
- `no history → dispatch`: runs=[] → decision `dispatch` → POST 記録（fixture）→ postDispatchRuns に新規 `workflow_dispatch` run(head_sha:S) → `dispatch-confirmed`。
- `in_progress same sha → suppress`: runs=[{head_sha:S, branch:main, event:workflow_dispatch, status:in_progress}] → `suppress-in-flight`・dispatch 0 件。
- `2 auto dispatches → budget-exhausted`: runs=[2 件の {event:workflow_dispatch, triggering_actor:github-actions[bot], head_sha:S, status:completed, conclusion:failure}] → `budget-exhausted`・warning・dispatch なし。manual actor の 3 件目を混ぜても計上は bot のみ。
- `manual actor dispatch does not consume budget`: 両側識別可能なユーザ actor の dispatch 2 件 → dispatch decision（budget 未消費）。
- `unknown status → hold-unknown`: status:`"mystery"` → `hold-unknown`・dispatch なし。
- `unknown conclusion → hold-unknown`: conclusion:`"unknown_future_state"`/null/`"weird"` → `hold-unknown`・dispatch なし（POST しない）。
- `rerun ordering`: 同 SHA の古い run（created_at 古）が新しい success より後に rerun で failure → `skip-verified` にならない。`run_started_at` が動いた形なら最新=その run で `dispatch`、`updated_at` のみ動いた形なら順序曖昧で `hold-unknown`。
- `tied/unorderable ordering`: success と非 success が同一 attemptTime → `hold-unknown`（success が最新側）。非 success が最新側の tie → `dispatch`。時刻解釈不能の非 success run 存在 → `hold-unknown`。
- `bot dispatch rerun by human`: actor=github-actions[bot]・triggering_actor=maintainer・run_attempt=2 の failure 2 件 → autoDispatches=2 → `budget-exhausted`（budget 払い戻しなし）。
- `unidentifiable actor dispatch`: actor/triggering_actor 未識別の workflow_dispatch 2 件 → budget 計上 → `budget-exhausted`。片側未識別の variant（`actor=null`+`triggering_actor`=human、`actor`=human+`triggering_actor`=null、`actor={}`、空 login、非 object actor、両フィールド欠落）も同様に計上。
- `unidentifiable history record`: `{}`・`head_sha` 欠落・破損 sha・on-target か否か判定不能な record が履歴に 1 件でも → `hold-unknown`（可視 success があっても `skip-verified` にしない・dispatch しない）。明確に非対象と立証できる record（pull_request event・他 branch・他有効 sha）は従来どおり除外。
- `malformed list response`: `workflow_runs` を欠く 200 応答・record が必須フィールド（`id`/`head_sha`/`head_branch`/`event`/`status`）を欠く 200 応答（実 client 境界・fake fetch）→ `error` outcome・exit 1・dispatch 0 件。schema は通るが意味的に unidentifiable な record（破損 sha）は `hold-unknown`。
- `truncated history`: `truncated: true` → `history-truncated` outcome・warning・dispatch 0 件。success run が見えていても `skip-verified` にしない（未観測ページが判定を覆しうる）。
- `manual dispatch race (limit documented)`: 我々の POST 後の poll で人 actor の新規 workflow_dispatch run が見つかった場合 → `dispatch-confirmed`（API 上は区別不能 — 「我々の run」保証ではなく report に actor/URL で観測事実を記録）。
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
