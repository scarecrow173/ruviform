---
id: "DESIGN-MxFLFDKeJa955vgMTzSni8"
type: "design"
status: "approved"
title: "Issue #294 consumer-verify → ci-issue-tracker 配線設計 — 既存 scope 機構の再利用・scope 別 job・reconcile 全 scope 化"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "docs/specs/issue-294-consumer-verify-tracker-wiring.md"
  changes:
    added:
      - type: "file"
        path: "ci/tracker/tests/issue-tracker-workflows-contract.test.mjs"
      - type: "file"
        path: "docs/specs/issue-294-consumer-verify-tracker-wiring.md"
    modified:
      - type: "file"
        path: "ci/tracker/tracker.mjs"
      - type: "file"
        path: "ci/tracker/tests/tracker.test.mjs"
      - type: "file"
        path: "ci/tracker/README.md"
      - type: "workflow"
        path: ".github/workflows/ci-issue-tracker.yml"
      - type: "workflow"
        path: ".github/workflows/ci-issue-tracker-reconcile.yml"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-294-consumer-verify-tracker-wiring.md"
  implemented-by:
    - "docs/plans/issue-294-consumer-verify-tracker-wiring.md"
  depends-on:
    - "docs/specs/issue-294-consumer-verify-tracker-wiring.md"
    - "docs/specs/issue-294-cross-consumer-candidate-gate.md"
    - "docs/specs/issue-368-ci-issue-tracker.md"
    - "docs/specs/issue-368-tracker-reconcile.md"
    - "docs/specs/issue-294-daily-tuple-trigger.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-368-ci-issue-tracker.md"
    - "docs/designs/issue-368-tracker-reconcile.md"
    - "docs/designs/issue-294-daily-tuple-trigger.md"
    - "docs/designs/issue-294-cross-consumer-candidate-gate.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-294-consumer-verify-tracker-wiring.md"
  derived-by:
    - "docs/plans/issue-294-consumer-verify-tracker-wiring.md"
    - "docs/test-specs/issue-294-consumer-verify-tracker-wiring.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-294-consumer-verify-tracker-wiring.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/368"
    - "https://github.com/scarecrow173/ruviform/issues/367"
  defers: []
  deferred-by: []
---

# Issue #294 consumer-verify → ci-issue-tracker 配線設計 — 既存 scope 機構の再利用・scope 別 job・reconcile 全 scope 化

## Context

`ci/tracker/tracker.mjs`（Issue #368）は「repo × workflow × branch」の
scope を第一級の概念として持つ: marker JSON の `scope`、`scopeMatches`、
`--workflow`/`--workflow-file`/`--branch`/`--label` CLI、履歴 record の
`path`/`name` フィルタ（`classifyHistoryRecord`）。しかし event 入口は
`workflows: [ci]` のみを監視し、reconcile 入口は `ci` scope のみを呼ぶ。
`consumer-verify`（Issue #294・dispatch-only 情報 lane）は daily-ci の
tuple scheduler が `GITHUB_TOKEN` で dispatch するため、完了時に
`workflow_run` event が発火しない — **event 入口だけでは原理的に拾え
ない** scope であり、reconcile 側の拡張が必須。

確認済みの制約:

- tracker が消費するのは `workflow_run.conclusion` のみ。consumer
  evidence の `pass/fail/not_run/unsupported/skipped` は
  `consumer-verify.yml` の summary job が workflow conclusion に
  fail-closed で写像する（lane 失敗・cancelled・preflight 失敗・
  全 not_run/unsupported で run 失敗）。tracker 側は conclusion の
  既存行列をそのまま使う。
- `workflow_run.workflows` は workflow **name** フィルタ。#368 README
  は「同名 workflow 混入」を P2 既知制限として記録していた — event
  payload 側に workflow file/`workflow_id` 照合がなかったため。
- Issue title は `ISSUE_TITLE` 定数（`ci` 固定文字列）だったため、
  第 2 workflow を同じ title で起票すると scope が衝突する。

## Scope

### In Scope

- `.github/workflows/ci-issue-tracker.yml` / `ci-issue-tracker-reconcile.yml`
- `ci/tracker/tracker.mjs`（title 導出・payload identity・文言）
- `ci/tracker/tests/tracker.test.mjs` + 新規
  `ci/tracker/tests/issue-tracker-workflows-contract.test.mjs`
- `ci/tracker/README.md`・`docs/development-environment.md`・chain docs

### Out of Scope

- `consumer-verify.yml`・`daily-ci.yml`・`ci/consumer-verify/` 本体
  （verdict 語彙・証拠契約は既存のまま消費する）
- Issue label・marker 形式・close guard・retry 規約の変更
- 実 run での発火実測・credential・実機 lane（NotRun）

## Components and Boundaries

### 1. `tracker.mjs` — scope 対応の最小整備

既存の scope 抽象をそのまま使い、3 点だけを直す:

- **`issueTitle(scope)`**（`ISSUE_TITLE` 定数を置換）: title を
  `[CI failure][<branch>] <workflow display> failed` で導出。
  `WORKFLOW_TITLES` map で `ci` → `Full CI`（従来 title と byte 一致）、
  `consumer-verify` → `consumer-verify`。未知 workflow は sanitize 済み
  workflow id に fallback — 別 scope の title を借用しない。
  既存 open Issue は marker scope で同定されるため title 変更の影響は
  `ci` scope ではゼロ（同一文字列を維持）。
- **`classifyRun` の workflow identity 照合**: repo/branch 確認の直後に
  `workflow_run.name`/`path` を scope の `workflow`/`workflowFile` と
  照合。存在する field は全て一致必須・両方欠落は malformed → skip。
  `classifyHistoryRecord` の provable 規則（`path != null || name !=
  null`）と同じ境界。skip は安全 — reconcile が権威履歴で回復する。
  これで #368 の P2 既知制限（同名 workflow 混入）を event 側でも解消。
- **scope 対応文言**: `renderBody` の triage/policy 節と
  `close-recovered` comment の `ci` 固定表記を `scope.workflow`/
  `scope.branch` 由来にする（`consumer-verify` の Issue が
  「re-run `ci`」を指示しないように）。

変更しないもの: `ALLOWED_EVENTS`（両 scope で共有 — consumer-verify は
dispatch-only なので実質 `workflow_dispatch` のみ届くが、allowlist を
scope 別に分ける必然性がない）、`FAILURE_CONCLUSIONS`、marker 形式、
`plan`/`apply` の全ロジック、reconcile mode 分離。

### 2. `ci-issue-tracker.yml` — scope 別 job

```yaml
on:
  workflow_run:
    workflows: [ci, consumer-verify]
    types: [completed]
    branches: [main]
```

- `track` job: 既存のまま + `if: github.event.workflow_run.name == 'ci'`。
- `track-consumer-verify` job（新規）: 同一 step 構成 +
  `if: ... == 'consumer-verify'`、evaluate に
  `--workflow consumer-verify --workflow-file consumer-verify.yml`、
  report/artifact 名は `ci-issue-tracker-report-consumer-verify*`。
- 代替案（単一 job + shell case 分岐・matrix）は不採用: job `if` は
  matrix 展開前に評価されるため scope 別 gate を宣言的に書けず、
  bash 分岐は audit しにくい。2 job の明示複写がこの repo の workflow
  記述慣行（verbose・self-test 先行・attempt 固有 artifact）に合う。
- concurrency group・権限・checkout pin（`github.sha` = default branch）
  ・self-test は両 job で不変。

### 3. `ci-issue-tracker-reconcile.yml` — 全 scope 逐次 reconcile

`reconcile` job の evaluate step を 1 つの `run: |` block にし、
同一判定器を 2 回呼ぶ:

```bash
rc=0
mise exec -- node ci/tracker/tracker.mjs --reconcile \
  --report ci-issue-tracker-report.json || rc=1
mise exec -- node ci/tracker/tracker.mjs --reconcile \
  --workflow consumer-verify --workflow-file consumer-verify.yml \
  --report ci-issue-tracker-report-consumer-verify.json || rc=1
exit "$rc"
```

- 逐次実行は共有 concurrency group（repo 単位・workflow 横断で既に
  直列化済み）と整合し、Issue 書き込みの競合を構造的に排除する。
- `|| rc=1` 集約: 片 scope の API 失敗が他 scope を飢餓させないが、
  いずれか失敗で step/job は失敗（fail-closed — 「片方だけ成功」を
  黙って通さない）。
- 別 job 案（`needs` + `always()`）は step 重複と「片方 skipped で
  もう片方が待つ」複雑さを持ち込むため不採用。
- schedule・`workflow_dispatch`・`if: ref_name` guard・checkout pin・
  権限・self-test・report artifact（両 file を同梱）は不変。

### 4. verdict → tracker input の写像（変更なし・固定のみ）

tracker の入力は workflow conclusion であり、consumer verdict の写像は
`consumer-verify.yml` summary job が既に fail-closed で実装済み
（lane 失敗/cancelled/preflight 失敗/全 not_run・unsupported → run
失敗）。本 slice はこの結合を**壊さない**ことと、証拠 status 語彙が
pass/fail に化けないことを contract test で固定するだけ — evidence
status を tracker が直接読む設計にはしない（artifact 取得は tracker の
trust model 外: completed run の artifact を読まない原則を維持）。

## Data and Control Flow

```
consumer-verify.yml 完了（手動 or daily-ci GITHUB_TOKEN dispatch）
  ├─ 手動 dispatch → workflow_run event → ci-issue-tracker.yml
  │    ├─ track (if name=='ci')        → skip（consumer-verify payload）
  │    └─ track-consumer-verify        → tracker.mjs --workflow consumer-verify
  │         classifyRun: name/path 照合 → conclusion 分類 → scope marker
  │         で managed Issue 集約 → 失敗なら create/update、
  │         被覆 success なら close-recovered
  └─ GITHUB_TOKEN dispatch → workflow_run event 不発（anti-recursion）
       → ci-issue-tracker-reconcile.yml（6h schedule / 手動）
            → --reconcile --workflow consumer-verify …
            → 権威履歴のみで同一判定 → 同じ Issue に収束
```

## Non-Functional Constraints

- stdlib-only Node・mock client テストのみ（実 API・実 Issue を起こさ
  ない）。新規依存なし。
- workflow_run job は default branch のコードのみ実行 — completed run
  の head/artifact を checkout/download しない原則を維持。
- Issue 書き込みは scope ごとに高々 1 open — `ci` と `consumer-verify`
  の Issue は title・marker scope・履歴が全て分離。

## Risks and Trade-offs

- **workflow name filter の残存リスク**: `workflows: [ci,
  consumer-verify]` は name 一致のため、将来 `name: consumer-verify`
  を持つ別ファイルが増えると event job は起動し得る — ただし
  `classifyRun` の name/path 照合で証拠にはならず、誤 scope の Issue
  も作らない（起動は無駄な runner のみ）。
- **reconcile の実行時間**: scope 2 回分の API 呼出しが 1 job に直列
  化される。現行履歴量では問題なし（timeout 15 分・各 scope は
  独立して完了）。
- **`ci` title の byte 一致維持**: `issueTitle` の `ci` 分岐は従来
  `ISSUE_TITLE` と同一文字列を生成することを test で固定 — 既存 open
  Issue の title が update で書き換わっても表示上の差分は出ない。
- **anti-recursion の残存**: 手動 `gh workflow run consumer-verify`
  （PAT/user token）は event が発火するが、daily の GITHUB_TOKEN
  dispatch は発火しない — reconcile の 6h tick が worst-case の回復
  遅延（~6h+実行時間）を持つ。これは #368 で許容済みの運用契約。

## Deferred Design Concerns

| 後送り項目 | 理由 | 追跡先 |
| --- | --- | --- |
| 実 workflow_run/reconcile 発火の実測 | merge 後の実走事項（NotRun） | Issue #294 本体 |
| `consumer-verify` 失敗 Issue から consumer repo 側 Issue への自動リンク | Issue 横断の責務は別設計 | Issue #294 本体 |
| reconcile cadence の consumer-verify 向け調整 | 現行 6h で十分（dispatch は日次） | Issue #294 本体 |

## Verification Notes

- [ ] `mise run test-ci-tracker` — scope 分離・identity 照合・reconcile
  回復・既存 ci 非回帰
- [ ] `mise run test-ci-consumer-verify` — verdict 語彙の非回帰
- [ ] `mise exec -- actionlint .github/workflows/ci-issue-tracker.yml
  .github/workflows/ci-issue-tracker-reconcile.yml`
- [ ] `mise run fmt-check` / `mise run check` / `mise run lint` /
  `mise run test`
