---
id: "IMPL-034Z78UHtD5xUTmbZ2lpQh"
type: "impl"
status: "completed"
title: "Issue #744 JobExecutor 実装記録"
created: "2026-11-17"
updated: "2026-11-17"
owners:
  - "devin-worker"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/744"
    - "https://github.com/scarecrow173/ruviform/issues/727"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-exec/src/executor.rs"
      - type: "doc"
        path: "docs/discovery/issue-744-job-executor.md"
      - type: "doc"
        path: "docs/specs/issue-744-job-executor.md"
      - type: "doc"
        path: "docs/designs/issue-744-job-executor.md"
      - type: "doc"
        path: "docs/plans/issue-744-job-executor.md"
      - type: "doc"
        path: "docs/tasks/issue-744-job-executor.md"
      - type: "doc"
        path: "docs/test-specs/issue-744-job-executor.md"
      - type: "doc"
        path: "docs/impl/ir/issue-744-job-executor.md"
      - type: "doc"
        path: "docs/impl/exp/issue-744-job-executor.md"
      - type: "file"
        path: "docs/impl/exp/issue-744-job-executor-exp.jsonl"
      - type: "file"
        path: "evidence/rf744/"
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
    - "docs/tasks/issue-744-job-executor.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/exp/issue-744-job-executor.md"
    - "evidence/rf744/verification.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/test-specs/issue-744-job-executor.md"
  verified-by:
    - "evidence/rf744/verification.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/744"
    - "docs/specs/issue-744-job-executor.md"
    - "docs/designs/issue-744-job-executor.md"
    - "docs/test-specs/issue-744-job-executor.md"
    - "evidence/rf744/verification.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-744-job-executor-exp.jsonl"
    rejected: []
---

# Issue #744 JobExecutor 実装記録

## Summary

`scarecrow173/ruviform#744` の実装 slice。#727 approved
API 契約どおりの bounded 非 RT `JobExecutor` を
`crates/ruviform-exec` の `executor` module として実装し、
`lib.rs` export・`README.md` 責務記述を更新した。queued
状態の表現として `JobState::Queued`、queued cancel の結果と
して `JobError::Cancelled` を additive に追加し、
`OfflineJob` の observe surface を `QueuedJob` で再利用した。

## Context

- spec: `docs/specs/issue-744-job-executor.md`（approved）
- design: `docs/designs/issue-744-job-executor.md`（approved）
- plan: `docs/plans/issue-744-job-executor.md`（approved）
- test-spec: `docs/test-specs/issue-744-job-executor.md`
  （approved）
- task: `docs/tasks/issue-744-job-executor.md`
- API 契約正本: `docs/designs/issue-727-bounded-nonrt-job-executor.md`
- 配置決裁: `docs/adr/issue-727-bounded-nonrt-job-executor-placement.md`
- Issue: `scarecrow173/ruviform#744`（consumer は
  `scarecrow173/synvelis#518` — 配線は範囲外）

## Implementation

### Changes

- `crates/ruviform-exec/src/executor.rs`（新規）—
  `ExecutorConfig`/`JobExecutor`/`QueuedJob`/`SubmitError`/
  `ExecutorError` + 内部 `Inner`/`QueueState`/`worker_loop`/
  `run_job` + 14 件の `#[cfg(test)]` 単体テスト。
- `crates/ruviform-exec/src/job.rs` — `JobState::Queued`・
  `JobError::Cancelled` variant 追加、state 定数と
  `state_from_u8`/`request_cancel` の `pub(crate)` 共有化、
  `JobScope::new` の `pub(crate)` 追加。`OfflineJob` の公開
  挙動は不変。
- `crates/ruviform-exec/src/lib.rs` — `pub mod executor` +
  5 型の re-export + crate doc の責務記述更新。
- `crates/ruviform-exec/README.md` — executor 責務の追記と
  stale 記述（executor 非提供）の修正。
- `docs/{discovery,specs,designs,plans,tasks,test-specs,impl}/
  issue-744-*` — Issue #744 の内部文書チェーン。
- `evidence/rf744/` — 検証証拠。

### Decisions During Implementation

- **`Mutex<VecDeque> + Condvar` queue を採用**: `sync_channel`
  は blocking `submit` が満杯時に parked したまま shutdown で
  起床できない。単一 mutex で queue と shutdown flag を
  直列化し、「push された task は必ず `live` 登録済み」の
  不変条件を一箇所で保証した。
- **`JobState::Queued` を追加**: #727 契約の「queued と実行中を
  区別可能」要件を直接満たす。`#[non_exhaustive]` のため
  additive 安全。`OfflineJob` は `Queued` を報告しない。
- **`JobError::Cancelled` を追加**: queued cancel で body が
  走らない job の `wait()` は `T`/`E` を捏造できないため、
  正直な結果 variant が必要。`OfflineJob` は到達不能。
- **result → state の publish 順序**: `run_job` は result を
  `result_tx` へ送ってから `Finished` を store する。
  `try_wait` が finished job の result を必ず観測できる
  ようにする順序契約。
- **live bookkeeping は内部限定**: `Mutex<HashMap<u64,
  CancellationToken>>` は shutdown 時の一括 cancel のためだけ
  に使い、job 完了時に除去。product Job Registry ではない。
- **worker panic の二重隔離**: body panic は `catch_unwind` で
  `JobError::Panicked` に写像。さらに task 全体を
  `catch_unwind` で包み、delivery path 自体の panic でも
  pool worker が死なないようにした。
- **並行 shutdown の直列化**: `Inner::shutdown` は workers
  mutex の guard を join 中も保持し、2 つ目の呼出が worker
  完了前に return しないようにした。
- **deadlock contract の明文化**: blocking `submit` を pool
  worker 上の job body から呼ぶと deadlock し得る旨、および
  `shutdown`/`Drop` の job body 内呼出も自己 join で
  deadlock する旨を型・doc に明記。
- **docs index 未同期のまま**: `docs/*/README.md` は
  Coordinator が統合時に再生成する運用のため本 slice では
  一切触れない。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-744-job-executor-exp.jsonl`（+
  `issue-744-job-executor.md`）— queue 実装選択・queued
  状態/結果表現・publish 順序・shutdown 直列化の観察/判断/
  検証記録。

### Rejected

- `sync_channel<JobTask>` queue — blocking submit が shutdown
  で起床しないため却下。
- queued job を `Running` と報告 — 契約の区別要件を満たさ
  ないため却下。
- queued cancel を `JobError::Operation` で表現 — `E` の捏造が
  必要で契約不適合のため却下。
- `OfflineJob` の pool 化・公開 API 統合 — #727 ADR の却下
  根拠どおり範囲外。

## Validation

canonical 検証（`evidence/rf744/verification.md` に一覧）:

- `mise run check ruviform-exec` — Pass（exit 0）
- `mise run lint ruviform-exec` — Pass（exit 0、
  `clippy --locked --all-targets -p ruviform-exec -- -D
  warnings`）
- `mise run test ruviform-exec` — Pass（exit 0、unit 39 +
  doctest 9）
- `mise run fmt-check` — Pass（exit 0）
- `mise run check` × dependent（ruviform-audio/
  ruviform-cli-sample/ruviform-ui/ruviform-viz）— 全て Pass
  （exit 0。`JobState`/`JobError` の additive 変更に互換）
- `mise exec -- prek run --files <変更ファイル>` — Pass
- review-protocol self-review — blocker なし

NotRun: `*-msrv`・`*-full`・3 OS 横断・実機/native/GUI
（POLICIES.md §5・Issue #744「実機/native gate: 不要」
により対象外、daily CI 委譲）。Unsupported: なし。

## Risks

- `JobState::Queued`/`JobError::Cancelled` の additive 変更
  が下流の exhaustive match を壊す可能性 — `#[non_exhaustive]`
  で crate 外 match は wildcard 必須のため理論上安全だが、
  dependent 4 crate の `check` で実測 pass を確認済み。
- blocking `submit` の parked 中に executor が Drop される
  経路 — `has_space.notify_all` で `Shutdown` を返して
  起床する設計。テストで shutdown 後 submit 拒否を確認済み
  だが、parked submitter 自体の起床は実装レビューで確認
  （condvar wait は shutdown flag 設定 + notify_all で
  必ず再評価される）。
- timing 依存テストの flake — blocker/gate・`poll_until`・
  FIFO 保証で決定的に制御。

## Follow-ups

- Coordinator による docs index（`docs/*/README.md`）再生成。
- PR merge 後の Issue #744 close は Coordinator が実施。
- consumer 配線（`RenderExecutor` port への `JobExecutor`
  注入）は `scarecrow173/synvelis#518` の範囲。
- 強制 preemption・動的 scaling・優先度 pool は consumer
  要求が出た時点で別 Issue/ADR。
