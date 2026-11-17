---
id: "DISC-034Z78u4glu94S3wActSDq"
type: "discovery"
status: "resolved"
title: "Issue #744 bounded non-RT JobExecutor — 実装 slice の探索・設計確定"
created: "2026-11-17"
updated: "2026-11-17"
owners:
  - "devin-worker"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/744"
    - "https://github.com/scarecrow173/ruviform/issues/727"
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
  related:
    - "docs/designs/issue-727-bounded-nonrt-job-executor.md"
    - "docs/adr/issue-727-bounded-nonrt-job-executor-placement.md"
    - "docs/specs/issue-744-job-executor.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/designs/issue-727-bounded-nonrt-job-executor.md"
    - "docs/adr/issue-727-bounded-nonrt-job-executor-placement.md"
    - "crates/ruviform-exec/src/job.rs"
    - "crates/ruviform-exec/src/offline.rs"
  defers: []
  deferred-by: []
---

# Issue #744 bounded non-RT JobExecutor — 実装 slice の探索・設計確定

## Summary

Issue #727 で approved となった API 契約
（`docs/designs/issue-727-bounded-nonrt-job-executor.md`）と
accepted ADR（`ruviform-exec` への module 追加）を、
`crates/ruviform-exec` の実装として確定する探索 slice。
結論: 既存 `OfflineJob` の observe surface を `QueuedJob` として
再利用し、`JobState::Queued` variant と `JobError::Cancelled`
variant を追加する形で契約を満たせる（confirmed）。

## Exploration Goal

#727 の API 契約を変更せずに `ruviform-exec` へ実装するため、
(a) queued 状態の表現方法、(b) queued cancel 時の結果表現、
(c) bounded queue の同期機構、(d) worker panic/shutdown の
隔離方法を既存コードと契約正本に照合して確定する。

## Key Issues and Assumptions

- **queued と running の区別**: 契約は「consumer が区別できる」
  ことのみ要求。`JobState` は `#[non_exhaustive]` のため
  `Queued` variant 追加は additive で安全（confirmed —
  `job.rs` の enum 属性と exhaustive_enums lint 方針を確認）。
- **queued cancel の結果**: body が走らない job の `wait()` は
  `T`/`E` を捏造できないため `JobError` に `Cancelled` variant
  が必要（confirmed — `JobError<E>` は `#[non_exhaustive]`）。
- **blocking submit の back-pressure**: `sync_channel` は
  「満杯時に blocking send」しか提供せず、shutdown による
  parked submitter の起床を表現できない → `Mutex<VecDeque> +
  Condvar×2` を採用（confirmed — worker 側も同じ mutex で
  shutdown flag と直列化できる）。
- **`OfflineJob` 互換**: observe surface（`state`/`is_finished`/
  `cancel`/`cancel_token`/`progress`/`events`/`try_wait`/`wait`/
  `detach`）を同一 semantics で提供。`OfflineJob` 自体は
  1 job = 1 thread のまま pool 化しない（confirmed — #727 ADR
  の却下根拠を再確認）。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| `JobState::Queued` variant 追加 | consumer が queued/running を区別可能・契約要求を直接満たす | `JobState` の variant 増加（non_exhaustive で additive 安全） | yes |
| queued job を `Running` と報告 | `JobState` 無変更 | 契約の「区別可能」要件を満たさない | no |
| `sync_channel<JobTask>` | 実装が単純 | blocking submit が shutdown で起床しない・queue 長の正確な管理が分散 | no |
| `Mutex<VecDeque> + Condvar` | shutdown との直列化・blocking/try submit の統一・境界の一元化 | mutex/condvar の手管理 | yes |
| queued cancel を `JobError::Operation` で誤魔化す | `JobError` 無変更 | `E` を捏造するか panic するしかなく契約不適合 | no |
| `JobError::Cancelled` variant 追加 | 正直な結果表現・`OfflineJob` には到達不能で影響限定 | `JobError` の variant 増加（non_exhaustive で安全） | yes |

## Tentative Conclusions and Hypotheses

- confirmed: `executor.rs` 新規 module + `lib.rs` export +
  `JobState::Queued`/`JobError::Cancelled` 追加で #727 契約を
  実装できる。`offline.rs`/`thread_priority.rs`/`Cargo.toml` は
  不変で済む（新規依存ゼロ）。
- confirmed: 内部 live-job bookkeeping（job id → CancellationToken）
  は shutdown 時の一括 cancel に必要で、product Job Registry では
  ない — job 完了時に除去し外部へ公開しない。
- hypothesis: dependent crate（ruviform-audio 等）は
  `JobState`/`JobError` を match していなければ additive variant
  で影響なし — `#[non_exhaustive]` により crate 外 match は
  wildcard 必須のはずだが、check で実測確認する。

## Open Questions

- なし（契約正本・配置決裁とも確定済み。残る実装詳細は
  design doc の Deferred 範囲内）。

## Promotion Candidates

- [x] spec-doc needed: `docs/specs/issue-744-job-executor.md`
- [ ] adr-doc needed: 不要 — #727 ADR が配置を決裁済み、
      本 slice は契約内の実装詳細のみ（`JobState::Queued`/
      `JobError::Cancelled` の additive 変更は設計 doc に記録）

## Sources and Relations

- Issue: `scarecrow173/ruviform#744`（本 slice）、
  `scarecrow173/ruviform#727`（API 契約・配置の決裁、CLOSED）
- API 契約正本: `docs/designs/issue-727-bounded-nonrt-job-executor.md`
- 配置決裁: `docs/adr/issue-727-bounded-nonrt-job-executor-placement.md`
- 既存コード: `crates/ruviform-exec/src/{job,offline,lib}.rs`、
  `crates/ruviform-exec/README.md`
- consumer 参照（read-only）: `synvelis/crates/synvelis-render/
  src/render_job/orchestrator.rs`（`RenderExecutor` port —
  `scarecrow173/synvelis#518` の注入先、配線は範囲外）

## Evidence

- 2026-11-17: `job.rs`/`offline.rs`/`lib.rs`/`README.md` 精読 —
  `OfflineJob` の observe surface・`JobScope` 構築・state
  AtomicU8 encoding・`EVENT_CAPACITY`・spawn-armed cancel trigger
  を確認。`lib.rs` L5-7 と `README.md` L5-6 の stale 記述
  （executor 非提供）を特定。
- 2026-11-17: #727 design doc の API 契約節を精読 —
  `ExecutorConfig`/`JobExecutor`/`QueuedJob`/`SubmitError`/
  `ExecutorError` の公開面・bounded 性・協調 cancel・result
  joining・強制 preempt なしを確認。
