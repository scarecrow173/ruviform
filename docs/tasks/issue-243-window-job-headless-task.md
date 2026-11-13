---
id: "TASK-034WZGuoovHD3cTIYBWUGN"
type: "task"
status: "done"
title: "Issue #243 — Window/Job 不変条件 Headless 検証 実装タスク"
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
        path: "docs/impl/ir/issue-243-window-job-headless.md"
      - type: "file"
        path: "docs/impl/exp/issue-243-window-job-headless.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-243-window-job-headless-plan.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-243-window-job-headless-plan.md"
    - "docs/designs/issue-243-window-job-headless-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-243-window-job-headless.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-243-window-job-headless-plan.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-243-window-job-headless.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #243 — Window/Job 不変条件 Headless 検証 実装タスク

計画 `PLAN-034WZGtEYdaixOYopRPVvk` のスライスを実施順の
タスクに分けたもの。

## Work

`window_job_headless.rs`（`UiRuntime` + `FakeDriver` 合成
の Headless Scenario テスト 4 件）・`evidence/rf243/`
証拠・docs チェーン・PR（`Refs #243`）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-243-window-job-headless-plan.md`
- `docs/specs/issue-243-window-job-headless.md`
- `docs/designs/issue-243-window-job-headless-design.md`
- `docs/test-specs/issue-243-window-job-headless.md`

## Dependencies

- `UiRuntime` の `dispatch_window_intent`/`drain_jobs_routed`/
  `cancel_window_jobs`/`dispatch_op`（#181/#386/#456 merged）
- `FakeDriver`/`deliver_routed`/`DriverEvent` 語彙
  （#386 merged）
- `DemoService` の `demo.sleep`/`demo.echo`/`job.cancel`/
  `sys.ping`
- 先行 slice `issue-243-windows-multiwindow-smoke`
  （実 OS 証拠、本スライスの参照元）

## Affected Files

- `crates/ruviform-ui-test/tests/window_job_headless.rs`（新規）
- `evidence/rf243/window-job-headless-coverage.md`（新規）
- `evidence/rf243/window-job-headless-verification.md`（新規）
- `docs/{specs,designs,plans,tasks,test-specs,impl/ir,impl/exp}/issue-243-window-job-headless*.md`

## Tasks

- [x] T1 既存 Window/Job API・Fake/Headless 基盤・被覆状況の
  棚卸し（`multi_window.rs`/`fake_driver.rs`/`driver_
  contract.rs`/`lifecycle_fixture.rs`/`e2e_process.rs`/
  `ipc_conformance.rs`/`scenario_contract.rs`/`dock.rs`/
  `native_slot_contract.rs`/`driver_parity.rs`）。
- [x] T2 未被覆不変条件の特定と `window_job_headless.rs`
  実装（Harness + 4 テスト: cancel scoping・close policy・
  orphan job terminal routing・stale id cleanup）。
- [x] T3 `mise run fmt-check`/`check`/`lint`/`test`
  `ruviform-ui-test` 検証（全 lane exit 0）。
- [x] T4 `evidence/rf243/` 被覆対応表・実行記録・NotRun
  残件の作成。
- [x] T5 docs チェーン + index 再生成、review-protocol
  レビュー、PR 作成（`Refs #243`、Issue は Open 継続）。

## Done When

- [x] 4 テストが `mise run test ruviform-ui-test` で pass
- [x] 被覆対応表・実行記録・NotRun 残件が `evidence/rf243/`
  に存在
- [x] fmt/check/lint/test が pass
- [x] docs チェーン完備・index 再生成済み

## Verification

- [x] `mise run fmt-check` — exit 0
- [x] `mise run check ruviform-ui-test` — exit 0
- [x] `mise run lint ruviform-ui-test` — exit 0
- [x] `mise run test ruviform-ui-test` — exit 0（新規 4 件
  含む全 target pass）

## Upstream

- `docs/test-specs/issue-243-window-job-headless.md`
