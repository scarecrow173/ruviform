---
id: "ADR-034SraR8CrkZ0SSa7tQEwh"
type: "adr"
status: "accepted"
title: "Issue #368 CI failure Issue Tracker — workflow_run 監視・Issue 集約・(run id, attempt) 順序と compare 2 重 guard"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/368"
    - "https://github.com/scarecrow173/ruviform/issues/365"
    - "https://github.com/scarecrow173/ruviform/issues/366"
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
  implements: []
  implemented-by:
    - "docs/designs/issue-368-ci-issue-tracker.md"
  depends-on:
    - "docs/specs/issue-368-ci-issue-tracker.md"
    - "docs/specs/issue-366-full-ci-stage1.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/issue-328-ci-concurrency-grouping.md"
    - "docs/designs/issue-366-full-ci-stage1.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "docs/designs/issue-368-ci-issue-tracker.md"
  verifies: []
  verified-by: []
  references:
    - "https://docs.github.com/en/actions/reference/events-that-trigger-workflows#workflow_run"
    - "https://docs.github.com/en/rest/issues/issues#list-repository-issues"
    - "https://docs.github.com/en/rest/commits/commits#compare-two-commits"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# Issue #368 CI failure Issue Tracker — workflow_run 監視・Issue 集約・(run id, attempt) 順序と compare 2 重 guard

## Context and Problem Statement

Issue #365 ルール 5 は「repo × workflow × main の未解決障害を Issue 1 件に
集約し、後続最新 SHA の完全成功のみで自動 close する」ことを求める。
Issue #366 で Full CI（無条件全 lane + fail-closed `ci-summary`）が main
に入ったが、main の `ci` 失敗は Actions UI の赤い結果でしか残らず、
追跡・共有・復旧確認のための永続レコードがない。問題は「完了イベントを
どう検知し」「どう同一障害と同定し」「どう古い/壊れた成功で誤 close し
ないか」の 3 点に分離できる。`workflow_run` 完了は ordering が保証され
ない（古い run が新しい run より後に completed になり得る）上、GitHub
rerun は同じ run.id で run_attempt を増やすため「run.id が大きい =
新しい」だけでは不十分。main SHA は push・rebase・履歴書換えで動くため、
「新しい run の成功」は「失敗 revision を履歴に含む成功」とは限らない。

## Decision Drivers

- fail-closed: 失敗・cancel・unknown・API 失敗・証拠不足は一切成功扱い
  しない（#365/#366 の契約を弱めない）。
- 未信頼コード不実行: 特権権限（issues:write）を持つ job が失敗 run の
  PR コード・artifact を checkout/download/実行してはならない。
- 冪等・重複防止: workflow_run 再配送・同時完了・手動 dispatch 多発で
  Issue が二重作成されてはならない。
- 完了順逆転に強い ordering と「失敗 revision をカバーする成功」のみを
  回復とみなす close guard が必須。
- 追加依存なし（DB・外部 bot・新規 framework 不使用）。GitHub 標準
  REST API と Node stdlib のみで実現する。
- CI 移行中の既存 PR/push trigger・required context を壊さない。

## Considered Options

- **Option A（採用）**: 独立 `ci-issue-tracker.yml` を `workflow_run
  {workflows:[ci], types:[completed], branches:[main]}` で起動し、
  default branch の tracker コードが REST メタデータのみで Issue を
  管理する。権限は track job のみ actions:read/issues:write。
- **Option B**: `ci.yml` 内に失敗時 Issue 管理 job を追加（`if: failure()`
  + `always()`）。**却下** — `ci` job 群の permissions は contents:read
  であり issues:write を ci.yml に混入させると PR コンテキストの権限
  設計が壊れる。また失敗 run 自身の権限で Issue を書く経路は fork PR
  由来の権限昇格リスクを抱え、ci-summary とは責務が異なる。
- **Option C**: schedule ポーリング（cron で `gh run list` を走査）。
  **却下** — 検知遅延・schedule 自体の停止盲点・「どの run が未処理か」
  の state を Issue 側に持たせる必要があり複雑。workflow_run push 型は
  イベント駆動で遅延なく、queue に溜まっても直列処理できる。
- **Option D**: Issue 同定を search API + label のみ。**却下** — search
  index は eventually consistent で「直前に作った Issue が検索に出ない」
  → 二重作成を防げない。label だけでは人間が付けた同名 label の無関係
  Issue と tracker 管理 Issue を区別できない。採用案は
  `issues?labels=…&state=open` の**列挙 endpoint** + body 先頭の安定
  marker JSON（scope/repo/workflow/branch + version）の 2 重同定。
- **Option E**: run 新旧を `run.id` 単独、`(run.id, run_attempt)` 辞書順、
  または `run_started_at` で判定。**却下** — run.id 単独では同一 run の
  rerun（attempt 増加）が「同じ run」と見える。`(id, attempt)` 辞書順は
  **旧 run の新しい attempt が新しい run より後に完了する**ケースを過小
  評価する（完了順で言えば後者が新しい証拠）。`run_started_at` は
  attempt の開始時刻で完了順の証跡にならず、`actor`/`triggering_actor`
  は「誰が」であって順序証跡ではない。採用案は **同 id → attempt、
  異 id → `updated_at`**（レコード最終更新 ≈ 完了時刻）。`updated_at`
  欠損・異 run 同一時刻は `ambiguous` とし、close/成功判定には使わない
  （状態更新のみ run id で保守解決）。`latest`/`failure` 参照は
  `{runId, attempt, updatedAt}` の atomic tuple — field 別 Math.max で
  実在しない順序キーを捏造しない。
- **Option F**: 「新しい成功 = close」。**却下** — main が force-push/
  rebase されると新しい成功が失敗 revision を履歴に含まない場合が
  ある。さらに最新実行が cancelled/unknown の間は main 未検証であり、
  古い成功は回復ではない。採用案は **多重 guard**: 最新証拠がその成功・
  記録最新失敗より曖昧なく新しい・`compare(失敗 sha → 成功 sha) ∈
  {identical, ahead}`・`compare(現在 main head → 成功 sha) ∈
  {identical, ahead}`（main tip カバー）。
- **Option G（新規・採用）**: イベント payload を真実とせず、非 skip
  イベントごとに `actions/workflows/{file}/runs?branch=main` の権威一覧を
  取得し payload/marker と突き合わせる **reconciliation**。**採用** —
  tracker workflow の concurrency pending 置き換え・通知逆転・eventual
  consistency で完了通知が落ちても、一覧を事実として自己修復できる
  （遅延した古い失敗の誤起票・新しい失敗の見落としを両方防ぐ）。
  concurrency 直列化だけでは全通知の順序処理を保証できない。

## Decision Outcome

Chosen option: **Option A**（+ D/E/F/G の採用側を構成要素として統合）。

- `.github/workflows/ci-issue-tracker.yml`: `workflow_run` のみ、
  `concurrency: ci-issue-tracker-<repo>` + `cancel-in-progress: false`
  で repo 単位直列化（pending 置き換えの限界をコメントに明記）。
  track job のみ contents:read/actions:read/issues:write。
- `ci/tracker/tracker.mjs`: classify（scope 一致 + event allowlist
  {push,workflow_dispatch}）→ label 冪等確保 → open Issue 列挙 →
  marker+scope で managed 同定・**全 marker 状態 merge**（canonical =
  最小番号、重複は merge 後に superseded close）→ **run-history
  reconciliation**（権威一覧・payload・marker の突き合わせ）→ plan
  （create/update/merge-state/close-duplicate/close-recovered/noop）→
  apply。
- ordering: 同 id → `run_attempt`、異 id → `updated_at`。欠損・同一
  時刻は `ambiguous`（close 不使用・状態更新は run id で保守解決）。
  run 参照は `{runId, attempt, updatedAt}` atomic tuple。
- close: 最新証拠が成功・記録最新失敗より曖昧なく新しい・
  `compare(失敗→成功)` と `compare(現在 main head→成功)` がともに
  {identical, ahead} の時のみ。close 候補で branch head が取れない
  場合は job 失敗。
- cancelled/skipped/unknown → operational（Issue 新規作成も close も
  しない、canonical があれば note 更新のみ）。API 失敗は propagate し
  tracker job を失敗させる。

### Consequences

- Good, because ci.yml と required checks・PR/push trigger に一切触れず
  監視層を追加できる（移行中の deadlock 防止契約を維持）。権限は 1 job
  に限定され、default branch コードのみ実行。Issue 列挙 + marker で
  index 遅延・人為 label 付けを両方排除しつつ dedup できる。
  workflow_run 再配送・完了逆転・rerun を ordering 規則で全て冪等吸収。
- Bad, because `workflow_run` は default branch merge 後にのみ起動する
  （本ブランチ上では未稼働 — 初回実測は merge 後）。GitHub Actions 自体
  が停止している間は tracker も起動せず検知できない（README に手動
  運用手順を記録）。
- Neutral: Issue は「scope あたり 1 open episode」モデル。人間の手動
  close は尊重され、次の失敗は新 episode を開く（reopen しない）。

### Confirmation

`ci/tracker/tests/tracker.test.mjs` の 45 件の mock-client contract
test が全判定経路を網羅（scope 分類・dedup・stale 拒否・rerun・
compare/main-tip guard・operational・API 失敗 propagate に加え、
reconciliation 系: stale 成功の誤 close 拒否・遅延失敗の誤起票拒否・
duplicate 最新状態 merge・旧 run rerun の updated_at 順序・ambiguous
ordering の hold・dropped notification 復元・atomic tuple 採用）。
`mise run test-ci-tracker` で CI/ローカル共通に実行可能。実 GitHub
API・実 Issue・実 dispatch はテストで発生させない。

## Implementation Plan

- Affected paths: `.github/workflows/ci-issue-tracker.yml`（新規）、
  `ci/tracker/{tracker.mjs,README.md,tests/tracker.test.mjs}`（新規）、
  `mise.toml`（`test-ci-tracker` 追加）、
  `docs/development-environment.md`（追記）。
- Patterns to follow: `ci/summary/` の stdlib `.mjs` + `node --test` +
  mise タスク構成。job 先頭の self-test → API 実行の順序は
  `ci-summary` の `test-ci-summary` パターンに倣う。
- Patterns to avoid: PR/fork head の checkout・artifact download・
  `pull_request`/`push`/`schedule`/`workflow_dispatch` trigger の追加・
  search API による Issue 同定・`(run.id, run_attempt)` 辞書順のみの
  異 run 比較・compare/main tip guard なしの close・field 別 max に
  よる順序キー合成・最新 operational の存在下での古い成功 close・
  marker merge 前の duplicate close。
- Dependencies or configuration: GitHub-hosted runner 標準の `gh` CLI、
  mise pin の Node。新規パッケージ依存なし。権限は workflow/job
  YAML のみ。

## Verification

- [x] `mise run test-ci-tracker` → 45/45 pass（hardening 前は 34/34）。
- [x] `mise exec -- actionlint .github/workflows/ci-issue-tracker.yml` → OK。
- [x] `mise exec -- actionlint .github/workflows/ci.yml` → OK（ci.yml 無変更）。
- [x] `mise exec -- prek run --all-files` → 全 hook pass。
- [ ] `workflow_run` 実発火による end-to-end（merge 後・本タスク範囲外）。

## Pros and Cons of the Options

### Option A（採用）— 独立 workflow_run 監視 workflow

- Good, because 権限分離が job 単位で明確・失敗 run コード不実行が構造で
  保証・ci.yml 無変更・イベント駆動で遅延最小・concurrency 直列化で
  同時完了も安全。
- Bad, because default branch に無い間は起動しない・GitHub Actions 停止
  時は検知不能・workflow が 1 ファイル増える。

### Option B — ci.yml 内 Issue 管理 job

- Good, because 実装場所が 1 workflow にまとまる。
- Bad, because issues:write を ci 実行と同居させ権限境界が曖昧・fork PR
  での権限昇格リスク・「失敗した run が自分の失敗を Issue 化する」責務
  の混在・ci-summary の fail-closed 設計と権限モデルが衝突。

### Option C — schedule ポーリング

- Good, because workflow_run 信頼モデルに依存しない。
- Bad, because 検知遅延・schedule 停止盲点・未処理 run の state 管理が
  必要・cron 最小粒度の制約。

### Option D — search API + label のみ同定

- Good, because 実装が単純。
- Bad, because search index の eventual consistency で dedup 不可・label
  だけでは人為 Issue と tracker Issue を区別不能。

### Option E — run.id 単独 / 時刻 ordering

- Good, because コードが最も単純。
- Bad, because rerun（同 id・高 attempt）を扱えず失敗 run の成功 rerun
  が永久に close 不能・完了時刻は逆転そのものが想定される。

### Option F — ordering のみで close

- Good, because compare API 呼出し不要。
- Bad, because main 履歴書換え時に「失敗 revision を含まない新しい成功」
  で誤 close し得る — fail-closed 契約違反。

## More Information

- Issue #368（本トラッカー要求）、#365（横断 CI 移行計画・ルール 5）、
  #366（Full CI Stage 1 = 監視対象 run を生む前提）、#367（並行の
  Daily scheduler — `workflow_dispatch` 起動 run も本 tracker の
  allowed event に含まれる）。
- AudioLens #159 / Synvelis #167 への展開を想定した汎用 scope 設計
  （marker に repo/workflow/branch を含める）— 詳細は
  `ci/tracker/README.md` の Consumer 再利用契約。
