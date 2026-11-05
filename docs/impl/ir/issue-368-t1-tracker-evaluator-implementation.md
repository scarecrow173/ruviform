---
id: "IMPL-034Sr2ILuowBCesihjbzRz"
type: "impl"
status: "completed"
title: "Issue 368 T1 tracker evaluator implementation"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "docs/plans/issue-368-ci-issue-tracker.md"
  changes:
    added:
      - type: "file"
        path: "ci/tracker/tracker.mjs"
      - type: "file"
        path: "ci/tracker/tests/tracker.test.mjs"
    modified:
      - type: "file"
        path: "mise.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-368-t1-tracker-evaluator.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-368-ci-issue-tracker.md"
    - "docs/designs/issue-368-ci-issue-tracker.md"
    - "docs/test-specs/issue-368-ci-issue-tracker.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue 368 T1 tracker evaluator implementation

## Summary

`ci/tracker/tracker.mjs` — Issue #368 の CI failure Issue Tracker 判定器を
stdlib-only Node `.mjs` として実装。`workflow_run` 完了ペイロードを
scope（repo × workflow `ci` × branch `main` × event ∈
{push, workflow_dispatch}）で分類し、GitHub Issue 一覧・job メタ・
compare の注入 client 経由で create/update/close-recovered/
close-duplicate/noop の action plan を組み立てて適用する。
34 件の mock-client contract test を `ci/tracker/tests/` に追加し、
`mise.toml` に `test-ci-tracker` タスクを定義した。

## Context

`docs/tasks/issue-368-t1-tracker-evaluator.md`（design
`docs/designs/issue-368-ci-issue-tracker.md` 由来）に従う。Issue #365
ルール 5「repo × workflow × main の未解決障害 Issue を原則 1 件に集約、
後続最新 SHA の完全成功時のみ自動 close」を、#368 の workflow_run 監視
モデルとしてコード化したもの。

## Implementation

### Changes

- `ci/tracker/tracker.mjs`
  - `classifyRun`: action==completed + head_repository/repo + head_branch +
    event allowlist + run メタ検証 → skip/success/failure/operational。
  - sanitize 層: SHA/URL allowlist・free-text（制御文字・`|`・backtick 除去、
    100 字上限）・conclusion allowlist・ISO 日時・run id。
  - stable marker: Issue body 先頭 HTML comment に
    `{v, scope:{repo,workflow,branch}, state}` JSON。`<`/`>` を `\u003c/
    \u003e` に escape し marker 内に `-->`/`<!--` 文字列を生成不能化。
  - **evidence reconciliation**（post-review hardening）: 非 skip イベント
    ごとに `getRunHistory`（`actions/workflows/{file}/runs?branch=main&
    exclude_pull_requests=true`）で権威一覧を取得し、payload・全 managed
    marker 状態と突き合わせる。一覧が payload より新しい attempt/run を
    示せば採用し、一覧遅延では payload が勝つ。一覧 API 失敗は propagate。
  - `compareRunOrder`: 同 run id → `run_attempt`、異 id → `updated_at`。
    欠損・異 run 同一時刻は `ambiguous`（close 不使用・状態更新は
    `orderForState` で run id に保守解決）。`latest`/`failure` 参照は
    `{runId, attempt, updatedAt}` の atomic tuple。
  - `mergeMarkerStates`: duplicate 整理前に全 marker の最新状態を merge —
    最新失敗を持つ marker の状態を丸ごと採用（field 混ぜなし）、history
    を union。duplicate が最新障害を握っていても失わない。
  - `plan`: 失敗証拠未解決 + open 無し → create（最新失敗をカバーする
    新しい成功が履歴にある古い通知は起票しない）。open あり + 最新
    失敗証拠が記録より新しい → update。成功 → **最新証拠がその成功・
    記録最新失敗より曖昧なく新しい・compare(失敗→成功) ∈ {identical,
    ahead}・compare(現在 main head→成功) ∈ {identical, ahead}** の全てで
    初めて close-recovered — それ以外は success-not-covering 記録か
    stale noop（最新 operational・ambiguous・main tip 未カバー・
    diverged は close しない）。operational → 新しい場合のみ note 更新、
    close/create しない（証跡中の新しい失敗は併せて反映）。
  - `makeGhClient`: `gh api` 薄 wrapper（labels/issues/run history/jobs/
    compare/branches）。`--paginate --jq '.[]'`/`'.workflow_runs[]'` の
    compact JSONL を行単位 parse。
  - `main`: TRACKER_EVENT_PATH/GITHUB_EVENT_PATH から payload 読込み、
    report JSON + Job Summary 出力。API 失敗は propagate して exit 1。
- `ci/tracker/tests/tracker.test.mjs`: 45 tests。scope 分類・sanitize・
  marker parse/create（legacy flat latestRunId marker の正規化含む）・
  初回 create・連続失敗 update・dedup・stale 拒否・rerun（同 id 高
  attempt・旧 run の遅い rerun）success close/failure update・
  identical/ahead close・behind/diverged 拒否・cancelled/unknown
  operational・API 失敗 propagate・label 確保に加え、hardening 系:
  **最新 cancelled 下での stale 成功誤 close 拒否・main tip 未カバー
  成功の close 拒否・branch head 不能時の job 失敗・遅延失敗の誤起票
  拒否（coverage あり/なし両経路）・duplicate 最新失敗 merge 保全・
  ambiguous ordering hold・dropped notification 復元・atomic tuple
  採用・対象外 history run 除外** — 全て mock client、外部副作用なし。
- `mise.toml`: `[tasks.test-ci-tracker] run = "node --test ci/tracker/"`。

### Decisions During Implementation

- **(run.id, run_attempt) 順序**: 実装中の自己レビューで「GitHub rerun は
  同じ run.id で run_attempt が増える」ことに気付き、run.id 単独比較だと
  失敗 run の成功 rerun が stale 扱いで永久に close 不能になる bug を
  発見。新旧判定・latest 記録・history・failure 状態の全経路を複合キー
  比較に修正し、rerun 3 ケースを test に追加。
- **ordering を `updated_at` 中心に強化（post-review hardening）**: 後続
  review で「旧 run の新しい rerun が新しい run より後に完了する」ケースを
  `(id, attempt)` 辞書順が過小評価し得ると指摘された。GitHub REST の
  `run_started_at`（attempt 開始時刻で完了順の証跡にならない）・
  `actor`/`triggering_actor`（順序証跡でない）を調査の上、異 run 間は
  `updated_at`（レコード最終更新 ≈ 完了時刻）を採用。`updated_at` 欠損・
  異 run 同一時刻は `ambiguous` として close/成功判定から外し、状態
  更新のみ run id で保守解決。`latest`/`failure` の参照は `{runId,
  attempt, updatedAt}` の atomic tuple — field 別 Math.max で実在しない
  順序キーを捏造しない。
- **run-history reconciliation を追加（post-review hardening）**: review
  で「concurrency pending 置き換え・通知逆転・eventual consistency で
  完了通知が落ちる/遅れると、到着順に依存する判定が誤る」ことが mock
  replay で実証された（stale 成功の誤 close・遅延失敗の誤起票・
  duplicate 最新失敗の喪失）。非 skip イベントごとに対象 workflow の
  権威 run 一覧を取得して payload/marker と突き合わせ、最新証拠で判断
  する設計に変更。一覧 API 失敗は propagate（証跡欠損で黙って処理しない）。
- **close を多重 guard 化（post-review hardening）**: 「最新 operational
  がある間は古い成功で閉じない」ことと「成功が現在 main tip をカバー
  する」ことを必須条件に追加。close 候補で branch head が取れない場合は
  job を失敗させて誤 close を防ぐ（従来は診断専用だった probe を close
  経路では必須証跡に昇格）。duplicate 整理は全 marker merge 後に行い、
  最新失敗を持つ marker の状態を丸ごと採用する。
- **compare 遅延取得**: 失敗をカバーし得る成功証跡がある時のみ
  `compareCommits` を呼ぶ設計にし、failure イベントごとの不要な API
  呼出しを排除。compare 失敗は close せず job 失敗として伝播。
- **marker JSON の injection 対策を二重化**: state に jobs が含まれる
  設計（body 再描画に必要）のまま、`-->` を含む job 名で HTML comment
  breakout できる injection 経路を発見。(a) job 名を入力境界
  （`run()` の fetch 直後）で sanitize、(b) marker JSON 内の `<`/`>` を
  unicode escape、の二重防御に変更。test に breakout case を追加。
- **fork/PR run の扱い**: `workflow_run` は default branch 側のコンテキスト
  で実行されるため head_repository が fork の run も到達し得る。
  `head_repository.full_name !== scope.repo` で hard skip し、fork 由来
  run は issue を一切触らない。対象外（他 repo・他 branch・許可外 event・
  未 completed）の history run も evidence にしない。

## Related Experiments

### Adopted

なし（exp 未作成 — 設計が pre-determined で探索不要だったため）。

### Rejected

なし。

## Validation

- `mise run test-ci-tracker` → 45/45 pass（hardening 前は 34/34）。
- `mise exec -- actionlint .github/workflows/ci-issue-tracker.yml` → OK。
- `mise exec -- actionlint .github/workflows/ci.yml` → OK（無変更確認）。
- `mise exec -- prek run --all-files` → 全 hook pass。
- `git diff --check` → clean。
- tracker.mjs skip 経路の end-to-end dry run（fixture event → exit 0、
  `classified: skip (branch:feature-x)`、API 呼出しなし）。
- `gh api repos/.../labels --jq '.[]'` の compact JSONL 出力を実測確認
  （read-only、副作用なし）。
- 局所 eval での edge case 確認: 一覧遅延（payload が history 未収載）・
  lone success・証跡欠損時の保守判定が mock client で再現。
- graph router（implementation node, focus=本 spec,
  signal=implementation-verified）→ `followup-triage` へ edge、
  taskGraph 両タスク done・issues なし。

## Risks

- `workflow_run` 経路そのものは GitHub Actions 実行環境でのみ検証可能
  （実 workflow 実行は本タスクのスコープ外・禁止事項）。offline contract
  test と actionlint で担保。
- Issue body marker は v1 スキーマ。将来 scope/state 拡張時は marker
  version bump と後方互換 parse が必要（design に記録済み）。

## Follow-ups

- T2 workflow 実装（`docs/impl/ir/issue-368-t2-tracker-workflow-implementation.md`）。
- AudioLens #159 / Synvelis #167 への再利用適用（README に adoption
  contract 記載）。
- Issue #367 daily scheduler が `workflow_dispatch` で `ci` を起動する
  経路が本 tracker の allowed event に含まれるため、#367 着地後の統合
  確認。
