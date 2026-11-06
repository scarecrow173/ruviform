---
id: "PLAN-034T2Nmh2tErwGOV4X6guz"
type: "plan"
status: "approved"
title: "Issue #289 OBS-07 Collector/Metrics/Snapshot 実装計画"
created: "2026-11-06"
updated: "2026-11-06"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/289"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-289-observability-collector-metrics.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-277-observability-core-source-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/shared-foundations-and-modularization-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-289-observability-collector-metrics.md"
    - "docs/designs/issue-289-observability-collector-metrics.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-289-observability-collector-metrics-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/277"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/278"
    - "https://github.com/scarecrow173/ruviform/issues/291"
  deferred-by: []
---

# Issue #289 OBS-07 Collector/Metrics/Snapshot 実装計画 Implementation Plan

## Goal

`ruviform-observability` クレート内に、OBS-02 登録ポートを実装する
`Collector` を `src/collector/`・`src/metrics/` に実装し、Counter/Gauge/
Histogram の Baseline・実測 Delta/Rate・不連続・drain・有界履歴・
`MetricsSnapshot`・`CollectorHealth` を Logging/Tracing 非依存で提供する。

## Scope

単一実装ストリーム（`ruviform-observability` 内のみ）。`src/metrics/`
（値語彙）と `src/collector/`（poll エンジン）を新設し、既存
`core`/`source`/`record`/`rt_bridge` は最小の `pub(crate)` アクセサ追加
のみで回帰させない。OBS-03/09/11 の Exporter・Diagnostics・Host runner
接続は本計画の責務外。

## Upstream Documents

- `docs/specs/issue-289-observability-collector-metrics.md`（approved）
- `docs/specs/issue-277-observability-core-source-contract.md`（approved）
- `docs/designs/issue-289-observability-collector-metrics.md`（approved）
- `docs/designs/shared-foundations-and-modularization-policy.md`（approved）

## Design Inputs

- `docs/designs/issue-289-observability-collector-metrics.md`
  - 所有権構造（`Collector` !Clone・`Shared` Mutex+AtomicU8・`Drop=shutdown`）
  - Registry（KeyClaim 永続・bare key・cell ptr live-only・descriptor 検査）
  - Poll 単一 lock 区間 + commit 時 `lifecycle()` 再読
  - Measurement/Validity（`cumulative_since_registration`・forced/abandoned
    `Partial`・`MeasurementIssue` 4 variant・`checked_duration_since`）
  - Window/Batch/Snapshot（`latest_batch`・世代共有・`PollError`）
  - History（FIFO ring・oversized 拒否・`history_*=0`=disabled）
  - Health（`lifecycle_errors_total` 等・`AtomicU8` 状態・非再帰）
  - `CollectorConfigError`（ゼロ値 6 variant・`SeriesBelowSources`）

## File Map

- `crates/ruviform-observability/src/metrics/mod.rs` - `Measurement` /
  `MetricValue` / `MeasurementValidity` / `MeasurementIssue` / re-export
- `crates/ruviform-observability/src/collector/mod.rs` - 公開 re-export +
  crate 内 `Shared` 定義
- `crates/ruviform-observability/src/collector/config.rs` -
  `CollectorConfig` / `CollectorConfigError`
- `crates/ruviform-observability/src/collector/window.rs` -
  `ObservationWindow`
- `crates/ruviform-observability/src/collector/batch.rs` -
  `ObservationBatch`（`Arc` 共有・`snapshot()` accessor）
- `crates/ruviform-observability/src/collector/snapshot.rs` -
  `MetricsSnapshot`（Arc 共有）
- `crates/ruviform-observability/src/collector/health.rs` -
  `CollectorHealth` / `CollectorState`
- `crates/ruviform-observability/src/collector/history.rs` - 有界 ring
- `crates/ruviform-observability/src/collector/registry.rs` - `Registry` /
  `SourceEntry` / `KeyClaim` / `retained_removed` / 登録検査
- `crates/ruviform-observability/src/collector/collector.rs` -
  `Collector` / `CollectorRegistrar` / `CollectorHandle` / `PollRunner` /
  `PollError`
- `crates/ruviform-observability/src/collector/tests.rs` - unit test
  （contract 試験・`#[cfg(test)]` cell 補助を使う）
- `crates/ruviform-observability/src/core/counter.rs` -
  `shared_cell_ptr` `pub(crate)` + `#[cfg(test)]` cell constructor
- `crates/ruviform-observability/src/core/gauge.rs` -
  `shared_cell_ptr` `pub(crate)`
- `crates/ruviform-observability/src/core/histogram.rs` -
  `shared_cell_ptr` `pub(crate)` + `boundaries()` 既存公開の再利用
- `crates/ruviform-observability/src/source/metric_source.rs` -
  `MetricReader::shared_cell_ptr` `pub(crate)`
- `crates/ruviform-observability/src/lib.rs` - `mod metrics`/`mod
  collector` 登録 + crate doc（RT 契約表・モジュール一覧）更新
- `crates/ruviform-observability/Cargo.toml` - 変更なし（新規依存なし）

## Implementation Sequence

- [ ] `src/metrics/` の値語彙を実装（`Measurement`/`MetricValue`/
      `MeasurementValidity`/`MeasurementIssue`。全 enum `#[non_exhaustive]`）。
- [ ] `src/collector/config.rs`・`window.rs`・`batch.rs`・`snapshot.rs`・
      `health.rs`・`history.rs` の値型を実装（`CollectorConfigError`・
      `ObservationWindow`・`ObservationBatch`・`MetricsSnapshot`・
      `CollectorHealth`・`CollectorState`・有界 ring）。
- [ ] `src/collector/registry.rs` の `Registry`/`SourceEntry`/`KeyClaim`/
      `retained_removed` を実装（登録検査・KeyConflict/InvalidDescriptor/
      SourceReused・cell ptr claim・bare key 一意性・
      `request_retire`/`begin_final_drain`/`mark_removed` 呼出し）。
- [ ] `src/core/{counter,gauge,histogram}.rs` に `shared_cell_ptr`
      `pub(crate)` を追加し、`metric_source.rs` の `MetricReader` に
      委譲する。`counter.rs` に `#[cfg(test)]` cell constructor を追加。
- [ ] `src/collector/collector.rs` に `Collector`/`CollectorRegistrar`/
      `CollectorHandle`/`PollRunner`/`PollError` を実装（`!Clone`・
      `Drop=shutdown`・`AtomicU8` CAS・`Mutex<State>` 単一 lock 区間・
      commit 時 `lifecycle()` 再読・`checked_duration_since`・
      `request_retire()` 戻り値で forced drain 判定・`latest_batch`）。
- [ ] `lib.rs` に `mod metrics`/`mod collector` を登録し、crate doc
      （モジュール一覧・RT 契約表）を更新。
- [ ] `src/collector/tests.rs` に contract 試験を実装（Baseline/Rate/
      不連続/再登録/drain/gauge/histogram/window/stale/partial/
      unavailable/unsupported/logging-off/同時 poll 型保証/競合/容量/
      履歴 evict/世代 gap/shutdown poison）。
- [ ] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` を pass。
- [ ] `origin/main` を merge・生成 index 再生成・最終検証・コミット・
      push・日本語 PR。

## Dependencies and Risks

- `MetricReader::shared_cell_ptr` の実装には `Arc::as_ptr` の比較が必要。
  `Counter`/`Gauge` の cell は `Arc<AtomicU64>`、`Histogram` は
  `Arc<HistogramShared>` なので型は統一できない → `*const ()` または
  `usize` で返すか、`MetricReader` 側で kind 別の比較を行う（実装時に
  `pub(crate)` の最小形を決める）。
- `Instant` の `checked_duration_since` が全ての測定経路（初回
  `registered_at`、連続 `observed_at`、window `started`）で使われる点は
  `now < anchor` の全分岐を網羅する必要がある（test で固定）。
- `AtomicU8` の collector 状態と `Mutex<State>` の二相化は、`Draining`
  を lock 不要で観測可能にするため。`register` は atomic 先行読み +
  lock 下再検査で race をなくす（design で確定）。
- `Drop for Collector` は `shutdown()` であり `PollRunner` の thread 内
  drop でも発火するため、thread 内で `Collector` を `move` する closure
  の終了順序に注意（thread 関数を抜けた時点で drop される）。
- `poison_recoveries_total` を実際に計数するには全 `lock()` 呼出を
  `unwrap_or_else` で wrap する共通化が必要（`Shared::lock` 補助を
  `pub(crate)` で一本化する）。
- `#[cfg(test)]` cell constructor は `tests/` 統合試験から見えないため、
  contract 試験は `src/collector/tests.rs`（crate 内 unit test）に置く
  （test-spec の「試験コードは crate 内 unit test」に対応）。

## Task Breakdown

- Task-1: `src/metrics/` + `src/collector` の値型（config/window/batch/
  snapshot/health/history）を実装し、`mise run check` で compile 確認。
- Task-2: `src/collector/registry.rs` + core reader の `shared_cell_ptr`/
  cfg(test) cell constructor を実装。
- Task-3: `src/collector/collector.rs`（`Collector`/handles/`PollRunner`/
  `PollError`）を実装し、`mise run check` で compile 確認。
- Task-4: `src/collector/tests.rs` に全 contract 試験を実装し、
  `mise run test` で pass 確認。
- Task-5: lib.rs doc 更新・`mise run fmt-check`/`check`/`lint`/`test`/
  `test-rt`・`origin/main` merge・PR。

## Verification

- [ ] `mise run fmt-check` が pass（`rustfmt --check` 相当）
- [ ] `mise run check` が pass（`cargo check --workspace --all-targets` 相当）
- [ ] `mise run lint` が pass（`clippy -D warnings` 相当）
- [ ] `mise run test` が pass（`cargo test --workspace` 相当）
- [ ] `mise run test-rt` が pass（既存 RT 試験・`rt` feature 有効時の
      回帰なし）
- [ ] `docs/specs|designs|plans|test-specs/README.md` index が再生成される
- [ ] `origin/main` merge 後に `mise run check`/`lint`/`test` が再 pass
- [ ] golden 試験: Baseline/Rate/不連続/再登録/drain/gauge/histogram/
      window/stale/partial/unavailable/unsupported/logging-off/同時
      poll 型保証/競合/容量/履歴 evict/世代 gap/shutdown/poison
- [ ] 既存 `crates/ruviform-observability` の test が無改変で pass

## Open Questions / Gaps

- `shared_cell_ptr` の返却型（`*const ()` vs `usize` vs 内部 token）は
  実装時に `pub(crate)` の最小形で確定する（公開 API には出さない）。
- `PollRunner::stop` の戻り値は `thread::Result<()>` を維持する
  （thread panic は join で検出）。
- `Instant` 逆行時に `observed_at`/`last_change_at` をどう進めるかは
  `ClockAnomaly` の扱いで確定（design で確定済み・test で固定）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（本 worker が実装する）
- Independent work streams: なし（単一実装ストリーム）
- User approval required before dispatch: no（実装自体が本タスク）
- If approved: 上記 Task-1〜Task-5 を順に実装し、各 Task 完了時に
  `mise run check` で compile 確認する。
- If declined or unavailable: 同上（inline）。

## Review Handoff

- 設計の独立レビューは完了済み（Reviewer A/B → Validator → Judge、
  verdict: approve-after-fixes・全 fix 適用済み）。実装レビューは
  実装完了後に同じプロトコルで実施する。
- 注視点: `Shared` Mutex の poison 回復・`Drop` での shutdown 冪等性・
  `latest_batch` の `Stopped` 後参照・`PollRunner` thread 内 drop。
