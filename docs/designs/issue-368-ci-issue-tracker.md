---
id: "DESIGN-UGm4KjCwx7pEetENeTzIwB"
type: "design"
status: "approved"
title: "Issue #368 CI failure Issue Tracker 設計 — 有効 run 判定・1 Issue 集約・完了順逆転・coverage close 条件"
created: "2026-11-06"
updated: "2026-11-06"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/368"
    - "https://github.com/scarecrow173/ruviform/issues/365"
    - "docs/specs/issue-368-ci-issue-tracker.md"
  changes:
    added:
      - type: "file"
        path: ".github/workflows/ci-issue-tracker.yml"
      - type: "directory"
        path: "ci/tracker"
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
    - "docs/specs/issue-368-ci-issue-tracker.md"
    - "docs/adr/issue-368-ci-failure-issue-tracker-workflow-run-issue-run-id-attempt-compare-2-g.md"
  implemented-by:
    - "docs/plans/issue-368-ci-issue-tracker.md"
  depends-on:
    - "docs/specs/issue-368-ci-issue-tracker.md"
    - "docs/designs/issue-366-full-ci-stage1.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-366-full-ci-stage1.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-368-ci-issue-tracker.md"
    - "docs/adr/issue-368-ci-failure-issue-tracker-workflow-run-issue-run-id-attempt-compare-2-g.md"
  derived-by:
    - "docs/plans/issue-368-ci-issue-tracker.md"
    - "docs/test-specs/issue-368-ci-issue-tracker.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-368-ci-issue-tracker.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/367"
    - "https://github.com/scarecrow173/ruviform/issues/294"
  defers: []
  deferred-by: []
---

# Issue #368 CI failure Issue Tracker 設計 — 有効 run 判定・1 Issue 集約・完了順逆転・coverage close 条件

## Context

Issue #366 Stage 1（merged）で `ci.yml`（`name: ci`）は全 lane 無条件実行 + 終端 `ci-summary` の fail-closed run になった。run conclusion が `success` なら全 lane が同一 revision を検証したことが `ci-summary` 内部で証明済み（実測 checkout 証跡を含む）— つまり tracker は **run の `conclusion` と `head_sha` を信用してよく**、artifact の再評価は要らない。

残る問題: `workflow_run` 完了イベントは順不同で届き、同一 sha の複数 run（rerun・手動+daily 重複）・main 更新・fork/PR run が混在する。Issue の現在状態は常に「run id が最大の有効 run」を反映し、close は「失敗 sha をカバーする、失敗 run より新しい完全成功」だけが行う。

## Components and Boundaries

### `.github/workflows/ci-issue-tracker.yml`

- `name: ci-issue-tracker`。`on.workflow_run: {workflows: [ci], types: [completed], branches: [main]}` — trigger 側でも main に絞る（defense in depth。script 側でも再検証）。
- trigger 自体は `pull_request`/`push`/`schedule`/`workflow_dispatch` を持たない — tracker は他のイベントでは絶対に起動しない。`workflow_dispatch` も持たないため手動起動は不可（手動対応は GitHub UI での Issue 操作 — README 記載）。
- `concurrency: {group: ci-issue-tracker-${{ github.repository }}, cancel-in-progress: false}` — 完了イベントの直列化。進行中の tracker を潰さず順番に処理する。
- workflow 直下 `permissions: contents: read`。tracker job `track`（`ubuntu-latest`）のみ job-level `permissions: {contents: read, actions: read, issues: write}` — `contents: read` は `compare`/branches API と default branch checkout に必要（private repo で GITHUB_TOKEN の API read に必須）。
- steps: `actions/checkout`（**default branch のコード** — `workflow_run` job は `GITHUB_SHA` = default branch tip で動く。失敗した run の head・PR head・artifact は一切 checkout/download/実行しない）→ `jdx/mise-action`（`MISE_ENABLE_TOOLS: node` のみ）→ `mise run test-ci-tracker` で判定器の自己検証（`ci-summary` job の `test-ci-summary` 先行パターンと同じ — 壊れた判定器が Issue を誤操作しない）→ `node ci/tracker/tracker.mjs`（`GH_TOKEN` = `secrets.GITHUB_TOKEN`・`TRACKER_EVENT_PATH` = `github.event_path`）。
- 判定器の失敗（exit != 0）は job 失敗として Actions UI に残る — API 障害・権限不足・marker 破損は全て診断可能な赤い run になる（R8）。

### `ci/tracker/tracker.mjs`（stdlib-only・client 注入型）

3 層に分離し、判定は純関数・副作用は注入 client のみ:

1. `classifyRun(payload, scope)` → `{kind: "skip"|"failure"|"success"|"operational", run}`:
   - **有効 run フィルタ**（いずれか非一致なら `skip`・Issue に一切触れない）: `payload.action == "completed"`、`workflow_run.head_repository.full_name == scope.repo`、`head_branch == scope.branch`、`workflow_run.event ∈ scope.allowedEvents`（既定 `["push","workflow_dispatch"]` — `pull_request`/`merge_group`/未知 event は skip。fork run は repo 一致で除外済みだが二重に確認）、`workflow_run.conclusion` 存在。
   - **conclusion 分類**: `success`→success。`failure`/`timed_out`/`startup_failure`→failure（検証未完了のまま終了した run — 原因の自動分類はしない）。`cancelled`/`skipped`/`neutral`/`stale`/`action_required`/null/未知値→operational（**成功でも失敗起票でもない**。retry 待ち運用状態）。
   - run メタ: `id`（整数必須）・`head_sha`（40 hex 検証）・`html_url`（`https://github.com/` prefix 検証）・`run_attempt`・`updated_at`。

2. `plan(classified, {issues, compare, jobs})` → action list。全判定はここで純粋に決まる:

   | 状況 | action |
   |---|---|
   | skip | `noop`（Job Summary のみ） |
   | operational + canonical open あり + run が最新より新しい | `update`（operational note のみ — 失敗状態は変えない） |
   | operational + open 無し / stale run | `noop` |
   | failure + managed open 無し | `create`（+ 複数 open 時は duplicate close） |
   | failure + canonical open + run が state.failure より新しい | `update`（失敗状態を新 run に更新 + history 追記） |
   | failure + canonical open + run が新しくない | `noop`（stale — 古い完了が新しい状態を上書きしない） |
   | success + managed open 無し | `noop` |
   | success + canonical open + run が state.failure より新しい かつ `compare(state.failure.sha → run.sha) ∈ {identical, ahead}` | `close`（回復 SHA/URL を comment + `state: closed`。複数 managed open は各 Issue の state で個別判定） |
   | success + canonical open + 上記不成立（behind/diverged/古い run.id/compare 失敗） | `noop`（Issue は open のまま — fail-closed。compare API 失敗は plan ではなく apply で例外→job 失敗） |

   - **canonical 選定**: label `ci-failure` を持つ open Issue のうち、body 先頭の `<!-- ci-issue-tracker:{json} -->` marker が parse 可能で `scope` が一致するもののみ managed。managed が複数なら最小 Issue 番号を canonical。残りの managed open は **duplicate として close**（comment「superseded by #N」+ `state: closed`）— `search` API の index 遅延を避けるため `GET /issues?labels=…&state=open`（list endpoint・非検索）で同定する。label だけで marker の無い Issue（人間が label を付けた等）は foreign として一切触れない。
   - **run 順序キー**: 「最新の有効 run」の秩序キーは **`(run.id, run_attempt)` の辞書順**。run id は repo 内で単調増加するため完了時刻・`updated_at` より堅牢で、同一 run の rerun（`run_attempt` 増分・id 同一）は「より新しい証拠」として扱う — 失敗した run の rerun 成功は `identical` coverage で正しく close できる（id だけだと永久に stale になる）。完了順逆転対策の本体。
   - **close 条件の 2 重 guard**: `isNewerRun(成功 run, failure)`（id が大きいか、同 id で attempt が大きい = 新しい証拠）∧ `compare ∈ {identical, ahead}`（失敗 sha が成功 run の sha の祖先 or 同一 = その失敗は newer green で確実にカバー）。`behind` は「古い revision の成功」、`diverged` は「失敗 sha が main 履歴に残っていない（force push 等）」— 両方 close しない（diverged は手動確認が要る状態として Job Summary に記録）。
   - **同 sha で failure+success が混在**する場合: `isNewerRun` を要求するため「同じ sha に対する後からの成功 rerun」は close 可能（failure attempt より新しい証拠）だが、逆（failure が新しい）なら open のまま — flaky と確定できない以上 open 側が安全。

3. `renderBody(state)` / `parseMarker(body)` / `apply(actions, client)`:
   - body = 先頭 marker comment（1 行 JSON・`v`/`scope`/`state`）+ 人間向けセクション（status・失敗 SHA・Run URL・失敗 job 表・初回検出/最終検出・履歴最大 10 件・対応手順・運用方針）。
   - **sanitize**: sha は `^[0-9a-f]{40}$`（違反は `?` 表記かつ marker には書かない）・URL は `https://github.com/` prefix のみ・job 名/OS/branch は制御文字・`|`・バッククォート・改行を除去し 100 chars 上限・conclusion は allowlist（違反は `unknown`）・日時は ISO 文字列検証。marker JSON は `{v, scope, state}` のみを持ち人間向け装飾を含まない。
   - `apply` は注入 `client`（後述）を通じて API を実行し、**いかなる API 失敗も catch しない**（例外→`main` で非 0 exit→job 失敗）。順序: `ensureLabel` → `listOpenIssues` →（failure/success なら）`getRunJobs` →（success close 判定時のみ）`compareCommits` → create/update/close/comment。

### GitHub API client（`gh api` 経由・テストでは mock）

- `makeGhClient({repo, ghPath})` — `execFileSync(gh, ["api", ...])` で以下を実装（`GH_TOKEN` env、JSON は `--jq`/`--input` ではなく `-f`/`--paginate` を使う）:
  - `ensureLabel(name,color)` → `POST /repos/{repo}/labels`（422 `already_exists` のみ許容 → `GET` で存在確認、他の 422/権限不足は失敗伝播）
  - `listOpenIssues(label)` → `GET /repos/{repo}/issues?labels={label}&state=open&per_page=100` `--paginate`（list endpoint — 検索 index 遅延を受けない。PR は `pull_request` key で除外）
  - `getRunJobs(runId)` → `GET /repos/{repo}/actions/runs/{runId}/jobs?per_page=100` `--paginate`（`jobs[]` から `name`/`conclusion`/`html_url`）
  - `compareCommits(base, head)` → `GET /repos/{repo}/compare/{base}...{head}` の `status`（`identical|ahead|behind|diverged`、他は異常として例外）
  - `createIssue({title,body,labels})` → `POST /repos/{repo}/issues`・`updateIssue(n,{title,body})` → `PATCH`・`addComment(n,body)` → `POST /repos/{repo}/issues/{n}/comments`・`closeIssue(n)` → `PATCH state=closed`
- テストでは `client` オブジェクトを差し替え、`plan`/`run` を完全 offline で検証する。`run(event, client)` は `classifyRun` → `plan` 用の state 取得 → `apply` の orchestrator。

## Data and Control Flow

- `ci` run 完了 → `workflow_run` 発火 → `track` job（concurrency 直列）→ event payload 読込 → `classifyRun`（skip なら即終了・exit 0）→ `ensureLabel`+`listOpenIssues` → failure/success なら `getRunJobs`（失敗 job 表用）→ `plan` → `apply` → Job Summary（event 分類・対象 run・採用/棄却理由・実行 action・Issue URL）。
- Issue body 内の marker state（JSON）:
  `{v:1, scope:{repo,workflow:"ci",branch:"main"}, state:{latestRunId, latestAttempt, failure:{sha,runId,attempt,runUrl,conclusion,firstSha,firstAt,lastAt,jobs[]}, history:[{runId,attempt,sha,conclusion,at,kind} ≤10], operationalNote}}`
- 手動 close 方針: tracker は **closed Issue を再 open しない**。close 済み Issue は list（state=open）に現れないため、人間 close 後の次回失敗は **新規 Issue 作成**（新しい障害エピソード）。これを README と Issue 本文の運用方針節に明記する。

## Non-Functional Constraints

- stdlib-only Node.js（`ci/summary` と同方針）。外部 action 追加は既存 `actions/checkout`/`jdx/mise-action` の再利用のみ。
- tracker は失敗 run のコード・artifact・checkout を一切触れない。実行する `tracker.mjs` は default branch のもの — `workflow_run` の信頼モデル上、privilege を持つのは常に default branch の workflow/script のみ。
- 秘密の非露出: `GH_TOKEN` は `secrets.GITHUB_TOKEN` のみ・env 経由・ログに出さない。Issue 本文に token・env・内部パスを書かない。
- 判定器の自己検証: `track` job は `test-ci-tracker` を apply 前に実行し、壊れた判定器による誤 Issue 操作を防ぐ。
- fail-closed: unknown conclusion・API 失敗・marker 破損・compare 不明値は全て「close/成功扱いしない」側に倒すか job 失敗にする。`search` API ではなく list endpoint を使うのも同じ理由。

## Risks and Trade-offs

- **run id 単調増加への依存**: GitHub 仕様として run id は増加するが契約保証は弱い。`updated_at` との併用も検討したが、完了順逆転対策には「発生順」の方が意味論的に正しく、`(id, attempt)` が最も単純な近似。誤った順序でも影響は「stale として扱う/扱わない」だけで誤 close には至らない（close は coverage guard が別途ある）。
- **manual close → 新 Issue**: 人間が誤 close すると同じ障害でも新 Issue が立つ（Issue 数の増加）。reopen する方針は「人間の判断を機械が覆す」ため却下 — 本文の運用方針で明示する。
- **fork/外部 contributor による workflow file 変更の影響**: `workflow_run` job は default branch の workflow file を使うため、fork が tracker 権限を拡張することはできない。PR で tracker workflow を編集しても merge まで反映されない — ただし悪意のある collaborator が `ci-issue-tracker.yml` を main に入れれば `issues: write` で Issue 操作できるのは GitHub 権限モデルそのもの（ruleset `main-protect` の PR 必須で緩和）。
- **diverged 放置**: main force push 等で失敗 sha が履歴から消えると compare が永久に `diverged` → Issue は自動 close しない。手動 close の運用を README に明記する（fail-closed 側の選択）。

## Deferred Design Concerns

- `consumer-verify` 追跡・複数 scope 管理 | scope を `workflow`/`検証対象` 軸で拡張可能な marker 構造にはした | #294 別ステップ | 本設計は `ci`×`main` 固定
- Tracker 自身の実行失敗監視 | tracker job 失敗は Actions UI に残るが通知は無い | 運用観測後 | scheduler の Job Summary と併せて確認手順を README に記載
- `workflow_run` 遅延・大量完了時の rate limit | concurrency 直列化で 1 run=1 tracker run。jobs/list/compare は数 call のみ | 運用観測後 | `--paginate` で 100+ Issue/job に対応済み

## Verification Notes

- [ ] `mise exec -- actionlint .github/workflows/ci-issue-tracker.yml` pass。
- [ ] `mise run test-ci-tracker`（`node --test ci/tracker/`）: classify 行列・plan 全経路・marker 往復・sanitize・mock client の API 呼出列・API 失敗伝播。
- [ ] `mise exec -- prek run --all-files` pass。
- [ ] `ci.yml`/`consumer-verify.yml` に差分が無いこと（`git diff` で確認 — 既存 trigger・required context 不変）。
- [ ] 実 GitHub API 呼出し・実 Issue 操作は行わない（全て mock client・offline）。
