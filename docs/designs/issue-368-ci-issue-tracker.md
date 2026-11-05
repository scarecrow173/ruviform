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

残る問題: `workflow_run` 完了イベントは順不同で届き、同一 sha の複数 run（rerun・手動+daily 重複）・main 更新・fork/PR run が混在する。さらに tracker workflow の `concurrency` group は running 1 + pending 1 に制約され、**新しい pending がキュー中の pending を置き換える**ため、`cancel-in-progress: false` でも完了通知そのものが落ち得る — 直列化だけでは全通知の順序処理を保証できない。そこでイベントを「トリガ」とみなし、**権威ある run 一覧（`actions/workflows/{file}/runs`）と marker 状態を毎回突き合わせて最も新しい証拠で判断する**（reconciliation）。Issue の現在状態は常に最新証拠を反映し、close は「最新証拠が失敗 sha と現在 main tip をカバーする、曖昧なく新しい完全成功」だけが行う。

## Components and Boundaries

### `.github/workflows/ci-issue-tracker.yml`

- `name: ci-issue-tracker`。`on.workflow_run: {workflows: [ci], types: [completed], branches: [main]}` — trigger 側でも main に絞る（defense in depth。script 側でも再検証）。
- trigger 自体は `pull_request`/`push`/`schedule`/`workflow_dispatch` を持たない — tracker は他のイベントでは絶対に起動しない。`workflow_dispatch` も持たないため手動起動は不可（手動対応は GitHub UI での Issue 操作 — README 記載）。
- `concurrency: {group: ci-issue-tracker-${{ github.repository }}, cancel-in-progress: false}` — 完了イベントの直列化。進行中の tracker を潰さず順番に処理するが、pending 枠は 1 件のみで新しい pending は既存 pending を置き換える — **通知の落失を防げないため reconciliation で補う**（workflow コメントにも明記）。
- workflow 直下 `permissions: contents: read`。tracker job `track`（`ubuntu-latest`）のみ job-level `permissions: {contents: read, actions: read, issues: write}` — `contents: read` は `compare`/branches API と default branch checkout に必要（private repo で GITHUB_TOKEN の API read に必須）。
- steps: `actions/checkout`（**default branch のコード** — `workflow_run` job は `GITHUB_SHA` = default branch tip で動く。失敗した run の head・PR head・artifact は一切 checkout/download/実行しない）→ `jdx/mise-action`（`MISE_ENABLE_TOOLS: node` のみ）→ `mise run test-ci-tracker` で判定器の自己検証（`ci-summary` job の `test-ci-summary` 先行パターンと同じ — 壊れた判定器が Issue を誤操作しない）→ `node ci/tracker/tracker.mjs`（`GH_TOKEN` = `secrets.GITHUB_TOKEN`・`TRACKER_EVENT_PATH` = `github.event_path`）。
- 判定器の失敗（exit != 0）は job 失敗として Actions UI に残る — API 障害・権限不足・marker 破損は全て診断可能な赤い run になる（R8）。

### `ci/tracker/tracker.mjs`（stdlib-only・client 注入型）

3 層に分離し、判定は純関数・副作用は注入 client のみ:

1. `classifyRun(payload, scope)` → `{kind: "skip"|"failure"|"success"|"operational", run}`:
   - **有効 run フィルタ**（いずれか非一致なら `skip`・Issue に一切触れない）: `payload.action == "completed"`、`workflow_run.status == "completed"`（action と矛盾・欠落する status は skip — null を completed と仮定しない）、`workflow_run.head_repository.full_name == scope.repo`、`head_branch == scope.branch`、`workflow_run.event ∈ scope.allowedEvents`（既定 `["push","workflow_dispatch"]` — `pull_request`/`merge_group`/未知 event は skip。fork run は repo 一致で除外済みだが二重に確認）、`workflow_run.conclusion` 存在。
   - **conclusion 分類**: `success`→success。`failure`/`timed_out`/`startup_failure`→failure（検証未完了のまま終了した run — 原因の自動分類はしない）。`cancelled`/`skipped`/`neutral`/`stale`/`action_required`/null/未知値→operational（**成功でも失敗起票でもない**。retry 待ち運用状態）。
   - run メタ: `id`（整数必須）・`head_sha`（40 hex 検証）・`html_url`（`https://github.com/` prefix 検証）・`run_attempt`・`updated_at`。

2. **evidence reconciliation**（`run` 内部・`plan` 前段）: 非 skip イベントで権威ある run 一覧を取得し、payload・全 managed marker 状態と突き合わせる。一覧が payload より新しい attempt/run を示せばそちらを採用（一覧遅延では payload が勝つ）。一覧 API 失敗・非配列レスポンスは伝播して job を落とす — 証跡欠損で黙って処理しない。

   履歴 record は `classifyHistoryRecord` で 3 分類する:
   - **out**: 提示 field が scope と矛盾（他 repo・fork head・他 branch・許可外 event・他 workflow `path`/`name`）→ 証拠から除外。
   - **in**: repo/branch/event/workflow 同一性と `id`/`head_sha`/`run_attempt`/`updated_at`/`status` が全て検証可能。`completed` → conclusion 分類で証拠化、非 completed（`queued`/`in_progress`/`requested`/`waiting`/`pending`/未知 status）→ **`pending` 証拠**（live run — 完了は未来にあるため常に「より新しい可能性」を持ち、success 昇格も close もさせない）。
   - **unknown**: 対象性または順序が証明不能（上記必須 field 欠落・record 自体不正）→ **`unknown` 証拠** — 黙って捨てず、close 抑制と stale-failure 起票抑止の両方を止める。success/failure には決してならない。

3. `plan({classified, canonical, duplicates, mergedState, newest, newestFailure, coveringSuccess, coverage, mainCoverage, jobs, evidencePending, evidenceUnknown})` → action list。全判定はここで純粋に決まる:

   | 状況 | action |
   |---|---|
   | skip | `noop`（Job Summary のみ） |
   | 最新証拠が operational + canonical open あり + 記録最新より新しい | `update`（operational note のみ — 失敗状態は変えない。ただし証跡中に記録より新しい失敗があれば併せて反映） |
   | 最新証拠が operational + open 無し + 未解決失敗証拠無し | `noop` |
   | 最新証拠が operational + open 無し + 未解決失敗証拠あり | `create`（最新検証は pending — 最新失敗で起票） |
   | 失敗証拠が未解決 + managed open 無し | `create`（+ 複数 open 時は duplicate close） |
   | 最新証拠が失敗 + canonical open + 記録失敗より新しい | `update`（失敗状態を最新失敗 run に更新 + history 追記） |
   | 最新証拠が失敗 + canonical open + 新しくない | `noop`（stale — 古い完了が新しい状態を上書きしない） |
   | 最新証拠が成功 + managed open 無し + 失敗証拠をカバー | `noop`（遅延した古い失敗通知は起票しない） |
   | 最新証拠が成功 + canonical open + **全 guard**（後述） | `close`（回復 SHA/URL を comment + `state: closed`） |
   | 最新証拠が成功 + canonical open + guard 不成立 | `update` or `noop`（Issue は open のまま — fail-closed。理由を operational note に記録） |

   - **canonical 選定**: label `ci-failure` を持つ open Issue のうち、body 先頭の `<!-- ci-issue-tracker:{json} -->` marker が parse 可能で `scope` が一致するもののみ managed。managed が複数なら最小 Issue 番号を canonical。残りの managed open は **duplicate として close**（comment「superseded by #N」+ `state: closed`）— `search` API の index 遅延を避けるため `GET /issues?labels=…&state=open`（list endpoint・非検索）で同定する。label だけで marker の無い Issue（人間が label を付けた等）は foreign として一切触れない。**duplicate close 前に全 marker の最新状態を merge して canonical に反映する**（duplicate が最新失敗を握っていても消えない）。
   - **run 順序キー（atomic tuple）**: 同一 run id は `run_attempt` で比較（attempt は厳密に逐次）。異なる run id は `updated_at` で比較 — **旧 run の新しい attempt が新しい run より後に完了し得る**ため `(run.id, run_attempt)` 辞書順では過小評価する。`updated_at` が欠損・異 run で同一値なら `ambiguous` とし、**close/成功判定には一切使わない**（記録は残し Issue は open 維持）。状態更新のみ run id 単調性で保守的に解決する。`run_started_at` は attempt の開始時刻で完了順の証跡にならず、`actor`/`triggering_actor` は順序証跡でないため使わない。`latest`/`failure` の run 参照は `{runId, attempt, updatedAt}` tuple をまるごと採用 — 別 run の値を Math.max で混ぜて実在しない順序キーを捏造しない。
   - **close 条件の多重 guard**: 以下を全て満たす時のみ `close-recovered`。(a) 最新証拠がその成功（最新が operational/ambiguous なら close しない — main は未検証）、(b) その成功が記録最新失敗より**曖昧なく新しい**、(c) `compare(失敗 sha → 成功 sha) ∈ {identical, ahead}`（失敗 revision を履歴に含む）、(d) `compare(現在 main head → 成功 sha) ∈ {identical, ahead}`（**現在の main tip をカバー** — 古い失敗 revision の検証だけでは閉じない）、(e) **証跡が完全**（live `pending` run・`unknown` 識別不能 record が履歴に残っていない）。`behind`/`diverged`/ambiguous/main tip 未カバー/pending・unknown 残存は全て close しない。close 候補で branch head が取れない場合は job を失敗させて誤 close を防ぐ。rerun 成功（同 id・高 attempt）は `updated_at` で新しい証拠と証明され `identical` coverage で close できる。
   - **同 sha で failure+success が混在**する場合: 失敗 attempt より新しい成功は close 可能だが、逆（failure が新しい）なら open のまま — flaky と確定できない以上 open 側が安全。

3. `renderBody(state)` / `parseMarker(body)` / `apply(actions, client)`:
   - body = 先頭 marker comment（1 行 JSON・`v`/`scope`/`state`）+ 人間向けセクション（status・失敗 SHA・Run URL・失敗 job 表・初回検出/最終検出・履歴最大 10 件・対応手順・運用方針）。
   - **sanitize**: sha は `^[0-9a-f]{40}$`（違反は `?` 表記かつ marker には書かない）・URL は `https://github.com/` prefix のみ・job 名/OS/branch は制御文字・`|`・バッククォート・改行を除去し 100 chars 上限・conclusion は allowlist（違反は `unknown`）・日時は ISO 文字列検証。marker JSON は `{v, scope, state}` のみを持ち人間向け装飾を含まない。
   - `apply` は注入 `client`（後述）を通じて API を実行し、**いかなる API 失敗も catch しない**（例外→`main` で非 0 exit→job 失敗）。順序: `ensureLabel` → `listOpenIssues` → `getRunHistory`（権威一覧・必須）→ `getBranchHead`（非 close 経路では診断用・close 候補では必須証跡）→ `compareCommits`（失敗をカバーし得る成功証跡がある時のみ。close 候補では main tip compare も追加）→ `getRunJobs`（失敗状態を書く時のみ）→ create/update/close/comment。

### GitHub API client（`gh api` 経由・テストでは mock）

- `makeGhClient({repo, ghPath})` — `execFileSync(gh, ["api", ...])` で以下を実装（`GH_TOKEN` env、JSON は `--jq`/`--input` ではなく `-f`/`--paginate` を使う）:
  - `ensureLabel(name,color)` → `POST /repos/{repo}/labels`（422 `already_exists` のみ許容 → `GET` で存在確認、他の 422/権限不足は失敗伝播）
  - `listOpenIssues(label)` → `GET /repos/{repo}/issues?labels={label}&state=open&per_page=100` `--paginate`（list endpoint — 検索 index 遅延を受けない。PR は `pull_request` key で除外）
  - `getRunHistory({workflowFile, branch})` → `GET /repos/{repo}/actions/workflows/{workflowFile}/runs?branch={branch}&exclude_pull_requests=true&per_page=30` `--paginate` — 対象 workflow の権威ある最新 run 一覧（各 run の最新 attempt 状態）。event/branch/repo は client 側でも再検証する。
  - `getRunJobs(runId)` → `GET /repos/{repo}/actions/runs/{runId}/jobs?per_page=100` `--paginate`（`jobs[]` から `name`/`conclusion`/`html_url`）
  - `compareCommits(base, head)` → `GET /repos/{repo}/compare/{base}...{head}` の `status`（`identical|ahead|behind|diverged`、他は異常として例外）
  - `getBranchHead(branch)` → `GET /repos/{repo}/branches/{branch}` の `commit.sha`（close 候補での main tip 検証用。非 close 経路では診断用として失敗を許容するが、close 候補では必須証跡 — 取れなければ job 失敗）
  - `createIssue({title,body,labels})` → `POST /repos/{repo}/issues`・`updateIssue(n,{title,body})` → `PATCH`・`addComment(n,body)` → `POST /repos/{repo}/issues/{n}/comments`・`closeIssue(n)` → `PATCH state=closed`
- テストでは `client` オブジェクトを差し替え、`plan`/`run` を完全 offline で検証する。`run(event, client)` は `classifyRun` → Issue/marker 走査・merge → `getRunHistory` reconciliation → plan → `apply` の orchestrator。

## Data and Control Flow

- `ci` run 完了 → `workflow_run` 発火 → `track` job（concurrency 直列）→ event payload 読込 → `classifyRun`（skip なら即終了・exit 0）→ `ensureLabel`+`listOpenIssues` → managed marker 走査・merge → `getRunHistory`（権威一覧）→ evidence 構築（marker refs + payload + history）→ `getBranchHead` → `compareCommits`（coverage 判定が要る時のみ）→ `getRunJobs`（失敗を書く時のみ）→ `plan` → `apply` → Job Summary（event 分類・最新証跡・採用/棄却理由・実行 action・Issue URL）。
- Issue body 内の marker state（JSON）:
  `{v:1, scope:{repo,workflow:"ci",branch:"main"}, state:{latest:{runId,attempt,updatedAt}, failure:{sha,runId,attempt,updatedAt,runUrl,conclusion,firstSha,firstAt,lastAt,jobs[]}, history:[{runId,attempt,updatedAt,sha,conclusion,at,kind,src} ≤10], operationalNote}}`
  （`latest`/`failure` の run 参照は atomic tuple。旧 `latestRunId`/`latestAttempt` フラット形式の marker は parse 時に正規化して読む — `updatedAt` 不明のため異 run との順序は `ambiguous` として保守的に扱う）
- 手動 close 方針: tracker は **closed Issue を再 open しない**。close 済み Issue は list（state=open）に現れないため、人間 close 後の次回失敗は **新規 Issue 作成**（新しい障害エピソード）。これを README と Issue 本文の運用方針節に明記する。

## Non-Functional Constraints

- stdlib-only Node.js（`ci/summary` と同方針）。外部 action 追加は既存 `actions/checkout`/`jdx/mise-action` の再利用のみ。
- tracker は失敗 run のコード・artifact・checkout を一切触れない。実行する `tracker.mjs` は default branch のもの — `workflow_run` の信頼モデル上、privilege を持つのは常に default branch の workflow/script のみ。
- 秘密の非露出: `GH_TOKEN` は `secrets.GITHUB_TOKEN` のみ・env 経由・ログに出さない。Issue 本文に token・env・内部パスを書かない。
- 判定器の自己検証: `track` job は `test-ci-tracker` を apply 前に実行し、壊れた判定器による誤 Issue 操作を防ぐ。
- fail-closed: unknown conclusion・API 失敗・marker 破損・compare 不明値は全て「close/成功扱いしない」側に倒すか job 失敗にする。`search` API ではなく list endpoint を使うのも同じ理由。

## Risks and Trade-offs

- **`updated_at` を順序証跡に使う限界**: `updated_at` はレコード最終更新時刻であり、秒粒度・同一時刻の並走 run では同値になり得る。同値・欠損は `ambiguous` として close/成功判定から外す — fail-closed 側の選択。`run_started_at` は attempt 開始時刻で完了順の証跡にならないため採用しなかった。旧 run の遅い rerun を `updated_at` で正しく「新しい証拠」と判定できるのが本採用の理由。
- **run id 単調増加への依存は保守解決のみに限定**: close には使わず、ambiguous 時の状態更新だけが run id に倒す（誤っても「新しい記録を採用しない/する」の範囲で、誤 close には至らない — close は coverage・main tip・ambiguous 排除の guard が別途ある）。
- **reconciliation の限界**: run 一覧は最新 N 件（per_page=30）のスナップショットで、それより古い marker 状態や一覧 lag は復元できない — 一覧 lag 中は payload の最新 attempt が勝つ設計で補う。pending 置き換えで tracker 自体の run が落ちた場合、その run の処理は行われないが、後続の処理済みイベントが権威一覧を見て代わりに復元する（最終的な収束は「次に処理されるイベント」に依存する — 完了が全て落ちる極端ケースでは次の ci 完了で復元される）。
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
