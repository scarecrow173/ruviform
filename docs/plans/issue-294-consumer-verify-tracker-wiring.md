---
id: "PLAN-J2SBIDqNyZcnFDIJmvppYM"
type: "plan"
status: "completed"
title: "Issue #294 consumer-verify → ci-issue-tracker 配線実装計画 — scope 別 job・reconcile 全 scope 化・contract test"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "docs/specs/issue-294-consumer-verify-tracker-wiring.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/designs/issue-294-consumer-verify-tracker-wiring.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-294-consumer-verify-tracker-wiring.md"
    - "docs/specs/issue-294-consumer-verify-tracker-wiring.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-294-consumer-verify-tracker-wiring.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-294-consumer-verify-tracker-wiring.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #294 consumer-verify → ci-issue-tracker 配線実装計画 — scope 別 job・reconcile 全 scope 化・contract test Implementation Plan

## Goal

`consumer-verify` の失敗を既存 `ci-issue-tracker` 機構で追跡可能にする:
event 入口に `consumer-verify` scope の job を追加し、reconcile 入口を
全 scope 逐次実行に拡張し、evaluator の title 導出と payload workflow
identity 照合を scope 対応にする。専用 framework は作らず、既存の
上限付き retry・Issue 集約・marker/close guard 規約をそのまま流用する。

## Scope

単一実装 stream。`tracker.mjs` の scope 対応 + 2 workflow YAML +
contract test + docs 同期で完結する。`consumer-verify.yml`・
`daily-ci.yml`・consumer repo・secret/権限は変更しない。

## Upstream Documents

- `docs/specs/issue-294-consumer-verify-tracker-wiring.md`
- `docs/designs/issue-294-consumer-verify-tracker-wiring.md`（approved）
- `docs/test-specs/issue-294-consumer-verify-tracker-wiring.md`（検証契約）

## Design Inputs

- `docs/designs/issue-294-consumer-verify-tracker-wiring.md`

## File Map

- `ci/tracker/tracker.mjs` — `ISSUE_TITLE` → `issueTitle(scope)` +
  `WORKFLOW_TITLES`、`classifyRun` に workflow `name`/`path` scope 照合、
  `renderBody`/close comment の scope 対応文言。
- `ci/tracker/tests/tracker.test.mjs` — `payload()`/`histRun()` helper に
  workflow identity 既定を追加、consumer-verify scope テスト群を追加。
- `ci/tracker/tests/issue-tracker-workflows-contract.test.mjs`（新規）—
  両 workflow YAML 構造契約 + verdict 写像固定。
- `.github/workflows/ci-issue-tracker.yml` — `workflows` 拡張 +
  `track-consumer-verify` job 追加 + `track` に `if` gate。
- `.github/workflows/ci-issue-tracker-reconcile.yml` — evaluate step を
  全 scope 逐次実行に拡張 + report artifact 2 file 化。
- `ci/tracker/README.md` — 2 scope 構成・identity 照合・P2 解消の同期。
- `docs/development-environment.md` — tracker/reconcile 節の同期。

## Implementation Sequence

- [x] T1: `tracker.mjs` scope 対応（title 導出・payload identity・文言）
- [x] T2: `tracker.test.mjs` helper 拡張 + consumer-verify scope テスト
- [x] T3: `ci-issue-tracker.yml` scope 別 job + `ci-issue-tracker-reconcile.yml` 全 scope reconcile
- [x] T4: `issue-tracker-workflows-contract.test.mjs` 新規（YAML + verdict 写像）
- [x] T5: `ci/tracker/README.md` + `docs/development-environment.md` 同期
- [ ] T6: docs chain（spec/design/plan/test-spec/task）+ impl record +
  索引再生成 + 最終検証（fmt-check/check/lint/test/node tests/actionlint）

## Dependencies and Risks

- 依存: #368 tracker 機構（main 済み・scope 引数対応済み）、
  #294 consumer-verify workflow + 証拠契約（main 済み・変更なし前提）、
  #367/#294 daily tuple scheduler（GITHUB_TOKEN dispatch — anti-recursion
  gap の根拠）。
- リスク: `workflow_run.workflows` は name フィルタのため同名別ファイル
  の混入余地が残る → `classifyRun` の name/path 照合で証拠化を防ぐ。
  reconcile の scope 直列化で job 時間が伸びる → 現行履歴量では
  timeout 15 分に十分収まる。
- 最重要制約: `ci` scope の非回帰 — 既存テスト全件・title verbatim・
  marker 形式・mode 分離を壊さない。

## Task Breakdown

- T1+T2: evaluator scope 対応 + テスト（本体実装・test 同梱）
- T3: 両 workflow YAML
- T4: YAML/verdict contract test
- T5+T6: docs 同期・impl record・索引再生成・最終検証

## Verification

- [ ] `mise run test-ci-tracker` — scope 分離・identity・reconcile・ci 非回帰
- [ ] `mise run test-ci-consumer-verify` — verdict 語彙非回帰
- [ ] `mise exec -- actionlint .github/workflows/ci-issue-tracker.yml .github/workflows/ci-issue-tracker-reconcile.yml`
- [ ] `mise run fmt-check` / `mise run check` / `mise run lint` / `mise run test`
- [ ] 実 workflow_run/reconcile 発火は merge 後に実測（NotRun）

## Open Questions / Gaps

- なし（spec/design で確定済み）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（worker dispatch のため subagent 不使用）
- Independent work streams: none
- User approval required before dispatch: yes（review-protocol で代替済み — design approved）
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- reviewer focus: (1) `classifyRun` identity 照合の fail-closed 境界
  （両欠落 skip・片方一致受理）、(2) `issueTitle` の `ci` byte 一致、
  (3) reconcile の `|| rc=1` 集約と mode 分離維持、(4) 両 job の
  `if` gate と scope args の一致、(5) verdict 写像テストが
  not_run/unsupported を pass/fail に化けさせないこと。
