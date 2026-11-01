---
id: "IMPL-034Qe219CrossRepoCiGateX7k"
type: "impl"
status: "completed"
title: "Issue #219 Consumer 影響判定テンプレート・consumer-verify workflow・AGENTS.md CI drift 修正"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/219"
  changes:
    added:
      - type: "workflow"
        path: ".github/workflows/consumer-verify.yml"
      - type: "template"
        path: ".github/PULL_REQUEST_TEMPLATE.md"
      - type: "template"
        path: ".github/ISSUE_TEMPLATE/consumer-impact.yml"
      - type: "template"
        path: ".github/ISSUE_TEMPLATE/consumer-followup.yml"
      - type: "template"
        path: ".github/ISSUE_TEMPLATE/config.yml"
      - type: "file"
        path: "docs/impl/ir/issue-219-consumer-verify-gate.md"
    modified:
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../tasks/issue-219-t1-impact-templates.md"
    - "../../tasks/issue-219-t2-consumer-verify-workflow.md"
    - "../../tasks/issue-219-t3-agents-ci-drift.md"
  implemented-by: []
  depends-on:
    - "../../designs/issue-219-cross-repo-ci-gate-credential-runbook.md"
    - "../../plans/issue-219-consumer-cross-repo-ci-gate.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../specs/issue-219-consumer-compat-contract.md"
    - "../../adr/cross-repo-ci-revision.md"
    - "../../discovery/issue-219-consumer-cross-repo-ci.md"
    - "../../test-specs/consumer-verify-workflow-gate-not-run-pass-preflight-gating-sha-guard.md"
    - "../../test-specs/consumer-pr-issue-template.md"
  refines: []
  refined-by: []
  derives-from:
    - "../../plans/issue-219-consumer-cross-repo-ci-gate.md"
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

# Issue #219 Consumer 影響判定テンプレート・consumer-verify workflow・AGENTS.md CI drift 修正

## Summary

plan `PLAN-034QdTvxzjsU3jbWDIgBee` の T1–T3 を実装した。`consumer-verify.yml`（preflight + verify-audiolens + verify-synvelis + summary の 4 job、手動 dispatch 専用情報 lane）、PR/Issue テンプレート 4 ファイル、AGENTS.md の stale な CI 記述を現行実態（pull_request/push/workflow_dispatch trigger・required checks 12 job）に修正した。review-protocol の CONFIRMED 修正（`.git` suffix・`secrets` の if 不使用・`inputs.default` 非使用・`if: always()` 証拠出力・repo URL キーの単一 SHA guard・LFS pull・hardcoded fingerprint pin）は全て workflow に反映済み。

## Context

- 実施タスク: `docs/tasks/issue-219-t1-impact-templates.md`・`issue-219-t2-consumer-verify-workflow.md`・`issue-219-t3-agents-ci-drift.md`（いずれも `docs/plans/issue-219-consumer-cross-repo-ci-gate.md` 配下）
- 上流: `docs/designs/issue-219-cross-repo-ci-gate-credential-runbook.md`（approved）・`docs/specs/issue-219-consumer-compat-contract.md`（R1–R9/AC1–AC10）・`docs/adr/cross-repo-ci-revision.md`（Option A+1+i）
- review-protocol の verdict: ACCEPT WITH NON-BLOCKING NOTES（P1×2+P2+P3 全て設計反映済み）

## Implementation

### Changes

- `.github/workflows/consumer-verify.yml` — 新設。`workflow_dispatch` inputs（`ruviform_rev` 空・`audiolens_ref`/`synvelis_ref` = main）+ workflow env で `inputs.ruviform_rev || github.sha` を `RUVIFORM_REV` に解決。`preflight`（入力検証 + `env` 経由の credential 有無判定→output）。`verify-audiolens`/`verify-synvelis`（`needs: [preflight]`・`if: always()`・gating step が preflight 失敗を `fail (preflight failed)` に、credential 未設定を `not run` に分離）。audio-lens lane: Deploy Key 配置→`ssh-keyscan`+公開 fingerprint 定数照合→SSH clone（depth 1・`GIT_LFS_SKIP_SMUDGE=1`）→`url.insteadOf`（`.git` suffix 付き）→manifest rev 一括書換→`cargo fetch`→`Cargo.lock` を repo URL キーで単一 SHA assert→`mise install --locked`→`check-ci`→`check-cli-headless`→`git lfs pull`→`test-ci`→`if: always()` 証拠表→cleanup。synvelis lane: clone→workspace `Cargo.toml` presence check→`not run (no buildable workspace)`/`pass`。`summary`（`if: always()`・全 needs）: 証拠表集約 + fail/cancelled→失敗・全 lane not run→失敗（証拠ゼロを緑にしない）。
- `.github/PULL_REQUEST_TEMPLATE.md` — Consumer 影響分類表（両 Consumer × impacted/no impact/unknown）+ 検証証拠表（lane・OS・feature・revision・pass/fail/not run・許容例外・Rollback）+ `unknown` 解消先必須記述 + human gate 注記。
- `.github/ISSUE_TEMPLATE/consumer-impact.yml` — 変更種別・両 Consumer 分類・根拠・証拠・相互リンクの Issue form。
- `.github/ISSUE_TEMPLATE/consumer-followup.yml` — 追従用 form（対象 Consumer・ruviform revision・lane 結果・Rollback・相互リンク）。
- `.github/ISSUE_TEMPLATE/config.yml` — synvelis/audio-lens Issue への contact link。
- `AGENTS.md` — `ci.yml` 記述を「`workflow_dispatch` 専用」→「`pull_request`/`push`/`workflow_dispatch` 3 trigger・required checks 12 job」に修正し、現在の job 列挙（verify/l1-real-verify/ui-verify の 3 OS matrix + rt-verify/icc-verify/bench ubuntu 単独）・consumer-verify 情報 lane・Consumer 影響分類の merge 時点 human gate を追記。

### Decisions During Implementation

- **`RUNNER_TEMP` の context 参照**: `${{ env.RUNNER_TEMP }}` は `env` context に無い（default env var は `env:` 宣言値のみ）。step の `working-directory`・`with:` には `${{ runner.temp }}` を使い、shell 内は `$RUNNER_TEMP` を使う形に統一。
- **証拠出力 step の working-directory 排除**: `if: always()` の証拠記録 step が clone 未実行（not run）時に `working-directory` 不在で失敗しないよう、`working-directory` は gate==run の step のみに限定し、結果記録は repo 既定 dir + `git -C` 参照にした。
- **mise-action の配置**: `cargo fetch` step が `mise exec` を呼ぶため、`jdx/mise-action` を clone 前に配置。
- **fingerprint 定数**: GitHub 公表 3 値（RSA `uNiVztks…`・ECDSA `p2QAMXNI…`・Ed25519 `+DiY3wvvV…`）を workflow 内ハードコードとし `ssh-keyscan` 結果を照合（真の pin）。

## Related Experiments

### Adopted

なし（機械的実装タスク。ローカル実測の再現経路は discovery Evidence に記録済み）

### Rejected

なし

## Validation

| 検証 | コマンド | 結果 |
| --- | --- | --- |
| workflow 構文 | `mise exec -- actionlint .github/workflows/consumer-verify.yml` | exit 0（`env.RUNNER_TEMP`/`steps.result` 自参照の 2 件を修正後 clean） |
| issue form・workflow YAML parse | `uv run --with pyyaml python -c yaml.safe_load(4 files)` | 全 4 ファイル parse OK |
| 既存 ci.yml 無改変 | `git status .github/workflows/ci.yml` | 変更なし（consumer-verify.yml は別 workflow） |
| AGENTS.md 記述と実態一致 | `ci.yml` の `on:`/job 名と `gh api` required contexts 12 件を照合 | pull_request/push/workflow_dispatch・12 required contexts と一致 |
| doc audit | `audit_docs.js --type spec|adr|design|plan|task|test-spec`（本 Issue 新規ファイル） | blocking finding 0（plan の verified-by 欠落を修正済み） |
| ローカル実測（参考） | audio-lens scratch clone で rev 書換→`cargo fetch`→`cargo check --workspace --all-targets` | 全 15 ruviform crate が候補 SHA `41772a7…` に再解決・check exit 0・25.78s（discovery Evidence 記録済み） |

## 未検証（意図的・別運用）

- `consumer-verify` の実 dispatch 実行: `CROSS_REPO_DEPLOY_KEY` secret・3 repo への read-only Deploy Key 登録が未設定のため未実施。初回実走は credential 一次設定後の別運用（design Appendix runbook step 5）。未設定時の `not run` 振る舞い・全 lane not run→workflow 失敗は設計/test-spec で契約化済み。
- audio-lens `test-ci` の実行: ローカル実測は `cargo check` のみ。`test-ci`・`check-cli-headless`・`git lfs pull` の実走は credential 設定後の初回 dispatch で検証する。
- synvelis `presence` 以外の lane: workspace 未存在のため v1 対象外。

## Risks

- workflow の SSH/LFS/credential 経路は初回実走まで実測未確認（audio-lens CI 自身も実走 0 件）。設計は review で P1/P2 を全て修正済みだが、runner 環境差（`ssh-keyscan` 到達性・`git lfs` の存在）が残る。
- audio-lens の `mise.toml` 変更（`check-cli-headless` 削除・`test-ci` 名変更）に workflow が追従する必要がある。

## Follow-ups

- owner/admin: `CROSS_REPO_DEPLOY_KEY` 一次設定（design Appendix runbook）→ 初回 dispatch 実走 → 結果を Issue #219 と discovery `## Evidence` 追記へ記録。
- Consumer 側 Issue 起票案: audio-lens #127 系の追従 PR 経路が本 lane と一致していることの確認。synvelis #28 の workspace buildable 化は別 Issue。
- T4 完了後の本 plan status → `completed`。
