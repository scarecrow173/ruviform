---
id: "SPEC-034Z78vorbxe5h4YnZofGo"
type: "spec"
status: "approved"
title: "Issue #744 bounded 非 RT JobExecutor 実装（ruviform-exec executor module）"
created: "2026-11-17"
updated: "2026-11-17"
owners:
  - "devin-worker"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/744"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-727-bounded-nonrt-job-executor.md"
    - "docs/adr/issue-727-bounded-nonrt-job-executor-placement.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-744-job-executor.md"
    - "docs/designs/issue-744-job-executor.md"
    - "docs/test-specs/issue-744-job-executor.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-727-bounded-nonrt-job-executor.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/synvelis/issues/518"
    - "docs/designs/issue-727-bounded-nonrt-job-executor.md"
  defers: []
  deferred-by: []
---

# Issue #744 bounded 非 RT JobExecutor 実装（ruviform-exec executor module）

## What Should Be Built

`crates/ruviform-exec` に `executor` module を追加し、
Issue #727 で approved となった API 契約どおりの bounded
非 RT `JobExecutor` primitive を実装する。`ExecutorConfig`/
`JobExecutor`/`QueuedJob`/`SubmitError`/`ExecutorError` を
`lib.rs` から export し、`OfflineJob` と同一の
observe/cancel/join surface を pool worker 上で提供する。
あわせて `lib.rs`・`README.md` の「executor API は持たない」
責務記述を実態に整合させる。

## Why Now

Issue #727 で API 契約と配置（ruviform-exec への module
追加、accepted ADR）が決裁済みであり、consumer
（`scarecrow173/synvelis#518` の `RenderExecutor` port 注入）
が本 primitive を待っている。headless で完結する実装 slice
として分離可能。

## Users and Value

- synvelis（`scarecrow173/synvelis#518`）: render job を
  bounded に投入・観測・cancel・join する基盤 primitive を
  `RenderExecutor` port の実装として注入できる。
- ruviform 自体: `OfflineJob`（1 job = 1 thread）と
  `OfflineDriver::run_parallel`（scoped worker）に続く第 3 の
  実行形態として、共有 bounded pool を提供する。

## Scope

### In Scope

- `crates/ruviform-exec/src/executor.rs` 新規:
  `ExecutorConfig`/`JobExecutor`/`QueuedJob`/`SubmitError`/
  `ExecutorError` + 内部 queue/worker 実装 + 単体テスト。
- `crates/ruviform-exec/src/job.rs`: `JobState::Queued`・
  `JobError::Cancelled` の additive variant 追加、state
  encoding/const の `pub(crate)` 共有化（queued 表現に必要な
  場合のみ — 本 slice では必要と判断）。
- `crates/ruviform-exec/src/lib.rs`: module 宣言・re-export・
  crate doc の責務記述更新。
- `crates/ruviform-exec/README.md`: 責務記述の更新。
- Issue #744 の docs chain（discovery/specs/designs/plans/
  tasks/test-specs/impl ir+exp）と `evidence/rf744/`。

### Out of Scope

- API 契約そのものの変更（必要なら別 Issue で ADR 修正）。
- `offline.rs`/`thread_priority.rs`/`Cargo.toml` の変更
  （新規依存ゼロ前提）。
- Job Registry・job ID・永続化・retry/resume（consumer 責務）。
- 強制 preemption・worker 動的 scaling・優先度/複数 pool
  （YAGNI — #727 design の Deferred）。
- `OfflineJob` の pool 化・公開 API の統合。
- consumer 配線（synvelis#518 の `RenderExecutor` 実装注入）。
- docs index（`docs/**/README.md`）の編集・再生成
  （Coordinator が統合時に実施）。

## Requirements

- `workers + queue_capacity` が in-flight 上限（bounded）。
- `submit`（blocking back-pressure）と `try_submit`
  （fail-closed `SubmitError::QueueFull`/`Shutdown`）を提供。
- `QueuedJob` は `OfflineJob` と同一の observe surface
  （`state`/`is_finished`/`cancel`/`cancel_token`/`progress`/
  `events`/`try_wait`/`wait`/`detach`）を持ち、queued と
  running を区別可能。
- 協調 cancel のみ: queued job は body 未実行で skip、
  running job は token 観測で unwind。強制 preempt なし。
- job body の panic は `JobError::Panicked` に写像し pool
  worker は継続する。
- `shutdown`（= `Drop`）は新規 submit 拒否 + live job 全件
  cancel + queue drain + worker join を deadlock なく行う。
- 全公開型に `**RT contract:** non-RT only` を付与。
  公開 enum は `#[non_exhaustive]`。
- blocking `submit` を pool worker 上の job body から呼ぶと
  deadlock し得る旨を contract/doc に明記（body 内投入は
  `try_submit`）。

## Acceptance Criteria

- [x] `crates/ruviform-exec` に design doc の API 契約どおりの
  executor module が追加され、`lib.rs` から export される
- [x] bounded 性（queue_capacity 超過で `SubmitError` 系の
  fail-closed）・協調 cancel・join が単体テストで検証される
- [x] `OfflineJob` との併存で observe surface の一貫性が
  保たれる（`JobState`/`JobEvent`/`JobOutcome`/`JobError`/
  `JobScope` を再利用、既存 `OfflineJob` テスト無変更で pass）
- [x] `mise run test ruviform-exec` と変更が及ぶ crate の
  正規検証が pass する

## Deferred Design Concerns

- worker 動的 scaling・優先度 queue・複数 pool — consumer
  要求が出た時点で再検討（#727 design Deferred と同一）。
- 任意 Rust コードの強制 preemption — 安全に提供不能、
  別途 ADR が必要（#727 design Deferred と同一）。
- 内部 helper（state encoding・cancel 遷移・event 組立）の
  `OfflineJob`/`QueuedJob` 間共通化の更なる整理 — 現状は
  `pub(crate)` 関数共有に留め、公開 API 統合は行わない。

## Sources and Relations

- Issue: `scarecrow173/ruviform#744`（本 slice）
- API 契約正本: `docs/designs/issue-727-bounded-nonrt-job-executor.md`
- 配置決裁: `docs/adr/issue-727-bounded-nonrt-job-executor-placement.md`
- consumer: `scarecrow173/synvelis#518`（`RenderExecutor` port —
  配線は consumer 側 Issue の範囲）
