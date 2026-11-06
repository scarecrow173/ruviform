---
id: "TSPEC-nB4WC4Q4CYMJmm6HVksOwr"
type: "test-spec"
status: "approved"
title: "Issue #326 ci.yml dispatch 専用化 test specification — trigger 不存在契約・dispatch 契約維持・PR 起動ゼロ実証"
created: "2026-11-06"
updated: "2026-11-06"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/326"
    - "docs/specs/issue-326-ci-manual-only.md"
    - "docs/test-specs/issue-367-daily-scheduler.md"
  changes:
    added:
      - type: "file"
        path: "ci/daily/tests/ci-yml-contract.test.mjs"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-326-ci-manual-only.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-368-tracker-reconcile.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-326-ci-manual-only.md"
    - "docs/plans/issue-326-ci-manual-only.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-326-ci-manual-only.md"
    - "docs/adr/issue-326-ci-yml-dispatch-only.md"
    - "docs/designs/issue-326-ci-manual-only.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #326 ci.yml dispatch 専用化 test specification

## Scope / Contract Under Test

- `.github/workflows/` 配下の**全** workflow file に `pull_request`/`pull_request_target`/`push` trigger が存在しない（#365 受入「PR 作成/更新・main push だけでは重い CI/補助 Actions が動かない」の機械 guard — 将来の trigger 復活・新規 PR workflow 追加を回帰阻止）。
- `ci.yml` の trigger 集合が**厳密に** `{workflow_dispatch}` のみ（denylist ではなく allowlist — `schedule:`/`merge_group:`/`workflow_call:` 等の迂回入口も Daily decide+dispatch 単一正本契約への違反として回帰阻止）。
- `ci.yml` が `inputs.expected_sha:` 宣言を維持する（Daily dispatch 契約 — 削除・改名・型変更を回帰阻止）。
- `ci-issue-tracker.yml` が `workflow_run` trigger を維持する（event 入口維持の回帰 guard — ADR-034SraR8CrkZ0SSa7tQEwh）。
- PR 作成/更新時に当該 PR で `pull_request` event の run が起動しないこと（実 GitHub 挙動の観測証拠 — offline test 対象外・PR 上で実測記録）。

## Method

- 実装: `ci/daily/tests/ci-yml-contract.test.mjs`（`node --test`・Node stdlib のみ・外部通信なし — `.github/workflows/*.yml` を `readdirSync`+`readFileSync` で字句 scan。yaml parser 依存は既存方針どおり作らない）。
- `on:` block の抽出: top-level の `on:`/`"on":`/`'on':` key 行から次の top-level key まで（UTF-8 BOM 正規化・quoted 後続 key も境界認識）。`on:` を持たないファイルは別 test で fail — vacuous pass を防ぐ。
- trigger 検出: `on:` の**直接子のみ**を対象 — block map key（`  push:`）と block sequence item（`  - push`・親 key と同 column の col-0 seq を含む）を最小 indent で識別し、`on:` 行自体の inline 値（scalar `'push'` 含む・flow list `[a, b]`・flow map `{a: …}`・複数行 flow）を併せて解釈する。深い階層の key（`branches:`・`push` という名の dispatch input 等）は trigger と判定しない。コメント行・行末コメントは除去。
- PR 起動ゼロ実証は offline test ではなく PR ライフサイクルでの実測: `gh pr checks <n>`・`gh api repos/scarecrow173/ruviform/commits/<head>/check-suites`・`gh run list --event pull_request --branch <head>` で run/check suite 0 件を記録（起動した場合は trigger 評価が base/head どちらの file を使ったかも記録 — 受入は「実挙動を記録」）。

## Cases

### Contract test（offline・`mise run test-ci-daily` 同梱）

| # | Case | 入力 | 期待 |
| --- | --- | --- | --- |
| C0 | 全 workflow が `on:` block を持つ | `.github/workflows/*.yml` 実ファイル | 抽出失敗（vacuous pass）でなく loud fail |
| C1 | 全 workflow に pull_request trigger 0 件 | `.github/workflows/*.yml` 実ファイル | `on:` 直接子に `pull_request` が無い |
| C2 | 全 workflow に pull_request_target trigger 0 件 | 同上 | `pull_request_target` が無い |
| C3 | 全 workflow に push trigger 0 件 | 同上 | `push` が無い |
| C4 | ci.yml の直接 trigger 集合が厳密に `{workflow_dispatch}` | `ci.yml` | allowlist 一致 — `schedule:`/`merge_group:`/`workflow_call:` 等の迂回入口も fail |
| C5 | ci.yml は expected_sha input を持つ | `ci.yml` | `expected_sha:` 宣言が `workflow_dispatch` inputs 配下に存在 |
| C6 | ci-issue-tracker.yml は workflow_run を持つ | `ci-issue-tracker.yml` | `on:` block に `workflow_run` が存在（event 入口維持） |
| C7 | 検出器 self-check: 全合法形を検出 | フィクスチャ（block map / block seq / col-0 seq / inline scalar / quoted scalar/key / flow list / flow map / 複数行 flow） | 全形で trigger 名を正確に返す |
| C8 | 検出器 self-check: 非 trigger を誤検出しない | フィクスチャ（コメント内 `pull_request:`・`push` 名の dispatch input・seq の branches filter） | 誤検出なし |
| C9 | RED 確認: 撤去前 ci.yml で C1-C3 が失敗する | 変更前ファイル | guard が実効的であることを実装前に確認 |

### 実挙動観測（PR 上・offline 非対象）

| # | Case | 手段 | 記録 |
| --- | --- | --- | --- |
| O1 | 本 PR で `pull_request` event run が 0 件 | `gh run list --event pull_request --branch fix/rf-ci-manual-only`・`gh pr checks`・check-suites API | 件数・head SHA・時刻・（起動した場合）base/head どちらの file 評価か |
| O2 | reconcile の実 cron 起動 | 未実施 — cron 未観測として記録（manual dispatch 42178261480 成功と区別） | 次回 tick（00:23/06:23/12:23/18:23 UTC）後の `gh run list --workflow ci-issue-tracker-reconcile` を coordinator に委譲 |

## Pass/Fail Criteria

- Pass: C1-C8 全 pass・`test-ci-daily`/`test-ci-tracker`/`test-ci-summary` 無回帰・actionlint clean・4 workflow byte-identical・O1 が実測記録される。
- Fail: trigger 復活・`expected_sha` 変改・正当入口の破損・contract test が RED で失敗しない（guard 無効）。
- NotRun: 実 cron（O2）・merge 後の main 連続運用 — 本 PR では観測不能。

## Traceability

- Verifies: `docs/specs/issue-326-ci-manual-only.md`（R1/R2/R4/R8/R9/AC 全般）、`docs/adr/issue-326-ci-yml-dispatch-only.md`（Option A・patterns-to-avoid の機械 guard）。
