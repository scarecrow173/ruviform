---
id: "PLAN-034Z78H8qbs20QnotU2UIi"
type: "plan"
status: "approved"
title: "Issue #744 JobExecutor 実装計画"
created: "2026-11-17"
updated: "2026-11-17"
owners:
  - "devin-worker"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/744"
    - "https://github.com/scarecrow173/ruviform/issues/727"
  changes: {}
  implements:
    - "docs/specs/issue-744-job-executor.md"
  implemented-by:
    - "docs/tasks/issue-744-job-executor.md"
  depends-on:
    - "docs/designs/issue-727-bounded-nonrt-job-executor.md"
    - "docs/adr/issue-727-bounded-nonrt-job-executor-placement.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-744-job-executor.md"
    - "docs/test-specs/issue-744-job-executor.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-744-job-executor.md"
    - "docs/designs/issue-744-job-executor.md"
  derived-by:
    - "docs/tasks/issue-744-job-executor.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-744-job-executor.md"
  references:
    - "docs/specs/issue-744-job-executor.md"
    - "docs/designs/issue-744-job-executor.md"
    - "docs/designs/issue-727-bounded-nonrt-job-executor.md"
    - "docs/adr/issue-727-bounded-nonrt-job-executor-placement.md"
  defers: []
  deferred-by: []
---

# Issue #744 JobExecutor 実装計画

## Goal

#727 approved API 契約どおりの bounded 非 RT `JobExecutor` を
`crates/ruviform-exec` に実装し、単体テスト・正規検証・
docs chain・evidence を揃えて PR 化する。

## Scope

`crates/ruviform-exec`（executor module + job.rs additive
variant + lib.rs/README 更新）と Issue #744 docs chain、
`evidence/rf744/`。consumer 配線・他 crate・docs index は
範囲外。

## Upstream Documents

- spec: `docs/specs/issue-744-job-executor.md`（approved）
- design: `docs/designs/issue-744-job-executor.md`（approved）
- API 契約正本: `docs/designs/issue-727-bounded-nonrt-job-executor.md`
- 配置決裁: `docs/adr/issue-727-bounded-nonrt-job-executor-placement.md`

## Design Inputs

- queue: `Mutex<VecDeque<JobTask>> + Condvar(has_job/has_space)`
- state: `JobState::Queued` 追加 + `JobError::Cancelled` 追加
- cancel: `request_cancel`/`state_from_u8` を `pub(crate)` 共有
- result 順序: result send → state=Finished（try_wait 保証）

## File Map

| ファイル | 変更 |
| --- | --- |
| `crates/ruviform-exec/src/executor.rs` | 新規（公開 API + 内部実装 + テスト） |
| `crates/ruviform-exec/src/job.rs` | `Queued`/`Cancelled` variant、pub(crate) 共有 |
| `crates/ruviform-exec/src/lib.rs` | module/export/doc 更新 |
| `crates/ruviform-exec/README.md` | 責務記述更新 |
| `docs/{discovery,specs,designs,plans,tasks,test-specs,impl}/issue-744-*` | docs chain |
| `evidence/rf744/` | 検証証拠 |

## Implementation Sequence

1. `job.rs` に `JobState::Queued`・`JobError::Cancelled`・
   `pub(crate)` helper を追加。
2. `executor.rs` を実装（`ExecutorConfig` → `Inner`/`worker_loop`/
   `run_job` → `JobExecutor` → `QueuedJob`）。
3. `lib.rs` export + crate doc、`README.md` 責務記述を更新。
4. `#[cfg(test)]` テストを追加し `mise run test` で反復。
5. `check`/`lint`/`fmt-check` を通し、dependent crate の
   `check` を実行。
6. docs chain 記入・evidence 記録・review-protocol。
7. Conventional Commits で論理単位に分割 commit → push → PR。

## Dependencies and Risks

- blockedBy: `scarecrow173/ruviform#727`（CLOSED/COMPLETED —
  API 契約と配置の決裁済み）。
- リスク: `JobState`/`JobError` additive 変更の下流影響 —
  dependent crate `check` で実測。queued cancel の結果表現 —
  `JobError::Cancelled` で正直に解決済み。

## Task Breakdown

`docs/tasks/issue-744-job-executor.md` を参照（T1: job.rs
拡張 → T2: executor.rs → T3: export/doc → T4: テスト →
T5: 検証 → T6: docs/evidence → T7: commit/PR）。

## Verification

`mise run check ruviform-exec` / `lint ruviform-exec` /
`test ruviform-exec` / `fmt-check` + dependent crate
`check`（job.rs 変更のため）+ `prek run --files`。
NotRun: `*-msrv`/`*-full`/3 OS（daily CI 委譲）。

## Open Questions / Gaps

- なし。

## Self-Review

- spec の全 Requirements が design/task/test-spec に写像
  されていることを確認。契約正本（#727 design）との差分は
  `JobState::Queued`/`JobError::Cancelled` の additive 追加のみで、
  #727 が「実装内で確定」とした queued 表現の範囲内。

## Implementation Handoff

実装者: devin-worker（Orca dispatch ctx_353f87dd0826）。
worktree: `G:/orca/workspaces/ruviform/rf744-job-executor`、
branch: `feature/744-job-executor`。

## Review Handoff

review-protocol による self-review を実施し、blocker が
残っていないことを確認してから PR 化する。
