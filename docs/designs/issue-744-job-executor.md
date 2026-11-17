---
id: "DESIGN-034Z78w2daJkBjWWj2OXIP"
type: "design"
status: "approved"
title: "Issue #744 JobExecutor 実装設計 — bounded queue・協調 cancel・result joining"
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
    - "docs/specs/issue-744-job-executor.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-727-bounded-nonrt-job-executor.md"
    - "docs/adr/issue-727-bounded-nonrt-job-executor-placement.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-744-job-executor.md"
    - "docs/plans/issue-744-job-executor.md"
    - "docs/test-specs/issue-744-job-executor.md"
  refines:
    - "docs/designs/issue-727-bounded-nonrt-job-executor.md"
  refined-by: []
  derives-from:
    - "docs/designs/issue-727-bounded-nonrt-job-executor.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/specs/issue-744-job-executor.md"
    - "docs/designs/issue-727-bounded-nonrt-job-executor.md"
    - "crates/ruviform-exec/src/job.rs"
    - "crates/ruviform-exec/src/offline.rs"
  defers: []
  deferred-by: []
---

# Issue #744 JobExecutor 実装設計 — bounded queue・協調 cancel・result joining

## Context

Issue #727 の design doc
（`docs/designs/issue-727-bounded-nonrt-job-executor.md`）が
`ExecutorConfig`/`JobExecutor`/`QueuedJob`/`SubmitError`/
`ExecutorError` の公開 API 契約を approved として確定し、
ADR（`docs/adr/issue-727-bounded-nonrt-job-executor-placement.md`、
accepted）が配置を `ruviform-exec` への module 追加と決裁した。
本設計はその契約を `crates/ruviform-exec` の実コードへ落とす
内部設計であり、公開契約自体は変更しない。

既存資産: `job.rs` の `OfflineJob`（observe surface・
`JobScope`・`JobState`/`JobEvent`/`JobOutcome`/`JobError`）、
`offline.rs` の `CancellationToken`/`Progress`/`OfflineDriver`。

## Scope

- `src/executor.rs` 新規: 公開 5 型 + 内部 `Inner`/`QueueState`/
  `worker_loop`/`run_job` + `#[cfg(test)]` 単体テスト。
- `src/job.rs`: `JobState::Queued`・`JobError::Cancelled` の
  additive variant、state 定数と `state_from_u8`/`request_cancel`
  の `pub(crate)` 共有化、`JobScope::new` の `pub(crate)` 追加。
- `src/lib.rs`: `pub mod executor` + re-export + crate doc 更新。
- `README.md`: 責務記述更新。

## Components and Boundaries

### 公開 API（#727 契約どおり）

- `ExecutorConfig { workers, queue_capacity }` — 両者 `>= 1`、
  構築時固定の `Copy` 値。
- `JobExecutor` — `new`/`submit`/`try_submit`/`shutdown`。
  `Send + Sync`、clone なし（`Arc` 経由で共有・`shutdown` 呼出）。
- `QueuedJob<T, E>` — `OfflineJob` と同一の observe/cancel/join
  surface。相違点は pool worker 実行・`JobState::Queued` の
  報告・queued cancel 時の `JobError::Cancelled` のみ。
- `SubmitError { QueueFull, Shutdown }`、`ExecutorError {
  InvalidConfig }` — ともに `#[non_exhaustive]`。

### 内部構造

- `Inner` — `Mutex<QueueState>`（`VecDeque<JobTask>` +
  `shutdown` flag）・`Condvar has_job`/`has_space`・`capacity`・
  `Mutex<HashMap<u64, CancellationToken>> live`・
  `AtomicU64 next_job_id`・`Mutex<Vec<JoinHandle>> workers`。
- `JobTask = Box<dyn FnOnce() + Send>` — type-erased job。
  `run_job` が cancel check → body（`catch_unwind`）→ result
  publish → state 遷移 → event → live 除去を担う。
- bounded 性: `queue.jobs.len() <= capacity` を mutex 内で
  維持。in-flight 上限 = `workers + queue_capacity`（running
  worker 数 + queued 数）。

### queue 実装の選択

`Mutex<VecDeque> + Condvar` を採用。`sync_channel` 案は
blocking `submit` が満杯時に parked したまま `shutdown` で
起床できず、worker 側の shutdown flag 確認も queue とは別経路に
なる。単一 mutex で queue と shutdown flag を直列化する方が
「push された task は必ず `live` 登録済みで cancel 可能」の
不変条件を一箇所で保証できる。

## Data and Control Flow

- submit: surface 構築（events/result channel・token・state・
  progress listener）→ `live` 登録 → `enqueue`（queue mutex
  内で shutdown/容量判定 → push → `has_job.notify_one`）。
  reject 時は `live` 除去 + task drop（body は走らない）。
- worker: queue mutex で pop（空なら `has_job.wait`、shutdown
  なら drain 後 exit）→ `run_job` → `has_space.notify_one`
  （pop 時に 1 回 — queue slot が空いたことを blocking submit
  へ通知）。
- `run_job`: queued cancel 済みなら body skip →
  `JobError::Cancelled`。それ以外は `Started` 送信 → body を
  `catch_unwind` → outcome 分類（cancelled なら `Cancelled`、
  panic なら `Panicked`）→ **result を state=Finished より先に
  送信**（`try_wait` が finished job の result を必ず観測できる
  ようにする順序契約）→ `Finished` event（best-effort）→
  `live` 除去。
- shutdown: queue.shutdown=true + `has_job`/`has_space`
  notify_all → `live` 全 token cancel → workers を drain して
  join（guard 保持で並行 shutdown 呼出を直列化）。Drop =
  shutdown。

## Non-Functional Constraints

- **RT 分離**: 全公開型 `non-RT only`。spawn/join・mutex/
  condvar・channel のみ、`std` のみ（新規依存ゼロ）。
- **deadlock contract**: blocking `submit` を pool worker 上の
  job body から呼ぶと全 worker が満杯 queue を待ち deadlock
  し得る — 型・doc 双方に明記し、body 内投入は `try_submit`。
  `shutdown`/`Drop` を job body 内から呼ぶ場合も自己 join で
  deadlock する旨を doc に明記。
- **協調 cancel のみ**: 強制 preempt なし。token を観測しない
  body は `shutdown`/`wait` を待たせ続ける（`std::thread` join
  と同じ正直さ）。
- **poison 許容**: mutex は `PoisonError::into_inner` で回復
  （`OfflineJob` と同じ方針 — panic しても pool を止めない）。

## Risks and Trade-offs

- `JobState::Queued`/`JobError::Cancelled` の additive 変更は
  `#[non_exhaustive]` により下流互換だが、`JobState`/`JobError`
  を exhaustive に扱う内部コードがあれば影響 — dependent
  crate の `check` で実測確認し PR に記録。
- `live` bookkeeping は shutdown cancel のためだけの内部構造で
  あり product registry ではない — job 完了時に必ず除去し、
  増殖しないことを実装で保証。
- blocking `submit` の parked 中に `Drop`/`shutdown` が来た
  場合は `has_space.notify_all` で `SubmitError::Shutdown` を
  返して起床 — 未処理の parked submitter が残らないことを
  テストで確認。

## Deferred Design Concerns

- worker 動的 scaling・優先度・複数 pool — YAGNI（#727 design
  Deferred と同一）。
- 強制 preemption — 別途 ADR が必要（#727 design Deferred）。
- `OfflineJob` との内部 helper 更なる共通化 — `pub(crate)`
  関数共有に留め、公開統合は行わない。

## Verification Notes

- `mise run check/lint/test ruviform-exec` + `fmt-check`。
- `job.rs` 変更のため dependent crate（ruviform-audio/
  ruviform-cli-sample/ruviform-ui/ruviform-viz）の `check`。
- テスト観点は `docs/test-specs/issue-744-job-executor.md`。
