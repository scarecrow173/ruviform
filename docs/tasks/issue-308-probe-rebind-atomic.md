---
id: "TASK-034Ui3oOaxCn3ZMrQTqhed"
type: "task"
status: "done"
title: "Issue #308 — Probe再Binding全件整合の実装タスク"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/308"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-308-probe-rebind-atomic.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #308 — Probe再Binding全件整合の実装タスク

## Work

`crates/ruviform-audio/src/typed/probe.rs` の `on_publish` を staged all-or-nothing rebind へ改修し、`typed_probe.rs` に検証を追加する。plan T1–T3 を直列実行。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-308-probe-rebind-atomic.md`
- `docs/specs/issue-308-probe-rebind-atomic.md`
- `docs/designs/issue-308-probe-rebind-atomic.md`
- `docs/test-specs/issue-308-probe-rebind-atomic.md`

## Dependencies

- Issue #185 `typed::probe` 実装（worktree 内に存在）。外部 blocker なし。

## Affected Files

- `crates/ruviform-audio/src/typed/probe.rs`
- `crates/ruviform-audio/tests/typed_probe.rs`
- `docs/impl/ir/issue-308-probe-rebind-atomic.md`（実装 record）

## Done When

- [x] `RebindBegin`/`RebindStage`/`RebindCommit` が generation+staging epoch で fail-closed に適用され、commit が `mem::swap` で atomic
- [x] `on_publish` が `Result<RebindReceipt, ProbeError>` を返し、失敗時は Commit 無しの inert prefix のみ残る
- [x] `StatusKind::RebindDropped`・`StopReason::GenerationBoundary`・`CapturePolicy`/`set_capture_policy` が実装され doc と一致
- [x] queue 飽和点・連続切替・unresolved・Capture boundary・Golden のテストが RED→GREEN
- [x] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` 全緑

## Verification

- [x] `mise run check` `mise run lint` `mise run fmt-check` `mise run test` `mise run test-rt`
- [x] `git merge origin/main` 後の再検証
