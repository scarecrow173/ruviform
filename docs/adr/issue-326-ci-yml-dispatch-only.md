---
id: "ADR-SYd9kRt5mEssMKoPG8hnSG"
type: "adr"
status: "accepted"
title: "Issue #326 ci.yml workflow_dispatch 専用化 — PR/push trigger 撤去と concurrency 簡素化（Issue #328 ADR 補足）"
created: "2026-11-06"
updated: "2026-11-06"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/326"
    - "https://github.com/scarecrow173/ruviform/issues/365"
    - "docs/specs/issue-326-ci-manual-only.md"
  changes:
    added:
      - type: "file"
        path: "ci/daily/tests/ci-yml-contract.test.mjs"
    modified:
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "docs/adr/issue-328-ci-concurrency-grouping.md"
      - type: "file"
        path: "docs/development-environment.md"
      - type: "file"
        path: "mise.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-326-ci-manual-only.md"
  implemented-by:
    - "docs/designs/issue-326-ci-manual-only.md"
  depends-on:
    - "docs/specs/issue-326-ci-manual-only.md"
    - "docs/adr/issue-328-ci-concurrency-grouping.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/issue-368-tracker-reconcile-schedule-entry.md"
    - "docs/adr/issue-368-ci-failure-issue-tracker-workflow-run-issue-run-id-attempt-compare-2-g.md"
  refines:
    - "docs/adr/issue-328-ci-concurrency-grouping.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-326-ci-manual-only.md"
  derived-by:
    - "docs/designs/issue-326-ci-manual-only.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-326-ci-manual-only.md"
  references:
    - "https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/trigger-a-workflow"
  defers: []
  deferred-by: []
---

# Issue #326 ci.yml workflow_dispatch 専用化 — PR/push trigger 撤去と concurrency 簡素化

Status: accepted
Date: 2026-11-06
Deciders: Ruviform CI 移行 worker + 独立 review-protocol（ADR-034SraR8CrkZ0SSa7tQEwh と同じ「trigger 契約は ADR で固定」先例）

## Context and Problem Statement

Issue #365 決定 1 は「PR/push 通常 CI を廃止し `ci.yml` を `workflow_dispatch` 専用にする」と定め、Issue #326 は Required 解除確認後に trigger を撤去する段階（Stage 4）を規定した。前提は全て充足した — `main-checkstatus`（9 lane required contexts）は enforcement `disabled`（2026-11-06 Rulesets API 実測）、`main-protect`（PR 必須・non-fast-forward・deletion 禁止）は active、Classic branch protection は未設定、Tracker+reconcile（#368+PR #380）は main で manual dispatch 入口動作を実証済み（run 42178261480・権威履歴を読み pending に fail-closed noop。gap-heal の end-to-end サイクル「bot 失敗→Issue 作成→bot 成功→close」および実 cron 走行は未観測であり、これらは運用実績として後続確認する）。

`ci.yml` に `pull_request`/`push` trigger が残る限り、(a) 重い Full CI が通常 PR/push で起動して外部 runner 待機がクリティカルパスを塞ぎ、(b) Daily の「未検証判定→dispatch」単一正本モデルと二重経路が併存する。ADR-ieVahO2wRjuTcKd43fT59I（#328）が選択した event 条件付き concurrency 式（`pull_request` では per-PR group + cancel、非 PR では run 一意）は PR trigger 存在を前提としており、撤去後は常に非 PR 分岐に解決する dead 機構となる。

## Decision Drivers

- #365 決定 1・#326 Stage 4 の受入: 「PR 作成/更新・main push だけでは重い CI/補助 Actions が動かない」+「手動 Full CI は常時利用可能」。
- 「PR CI トリガーだけ先に止めて Required を残すデッドロックは禁止」→ Required 実効ゼロ（checkstatus disabled）を先に実測確認。
- ADR-ieVahO2wRjuTcKd43fT59I（Issue #328） の不変条件（非 PR run が互いを cancel しない・group は workflow 接頭辞で衝突しない）を満たしたまま dead 機構を除く。
- Tracker の `workflow_run` entry と reconcile entry（ADR-vJvyZNtrHi2xnycJxbnSPv）は独立の正当入口として不変。

## Considered Options

- **Option A（採用）**: `on:` を `workflow_dispatch` のみとし、concurrency を `${{ github.workflow }}-${{ github.run_id }}`・`cancel-in-progress: false` に簡素化。workflow 横断で「PR/push trigger を持つファイルが存在しない」ことを contract test で回帰固定。
- Option B: trigger だけ撤去し concurrency 式は残す。却下 — `github.event_name == 'pull_request'` は常に false に解決する dead 機構であり、将来 trigger が復活した際に「PR 連続 push で旧 run を cancel する」ADR-ieVahO2wRjuTcKd43fT59I（Issue #328） の挙動が**仕様として**戻るか読者には不明瞭になる。式を残すと契約が二重に語られる。
- Option C: PR trigger を残したまま jobs 側で skip する。却下 — status/check run が生成され「動かない」受入条件を満たさず、runner も消費する（#365 は trigger 除去を要求）。
- Option D: `main-checkstatus` ruleset を本変更で併せて廃止する。却下 — GitHub Settings への書き込みは管理者/Owner の操作であり、本タスクは前提条件（disabled 実測）の確認まで。

## Decision Outcome

Chosen option: **Option A**。

- `.github/workflows/ci.yml` の `on:` を `workflow_dispatch:`（`inputs.expected_sha` 不変 — Daily dispatch 契約）のみとし、`pull_request`（全 types）と `push` を除去する。
- concurrency を `group: ${{ github.workflow }}-${{ github.run_id }}`・`cancel-in-progress: false` に簡素化する。PR 存在下で ADR-ieVahO2wRjuTcKd43fT59I（Issue #328） が満たした「非 PR run は他 run を cancel しない・run ごと一意 group・workflow 接頭辞で他 workflow と衝突しない」不変条件は、全 trigger が非 PR となった後もこの単純形で同一に満たされる。
- 本 ADR は ADR-ieVahO2wRjuTcKd43fT59I を「PR trigger 存在を前提とした event 条件式の部分のみ撤廃」と補足する — その核心決定（per-PR cancel 機構）は trigger 撤去により対象 event が消えるため退役するものであり、旧 ADR の status・理由・当時の文脈は書き換えない（related/refined-by の metadata 追記のみ）。
- `.github/workflows/` 配下で `pull_request`/`pull_request_target`/`push` trigger を持つファイルが存在しないことを `ci/daily/tests/ci-yml-contract.test.mjs`（`mise run test-ci-daily` に同梱）で回帰固定する。`workflow_run` は event 反応であり対象外。
- `ci-issue-tracker.yml`（`workflow_run` 専用）・`ci-issue-tracker-reconcile.yml`（`schedule`+`workflow_dispatch`）・`daily-ci.yml`（`schedule`+`workflow_dispatch`）・`consumer-verify.yml`（`workflow_dispatch`）の正当入口は不変。
- ruleset/保護設定は変更しない — `main-protect` active・`main-checkstatus` disabled・Classic 未設定は前提条件の実測記録であり本 ADR の対象ではない。

### Consequences

- Good, because PR 作成/更新・main push で重い CI が一切起動せず（runner 待機のクリティカルパス除去）、検証証跡の正本が Daily/手動 dispatch + `expected_sha` 照合の単一路に統一される。bot dispatch 完了の通知 gap は reconcile 入口で既に埋まっている。contract test が「PR で CI が走り始める」復活を機械的に防ぐ。
- Bad, because PR に status/check run が生成されなくなり、レビューは実装者のローカル検証記録 + Daily/Tracker の Issue 集約を正本とする運用へ移る（#326 が定めた運用方針 — human gate）。PR 作成直後の自動検証は意図的に失われる。
- Neutral: `pull_request`/`push` event の run record は履歴に残り、tracker/daily の event allowlist（`push`/`workflow_dispatch`/`schedule`/`pull_request` 構造除外）は履歴互換のため維持 — 新規 record が生えなくなるだけで、過去 record の証拠価値は変わらない。rollback は trigger を戻す PR のみ（Required は disabled のままなので戻しても deadlock にはならない）。

## Patterns to Avoid

- `on:` に `pull_request`/`push` を残したまま jobs の `if:` で skip する「擬似撤去」— check run が生成され受入条件「動かない」を満たさない。
- Required checks が残った状態での trigger 撤去（#326 禁止の deadlock — 本 ADR は disabled 実測を前提にする）。
- `expected_sha` input の改名・型変更（daily-ci の dispatch 契約を壊す）。
- concurrency group への PR 条件式の温存（dead 機構が将来 trigger 復活時の挙動を曖昧にする）。
- ruleset 変更のコード PR への混入（権限境界 — Settings 書き込みは管理者操作）。
