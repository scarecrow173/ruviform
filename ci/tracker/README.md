# ci/tracker — CI failure Issue Tracker（Issue #368）

`.github/workflows/ci-issue-tracker.yml` が `ci`（Full CI）workflow の
`workflow_run` 完了を監視し、**この repo の `main` に対する有効な
Full CI 実行**だけを対象に、`main × ci` の未解決障害 Issue を原則
1 件に集約する（Issue #365 ルール 5）。GITHUB_TOKEN dispatch で
event が起きない完了は、別 workflow の
`.github/workflows/ci-issue-tracker-reconcile.yml` が定期・手動で
同一判定器を `--reconcile` モードで呼び、権威 run 履歴から回復する
（後述「reconcile 入口」参照）。

## 構成

- `ci-issue-tracker.yml` — `workflow_run: {workflows: [ci], types:
  [completed], branches: [main]}` でのみ起動。job `track` のみ
  `contents: read` / `actions: read` / `issues: write`。
  `concurrency` で repo 単位に直列化（`cancel-in-progress: false`）。
  event 入口は `schedule`/`workflow_dispatch`/`pull_request`/`push` を
  **持たない**（ADR-034SraR8CrkZ0SSa7tQEwh の契約）。
- `ci-issue-tracker-reconcile.yml` — `schedule: "23 */6 * * *"`
  （UTC・6 時間間隔）+ `workflow_dispatch`（入力なし）で起動する
  reconcile 入口。job `reconcile` は `if: github.ref_name == 'main'`
  で非 default ref 起動を skip し、checkout を
  `github.event.repository.default_branch` に pin する。権限・
  concurrency group は event 入口と同一（group を共有して workflow
  横断で直列化）。**TRACKER_EVENT_PATH は渡さない**。
- `ci/tracker/tracker.mjs` — stdlib-only。`classifyRun`（有効 run +
  conclusion 分類）→ run-history reconciliation（権威ある run 一覧と
  marker 状態の突き合わせ）→ `plan`（action list）→ `apply`（`gh api`
  client 注入）。GitHub API は全て client 経由で、テストは mock client。
  `--reconcile` モードは event payload を一切読まず、履歴+marker のみ
  で同一の判定を行う。
- `ci/tracker/tests/tracker.test.mjs` — `node --test` contract tests。
  実 GitHub API・実 Issue・実 dispatch を一切起こさない。

## 有効 run の定義

event モードの payload は JSON object 必須 — `null`/スカラ/配列に parse
される event file は破損入力として job を失敗させる（正規 skip と
区別する。reconcile モードの `payload:null` 正当化に伴い mode 境界を
強化したもので、object の場合の従来分類は不変）。

`workflow_run` payload が以下を全て満たすこと:

- `action: completed`
- `head_repository.full_name == この repo`（fork run 除外）
- `head_branch == "main"`
- `event ∈ {push, workflow_dispatch}`（PR・merge_group・schedule・未知
  event は skip — ci.yml に新 trigger が増えても黙って追跡対象にしない）

## conclusion 分類

| conclusion | 扱い |
| --- | --- |
| `success` | 成功 — coverage を満たす新しい完全成功なら open Issue を close |
| `failure` / `timed_out` / `startup_failure` | 失敗 — open Issue 更新、無ければ作成 |
| `cancelled` / `skipped` / `neutral` / `stale` / `action_required` / null / 未知 | operational — 成功でも失敗起票でもない。open Issue があれば operational note 更新のみ |

失敗原因（コード/依存/Runner 障害）の自動判定は行わない。Consumer 固有
原因の確定・横断 Consumer Issue の自動作成も行わない。

## Issue の同定と集約

- 同定は label `ci-failure` + body 先頭の
  `<!-- ci-issue-tracker:{json} -->` marker（`v`・`scope{repo,workflow,
  branch}`）の両方。一覧は `GET /issues?labels=…&state=open`（list
  endpoint — 検索 index 遅延を受けない）。label のみで marker の無い
  Issue・marker 破損・scope 不一致の Issue は foreign として一切触れない。
- managed open が複数ある場合、最小 Issue 番号を canonical とし、残りは
  「superseded by #N」comment + close で 1 件に集約する。**重複を閉じる
  前に全 marker の最新状態を保全**し、最も新しい失敗記録を持つ marker
  の状態を canonical Issue に引き継ぐ — duplicate が最新障害を握って
  いても失われない。
- 失敗更新はその run が記録済み失敗 run より新しい場合のみ — 完了順
  逆転で古い run が新しい状態を上書きしない（ordering は後述）。

## Evidence reconciliation（通知の喪失・逆転への対策）

イベント payload は「トリガ」であり真実ではない。GitHub Actions の
concurrency group は running 1 + pending 1 に制約され、**新しい pending
run はキュー中の pending を置き換える**ため、`cancel-in-progress: false`
でも tracker 側の workflow_run 通知が間に落ち得る（out-of-order 配送・
eventual consistency も同様）。そこで非 skip イベントごとに
`GET /actions/workflows/{workflow-file}/runs?branch=main&exclude_pull_requests=true`
でこの workflow ・ branch ・有効 event の最新 run 一覧を取得し、payload
と marker 状態と突き合わせてから判断する。

- 履歴により新しい失敗 run が見つかれば、その通知が落ちていても新しい
  失敗で状態を更新する（dropped notification の自己修復）。
- 履歴の最新が成功（coverage 充足）なら、遅延到着した古い失敗通知は
  Issue を起票しない — 到着順が事実にならない。
- 履歴 API 自体の失敗・非配列レスポンスは伝播して job を落とす —
  証跡欠損で黙って成功・close しない（fail-closed）。
- 履歴 record は 3 分類される。**対象外**（他 repo・fork head・他
  branch・許可外 event・他 workflow の `path`/`name`）だけが除外され、
  **対象内**は completed → conclusion 分類、非 completed
  （`queued`/`in_progress`/`requested`/`waiting`/`pending`/未知 status）
  → `pending` evidence として残る。pending run の完了は**未来**にある
  ため、どれだけ古い `updated_at` を持っていても実質最新であり、
  success 昇格も close もしない。
- 対象か非対象か識別できない record（repo/branch/event/workflow 同一性・
  `id`/`head_sha`/`run_attempt`/`updated_at`/`status` のいずれか欠落、
  または record 自体が不正）は `unknown` evidence として残り、close と
  resolved-suppression（stale failure の起票抑止）の両方を止める。
  「識別不能を黙って捨てる」はしない — 欠落項目は常に安全側に扱う。
- イベント payload 側も同様に `status === "completed"` 必須であり、
  `action: completed` と矛盾・欠落する status は skip される。

## Ordering（順序キーは一つの atomic tuple）

- 同一 run id: `run_attempt` で順序づく（attempt は厳密に逐次）。rerun
  は同 id・高 attempt なので失敗 run の rerun 成功は正しく回復になる。
  runs 一覧は各 run の**最新 attempt の状態**だけを返すため、marker に
  記録された失敗 attempt は `(runId, attempt)` 単位の独立した証拠とし
  て保持する — 同 id の新しい attempt（成功・operational・in_progress）
  が一覧に現れても記録済み失敗 attempt は消えず、rerun 成功は自分の
  失敗 attempt を `identical` coverage で close できる（attempt 別の
  実 record が存在する時だけ marker 参照を実 record で上書きする）。
- 異なる run id: `updated_at`（最終更新＝完了時刻）で順序づく。**古い
  run の新しい attempt が新しい run より後に完了し得る**ため、
  `(run.id, run_attempt)` だけでは過小評価する。
- `run_started_at` は attempt の「開始」時刻で完了順の証跡にならず使わ
  ない。`actor`/`triggering_actor` は「誰が」であって順序証跡ではない。
- `updated_at` が欠損、または異なる run で同一値 → `ambiguous`。**曖昧な
  まま成功・close には使わない**（記録は残し、issue は open のまま保持）。
  状態更新のみ run id 単調性で保守的に解決する。
- `latest`/`failure` の run 参照は `{runId, attempt, updatedAt}` の
  tuple をまるごと採用する — 別 run の値を Math.max で混ぜて実在しない
  順序キーを捏造しない。

## reconcile 入口（GITHUB_TOKEN dispatch の通知 gap 対策）

Daily Scheduler（Issue #367）が `GITHUB_TOKEN` で `workflow_dispatch`
した bot Full CI run は dispatch 自体は実行されるが、GitHub の
anti-recursion ルールにより完了時の `workflow_run` event は**発火し
ない**。bot 失敗も bot 成功も event 入口には届かず、起票済み Issue が
閉じない（観測: #375 で manual Replay が必要だった）。

`ci-issue-tracker-reconcile.yml` は同一判定器を event 非依存で呼ぶ
第二の入口:

- `schedule: "23 */6 * * *"`（UTC）— 6 時間ごとに権威履歴を reconcile。
  `workflow_dispatch` — 手動の即時 reconcile / 診断用。
- `tracker.mjs --reconcile` は `TRACKER_EVENT_PATH`/`GITHUB_EVENT_PATH`
  を**一切読まない**（runner が常に設定する ambient path も mode 分離
  のため無視）。`--event` との同時指定は usage error。
- reconcile は履歴+marker のみで event 入口と同一の fail-closed guard
  （上記 close 条件全て）を通る。pending/unknown が残れば close しない。
- event 入口と同一 concurrency group を共有するため同時書き込みは起き
  ない。pending 置換で reconcile が落ちても冪等であり、次回 tick または
  event が同じ権威履歴を見て同じ結論に収束する。
- **schedule auto-disable（既知の運用制約）**: GitHub は public repo で
  ~60 日間 repository activity が無いと scheduled workflow を自動無効化
  し、disabled workflow は **全 trigger 不応答** になる。cron を event
  入口と別ファイルに閉じ込めたのはこの blast radius 分離のため
  （ADR-vJvyZNtrHi2xnycJxbnSPv）。reconcile が Actions 一覧から消えた
  場合は `gh workflow enable ci-issue-tracker-reconcile` で再有効化する。

## 自動 close の条件（多重 guard）

以下を **全て** 満たす時のみ close（回復 SHA・Run URL を comment に記録）:

1. 最新 evidence が成功であること（最新が cancelled/unknown 等の
   operational なら close しない — main は未検証）
2. その成功が記録済み最新失敗より**曖昧でなく新しい**こと
3. `compare(失敗 sha → 成功 sha) ∈ {identical, ahead}`（失敗 revision を
   履歴に含む）
4. `compare(現在の main head → 成功 sha) ∈ {identical, ahead}`（**現在の
   main tip をカバー** — 古い失敗 revision の検証だけでは閉じない）
5. evidence が完全であること — live な `pending` run も識別不能な
   `unknown` 履歴 record も存在しない（live run の完了は未来にあり、
   欠損 record は新しい失敗を隠し得るため）

`behind`（古い revision の成功）・`diverged`（main 履歴書換えで失敗 sha
が残っていない）・順序 ambiguous・branch head 取得不能・compare 不能・
pending/unknown 証跡の残存は**close しない**。close 候補で branch head
が取れない場合は job を失敗させて誤 close を防ぐ。

## 運用ポリシー

- **手動 close**: tracker は closed Issue を再 open しない。人間が
  close した後の失敗は新規 Issue として起票する（新しい障害
  エピソード）。誤 close による連続起票を避けるため、解決前の手動
  close は避けること。
- **`diverged` の手動対応**: main への force push 等で失敗 sha が履歴
  から消えると compare が永久に `diverged` となり自動 close しない。
  失敗が無関係になったことを人間が確認して手動 close する。
- **`stale` / `action_required` の継続**: run が queue で滞留・承認待ち
  の場合は operational のまま。scheduler（Issue #367）の retry で次の
  run が完了するのを待つ。

## 制約・障害時の扱い

- **label `ci-failure` 未作成**: tracker が起動時に冪等確保する
  （`GET` → 404 なら `POST`）。手動作成は不要。
- **権限不足**（`issues: write` 無効化・org 制限等）: API が 403 を
  返し tracker job が失敗する。Job Summary の分類行と report
  artifact で診断可能。権限を復旧するか workflow を無効化するまでは
  Issue 集約は行われない（失敗自体は ci run の赤い結果として残る）。
- **GitHub API 障害・Issue 更新失敗**: 例外が伝播して tracker job が
  失敗する。次の ci 完了で再試行される（action は冪等）。
- **GitHub Actions 自体の停止**: tracker も起動しない（同基盤のため
  自力検知不可 — #367 実装 8 と同じ制約）。復旧後に溜まった
  `workflow_run` 完了が順に処理される。手動確認は
  `gh run list --workflow ci --branch main` で直近の conclusion を
  確認し、必要なら `gh workflow run ci` で再検証する。
- **tracker job 自身の失敗**: Actions UI の `ci-issue-tracker` run が
  赤くなる。自動通知は持たない — daily 運用での Job Summary 確認を
  運用手順とする。
- **同名 workflow の混入（既知の限界・P2 として別途管理）**:
  `workflow_run.workflows: [ci]` は workflow **name** でマッチするため、
  `name: ci` を持つ**別ファイル**の workflow が存在するとその run でも
  tracker が起動する。history 側は `path`/`name` で排除するが、event
  payload 側は workflow file/`workflow_id` を照合していない。現時点で
  同名 workflow は存在せず対応は別タスク — consumer 導入時は `name` の
  衝突がないことを確認すること。

## セキュリティ

- 実行されるコードは常に default branch のもの（`workflow_run` の
  信頼モデル）。失敗 run の head・PR コード・artifact は一切
  checkout / download / 実行しない。
- `GH_TOKEN` は `secrets.GITHUB_TOKEN` のみ・env 経由・ログに出さない。
  Issue 本文・comment・Job Summary に token・内部パス・env を書かない。
- Issue へ書く外部由来値（SHA・Run URL・job 名・OS・conclusion）は
  sanitize / allowlist 済み。marker JSON は `<`/`>` をエスケープし
  comment breakout を防ぐ。

## Consumer 再利用契約（AudioLens / Synvelis）

汎用コードは Ruviform が所有する。Consumer 導入時は scope を差し替える
だけで同一 script が使える:

- `ci-issue-tracker.yml` の `workflow_run.workflows`（対象 workflow 名）
  と `branches`（対象ブランチ）。
- `ci-issue-tracker-reconcile.yml` は追跡対象を参照しない（履歴は
  scope の `--workflow-file`/`--branch` で絞る）ため、変更点は
  concurrency group と artifact 名の repo 語だけ。スケジュール間隔は
  consumer の bot dispatch 頻度に合わせて調整してよい。
- `tracker.mjs` の `--repo`（`GITHUB_REPOSITORY` 既定）・`--branch`・
  `--workflow`・`--label`、および `ISSUE_TITLE` / `ALLOWED_EVENTS` /
  `FAILURE_CONCLUSIONS` の定数。
- marker の `scope` が `repo`/`workflow`/`branch` を持つため、同一 repo
  で複数 workflow を追跡する場合も scope 単位で Issue が分離される
  （`consumer-verify` 追跡は #294 の別ステップ）。
- 依存は GitHub-hosted runner 標準の `gh` CLI + Node のみ。Ruviform の
  Cargo/frontend artifact への依存は一切ない。
- Consumer repo 側の Actions 設定（`GITHUB_TOKEN` の default permission が
  permissive/read-only のどちらか）を確認する — workflow/job の
  `permissions` は明示済みだが、org/repo の default が `issues: write`
  を遮断する構成だと tracker job が 403 で失敗する。

## 検証

- `mise run test-ci-tracker`（`node --test ci/tracker/`）— 全判定経路
  を mock client で検証。実 GitHub API・実 Issue・実 dispatch を起こ
  さない。
- `mise exec -- actionlint .github/workflows/ci-issue-tracker.yml
  .github/workflows/ci-issue-tracker-reconcile.yml`。
