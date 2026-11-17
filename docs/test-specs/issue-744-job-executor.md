---
id: "TSPEC-034Z78HXuZljkt8u95PZgA"
type: "test-spec"
status: "approved"
title: "Issue #744 JobExecutor 実装 テスト仕様"
created: "2026-11-17"
updated: "2026-11-17"
owners:
  - "devin-worker"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/744"
  changes: {}
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-744-job-executor.md"
    - "docs/designs/issue-744-job-executor.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-744-job-executor.md"
    - "docs/impl/ir/issue-744-job-executor.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/issue-744-job-executor.md"
  verified-by: []
  references:
    - "docs/specs/issue-744-job-executor.md"
    - "docs/designs/issue-744-job-executor.md"
  defers: []
  deferred-by: []
---

# Issue #744 JobExecutor 実装 テスト仕様

## Purpose

`executor.rs` の `#[cfg(test)]` 単体テストが #727 API 契約の
bounded 性・協調 cancel・join・shutdown・panic 分離を
外部観測可能な振る舞いで検証する規範。

## Feature

- bounded queue: `workers + queue_capacity` が in-flight 上限。
- submit 系: `submit` blocking back-pressure / `try_submit`
  fail-closed。
- observe surface: `OfflineJob` と同一（state/progress/events/
  try_wait/wait/detach）。
- cancel: queued skip（`JobError::Cancelled`）・running
  cooperative unwind・Drop 起因 cancel・wait 非 cancel。
- shutdown: submit 拒否・live job cancel・worker join・冪等・
  Drop 同等。
- panic: body panic → `JobError::Panicked`・pool 継続。

## Rules

- R1: `workers=0`/`queue_capacity=0` は `ExecutorError::
  InvalidConfig`。
- R2: `try_submit` は queue 満杯で `SubmitError::QueueFull`、
  shutdown 後は `SubmitError::Shutdown`。
- R3: `submit` は満杯時にブロックし、空きが出るか shutdown
  で `Shutdown` を返す。
- R4: queued job は `state()==JobState::Queued` を報告し、
  worker が拾うと `Running` 以降へ遷移する。
- R5: queued cancel は body を実行せず `JobError::Cancelled`、
  `Started` event を出さず `Finished{Cancelled}` を出す。
- R6: running cancel は body の協調観測で unwind し、
  `JobError::Operation(E)` + `JobOutcome::Cancelled`。
- R7: body panic は `JobError::Panicked` + `JobOutcome::Failed`、
  worker は次の job を処理する。
- R8: `wait` は結果を一度だけ返し cancel を要求しない。
  `try_wait` は finished 後に `Some`、消費後は `None`。
- R9: `shutdown`/Drop は submit を `Shutdown` で拒否し、
  live job 全件へ協調 cancel を送り worker を join する。
  冪等。
- R10: `detach` は cancel せず job を完走させる。
- R11: handle Drop は cancel を要求する（`OfflineJob` と同じ）。
- R12: `QueuedJob` の events は bounded/lossy・channel-close
  が終端 marker（`OfflineJob` と同一契約）。

## Examples

- `submit(|scope| Ok(42))` → `wait()==Ok(42)`、events は
  `Started` → （progress）→ `Finished{Completed}`。
- workers=1,cap=1 + blocker で queue 満杯 → `try_submit` が
  `QueueFull`。
- queued victim に `cancel()` → worker が skip →
  `wait()==Err(JobError::Cancelled)`、body 未実行。
- panicking body の後に正常 job を submit → pool が継続し
  `Ok` を返す。

## Guarantees

- result は `Finished` state より先に channel へ publish され、
  `try_wait` が finished job の result を必ず観測できる。
- shutdown は queue を drain してから worker を join するため
  deadlock しない（協調 cancel に応答する job に限る）。
- `JobExecutor` は `Send + Sync`、`QueuedJob` は `Send`
  （`OfflineJob` と同じく `Receiver` 保持のため `Sync` では
  ない）。

## Non-goals

- 強制 preemption・優先度・動的 scaling・registry/永続化・
  retry — consumer/別 Issue の責務。
- `OfflineJob` の pool 化・公開 API 統合。
- 実機/native/GUI gate — headless で完結（Issue #744 明記）。

## Risk

- timing 依存テストの flake — blocker/gate・`poll_until` で
  決定的に制御し、queued 状態の assert は FIFO と worker 占有で
  確定させる。
- `JobState::Queued`/`JobError::Cancelled` の下流影響 —
  dependent crate `check` で実測（`#[non_exhaustive]` で
  additive 安全の想定を検証）。
