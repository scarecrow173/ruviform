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

### Rule: conclusion 分類

- `success` → `success`。
- `failure`・`timed_out`・`startup_failure` → `failure`（検証未完了終了。原因自動分類はしない）。
- `cancelled`・`skipped`・`neutral`・`stale`・`action_required`・null・未知値 → `operational`（成功でも失敗起票でもない）。

## Feature: Issue plan 判定（`plan`）

### Rule: failure — 未解決障害を 1 Issue に集約

- Example: managed open Issue 無し + failure → `create`（初回起票・label `ci-failure`・marker 付き）。
- Example: canonical open あり + run が `(id, attempt)` 順で state.failure より新しい → `update`（失敗状態・history・job 表を新 run で更新。同一 id の rerun も新しい証拠として扱う）。
- Example: canonical open あり + run が新しくない → `noop`（完了順逆転の stale — 古い完了が新しい状態を上書きしない。同 id 同 attempt の再配信も stale で冪等）。
- Example: managed open が 2 件 → 最小番号 canonical に update/create 相当、残りを duplicate close（comment + `state: closed`）。
- Example: label はあるが marker 無し/parse 不可の Issue（foreign）→ 一切触れず、managed 無しとして `create`。

### Rule: success — coverage を満たす新しい完全成功だけが close する

- Example: open あり + run が failure より新しい + `compare(failure.sha→run.sha)=identical` → `close`（回復 SHA・Run URL を comment）。
- Example: 同条件で `compare=ahead` → `close`（失敗 sha を履歴に含む新しい成功）。
- Example: `compare=behind` → `noop`（古い revision の成功で新しい失敗を閉じない）。
- Example: `compare=diverged` → `noop`（履歴書換え等・手動確認を Job Summary に記録）。
- Example: run が failure より新しくない（古い id・同 id の低 attempt）→ `noop`（古い run の成功で閉じない）。
- Example: rerun 成功（同 id・高 attempt・同 sha）→ `identical` coverage で `close`（再検証は新しい証拠）。
- Example: managed open 無し → `noop`。

### Rule: operational — 成功でも失敗でもない観測

- Example: canonical open あり + run が `(id, attempt)` 順で最新より新しい → `update`（operational note「retry 待ち/運用状態」のみ。失敗状態は変えない）。
- Example: open 無し → `noop`（cancelled で障害 Issue は作らない）。
- Example: run が新しくない → `noop`。

### Rule: API 失敗は成功にしない

- label 確保・Issue 一覧・jobs・compare・create/update/close の失敗は例外として伝播し、`main` は非 0 exit（tracker job 失敗・診断可能）。
- `compare` は close 判定が必要な時だけ呼ぶ（failure/operational/open 無しでは呼ばない）。

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
