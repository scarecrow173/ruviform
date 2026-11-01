---
id: "PLAN-034QdTvxzjsU3jbWDIgBee"
type: "plan"
status: "completed"
title: "Issue #219 Consumer互換契約・影響判定テンプレート・Cross-Repo CI Gate 実装計画"
created: "2026-11-01"
updated: "2026-11-01"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/adr/cross-repo-ci-revision.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/adr/cross-repo-ci-revision.md"
    - "docs/designs/issue-219-cross-repo-ci-gate-credential-runbook.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/consumer-verify-workflow-gate-not-run-pass-preflight-gating-sha-guard.md"
    - "docs/test-specs/consumer-pr-issue-template.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #219 Consumer互換契約・影響判定テンプレート・Cross-Repo CI Gate 実装計画 Implementation Plan

## Goal

spec `issue-219-consumer-compat-contract`（R1–R9・AC1–AC10）と ADR `cross-repo-ci-revision`（Option A/1/i）に基づき、Consumer 影響判定テンプレート・`consumer-verify.yml` 手動 lane・AGENTS.md drift 修正を実装する。

## Scope

単一の実装 stream（ruviform repo 内の `.github/`・`AGENTS.md`・`docs/` のみ）。Consumer repo・credential 実設定・自動 trigger・required check 昇格は scope 外（spec Out of Scope）。

## Upstream Documents

- `docs/specs/issue-219-consumer-compat-contract.md`（proposed・AC1–AC10）
- `docs/adr/cross-repo-ci-revision.md`（proposed・Option A+1+i）
- `docs/discovery/issue-219-consumer-cross-repo-ci.md`（棚卸し・Evidence）

## Design Inputs

- `docs/designs/issue-219-cross-repo-ci-gate-credential-runbook.md`（approved）

## File Map

- `.github/PULL_REQUEST_TEMPLATE.md` — 新設。Consumer 影響分類表＋検証証拠表（R1/R4・AC1）
- `.github/ISSUE_TEMPLATE/consumer-impact.yml` — 新設。Ruviform 側 Consumer 影響 Issue form（R2・AC2）
- `.github/ISSUE_TEMPLATE/consumer-followup.yml` — 新設。Consumer 追従 Issue form（R2・AC2）
- `.github/ISSUE_TEMPLATE/config.yml` — 新設。Consumer repo Issue への contact link（AC2）
- `.github/workflows/consumer-verify.yml` — 新設。preflight + verify-audiolens + verify-synvelis + summary（R5–R7・AC3）
- `AGENTS.md` — `ci.yml` 記述の drift 修正（AC10）
- `docs/impl/ir/` — 実装記録（IR）
- `docs/discovery/issue-219-consumer-cross-repo-ci.md` — Evidence への実測追記は行わない（実装後の初回実走証拠は別運用）

## Implementation Sequence

- [ ] T1: PR/Issue テンプレート 4 ファイルを作成（design §Components の表形式・form 項目に準拠）
- [ ] T2: `consumer-verify.yml` を作成（design の job 構造・gating・guard・summary 真偽表をそのまま実装）
- [ ] T3: AGENTS.md の `ci.yml` 記述を現行 trigger（pull_request/push/workflow_dispatch）・required checks・job 構成に修正
- [ ] T4: `actionlint` + doc audit + IR 記録で検証を完了する

## Dependencies and Risks

- credential 未設定（`CROSS_REPO_DEPLOY_KEY` 未投入）が既知の初回実走 blocker — workflow は `not run` 出力＋全 lane not run で workflow 失敗として正直に示す設計のため、実装自体は credential 非依存。
- `url.insteadOf` の `.git` suffix・`secrets` の `if:` 非対応・`inputs.default` 非評価など、review-protocol で確定した workflow の落とし穴 3 点は設計解消済み — 実装は設計の記述を忠実に再現すること（review B#1/#3/#4/#5）。
- audio-lens test lane は `git lfs pull` が前提（mp4 fixture）。smudge skip のまま test を走らせない。

## Task Breakdown

- T1 テンプレート群 → `docs/tasks/` の 1 task
- T2 workflow → 1 task
- T3 AGENTS.md → 1 task
- T4 検証・IR → 1 task

## Verification

- [ ] `mise exec -- actionlint .github/workflows/consumer-verify.yml` が pass
- [ ] `audit_docs.js --type spec|adr|design|plan|task|test-spec` で本 Issue 新規文書に blocking finding がない
- [ ] テンプレート 4 ファイルが design の表・form 項目を持つ（AC1/AC2）
- [ ] workflow が design の gating/guard/真偽表を実装している（AC3・review 確定修正の反映確認）
- [ ] AGENTS.md が現行 CI 実態を記述（AC10）
- [ ] IR に実装内容・検証結果・未検証項目が記録される

## Open Questions / Gaps

- `consumer-verify` の初回 dispatch 実走（credential 設定後）は本計画の scope 外 — 実走証拠は runbook step 5 の別運用として Issue #219 に記録する。
- audio-lens `test-ci` の全 test が LFS fixture のみで完結するか（他の外部依存がないか）は初回実走で確認する。design は fixture LFS 復元のみを保証する。

## Self-Review

- [ ] Every spec requirement maps to a task.
- [ ] No placeholders remain.
- [ ] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution
- Independent work streams: T1 と T2 は独立（T2 のみ review 修正の適用が多い）
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- review-protocol の CONFIRMED 修正（P1×2+P2×10+P3）を設計に反映済み。実装レビューでは workflow YAML が設計の記述（gating・guard・真偽表・credential パターン）を逸脱なく実装しているかに絞る。
