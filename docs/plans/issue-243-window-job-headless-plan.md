---
id: "PLAN-034WZGtEYdaixOYopRPVvk"
type: "plan"
status: "completed"
title: "Issue #243 — Window/Job 不変条件 Headless 検証 実装計画"
created: "2026-11-13"
updated: "2026-11-14"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/243"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-test/tests/window_job_headless.rs"
      - type: "file"
        path: "evidence/rf243/window-job-headless-coverage.md"
      - type: "file"
        path: "evidence/rf243/window-job-headless-verification.md"
      - type: "file"
        path: "docs/tasks/issue-243-window-job-headless-task.md"
      - type: "file"
        path: "docs/test-specs/issue-243-window-job-headless.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-243-window-job-headless.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-243-window-job-headless.md"
    - "docs/designs/issue-243-window-job-headless-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-243-windows-multiwindow-smoke.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-243-window-job-headless.md"
    - "docs/designs/issue-243-window-job-headless-design.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-243-window-job-headless.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #243 — Window/Job 不変条件 Headless 検証 実装計画 Implementation Plan

## Goal

`UiRuntime` + `FakeDriver` を consumer 配線で合成する
Headless Scenario テストを `ruviform-ui-test` に追加し、
Window/Job 不変条件（ownership・close・job cancel scoping・
stale Window/Receipt）の未被覆分を既存 CI lane で固定する
検証スライスを実施する。

## Scope

単一 implementation stream。`crates/ruviform-ui-test` の
integration test 追加 + `evidence/rf243/` 証拠 + docs
チェーンに限定し、production コード・他 crate・CI 設定には
触れない。

## Upstream Documents

- `docs/specs/issue-243-window-job-headless.md`
- `docs/designs/issue-243-window-job-headless-design.md`
- `docs/specs/issue-243-windows-multiwindow-smoke.md`（先行
  slice）

## Design Inputs

- `docs/designs/issue-243-window-job-headless-design.md`

## File Map

| ファイル | 変更 |
| --- | --- |
| `crates/ruviform-ui-test/tests/window_job_headless.rs` | 新規 — Harness + 4 テスト |
| `evidence/rf243/window-job-headless-coverage.md` | 新規 — 不変条件 × 既存被覆対応表 |
| `evidence/rf243/window-job-headless-verification.md` | 新規 — 実行記録・NotRun 残件 |
| `docs/{specs,designs,plans,tasks,test-specs,impl/ir,impl/exp}/issue-243-window-job-headless*.md` | docs チェーン |

## Implementation Sequence

- [x] 既存 Window/Job API・Fake/Headless 基盤・被覆状況の棚卸し
- [x] 未被覆不変条件の特定（cancel scoping vs unrouted・
  orphan job terminal routing・stale id cleanup・stale inbound
  drop）
- [x] `window_job_headless.rs` Harness + 4 テスト実装
- [x] `mise run fmt-check` / `check` / `lint` / `test`
  `ruviform-ui-test` で検証
- [x] `evidence/rf243/` 被覆対応表・実行記録の作成
- [x] docs チェーン + index 再生成、review-protocol レビュー

## Dependencies and Risks

- 依存: `UiRuntime`（#181/#386）、`FakeDriver`/`deliver_routed`
  （#386/#313 系）、`DemoService`（`demo.sleep`/`demo.echo`/
  `job.cancel`/`sys.ping`）、Issue #456 command receipt —
  すべて merged の既存資産のみ。新規 crate 依存なし。
- リスク: job worker thread の timing 依存 → deadline 付き
  bounded drain で吸収。既存被覆との重複 → 棚卸し済みの
  差分のみ追加。

## Task Breakdown

- T1 テスト実装（Harness + 4 scenario）
- T2 package 検証（fmt/check/lint/test）
- T3 証拠記録（coverage map + verification log）
- T4 docs チェーン・index・レビュー・PR

## Verification

- [x] `mise run fmt-check` — clean
- [x] `mise run check ruviform-ui-test` — clean
- [x] `mise run lint ruviform-ui-test` — clippy `-D warnings`
  clean
- [x] `mise run test ruviform-ui-test` — 全 test target pass
  （新規 4 件含む）
- [x] `evidence/rf243/` の対応表・実行記録が spec の受入
  条件を満たす

## Open Questions / Gaps

- なし（doc graph router は既存 repo-wide の broken
  relation により blocked を返すが、本スライスの文書は
  個別 skill の scaffold + 手動 front matter で作成済み。
  制約は PR 本文に記録）

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（単一 worker、
  依存鎖が直列の小規模スライス）
- Independent work streams: none
- User approval required before dispatch: no（是正タスク・
  受入条件確定済み。review-protocol レビューを承認相当
  として実施）
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- reviewer focus: 「既存被覆を重複追加していないか」
  「stale Window/Receipt の主張が delivery 層 drop と
  runtime routing を正しく分離しているか」「NotRun を
  Pass に置き換えていないか」
