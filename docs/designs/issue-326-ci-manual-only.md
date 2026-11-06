---
id: "DESIGN-bgU7RV8JrPVEY31dlXpJCG"
type: "design"
status: "approved"
title: "Issue #326 ci.yml dispatch 専用化 設計 — trigger 撤去・concurrency 簡素化・trigger 契約テスト"
created: "2026-11-06"
updated: "2026-11-06"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/326"
    - "https://github.com/scarecrow173/ruviform/issues/365"
    - "docs/specs/issue-326-ci-manual-only.md"
    - "docs/adr/issue-326-ci-yml-dispatch-only.md"
  changes:
    added:
      - type: "file"
        path: "ci/daily/tests/ci-yml-contract.test.mjs"
    modified:
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "docs/development-environment.md"
      - type: "file"
        path: "docs/adr/issue-328-ci-concurrency-grouping.md"
      - type: "file"
        path: "mise.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-326-ci-manual-only.md"
    - "docs/adr/issue-326-ci-yml-dispatch-only.md"
  implemented-by:
    - "docs/plans/issue-326-ci-manual-only.md"
  depends-on:
    - "docs/specs/issue-326-ci-manual-only.md"
    - "docs/adr/issue-326-ci-yml-dispatch-only.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-366-full-ci-stage1.md"
    - "docs/designs/issue-367-daily-scheduler.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-326-ci-manual-only.md"
    - "docs/adr/issue-326-ci-yml-dispatch-only.md"
  derived-by:
    - "docs/plans/issue-326-ci-manual-only.md"
    - "docs/test-specs/issue-326-ci-manual-only.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-326-ci-manual-only.md"
  references:
    - "https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/trigger-a-workflow"
  defers: []
  deferred-by: []
---

# Issue #326 ci.yml dispatch 専用化 設計

## Overview

`ci.yml` の `on:` を `workflow_dispatch`（`expected_sha` input 維持）のみとし、`pull_request`/`push` trigger とそれに連動した PR 条件 concurrency 機構を撤去する。Full CI の lane 構成・fail-closed 集約・SHA 証跡契約は一切変更しない — 変更は「いつ起動するか」の trigger 面と、それに従属する concurrency 式・説明コメントのみ。`.github/workflows/` 全体に `pull_request`/`pull_request_target`/`push` trigger を持つファイルが存在しないことを、stdlib-only の contract test で回帰固定する。

## Detailed Design

### `.github/workflows/ci.yml`（trigger 撤去 + 機構簡素化 + コメント同期）

- `on:` を次のみとする:
  ```yaml
  workflow_dispatch:
    inputs:
      expected_sha:  # 現行と同一の name/type/description/required:false
  ```
  `pull_request`（opened/synchronize/reopened/ready_for_review/edited）と `push`（branches: main）を丸ごと除去。`expected_sha` の宣言（name・type・required・description 内容を含む）は Daily dispatch 契約であり**一切変更しない** — コメント中の「Empty disables the check (manual dispatch, pull_request, push)」のみ実態に合わせ「manual dispatch without the input」に修正。
- header comment を書き換える: 「PRs and pushes to main verify the merged result; workflow_dispatch stays for …」→ dispatch 専用（daily-ci が未検証判定で `expected_sha` 付き dispatch する単一路 + 手動再検証）の説明にする。
- `concurrency:` を
  ```yaml
  group: ${{ github.workflow }}-${{ github.run_id }}
  cancel-in-progress: false
  ```
  に簡素化。`pull_request` event が存在しなくなったため `github.event_name == 'pull_request'` は常に false に解決する dead 機構であり、ADR-ieVahO2wRjuTcKd43fT59I（Issue #328） の採用形が既にこの値に退化している — 不変条件（非 PR run は他 run を cancel しない・run ごと一意 group・workflow 接頭辞）は同一に満たされる。
- 各 lane/checkout pin のコメント「(no-op for pull_request/push where checkout already resolves to github.sha; stops workflow_dispatch legs floating …)」を実態に合わせる（trigger は dispatch のみ — pin は常に有効で、lane job 間の ref 浮動を排除する意味に限定）。
- `ci-summary` job の transition note コメントを更新する: 「ruleset keeps the nine Linux/Windows lane contexts until the Owner migrates」→ `main-checkstatus` は disabled で required 実効ゼロ（2026-11-06 実測）、`ci-summary` は依然として run レベルの fail-closed 集約である旨を記述（job 自体の構造は変更しない）。

### `ci/daily/tests/ci-yml-contract.test.mjs`（新規 contract test）

- Node stdlib のみ（yaml parser 依存なし — 既存テストと同じ方針）。`.github/workflows/*.yml` を `readdirSync` で列挙し、各ファイルの `on:` block（top-level `on:` key から次の top-level key までの字句範囲・BOM 正規化・コメント除去）の**直接子**を trigger として識別 — 最小 indent の map key・block sequence item（col-0 seq 含む）・`on:` 行の inline scalar/flow 値（複数行 flow 含む）を全形カバーし、`branches:` や `push` 名の dispatch input 等の深い key は判定対象にしない:
  1. いかなるファイルにも `pull_request`・`pull_request_target`・`push` trigger が**存在しない**（`workflow_run` は event 反応で対象外）。
  2. `ci.yml` の直接 trigger 集合が厳密に `{workflow_dispatch}`（allowlist — `schedule:`/`merge_group:`/`workflow_call:` 等の迂回入口も fail）かつ `expected_sha:` input 宣言が存在する（Daily dispatch 契約の回帰 guard）。
  3. `ci-issue-tracker.yml` に `workflow_run` が存在する（event 入口維持の回帰 guard）。
  4. `on:` block を持たないファイルは loud fail — 抽出失敗の vacuous pass を防ぐ。self-check fixture が全合法形の検出とコメント/入れ子 key の非誤検出を担保する。
- `mise run test-ci-daily`（`node --test ci/daily/`）のディレクトリ実行で自動包含される — 新 mise task は作らない。

### `docs/development-environment.md`（同期）

- `ci.yml` の trigger 記述（`pull_request`/`push`/`workflow_dispatch` の 3 種 + required checks 説明 + concurrency 説明）を dispatch 専用・checkstatus disabled・簡素化 concurrency の実態に同期する。ADR-SYd9kRt5mEssMKoPG8hnSG への参照を追加。

### 変更しない部分（境界）

- `ci-issue-tracker.yml`・`ci-issue-tracker-reconcile.yml`・`daily-ci.yml`・`consumer-verify.yml` — byte-identical 不変（`git diff` で検証）。
- 全 lane job・`ci-summary` evaluator（`ci/summary/summary.mjs`）・daily 判定器（`ci/daily/decide.mjs`）・tracker（`ci/tracker/tracker.mjs`）— 実行系は一切触れない。
- `ALLOWED_EVENTS`（tracker:`push`/`workflow_dispatch`、daily:`push`/`workflow_dispatch`/`schedule`・`pull_request` 構造除外）— 履歴互換のため維持。
- Ruleset・secrets・permissions・PULL_REQUEST_TEMPLATE — 範囲外。
- AGENTS.md（trigger 記述を持たない）・historical docs（#328/#333/#346/#366 設計は当時の決定記録）。

## Verification

| 観点 | 検証方法 | 成功条件 |
| --- | --- | --- |
| trigger 撤去 | `ci/daily/tests/ci-yml-contract.test.mjs`（structural scan） | 全 workflow に pull_request/pull_request_target/push 0 件 |
| dispatch 契約 | 同 test + `mise run test-ci-daily` | `workflow_dispatch`+`expected_sha` 維持・50 既存 test pass |
| 他 workflow 不変 | `git diff origin/main...HEAD -- ci-issue-tracker.yml …4 file` | 0 行 |
| workflow 構文 | `mise exec -- actionlint .github/workflows/*.yml` | 0 findings |
| lint gate | `mise exec -- prek run --all-files` | 全 hook pass |
| PR 起動ゼロ実証 | 本 PR で `gh pr checks`/`gh api .../check-suites`・`gh run list --event pull_request` | `pull_request` 起動 0 件（実挙動を記録 — base 側評価で起動する場合も事実として記録） |

## Risks and Trade-offs

- **PR trigger 評価起点の不確実性**: GitHub が `pull_request` trigger のマッチングを head ref の workflow file で評価するかは文書に明示がなく、本 PR 自体で「起動 0 件」が確認できれば実証となるが、base 側で評価される場合はこの PR で Full CI が走る。どちらでも merge 後の trigger 不存在は不変のため受入は「実挙動を記録」でよい。
- **concurrency 簡素化の将来影響**: 仮に将来 `pull_request` trigger を復活させる場合、ADR-ieVahO2wRjuTcKd43fT59I（Issue #328） の per-PR cancel 機構はこの式には残らない — 復活時に意図を再設計する必要がある（新 ADR 側の判断として記録。現運用では復活は想定しない）。
- **PR の status 不在**: `main-protect` は status を要求しないため merge は可能 — ただし PR 作成直後の check が一切出ない画面になるのは意図的な運用変更（#326 運用方針）。
- **ci.yml のみ変更する根拠**: workflow 一覧を実測した結果、`pull_request`/`push` trigger を持つのは `ci.yml` のみ（consumer-impact-check は #366 で撤去済み・hooks は GH Actions trigger ではない）。「補助 Actions」の撤去対象は contract test で workflow 横断 0 件として固定する。

## Deferred Design Concerns

- `main-checkstatus` ruleset 本体の廃止 | disabled で実効ゼロ | Owner の Settings 操作として後続 | 本変更は実測記録まで
- tracker の同名 workflow 混入（event payload 側）| 現在同名なし・既知 P2 | 別タスク管理 | 変更なし
- 将来 PR 上での軽量 lint だけを復活させたい需要 | 現行方針は PR 一切無起動 | 需要発生時に新 ADR | 本設計は trigger 面のみ
