---
id: "TSPEC-3iZ6nd0UGy5yuVGAnGQI1y"
type: "test-spec"
status: "approved"
title: "Issue #368 CI failure Issue Tracker test specification — 有効 run 判定・plan 行列・close guard・sanitize"
created: "2026-11-06"
updated: "2026-11-06"
owners: []
relations:
  source:
    - "docs/specs/issue-368-ci-issue-tracker.md"
    - "docs/designs/issue-368-ci-issue-tracker.md"
  changes:
    added:
      - type: "file"
        path: ".github/workflows/ci-issue-tracker.yml"
      - type: "directory"
        path: "ci/tracker"
    modified:
      - type: "file"
        path: "mise.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-368-ci-issue-tracker.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-368-ci-issue-tracker.md"
    - "docs/plans/issue-368-ci-issue-tracker.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-368-ci-issue-tracker.md"
    - "docs/designs/issue-368-ci-issue-tracker.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata: {}
---

# Issue #368 test specification — tracker 有効 run 判定・plan 行列・close guard・sanitize

## Feature: 有効 run フィルタ（`classifyRun`）

### Rule: この repo の main に対する有効な `ci` 完了 run のみ処理する

- Example: `action=completed`・`head_repository=scarecrow173/ruviform`・`head_branch=main`・`event=push` → 分類対象。
- Example: `event=workflow_dispatch`（daily scheduler 起動・手動起動）→ 分類対象。
- Example: `head_branch=feature-x` → `skip`（PR/補助 run）。
- Example: `head_repository=somefork/ruviform` → `skip`（fork run）。
- Example: `event=pull_request`・`event=merge_group`・未知 event → `skip`（allowlist `push`/`workflow_dispatch` 以外）。
- Example: `action=requested`/`in_progress` → `skip`（completed 以外）。
- Example: `head_sha` が 40-hex でない・`id` が整数でない → `skip`（不完全な payload を有効 run にしない）。
- Example: `action=completed` だが `status` が非 completed・null・欠落 → `skip`（status=null を completed と仮定しない）。`run_attempt` 欠落も `skip`。

### Rule: conclusion 分類

- `success` → `success`。
- `failure`・`timed_out`・`startup_failure` → `failure`（検証未完了終了。原因自動分類はしない）。
- `cancelled`・`skipped`・`neutral`・`stale`・`action_required`・null・未知値 → `operational`（成功でも失敗起票でもない）。

## Feature: Issue plan 判定（`plan`）

## Feature: run-history reconciliation（`run`）

### Rule: イベントはトリガであり、判断は権威ある run 一覧に対して行う

- Example: 非 skip イベントで `GET /actions/workflows/{file}/runs?branch=main&exclude_pull_requests=true` を取得し、payload・marker 状態と突き合わせる。
- Example: history に payload 未収載の新しい失敗 run がある → その失敗で状態更新（dropped notification の自己修復）。
- Example: history が payload の attempt より新しい attempt を示す → 大きい attempt を採用（逆方向も同様 — 一覧遅延では payload が勝つ）。
- Example: 対象外（他 repo・fork head・他 branch・許可外 event・他 workflow `path`/`name`）の history run は evidence にしない — 併存しても有効な close を妨げない。
- Example: 対象内・非 completed（`queued`/`in_progress`/`requested`/`waiting`/`pending`/未知 status）の history run は `pending` evidence — 完了が未来にあるためどんな順序でも close を止める。
- Example: 対象性・順序が証明不能な record（必須 field 欠落・`{}`・非 object）は `unknown` evidence — close も stale-failure 抑止（resolved）も止め、success/failure にならない。
- Example: history API 失敗・非配列レスポンス → 例外伝播・job 失敗（証跡欠損で黙って処理しない）。

### Rule: ordering は `updated_at` を優先し、曖昧は成功にしない

- Example: 同 run id → `run_attempt` 比較（rerun は新しい attempt）。
- Example: marker の記録済み `failure` attempt は `(runId, attempt)` 単位の独立証拠 — runs 一覧が同 id の新しい attempt（成功・operational・in_progress）だけを返しても消えない（per-id merge/suppress で失わせない）。同一 attempt の実 record がある時のみ実 record が勝つ。
- Example: 同 id 失敗 a1 → 同 id 成功 a2（同 sha）→ `identical` coverage で close。同 id 成功 a2 が別 sha → compare カバー時のみ close、`diverged` は close しない。同 id の取消/in_progress rerun は close せず記録 attempt を保持。
- Example: 異なる run id → `updated_at` 比較（旧 run の遅い rerun を過小評価しない）。
- Example: `updated_at` 欠損・異 run 同一時刻 → `ambiguous` — close には使わない（issue open 維持）。状態更新のみ run id で保守解決。
- Example: `latest`/`failure` の `{runId, attempt, updatedAt}` は tuple をまるごと採用 — run 間で field 別 Math.max しない。

### Rule: 失敗 — 未解決の最新失敗証拠を 1 Issue に集約

- Example: managed open Issue 無し + 未解決の最新失敗証拠あり → `create`（初回起票・label `ci-failure`・marker 付き）。
- Example: 最新失敗証拠が記録失敗より新しい → `update`（失敗状態・history・job 表を更新。rerun ・duplicate marker・履歴由来の新失敗も同様）。
- Example: 証拠が記録失敗より新しくない → `noop`（完了順逆転の stale — 古い完了が新しい状態を上書きしない）。
- Example: managed open が 2 件 → 全 marker の最新状態を最小番号 canonical に merge してから残りを duplicate close — duplicate が持つ最新失敗を失わない。
- Example: label はあるが marker 無し/parse 不可の Issue（foreign）→ 一切触れず、managed 無しとして `create`。

### Rule: 成功 — 全 guard を満たす最新成功だけが close する

- Example: open あり + 最新証拠が成功 + 最新失敗より曖昧なく新しい + `compare(失敗→成功)∈{identical,ahead}` + `compare(現在main head→成功)∈{identical,ahead}` → `close`（回復 SHA・Run URL を comment）。
- Example: 最新証拠が operational（cancelled/unknown 等）→ close しない（古い成功で閉じない — main 未検証）。
- Example: `compare=behind` → close しない（古い revision の成功で新しい失敗を閉じない）。
- Example: `compare=diverged` → close しない（履歴書換え等・手動確認を記録）。
- Example: 成功が現在 main tip をカバーしない（`compare(main→成功)=behind`）→ close しない。
- Example: 順序 ambiguous → close しない（記録して open 維持）。
- Example: 履歴に live `pending` run（新しい in_progress/queued）・識別不能 `unknown` record が残る → close しない（証跡不完全）。失敗 run 自身の in_progress rerun でも同様。
- Example: 失敗 sha→成功 sha の compare は通るが main tip をカバーしない → 更新して open 維持。
- Example: managed open 無し + 失敗証拠をカバーする新しい成功 → `noop`（遅延した古い失敗通知は起票しない — 到着順が事実にならない）。

### Rule: operational — 成功でも失敗でもない観測

- Example: canonical open あり + 最新 run が operational 且つ記録最新より新しい → `update`（operational note のみ。失敗状態は変えない）。
- Example: open 無し + 未解決失敗証拠無し → `noop`（cancelled で障害 Issue は作らない）。
- Example: open 無し + 未解決失敗証拠あり（最新 run は operational）→ `create`（最新の検証は pending — 最新失敗で起票）。
- Example: run が新しくない → `noop`。

### Rule: API 失敗は成功にしない

- label 確保・Issue 一覧・run history・jobs・compare・branch head・create/update/close の失敗は例外として伝播し、`main` は非 0 exit（tracker job 失敗・診断可能）。
- `compare` は失敗をカバーし得る成功証跡がある時だけ呼ぶ。close 候補でのみ main head compare を呼び、branch head 取得不能なら close せず job を失敗させる。

## Feature: marker と sanitize

### Rule: scope marker 同定

- body 先頭 `<!-- ci-issue-tracker:{json} -->` が `v`・`scope{repo,workflow:"ci",branch:"main"}` 一致で managed。`renderBody`→`parseMarker` で state が往復する。
- label のみ・marker 無し・marker JSON 破損・scope 不一致 → foreign（変更しない）。

### Rule: 出力 sanitize

- sha: `^[0-9a-f]{40}$` 以外は `?`（marker には書かない）。
- URL: `https://github.com/` prefix のみ採用、他は落とす。
- job 名/branch/OS: 制御文字・改行・`|`・バッククォート除去 + 100 chars 上限。
- conclusion: allowlist 外は `unknown`。
- run id: 非整数は受理しない。

## Feature: ci-issue-tracker.yml wiring

### Rule: trigger・権限・排他

- `on.workflow_run.workflows: [ci]`・`types: [completed]`・`branches: [main]`。他の trigger を持たない。
- workflow 直下 `contents: read`・job `track` のみ `contents:read`/`actions:read`/`issues:write`。
- `concurrency` repo 単位・`cancel-in-progress: false`。
- `ci.yml`/`consumer-verify.yml` 無変更。

### 検証方法

- `mise run test-ci-tracker`（`node --test ci/tracker/`）が全 fixture/mock test pass。外部通信なし。
- `mise exec -- actionlint .github/workflows/ci-issue-tracker.yml` pass。
- `mise exec -- prek run --all-files` pass。
- `git diff` で `ci.yml`/`consumer-verify.yml` の差分 0 を机上確認。

## 関連文書

- 要求: [docs/specs/issue-368-ci-issue-tracker.md](../specs/issue-368-ci-issue-tracker.md)
- 設計: [docs/designs/issue-368-ci-issue-tracker.md](../designs/issue-368-ci-issue-tracker.md)
- 実装計画: [docs/plans/issue-368-ci-issue-tracker.md](../plans/issue-368-ci-issue-tracker.md)
