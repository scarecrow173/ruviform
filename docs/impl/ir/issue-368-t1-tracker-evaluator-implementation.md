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
  - `isNewerRun`: `(run.id, run_attempt)` 辞書順の新旧判定。
  - `plan`: 失敗 → canonical が無ければ create、新しければ update（first
    detection 保持）、古ければ stale noop。成功 → 失敗 run より新しく
    compare ∈ {identical,ahead} のみ close-recovered、それ以外は
    success-not-covering を記録するか noop。operational → 新しい場合のみ
    note 更新、close/create しない。重複 managed Issue は最小番号を
    canonical に集約（superseded comment + close）。
  - `makeGhClient`: `gh api` 薄 wrapper（labels/issues/jobs/compare/
    branches）。`--paginate --jq '.[]'` の compact JSONL を行単位 parse。
  - `main`: TRACKER_EVENT_PATH/GITHUB_EVENT_PATH から payload 読込み、
    report JSON + Job Summary 出力。API 失敗は propagate して exit 1。
- `ci/tracker/tests/tracker.test.mjs`: 34 tests。scope 分類・sanitize・
  marker parse/create・初回 create・連続失敗 update・同時完了 dedup・
  stale 拒否・rerun（同 id 高 attempt）success close/failure update・
  同 id 同 attempt 冪等・identical/ahead close・behind/diverged 拒否・
  cancelled/unknown operational・API 失敗 propagate・label 確保・
  branch-head probe — 全て mock client、外部副作用なし。
- `mise.toml`: `[tasks.test-ci-tracker] run = "node --test ci/tracker/"`。

### Decisions During Implementation

- **(run.id, run_attempt) 順序**: 実装中の自己レビューで「GitHub rerun は
  同じ run.id で run_attempt が増える」ことに気付き、run.id 単独比較だと
  失敗 run の成功 rerun が stale 扱いで永久に close 不能になる bug を
  発見。新旧判定・latest 記録・history・failure 状態の全経路を複合キー
  比較に修正し、rerun 3 ケースを test に追加。
- **compare 遅延取得**: success かつ canonical 存在かつ失敗 run より新しい
  場合のみ `compareCommits` を呼ぶ設計にし、failure イベントごとの不要な
  API 呼出しを排除。compare 失敗は close せず job 失敗として伝播。
- **marker JSON の injection 対策を二重化**: state に jobs が含まれる
  設計（body 再描画に必要）のまま、`-->` を含む job 名で HTML comment
  breakout できる injection 経路を発見。(a) job 名を入力境界
  （`run()` の fetch 直後）で sanitize、(b) marker JSON 内の `<`/`>` を
  unicode escape、の二重防御に変更。test に breakout case を追加。
- **fork/PR run の扱い**: `workflow_run` は default branch 側のコンテキスト
  で実行されるため head_repository が fork の run も到達し得る。
  `head_repository.full_name !== scope.repo` で hard skip し、fork 由来
  run は issue を一切触らない。
- **branch head probe は診断専用**: `getBranchHead` 失敗を握りつぶして
  `mainHead: null` とする（compare guard が本質であり、診断情報の欠如で
  tracking を止めない）。他の API 失敗は全て propagate。

## Related Experiments

### Adopted

なし（exp 未作成 — 設計が pre-determined で探索不要だったため）。

### Rejected

なし。

## Validation

- `mise run test-ci-tracker` → 34/34 pass。
- `mise exec -- actionlint .github/workflows/ci-issue-tracker.yml` → OK。
- `mise exec -- actionlint .github/workflows/ci.yml` → OK（無変更確認）。
- `mise exec -- prek run --all-files` → 全 hook pass。
- `git diff --check` → clean。
- tracker.mjs skip 経路の end-to-end dry run（fixture event → exit 0、
  `classified: skip (branch:feature-x)`、API 呼出しなし）。
- `gh api repos/.../labels --jq '.[]'` の compact JSONL 出力を実測確認
  （read-only、副作用なし）。
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
