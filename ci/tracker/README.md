# ci/tracker — CI failure Issue Tracker（Issue #368）

`.github/workflows/ci-issue-tracker.yml` が `ci`（Full CI）workflow の
`workflow_run` 完了を監視し、**この repo の `main` に対する有効な
Full CI 実行**だけを対象に、`main × ci` の未解決障害 Issue を原則
1 件に集約する（Issue #365 ルール 5）。

## 構成

- `ci-issue-tracker.yml` — `workflow_run: {workflows: [ci], types:
  [completed], branches: [main]}` でのみ起動。job `track` のみ
  `contents: read` / `actions: read` / `issues: write`。
  `concurrency` で repo 単位に直列化（`cancel-in-progress: false`）。
- `ci/tracker/tracker.mjs` — stdlib-only。`classifyRun`（有効 run +
  conclusion 分類）→ `plan`（action list）→ `apply`（`gh api` client
  注入）。GitHub API は全て client 経由で、テストは mock client。
- `ci/tracker/tests/tracker.test.mjs` — `node --test` contract tests。
  実 GitHub API・実 Issue・実 dispatch を一切起こさない。

## 有効 run の定義

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
  「superseded by #N」comment + close で 1 件に集約する。
- 失敗更新は run が `(run.id, run_attempt)` 辞書順で記録済み失敗 run
  より新しい場合のみ — 完了順逆転で古い run が新しい状態を上書きし
  ない。同一 run の rerun は attempt で新旧を区別する（失敗 run の
  rerun 成功は正しく回復として扱える）。

## 自動 close の条件（2 重 guard）

両方を満たす時のみ close（回復 SHA・Run URL を comment に記録）:

1. 成功 run が `(run.id, run_attempt)` 順で記録失敗 run より新しい
   （より新しい証拠であること。rerun 成功は同 id・高 attempt で該当）
2. `compare(失敗 sha → 成功 sha) ∈ {identical, ahead}`（失敗 revision を
   履歴に含む成功であること）

`behind`（古い revision の成功）・`diverged`（main 履歴書換えで失敗 sha
が残っていない）・compare 不能は **close しない**。

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
- `tracker.mjs` の `--repo`（`GITHUB_REPOSITORY` 既定）・`--branch`・
  `--workflow`・`--label`、および `ISSUE_TITLE` / `ALLOWED_EVENTS` /
  `FAILURE_CONCLUSIONS` の定数。
- marker の `scope` が `repo`/`workflow`/`branch` を持つため、同一 repo
  で複数 workflow を追跡する場合も scope 単位で Issue が分離される
  （`consumer-verify` 追跡は #294 の別ステップ）。
- 依存は GitHub-hosted runner 標準の `gh` CLI + Node のみ。Ruviform の
  Cargo/frontend artifact への依存は一切ない。

## 検証

- `mise run test-ci-tracker`（`node --test ci/tracker/`）— 全判定経路
  を mock client で検証。実 GitHub API・実 Issue・実 dispatch を起こ
  さない。
- `mise exec -- actionlint .github/workflows/ci-issue-tracker.yml`。
