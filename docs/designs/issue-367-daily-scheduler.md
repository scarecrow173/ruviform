---
id: "DESIGN-LKKyu5XTONFK84RN45ZdM2"
type: "design"
status: "approved"
title: "Issue #367 共通 Daily Scheduler 設計 — 履歴駆動判定・重複抑止・bounded retry・dispatch SHA race 対策"
created: "2026-11-06"
updated: "2026-11-06"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/367"
    - "docs/specs/issue-367-daily-scheduler.md"
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
  implemented-by:
    - "docs/plans/issue-367-daily-scheduler.md"
  depends-on:
    - "docs/specs/issue-367-daily-scheduler.md"
    - "docs/designs/issue-366-full-ci-stage1.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-366-full-ci-stage1.md"
    - "docs/designs/issue-327-cross-repo-ci-baseline.md"
    - "docs/designs/issue-346-ruleset-migration-runbook.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-367-daily-scheduler.md"
  derived-by:
    - "docs/plans/issue-367-daily-scheduler.md"
    - "docs/test-specs/issue-367-daily-scheduler.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-367-daily-scheduler.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/365"
    - "https://github.com/scarecrow173/ruviform/issues/368"
    - "https://github.com/scarecrow173/ruviform/issues/294"
  defers: []
  deferred-by: []
---

# Issue #367 共通 Daily Scheduler 設計 — 履歴駆動判定・重複抑止・bounded retry・dispatch SHA race 対策

## Context

`ci.yml`（PR #370 統合済み、base `7454d84`）は `pull_request`/`push`/`workflow_dispatch` で全 lane 無条件の Full CI を実行し、終端 `ci-summary` が全 lane の `success` と `expected_sha` 照合を fail-closed で判定する。`workflow_dispatch` には `expected_sha` 入力があり、非空なら全 lane leg の実測 checkout 共通値と照合する — 判定側が期待 SHA を渡す契約は #366 で確定済み。

Issue #367 はこの「起動側」を実装する。GitHub Actions run 履歴だけを正本とし、専用 DB・repository variable を持たない。状態復元の全困難（eventual consistency・保持期間・rerun・race）は API 履歴の厳格な読み取りと fail-closed な分類で吸収する。

## Scope

### In Scope

- `ci/daily/decide.mjs`: 判定器 + dispatch クライアント（stdlib-only、外部依存なし）
- `ci/daily/tests/decide.test.mjs`: fixture 駆動 contract test（`node --test`）
- `.github/workflows/daily-ci.yml`: UTC cron + `workflow_dispatch`、単一 job、最小権限、concurrency、report artifact
- `mise.toml` `test-ci-daily` task、`docs/development-environment.md` 同期

### Out of Scope

- `ci-issue-tracker.yml`（#368）、ruleset 変更、`ci.yml` 入力追加、Consumer 実導入（AL #159 / SV #167 の Owner 作業）

## Components and Boundaries

### `ci/daily/decide.mjs`

`ci/summary/summary.mjs` と同じ構造: 判定コアは副作用を持たない pure function、API アクセスは注入可能な client、`main()` が env/argv から設定を解決する。`import.meta.url === pathToFileURL(argv[1]).href` の `isMain` guard で test から直接関数を import 可能。

#### 設定（env / argv）

| 変数 | 既定 | 用途 |
|---|---|---|
| `GITHUB_TOKEN` | （必須・fixture 除く） | Actions API 認証（job の `GITHUB_TOKEN`） |
| `GITHUB_REPOSITORY` | （必須） | `owner/repo` |
| `DAILY_API_BASE` | `https://api.github.com` | API base（GHES/将来 mock 用） |
| `DAILY_TARGET_WORKFLOW` | `ci.yml` | 判定対象 workflow file 名 |
| `DAILY_MAIN_REF` | `main` | 判定対象 ref（heads/<ref>） |
| `DAILY_MAX_AUTO_DISPATCHES` | `2` | 同一 SHA 自動 dispatch 上限（初回 1 + retry 1） |
| `DAILY_MAX_PAGES` | `10` | runs list ページネーション上限（per_page=100 → 最大 1000 run） |
| `DAILY_AUTO_ACTORS` | `github-actions[bot]` | 「自動 dispatch」として数える actor（カンマ区切り） |
| `DAILY_CONFIRM_TIMEOUT_MS` / `DAILY_CONFIRM_INTERVAL_MS` | `120000` / `15000` | dispatch 後の確認 poll 上限/間隔 |
| `DAILY_REPORT` / `--report` | `daily-ci-report.json` | JSON report 出力先 |
| `--fixture <file>` | — | API 呼出しを fixture JSON に置換（offline 再現） |
| `--dry-run` | — | 判定のみ（dispatch POST を送らない。`workflow_dispatch` の `dry_run` 入力からも） |

#### 判定コア `decide({ mainSha, runs, config })`

**有効 Full CI run の同定**（`attemptsOn(sha)`）: `head_sha === sha` **かつ** `head_branch === mainRef` **かつ** `event ∈ FULL_CI_EVENTS`。`FULL_CI_EVENTS = {push, workflow_dispatch, schedule}`（移行中の push run も同一 lane 集合を走るため有効な検証。`pull_request` run は merge commit ではなく PR head を指し `head_branch` も main でないため構造的に除外される。未知 event は allowlist 外として無視 — 誤って検証済みに寄与しない）。`status`/`conclusion`/`run_attempt` は各 run の**最新 attempt** を表す（list API 契約）。

**非対象除外は positive proof 制**: run を attemptsOn から外せるのは、いずれかのフィールドが「対象でない」ことを立証できる場合のみ — 有効 40-hex `head_sha` ≠ sha・非空 `head_branch` ≠ mainRef・非空 `event` ∉ FULL_CI_EVENTS。`{}`・`head_sha` 欠落・破損 sha（"notasha"）など、on-target 立証も off-target 立証もできない record は **unidentifiable** — 任意の状態の attempt を隠しうるため、1 件でも存在すれば全 verdict を `hold-unknown` に倒す（dispatch でも skip-verified でもない）。これは listRuns の wire schema 検証（`id`/`head_sha`/`head_branch`/`event`/`status` 必須 → 欠落は malformed `error`）とは別層の防御: schema を通るが意味的に識別不能な record と、fixture 等 client を介さない経路を守る。

**attempt の時系列順序**（fail-closed）: `attemptTime(r) = max(created_at, run_started_at)` — `created_at` は run の作成時刻で動かず、`run_started_at` は**最新 attempt の開始時刻**なので rerun すると前に進む（実 API 実測: run 41958976200 attempt 2, created 03:01 / started 03:07 / updated 03:10）。`updated_at` は attempt 外の bookkeeping でも動きうるため順序キーには使わず、**曖昧性プローブ**として使う（後述 #3'）。タイムスタンプ tie は run id 降順、解釈不能な時刻の run は unorderable として最後尾（「最新」ではない）。

**decision 列**（この順で評価）:

| # | 条件 | decision | dispatch |
|---|---|---|---|
| 1 | attemptsOn(S) に in-flight（`status ∈ {queued, waiting, in_progress, pending, requested}`）あり | `suppress-in-flight` | × |
| 1' | 履歴内に target/non-target いずれとも立証できない record（unidentifiable、上記 positive-proof 制）が 1 件でも | `hold-unknown` | × |
| 2 | 最新 attempt の `status` が上記でも `completed` でもない、または `completed` で `conclusion` が既知集合（`success`/`failure`/`neutral`/`cancelled`/`skipped`/`timed_out`/`action_required`/`startup_failure`/`stale`）外 | `hold-unknown` | × |
| 3 | 最新 attempt が `completed` + `conclusion === 'success'` | → 3' |
| 3' | 3 の直前チェック — 他 run が (a) attemptTime 同一・(b) unorderable・(c) `updated_at` が最新 attemptTime より新しい（attempt 化していない不可解な活動、例: rerun 記録の反映遅延）のいずれかで「最新」の一意性を争う | `hold-unknown` | × |
| 3'' | 3' で争いなし | `skip-verified` | × |
| 4 | 最新 attempt が `completed` 非 success、または attempts 空（履歴なし・保持期間外） | candidate | → 5 |
| 5 | `autoDispatches(S) = attemptsOn(S) ∩ {event === workflow_dispatch ∧ ¬(actor.login と triggering_actor.login が両方とも非 auto actor と識別可能)}` が `maxAutoDispatches` 以上 | `budget-exhausted` | × |
| 6 | 上記以外 | `dispatch` | ○ |

- 優先度: in-flight > unknown > verified(曖昧性チェック付き) > retry/budget。実行中 run があれば結論を待つ（重複 dispatch 抑止）。未知状態は「分類不能 → 動かない」が安全側。success 主張は「最新が一意に立証できる」場合のみ許可 — tie・順序不能・説明のつかない新規活動があると `hold-unknown`（偽 success を絶対に記録しない）。tie で非 success が最新側なら `dispatch`（bounded retry は検証主張ではないため安全側解決）。
- `conclusion` の既知非 success 値は retry 候補。未知・null の conclusion は retry 候補にしない — 「分類不能」は #2 の `hold-unknown`（POST しない）。
- `autoDispatches` の provenance: `actor` = run の最初の発火者、`triggering_actor` = 最新 (re)run の発火者（API 契約）。manual origin を立証できるのは**両側が識別可能な非 auto actor**の場合のみ — `actor` が null/空/非 object（元 dispatch 元を否定できない）や `triggering_actor` が欠落の片側未識別 variant も計上する。例: `actor=null` + `triggering_actor=maintainer` + `run_attempt=2` は「人の rerun」に見えるが初回発火者が不明なため自動発火を否定できず計上（再現済み blocking candidate）。bot が dispatch した run を人間が rerun するケース（`actor`=bot, `triggering_actor`=人）も計上 — 人の rerun で budget が払い戻されない。過少計上は retry 暴走（unsafe）、過大計上は dispatch 減少（safe）のため安全側。list API は run の `inputs` を返さない（実測確認）ため「scheduler が作った dispatch」を inputs マーカーで識別できず、actor provenance で代替する。
- `latestVerified`: 全 runs（sha 不問・上記の branch/event 条件は同じ）から `completed`+`success` の最新を選び `{head_sha, updated_at, html_url}` を報告用に保持 — 「最終成功検証時刻」（#367 項目 8）の材料。順序は attemptTime ベース。

#### API client `createGitHubClient({ token, repo, apiBase })`

- `getRefSha(ref)`: `GET /repos/{repo}/git/ref/heads/{ref}` → `object.sha`（40 hex 検証。非 2xx・形式異常は throw）
- `listRuns(workflow, { branch })`: `GET /repos/{repo}/actions/workflows/{workflow}/runs?branch={branch}&per_page=100&page=N` を `maxPages` まで反復（`workflow_runs.length < 100` で終了）。サーバ側 `branch` 絞り込みは効率化であって、厳格性は client 側 `attemptsOn` 条件が担保する。**`workflow_runs` 配列を欠く 200 body・非 object の run entry は malformed response として `error`**（「履歴なし」と誤認すると誤 dispatch に直結する）。打ち切り時 `truncated: true`
- `dispatch(workflow, ref, inputs)`: `POST /repos/{repo}/actions/workflows/{workflow}/dispatches` `{ref, inputs}` → 204 期待（非 2xx は throw）
- 全呼出しで 2xx 以外・ネットワーク例外・JSON parse 失敗 → `error` outcome（exit 1）。token 未設定・repo 未解決も同様。403/429 の rate limit も `error`（再試行は次回 schedule — bounded retry は検証対象の retry であり API call の retry ではない）

#### `run()` orchestration

1. `sha = getRefSha(mainRef)` → `listRuns` → `decide`
2. **`truncated` → `history-truncated`**（warning・dispatch しない）。未観測の古いページに auto dispatch（budget 過少計上）や本 SHA の rerun（success 主張への contender）が隠れうるため、不完全な履歴では skip-verified を含む一切の判定を確定しない。復帰は `DAILY_MAX_PAGES` 拡大または手動再判定。再照合ループ内の再取得でも同じ。
3. decision ∈ {skip, suppress, hold, budget} → report + summary 書いて exit 0（budget/hold は `::warning::`）
4. decision = `dispatch` → **直前再照合**: `getRefSha` を再読し SHA が動いていたら対象を新 SHA に切替えて `listRuns`+`decide` をやり直す（再取得も `truncated` なら step 2 と同じく `history-truncated`）。再照合ループは `MAX_RECHECK = 2` で打ち切り、動き続けるなら `main-moved`（warning・dispatch しない — 誤 expected_sha の run を燃やさない）
5. dispatch 直前に既存 `workflow_dispatch` run の id 集合 `preDispatchIds` を記録 → `POST dispatches {ref: main, inputs: {expected_sha: sha}}`
6. **有界確認 poll**: `DAILY_CONFIRM_INTERVAL_MS` 間隔・`DAILY_CONFIRM_TIMEOUT_MS` 上限で `listRuns` を再取得し、`event === workflow_dispatch ∧ id ∉ preDispatchIds` の新規 run を待つ
   - 発見 → `dispatch-confirmed`、着地 `head_sha` を記録。`landed !== expected` なら `dispatch-landed-mismatch`（warning — その run は `expected_sha` 不一致で `ci-summary` が Fail にし、期待 SHA は検証済みにならない）
   - timeout → `dispatched-unconfirmed`（warning — dispatch POST は 204 成功、登場が遅延。eventual consistency を失敗と誤記しない）
7. report JSON + Job Summary を出力

**exit code**: 0 = 全ての正常 outcome（budget-exhausted/hold-unknown/history-truncated/main-moved/dispatched-unconfirmed/landed-mismatch を含む — scheduler の決定自体は完走し、未検証状態は warning + report に明示）。1 = `error`（API 失敗・malformed response・token/repo 未設定・fixture 破損など、scheduler 自身が決定を完遂できなかった場合）。2 = 用法エラー。**scheduler job の緑/赤は「決定の実行可否」を表し、main の検証状態を表さない** — 未検証は outcome + `::warning::` + report で可視化する。

**fixture モード**: `--fixture <file>` は client を `{refReads: [sha...], runsPages: [[run...]], postDispatchRuns: [[run...]...], dispatchError, now}` の canned response で置換し、confirm poll の sleep をスキップする。実 API/副作用なしで orchestration 全体（再照合・dispatch・確認・report）を deterministic に再現する。`--dry-run` は 4 以降を skip し `dry-run` outcome を出力（実 dispatch しない）。

#### 出力

- Job Summary（`GITHUB_STEP_SUMMARY`、または `--summary <file>`）: outcome・理由・判定 SHA・最新成功 SHA+時刻・対象 attempt 表（run id/event/status/conclusion/attempt/created_at/actor/url）・auto dispatch 回数/上限・dispatch 詳細（Run URL・着地 SHA・確認結果）・warnings
- `daily-ci-report.json` artifact（`--report`）: `{schemaVersion: 1, kind: "daily-ci-decision", repository, workflow, mainRef, observedShas: [...], decisionSha, outcome, reason, latestVerified, attemptsOnDecisionSha, autoDispatchCount, unidentifiableCount, maxAutoDispatches, historyTruncated, dispatch: {...}, warnings: [], generatedAt}`

### `.github/workflows/daily-ci.yml`

```yaml
name: daily-ci
on:
  schedule:
    - cron: "17 21 * * *"        # 06:17 JST 毎日（UTC 定義・分 != 0）
  workflow_dispatch:
    inputs:
      dry_run: { description: "判定のみ（dispatch しない）", required: false, type: boolean, default: false }
permissions: {}
concurrency:
  group: daily-ci-${{ github.workflow }}   # scheduler 自身の重複抑止・直列化
  cancel-in-progress: false                # 判定/dispatch 途中の打ち切りをしない
jobs:
  decide:
    runs-on: ubuntu-latest
    timeout-minutes: 15
    permissions:
      contents: read     # repo script の checkout
      actions: write     # runs list(read) + dispatches POST(write)
    steps:
      - checkout
      - mise-action + mise install --locked node    # ci-summary と同一の pinned toolchain 経路
      - self-test: mise run test-ci-daily           # 壊れた判定器が誤 decision を出さない（ci-summary パターン踏襲）
      - decide: mise exec -- node ci/daily/decide.mjs
          env: GITHUB_TOKEN, GITHUB_REPOSITORY, DAILY_* , DRY_RUN=inputs.dry_run
      - upload artifact: daily-ci-report-attempt-<n>
```

- `permissions: {}` を workflow 全体に置き、`actions: write` + `contents: read` を必要なその 1 job に限定（#367 項目 1）。`actions: write` は list runs（read）と create dispatch（write）の両方を含む単一 scope。
- `concurrency` は run 単位直列化。`cancel-in-progress: false` — 判定〜dispatch の途中打ち切りで不整合（dispatch 済みで報告なし）を作らない。`push`/`pull_request` trigger を持たないため PR 由来の cancel は発生しない。
- job 名 `decide` / workflow 名 `daily-ci` は check context になる — required context ではないため命名は自由だが、`daily-ci` は #365 受入の横断名（3 repo 共通）に合わせる。

## Data and Control Flow

```
schedule(UTC 06:17 JST) / workflow_dispatch
  └─ decide job (ubuntu, actions:write)
       ├─ test-ci-daily（自己検証）
       ├─ getRefSha(main) → S1
       ├─ listRuns(ci.yml, branch=main, ≤10 pages)
       ├─ decide(S1, runs)
       │    ├─ suppress-in-flight / hold-unknown / skip-verified / budget-exhausted
       │    │      → summary + report → exit 0
       │    └─ dispatch
       ├─ 再照合 getRefSha → S2（≠S1 なら再判定、最大2回まで。超過 → main-moved）
       ├─ POST dispatches {ref: main, expected_sha: S}
       ├─ 確認 poll ≤120s → dispatch-confirmed / landed-mismatch / unconfirmed
       └─ summary + daily-ci-report.json artifact
```

**fail-closed 不変条件**:
- `success` と記録できるのは「`ci.yml` run がその SHA で `completed`+`success` と API が報告した」場合のみ。API 欠落・未知値・判定不能は決して skip 側に倒さない。
- 反対側も対称: `error`（scheduler 失敗）を除き、decision を完遂した outcome は exit 0 — ただし `budget-exhausted`/`hold-unknown`/`main-moved`/`dispatched-unconfirmed`/`dispatch-landed-mismatch`/`history-truncated` は `::warning::` + report warnings に記録し、「緑の run が未検証を隠す」ことを防ぐ。
- retry 上限を超えた SHA は scheduler が触れない — 失敗 Issue への集約は #368 tracker の責務（scheduler は Issue を作らない）。手動 dispatch（ユーザ actor）は budget を消費しないため運用再開の手段が残る。

## Non-Functional Constraints

- stdlib-only Node（`ci/summary` と同一方針）。fetch は Node 26 の組込み。
- API call の retry なし（bounded retry は run 単位・日次粒度で設計済み。瞬間的な 5xx で scheduler が暴走しないよう、失敗はその日の error で閉じる）。
- 機密の非露出: token を summary/report/stdout に書かない。API 応答からの転写は run の公開メタデータ（id/sha/status/url/actor 名）のみ。
- 重い runner を使わない: 判定 job は ubuntu-latest のみで数分以内（timeout 15 分・confirm poll 2 分上限）。

## Consumer 採用契約（R10）

- 再利用単位 = `ci/daily/`（script + tests）+ `.github/workflows/daily-ci.yml`。Ruviform の Cargo/Frontend build/install・外部サービス・追加 secret を要求しない（`GITHUB_TOKEN` のみ）。
- 導入手順: (1) 上記ファイルを複製（vendoring。上流版は ruviform の commit SHA をファイル冒頭コメントに記録）(2) env の `DAILY_TARGET_WORKFLOW`/`DAILY_MAIN_REF` を自 repo に合わせる（`ci.yml`/`main` なら無変更）(3) 対象 `ci.yml` が `workflow_dispatch` + `expected_sha` 入力契約を持つこと（未対応なら `expected_sha` なし dispatch も可能 — ただし SHA race 検出は `ci-summary` 側保証に依存しなくなり、着地 SHA 報告のみで観測）(4) `test-ci-daily` 相当の `node --test` を self-test として残す。
- 権限最小化: job は `contents: read` + `actions: write` のみ。`issues: write` 等は tracker（#368 側）の責務で、本 workflow には持たせない。
- 再利用 Action 化（composite `action.yml`）は評価したが、cross-repo `uses:` は参照解決・versioning・private repo での取得経路に追加の不確実性を持ち、vendoring で同等の安全性が得られるため本 Issue では不採用（後続 Issue で要検討とする）。

## 運用手順（R11）

- 手動再判定: Actions → `daily-ci` → Run workflow（`dry_run` で判定のみ）。
- 定期 run 欠落: schedule run は GitHub 障害で起動しないことがあり、本方式では自己検出できない。`daily-ci` の Job Summary/report の「最終成功検証時刻」が日次確認の目安。長期欠落が疑われる場合は手動 `workflow_dispatch` で再判定する。外部監視の要否は後続判断。
- `budget-exhausted` からの復帰: 失敗原因を修正して main を進める（新 SHA で budget リセット）、または `ci.yml` を手動 dispatch（ユーザ actor は budget 非計上）。失敗 Issue の集約・close は #368 tracker に従う。

## Risks and Trade-offs

- **bot actor 識別の近似**: `workflow_dispatch` run は「`actor.login` と `triggering_actor.login` が両方とも識別可能な非 auto actor」でのみ手動扱いとし、それ以外（bot が片側、null/空/破損の片側未識別）を自動 dispatch として計上する。他 automation が `ci.yml` を dispatch すれば同一 budget に計上され、上限を早く使い切り得る — 安全側だが、`ci.yml` を dispatch する自動化を増やす場合は budget 計上に注意が必要（運用 note）。人の rerun は budget を払い戻さない（`actor` が bot のまま残る）。
- **判定〜dispatch 間の手動 dispatch race（残存・限界を保証と書かない）**: 初回 `listRuns` と POST の間に誰かが `ci.yml` を手動 dispatch した場合、本 scheduler は検出できず dispatch を送る — `dispatches` API に条件付き dispatch（idempotency key 相当）は存在せず、これは抑止不能な窓。結果は「同一 SHA に並行 run が 2 つ」であり誤検証にはならない（各自の `expected_sha`/checkout 実測で `ci-summary` が評価する）。軽減: scheduler 自身は `concurrency` で直列化済み、並行 run が queued 化すれば次回判定が `suppress-in-flight` で抑止、本 scheduler の dispatch も auto 計上されるため bounded retry が暴走を止める。
- **未観測 run の race（限界）**: 手動 dispatch が `listRuns` 取得直前に作成されても、run が list に反映されるまでの可視化遅延で見えないことがある — 履歴そのものが状態正本である以上、作成〜可視化の窓は原理的に残る。同様に、unknown な古い run が API 側で後から更新されても取得時点では検出不能。これらは「取得時点の履歴に基づく判定」という本方式の限界であり、誤検証を防ぐ方向（in-flight なら suppress、着地 SHA 報告、ci-summary の expected_sha 照合）で受け止める。保証できないものを保証とは書かない。
- **確認 poll の残存 race**: `preDispatchIds` 差分で新規 dispatch を同定するが、同時刻の手動 dispatch は区別できない（「我々の run が着地した」ことの保証ではなく「ある新規 dispatch を観測した」の意味）。着地 SHA・actor は report で可視化される。
- **schedule ずれ**: GitHub の schedule は分〜時単位で遅延し得る。毎時 0 分を避けることは負荷分散のみで、時刻精度は保証しない（契約として日次粒度）。
- **rerun（manual re-run）**: 人間が失敗 run を re-run すると `run_attempt` が増え最新 attempt の status/conclusion に置き換わる（`run_started_at`/`updated_at` が最新 attempt の時刻を指し、`created_at` は初回作成のまま — 時系列順序は attemptTime で評価する）。dispatch 数は増えない（budget 非消費）— rerun は「同じ dispatch の再試行」であり budget 契約と整合。
- **`ci.yml` 変更の影響**: `expected_sha` 照合・fail-closed は `ci-summary` 側の保証。`ci.yml` が contract を変えた場合、scheduler は着地 SHA 報告で矛盾を可視化するが、照合自体は dispatch 先の責務（層の境界を越えない）。

## Verification Notes

- [ ] `mise run test-ci-daily` — fixture 行列: skip/suppress/hold/budget/dispatch/dispatch-confirmed/landed-mismatch/unconfirmed/main-moved/error/dry-run、auto dispatch 計上（bot actor・手動 actor 非計上）、attemptsOn 厳格性（他 sha/branch/event 除外）、latestVerified、report/summary 形状、exit code。
- [ ] `mise exec -- actionlint .github/workflows/daily-ci.yml`。
- [ ] `mise exec -- prek run --all-files`（check-yaml 含む）。
- [ ] `git grep` で repository variable・DB 依存の不在、`ci.yml` trigger/inputs 非変更を確認。
- [ ] 実 dispatch 等の副作用ゼロ（fixture のみ）。

## Deferred Design Concerns

- 外部監視（schedule 欠落の検出） | 本方式の固有制約（#367 項目 8） | 運用観測後に別 Issue | 最終成功時刻の可視化で代替
- `triggering_actor` 近似の厳密化 | list API が run inputs を返さない制約（実測） | API 拡張 or artifact 証跡（ci-summary-report の expectedSha）を取り込む後続設計 | 現時点は保守的計上
- composite action 化 | cross-repo `uses:` の versioning/取得経路の不確実性 | Consumer 実導入時に再評価 | vendoring で十分
- 失敗 Issue 連携 | #368 tracker の責務境界 | tracker 完成時 | scheduler は Issue を作らない
