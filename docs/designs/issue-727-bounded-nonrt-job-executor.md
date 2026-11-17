---
id: "DESIGN-034Z0tS0DTi5T7v5ov5cWH"
type: "design"
status: "approved"
title: "Issue #727 bounded 非 RT job executor — ruviform-exec 内 executor primitive の API 設計"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/727"
    - "https://github.com/scarecrow173/synvelis/issues/518"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-727-job-executor.md"
      - type: "doc"
        path: "docs/designs/issue-727-bounded-nonrt-job-executor.md"
      - type: "doc"
        path: "docs/adr/issue-727-bounded-nonrt-job-executor-placement.md"
      - type: "doc"
        path: "docs/plans/issue-727-job-executor.md"
      - type: "doc"
        path: "docs/tasks/issue-727-job-executor.md"
      - type: "doc"
        path: "docs/test-specs/issue-727-job-executor.md"
      - type: "doc"
        path: "docs/impl/ir/issue-727-job-executor.md"
      - type: "doc"
        path: "docs/impl/exp/issue-727-job-executor.md"
      - type: "file"
        path: "docs/impl/exp/issue-727-job-executor-exp.jsonl"
      - type: "file"
        path: "evidence/rf727/"
    modified:
      - type: "file"
        path: "docs/designs/shared-foundations-and-modularization-policy.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-727-job-executor.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-727-job-executor.md"
    - "docs/adr/issue-727-bounded-nonrt-job-executor-placement.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/designs/shared-foundations-and-modularization-policy.md"
    - "docs/designs/offline-execution-driver.md"
    - "docs/designs/issue-155-headless-cli-foundation.md"
    - "docs/adr/synvelis-driven-generic-execution-boundaries.md"
    - "docs/adr/issue-727-bounded-nonrt-job-executor-placement.md"
    - "docs/plans/issue-727-job-executor.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-727-job-executor.md"
    - "docs/adr/issue-727-bounded-nonrt-job-executor-placement.md"
  derived-by:
    - "docs/plans/issue-727-job-executor.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-727-job-executor.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/727"
    - "https://github.com/scarecrow173/synvelis/issues/518"
    - "docs/specs/issue-727-job-executor.md"
    - "docs/adr/issue-727-bounded-nonrt-job-executor-placement.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/designs/shared-foundations-and-modularization-policy.md"
    - "docs/adr/synvelis-driven-generic-execution-boundaries.md"
    - "crates/ruviform-exec/src/lib.rs"
    - "crates/ruviform-exec/src/job.rs"
    - "crates/ruviform-exec/src/offline.rs"
    - "crates/ruviform-exec/README.md"
  defers: []
  deferred-by: []
---

# Issue #727 bounded 非 RT job executor — ruviform-exec 内 executor primitive の API 設計

## Context

synvelis#518 は production job system の配線として async
scheduler・preemption・結果 join を必要とするが、Ruviform 側
には bounded な非 RT job executor primitive が存在しない。
`crates/ruviform-exec` は以下を提供するが、複数 job を bounded
に受け付ける executor / scheduler は意図的に非提供である
（`src/lib.rs` 冒頭の責務記述、`src/job.rs` の「registry /
executor / 永続化はアプリ責務」明記、`README.md` の同旨記述）。

- `OfflineJob<T, E>`: 1 job = 1 専用 OS thread の薄い接着層。
  `spawn`/`spawn_with_cancel_after`/`state`/`is_finished`/
  `cancel`/`cancel_token`/`progress`/`events`/`try_wait`/
  `wait`/`detach` を提供。lifecycle は
  `Running → Cancelling → Finished`、cancel は協調、
  event は back-pressure で lossy、state/progress polling が
  authoritative。
- `OfflineDriver::run`/`run_parallel`: 供給された work を
  chunk 分割して逐次/並列駆動。`run_parallel` は呼び出しごとに
  `std::thread::scope` の scoped worker を生成し、永続 pool を
  持たない。bounded `sync_channel` queue で back-pressure を
  提供し、独立 chunk 前提で deterministic な出力順序を保証する
  （stateful DSP への無条件拡張はしない — synvelis-driven 実行
  境界 ADR）。
- `CancellationToken`（`Arc<AtomicBool>`）と `Progress`
  （`Arc` atomics + listener）: job / driver 双方が共有する
  協調 cancel・進捗 primitive。

配置判断は ADR
`docs/adr/issue-727-bounded-nonrt-job-executor-placement.md`
で「Ruviform 側・`ruviform-exec` 内モジュール追加」と確定した。
本設計はその API 契約を定義する。実装コードは本 slice の
対象外であり、採用時の実装 Issue で確定する。

## Scope

### In Scope

- `ruviform-exec` に追加する bounded 非 RT job executor の
  公開 API 契約（型・メソッド・エラー・lifecycle）
- bounded 性の保証内容（worker 数・queue 容量・in-flight 上限・
  job 予算の扱い）
- cancel / preemption 意味論（協調 cancel の範囲と、強制
  中断を提供しないことの明示）
- 結果 join / observe の契約（`OfflineJob` との surface 整合）
- RT オーディオ経路との分離契約
- 既存 `OfflineJob` / `OfflineDriver` との関係整理
- consumer（synvelis#518）が要求する最小 API への対応付け

### Out of Scope

- executor の実装コード・内部構造の最終決定（queue の具象
  型・worker loop・result 配送の内部機構は実装 Issue で確定）
- Job Registry・job ID・永続化・retry/resume・manifest・
  exit code・storage quota・user-facing scheduling（consumer
  責務 — synvelis-driven 実行境界 ADR Decision 6）
- 優先度付き scheduling・複数 pool・動的 worker 数・work
  stealing（YAGNI — consumer 要求が出た時点で再検討）
- 強制 preemption（任意 Rust コードの安全な強制中断は Rust の
  実行モデル上提供不能。unsafe 強制割り込みは非対象）
- RT オーディオスレッド向け scheduler（rf#316 の領域）
- queue の byte-size 上限（job は opaque であり、容量は件数で
  bound する。byte budget が必要なら consumer 側 policy）

## Components and Boundaries

`ruviform-exec` に新規 module（仮称 `executor`）を追加し、
以下の公開面を提供する。命名は実装 Issue で確定するが、契約
として以下を固定する。

### `ExecutorConfig`

```rust
pub struct ExecutorConfig {
    /// Fixed worker thread count. Must be >= 1.
    pub workers: usize,
    /// Maximum number of submitted-but-not-yet-started jobs
    /// waiting in the queue. Must be >= 1.
    pub queue_capacity: usize,
}
```

- worker 数は構築時固定（動的 scaling なし）。
- `queue_capacity` は「worker が dequeue する前の待機 job 数」
  の上限。`sync_channel` 相当の bounded channel で実現し、
  channel が満杯の間は blocking submit が待機し、
  `try_submit` は `QueueFull` を返す。
- in-flight 上限 = `workers + queue_capacity`。これを超える
  job は存在しない（submit が成功した時点で必ずいずれかに
  属する）。

### `JobExecutor`

```rust
pub struct JobExecutor { /* workers + bounded queue + live-job bookkeeping */ }
```

- `Send + Sync`。consumer は `Arc` で共有するか registry に
  保持する。clone は提供しない（所有権を一意にし、shutdown
  semantics を単純化する）。
- 内部に live job の `CancellationToken` 参照を保持し、
  shutdown 時に全 outstanding job へ cancel を伝播できる
  ようにする（内部 bookkeeping であり、product Job Registry
  ではない）。

```rust
impl JobExecutor {
    pub fn new(config: ExecutorConfig) -> Result<Self, ExecutorError>;
    pub fn submit<F, T, E>(&self, operation: F)
        -> Result<QueuedJob<T, E>, SubmitError>
    where
        F: FnOnce(&JobScope) -> Result<T, E> + Send + 'static,
        T: Send + 'static,
        E: Send + 'static;
    pub fn try_submit<F, T, E>(&self, operation: F)
        -> Result<QueuedJob<T, E>, SubmitError>
    where /* same bounds */;
    pub fn shutdown(&self);
}
```

- `submit`: queue に空きができるまで block する
  back-pressure 付き投入。queue 満杯時に呼び出し側 thread を
  待機させるため、**executor 自身の worker 上で動く job body
  から `submit` を呼ぶと deadlock し得る**。この制約は
  contract として明記し、job body 内からの投入が必要な
  consumer は `try_submit` を使う。
- `try_submit`: 空きがなければ即座に
  `SubmitError::QueueFull` を返す。event loop / GUI thread
  からの非同期投入経路として使う。
- `shutdown(&self)`: 新規 submit を `SubmitError::Shutdown` で
  拒否し、outstanding 全 job（queued + running）に協調 cancel
  を要求した上で、queue が空になり全 worker が終了するまで
  block して join する。`Arc` 経由でも呼び出せる。
  **cancel を観測しない job が残ると shutdown は返らない**
  （`std::thread` の join 意味論と同じ正直さ — 強制中断しない
  契約の裏返し）。
- `Drop`: `shutdown` と同じ振る舞い（明示 shutdown 忘れでも
  worker thread を leak させない。最後の `Arc` 参照が drop
  された時点で実行される）。

### `QueuedJob<T, E>`

`OfflineJob<T, E>` と同一の observe/cancel/join surface を持つ
handle。違いは「専用 thread ではなく pool worker 上で実行される」
点のみ。

```rust
impl<T, E> QueuedJob<T, E> {
    pub fn state(&self) -> JobState;
    pub fn is_finished(&self) -> bool;
    pub fn cancel(&self);
    pub fn cancel_token(&self) -> &CancellationToken;
    pub fn progress(&self) -> &Progress;
    pub fn events(&self) -> &Receiver<JobEvent>;
    pub fn try_wait(&mut self) -> Option<Result<T, JobError<E>>>;
    pub fn wait(self) -> Result<T, JobError<E>>;
    pub fn detach(self);
}
```

- `state`: `JobState`（`Running`/`Cancelling`/`Finished`）を
  polling する。queued 中の job を `Running` として報告するか、
  `JobState` に `Queued` variant を追加するかは実装 Issue で
  確定する。契約上の要求は「consumer が queued と実行中を
  区別できること」であり、`JobState` は `#[non_exhaustive]`
  enum のため variant 追加は範囲内で可能。ただし既存
  `OfflineJob` の `JobState` 3値契約を変える場合は全 consumer
  への影響を実装 Issue で評価する。
- `events`: `OfflineJob` と同じ bounded channel・lossy 配信・
  channel-close = terminal marker の契約。
- `wait`/`try_wait`/`detach`: `OfflineJob` と同じ意味論。
  `wait` は cancel を要求しない。`detach` は handle を捨てて
  job を pool 上で完走させる（executor shutdown 時は cancel
  対象になる点のみ異なる）。
- `Drop`（handle）: `OfflineJob` と同じく cancel 要求 + detach
  （pool worker の join は executor が管理するため、handle drop
  で worker を直接 join しない）。

### エラー型

```rust
#[non_exhaustive]
pub enum ExecutorError {
    /// workers == 0 or queue_capacity == 0
    InvalidConfig,
}

#[non_exhaustive]
pub enum SubmitError {
    /// try_submit on a full queue
    QueueFull,
    /// executor is shut down or dropped
    Shutdown,
}
```

job body の失敗は `JobError<E>`（`Operation(E)` /
`Panicked`）に既存型を再利用する。executor 層固有の失敗
（queue full・shutdown・config 不正）だけを新規 enum にする。
pub enum は crate 方針どおり全て `#[non_exhaustive]`。

## Data and Control Flow

```
consumer (synvelis job system)
  │  submit / try_submit(closure)
  ▼
JobExecutor ── bounded queue (capacity = queue_capacity)
  │  workers = N fixed OS threads
  ▼
pool worker ── dequeue ──▶ check cancel token
  │                            │ cancelled? → skip body, report Cancelled
  │                            ▼
  │                     run F(&JobScope) under catch_unwind
  │                            │
  │                            ▼
  │                     deliver Result<T, JobError<E>> to QueuedJob
  │                     + JobEvent stream (Started/Progress/Finished)
  ▼
QueuedJob<T,E> handle: state()/events()/try_wait()/wait()/detach()
```

- job body の signature は `OfflineJob::spawn` と同一:
  `FnOnce(&JobScope) -> Result<T, E>`。`JobScope` は
  `cancel_token()`/`progress()`/`is_cancelled()` を提供し、
  body は `OfflineDriver::run`/`run_parallel` に token を
  そのまま渡すか、自前 loop で `is_cancelled()` を poll する。
- result 配送の内部機構（channel / shared cell + condvar 等）
  は実装 Issue で確定するが、契約として「`wait`/`try_wait` が
  `OfflineJob` と同じ `Result<T, JobError<E>>` を一度だけ
  返す」ことを固定する。
- panic は worker 側で `catch_unwind` し `JobError::Panicked`
  に写像する。pool worker は panic 後も次の job を処理する
  （`OfflineJob` の専用 thread と異なり、1 job の panic が
  pool 全体を殺さない — これが pool 化の本質的利点）。

## Non-Functional Constraints

- **RT 分離**: 全公開型に non-RT contract を付与
  （`OfflineJob`/`OfflineDriver` と同じ doc 規約）。executor
  の worker thread は RT audio path と無関係の通常 OS thread
  であり、RT thread から `submit`/`wait` を呼ぶことは禁止
  する（blocking・allocation を伴う）。rf#316 の RT bounded
  scheduling とは領域が異なり、本設計は一切関与しない。
- **bounded 保証**: worker 数・queue 容量・in-flight 上限は
  `ExecutorConfig` で構築時に固定され、実行中に増減しない。
  内部に unbounded buffer を持たない（event channel は
  `OfflineJob` と同じ bounded `sync_channel`）。
- **job 予算**: executor は時間・メモリの job budget を
  強制しない。budget / deadline は consumer が
  `CancellationToken` 経由で実装する（協調 cancel の延長）。
  byte-size queue bound も同様に consumer 責務。
- **依存方向**: `ruviform-exec` の許可依存は Primitives 層
  （`ruviform-math`/`ruviform-rt` 等）のみ。executor は新規
  依存を追加せず、`std::thread`/`std::sync` のみで実装可能
  な範囲に留める（外部 pool crate は採用しない — 依存追加
  ゼロで要件を満たせる）。
- **module-first**: 新規 crate は作らず `ruviform-exec` 内
  module とする。根拠: 同一の依存層・同一の RT contract・
  同一の consumer（synvelis-render は既に `ruviform-exec` に
  依存）であり、独立 compile/OS/配布境界の必要がない。
  shared-foundations の分割判断基準（L299-312）に照らし
  crate 分割トリガに該当しない。

## Risks and Trade-offs

- **協調 cancel の限界**: cancel は body が観測するまで効か
  ない。無限 loop・長時間の非協調区間を持つ job は shutdown
  を block する。→ contract で明示し、consumer（synvelis）
  の job 定義側で checkpoint 義務を負う。強制 preemption は
  別途 ADR が必要な領域として Deferred に記録。
- **submit の deadlock**: blocking `submit` を pool worker 上の
  job body から呼ぶと全 worker が待機して deadlock し得る。
  → contract で禁止を明記し、`try_submit` を併用可能にする。
- **handle drop と result の行方**: `QueuedJob` を drop すると
  cancel + detach で result は破棄される。executor shutdown
  との相互作用（shutting down executor が live job を cancel
  する）は contract で固定する。
- **OfflineJob との API 重複**: observe surface がほぼ同一に
  なる。→ 意図的な整合であり、実装時に共通化（内部 helper
  module）を検討する。公開 API の統合・`OfflineJob` の pool
  化は行わない（`OfflineJob` の「1 job = 1 thread」契約を
  変えると既存 consumer の前提が崩れる）。
- **`JobState` の queued 表現**: queued job を `Running` と
  して報告するか `Queued` variant を足すかは実装 Issue で
  確定。契約上の要求は区別可能であることのみ。

## Deferred Design Concerns

- **executor の実装**: queue の具象型（`sync_channel` vs
  `Mutex<VecDeque>`+`Condvar`）、result 配送機構、worker
  thread naming / priority（既存の OS thread-priority 機構を
  適用するか）、`JobState` への queued 表現追加の要否は
  実装 Issue で確定する。
- **preemption の将来拡張**: 強制中断が必要になった場合は
  別途 ADR で検討（unsafe 強制割り込みは現行方針で非対象）。
- **優先度・複数 pool・動的 worker**: consumer 要求が出た
  時点で再検討（YAGNI）。
- **`OfflineJob` との共通化**: 内部 helper の共通化は実装
  Issue の判断。公開統合は行わない。

## Consumer 最小 API 対応（synvelis#518）

`synvelis/crates/synvelis-render/src/render_job/orchestrator.rs`
（read-only 参照）の要求に対する対応付け:

| synvelis 側の要求 | 本設計の対応 |
| --- | --- |
| `RenderExecutor` port（注入可能な executor） | `JobExecutor` を consumer 側 port の実装として注入。`submit`/`try_submit` が非同期投入経路 |
| 同期 `run()`/`drive()`（現行 orchestrator） | job body 内で `OfflineDriver::run`/`run_parallel` を呼ぶ構造を維持。body signature `FnOnce(&JobScope) -> Result<T,E>` は `OfflineJob::spawn` と同一で移行が容易 |
| `ruviform_exec::CancellationToken`（既使用） | `JobScope::cancel_token()`/`QueuedJob::cancel_token()` が同一型を返すためそのまま利用可能 |
| preemption | `QueuedJob::cancel()` + body 側 checkpoint（`is_cancelled()` poll / token を driver へ伝播）。queued job の cancel で未開始 job の実行自体を skip 可能 |
| 結果 join | `QueuedJob::wait()`/`try_wait()` で `Result<T, JobError<E>>` を取得。`events()` で進捗観測 |
| Job Registry・job ID・永続化 | **consumer 責務**（synvelis-automation / synvelis-app 層）。本 primitive は registry を持たない |

## Verification Notes

本 slice は docs-only であり、executor の実装・動作検証は
存在しない。検証は以下に限定される:

- 本設計が spec の R1–R8 を満たすこと（責務判断・配置・
  API 契約・責務境界・preemption 定義・sv#518 参照・stale
  表記・docs-only）。
- 参照した既存 API（`OfflineJob`/`OfflineDriver`/
  `CancellationToken`/`Progress`/`JobState`/`JobEvent`/
  `JobOutcome`/`JobError`/`JobScope`）が `crates/ruviform-exec`
  の実在シンボルと一致すること。
- `cargo` 系検証は docs-only slice のため NotRun（POLICIES.md
  §5 — 文書のみの変更は軽量チェックが原則）。
