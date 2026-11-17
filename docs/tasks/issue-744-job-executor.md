---
id: "TASK-034Z78J47Pz01PXhvix6fI"
type: "task"
status: "completed"
title: "Issue #744 JobExecutor 実装 — task"
created: "2026-11-17"
updated: "2026-11-17"
owners:
  - "devin-worker"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/744"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-exec/src/executor.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-exec/src/job.rs"
      - type: "file"
        path: "crates/ruviform-exec/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-exec/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-744-job-executor.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-744-job-executor.md"
    - "docs/designs/issue-744-job-executor.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-744-job-executor.md"
    - "docs/impl/ir/issue-744-job-executor.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/test-specs/issue-744-job-executor.md"
  verified-by: []
  references:
    - "docs/plans/issue-744-job-executor.md"
    - "docs/designs/issue-744-job-executor.md"
  defers: []
  deferred-by: []
---

# Issue #744 JobExecutor 実装 — task

## Work

- T1 `job.rs` 拡張: `JobState::Queued`・`JobError::Cancelled`
  variant、state 定数/`state_from_u8`/`request_cancel` の
  `pub(crate)` 共有、`JobScope::new` 追加。
- T2 `executor.rs` 実装: `ExecutorConfig`/`JobExecutor`/
  `QueuedJob`/`SubmitError`/`ExecutorError`、内部 `Inner`/
  `QueueState`/`worker_loop`/`run_job`。
- T3 export/doc: `lib.rs` module 宣言・re-export・crate doc、
  `README.md` 責務記述。
- T4 テスト: bounded・cancel・join・shutdown・panic 分離・
  `OfflineJob` 併存の単体テスト（`executor.rs` 内 `#[cfg(test)]`）。
- T5 検証: `mise run check/lint/test ruviform-exec`、
  `fmt-check`、dependent crate `check`、`prek run --files`。
- T6 docs/evidence: docs chain 記入、`evidence/rf744/` 記録、
  review-protocol。
- T7 commit/PR: Conventional Commits で論理単位に分割、
  push、`gh pr create`（日本語本文・検証証拠記載）。

## Classification

impl/foundation — #727 ADR 決裁の実装 slice。consumer 配線
（synvelis#518）は範囲外。

## Upstream Documents

- plan: `docs/plans/issue-744-job-executor.md`
- spec: `docs/specs/issue-744-job-executor.md`
- design: `docs/designs/issue-744-job-executor.md`
- test-spec: `docs/test-specs/issue-744-job-executor.md`

## Dependencies

- blockedBy `scarecrow173/ruviform#727`（CLOSED — API 契約・
  配置決裁済み）。
- consumer `scarecrow173/synvelis#518` は本 slice 完了後に
  `RenderExecutor` port へ注入（別 Issue の範囲）。

## Affected Files

- `crates/ruviform-exec/src/executor.rs`（新規）
- `crates/ruviform-exec/src/job.rs`（additive variant + pub(crate)）
- `crates/ruviform-exec/src/lib.rs`・`README.md`
- `docs/**/issue-744-*`、`evidence/rf744/`

## Done When

- 受入条件 4 項目（spec 参照）を全て満たし、正規検証が
  pass、review-protocol で blocker なし、PR が作成されている。

## Verification

`mise run check ruviform-exec` / `lint ruviform-exec` /
`test ruviform-exec` / `fmt-check` + dependent crate `check`
（job.rs 変更のため）+ `mise exec -- prek run --files`。
