---
id: "IMPL-034T3aKqPmXvNzRwYsUeGh"
type: "impl"
status: "completed"
title: "Issue #289 OBS-07 Collector・Metrics・履歴・Snapshot（単一クレート）"
created: "2026-11-06"
updated: "2026-11-06"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/289"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-observability/src/metrics/mod.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/collector/mod.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/collector/config.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/collector/health.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/collector/history.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/collector/registry.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/collector/engine.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/collector/window.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/collector/batch.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/collector/snapshot.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/collector/tests.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-observability/src/core/counter.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/core/gauge.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/core/histogram.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/core/mod.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/source/metric_source.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/lib.rs"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/implement-issue-289-observability-collector-metrics.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-289-observability-collector-metrics.md"
    - "docs/tasks/implement-issue-289-observability-collector-metrics.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-289-observability-collector-metrics.md"
    - "docs/designs/issue-289-observability-collector-metrics.md"
    - "docs/plans/implement-issue-289-observability-collector-metrics.md"
    - "docs/test-specs/issue-289-observability-collector-metrics-tests.md"
    - "docs/designs/issue-277-observability-core-source-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-289-observability-collector-metrics.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/277"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/278"
    - "https://github.com/scarecrow173/ruviform/issues/291"
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #289 OBS-07 Collector・Metrics・履歴・Snapshot（単一クレート）

## Summary

`ruviform-observability` に OBS-07 の authoritative 非 RT 観測層を
実装した。`metrics` モジュール（型付き `Measurement`/`MetricValue`/
`MeasurementValidity`/`MeasurementIssue`）と `collector` モジュール
（単一 poll owner の `Collector`、OBS-02 `ObservationRegistrar`
ポート実装の `CollectorRegistrar`、共有読取の `CollectorHandle`、
バックグラウンド補助 `PollRunner`）を追加した。Counter の
baseline/実測 window/rate/discontinuity、Gauge の 0/Unavailable
区別、固定 bucket Histogram、Active→Retiring→FinalDrain→Removed の
最終 drain 保全、有界 history/snapshot/health を一経路で提供する。
Logging/Tracing 依存・`tracing::Dispatch`・新規外部依存なし。
RT producer は collector の lock/alloc/IO に一切触れない。

## Context

- 実施タスク:
  docs/tasks/implement-issue-289-observability-collector-metrics.md
- 上流: spec（要求・AC）・design（所有権・単一 lock・lifecycle・
  validity 優先位・claim・履歴/health 契約）・test-spec（証拠行列）
  — 独立 Reviewer A/B→Validator→Judge で approve-after-fixes を経て
  R1–R12 全解決済み。
- 動機: 複数 Exporter/GUI/Diagnostics が各自 Counter 差分を算出する
  重複を排除し、Logging/Tracing OFF でも参照できる Snapshot・有界
  時系列を単一の非 RT poll で提供する（OBS-03 #278・OBS-09 #291・
  cross-repo #281 の下流契約）。

## Implementation

### Changes

- `src/metrics/mod.rs` — `Measurement`（`value: Option<MetricValue>`;
  Unavailable/Unsupported は `None` で 0 偽装しない）、
  `MetricValue`（Counter: raw/cumulative/delta/rate、Gauge:
  current/previous、Histogram: bucket counts/approx total）、
  `MeasurementValidity`（Valid/Stale/Partial/Discontinuous/
  Unavailable/Unsupported）、`MeasurementIssue`（discontinuity/
  elapsed・clock 問題・WriterNotQuiesced・histogram 不整合等の
  型付き reason）。全公開 enum `#[non_exhaustive]`。
- `src/collector/config.rs` — `CollectorConfig`/`CollectorConfigError`。
  非ゼロ poll interval・source/series/batch 上限・stale 閾値・
  `max_series >= max_sources` を検証。
- `src/collector/registry.rs` — `Registry`/`SourceEntry`/`KeyClaim`/
  `retained_removed`。SourceId mint・generation mint（再登録で
  単調増加）・key claim は登録意図寿命まで永続・shared-cell
  pointer claim で同一 cell の再登録を `SourceReused` として拒否・
  tombstone は snapshot 用に有界保持。
- `src/collector/engine.rs` — `Collector`（`!Clone`・
  `poll(&mut self, now) -> ObservationBatch` で単一 poll owner を
  型で保証）、`CollectorRegistrar`（`ObservationRegistrar` 実装・
  Clone/Arc 可能）、`CollectorHandle`（snapshot/latest_batch/
  series/history/health/shutdown の共有読取）、`PollRunner`
  （tracing 発行なしの汎用定期 poll 補助）、`PollError`。poll 中
  enumerate→sample→commit の lock 保持下で lock-free retire を
  再検出し同 poll 内 drain（design の mid-poll retire 契約）。
  shutdown/drain_all/`Drop` は Draining→Stopped で冪等。
- `src/collector/history.rs` — samples 数 + 概算 bytes の二重上限の
  bounded ring。古いものから evict。oversized 個別 entry は
  収容せず診断カウンタに記録。
- `src/collector/health.rs` — `CollectorHealth`（registered/
  removed/abandoned・polls/skipped・samples・history_evicted・
  oversized_dropped・discontinuity・lifecycle_errors・
  poison_recoveries 等）。logging 非依存。
- `src/collector/window.rs` — `ObservationWindow`（generation・
  planned/observed/drained・started/ended/measured elapsed・
  `ClockDomainId::MONOTONIC`）。generation 1 の起点は collector
  生成時、以降は前 batch の ended。遅延 poll は予定周期ではなく
  実測幅を記録。
- `src/collector/batch.rs` — `ObservationBatch`（generation 付き・
  Arc 共有・window・measurements・removed・snapshot 参照）。
- `src/collector/snapshot.rs` — `MetricsSnapshot`（tracked series 全件
  の最新 measurement + retained removed tombstone）。poll 後・
  shutdown 後も readable。
- core 各 reader — `pub(crate) fn shared_cell_ptr()`（cell claim 用
  生アドレス）。counter は `#[cfg(test)]` cell constructor
  `counter_reader_on_cell`（`core/mod.rs` で `pub(crate)` 再 export）
  を追加し、公開 API は increment-only のまま維持。
- `src/source/metric_source.rs` — `MetricReader::shared_cell_ptr`
  （kind 別 exhaustive dispatch）。
- `src/lib.rs` — `pub mod collector`/`pub mod metrics`・crate doc に
  モジュール要約と non-RT 契約（collector の全操作は lock/alloc する
  非 RT、registry lock は RT writer path で保持しない）を追記。
- docs — spec/design/plan/test-spec/task 作成・全 README index 更新。

### Decisions During Implementation

- 単一 poll owner は型で保証する選択を維持した（`Collector` は
  `!Clone`・`poll(&mut self)`。同時 poll はコンパイル時に不可能で、
  runtime flag を持たない）。
- `ObservationRegistrar::register` は `RegistrationLease` のみを返す
  OBS-02 契約どおりで、identity は `lease.identity()` から取る。
  collector 側で独自の戻り値は作らない。
- `observe_entry` は kind ごとの関数に分割（clippy `too_many_lines`
  対応）し、reader/series pairing 違反は登録時検証で到達不能な
  `unreachable!`（same-crate match は exhaustive、wildcard なし）。
- Counter 減少は巨大 delta を作らず `Discontinuous` + issue 付きで
  delta/rate を抑止し次回 baseline を再基準化（不確定な wrap/reset
  は推測せず typed で表現）。
- forced/abandoned drain は quiescence 未確認として
  `Partial`+`WriterNotQuiesced`。domain-retire 済みの最終 drain は
  `Valid`（design R-系解決どおり validity 優先位を分離）。
- eviction 会計は `observe_and_commit` 内で一括して数え、`poll`
  側の追加加算を除去した（二重計上による
  `history_evicted_total` 過剰を修正。history 長自体は常に正しく
  上限内だった）。
- poison recovery は `PoisonError::into_inner` で復旧し
  `poison_recoveries_total` に計上。poison 以後は lock 毎に
  recovery が発生するため計上は lock-recovery event 単位
  （design/Validator 確認どおり）。
- mutex poison を通常 API で起こせないため
  `#[cfg(test)] poison_state_for_test`（guard 保持中に panic）を
  追加して recovery 経路を実測。
- rate は表示級 `f64`（delta/measured elapsed）で
  `#[expect(clippy::cast_precision_loss)]` を局所適用（既存
  codebase の cast 規約に一致）。

## Related Experiments

### Adopted

なし（実験ログは作成していない）。

### Rejected

なし。

## Validation

| 検証 | 実行 | 結果 |
| --- | --- | --- |
| `mise run fmt-check` (`cargo fmt --all -- --check`) | 実行 | Pass |
| `mise run check` (`cargo check --workspace --all-targets`) | 実行 | Pass |
| `mise run lint` (`cargo clippy --workspace --all-targets -- -D warnings`) | 実行 | Pass |
| `mise run test` (`cargo test --workspace`) | 実行 | Pass — 全 test target 緑 |
| `mise run test-rt` (rt_ フィルタレーン) | 実行 | Pass — observability 17 件含む全 lane 緑 |
| `cargo test -p ruviform-observability` | 実行 | Pass — unit 104 + doctest 6（補助証拠） |
| `cargo check -p ruviform-observability --features logging` | 実行 | Pass — logging feature 両立 |
| `cargo check -p ruviform-observability --no-default-features` | 実行 | Pass — logging OFF でも collector 独立 |

実施環境: Windows 11 x64 / Git Bash / Rust 1.99.0 (mise pin)・MSRV
1.89 準拠コード・unsafe なし・新規外部依存なし。

主な pinned 証拠（test-spec の証拠行列に対応）:

- Counter baseline/rate/実測 window、減少時 Discontinuous で
  delta/rate 抑止、再登録 generation 再基準化。
- Gauge current/previous、0 と Unavailable の区別、stale 閾値。
- 固定 bucket Histogram の bucket clone・approx total・形状不整合
  issue。
- Active→Retiring→FinalDrain→Removed の最終 drain・lease drop
  abandoned drain・forced shutdown drain（Partial+WriterNotQuiesced）
  ・mid-poll lock-free retire の同 poll drain・tombstone 保持。
- `latest_batch() -> Option`・generation 連結 batch/snapshot・
  `max_batch_size` round-robin 公平性・drain は active 予算を
  消費しない。
- key/cell 重複登録拒否（`KeyConflict`/`SourceReused`）、
  容量上限、有界 history（samples+bytes）、oversized drop 計上、
  mutex poison recovery、同時 register/poll 競合の決定性試験、
  `PollRunner` 定期 poll・stop・handle 共有。
- logging feature 無し build で snapshot/health が利用可能
  （collector は logging/tracing 型に一切依存しない構造保証）。

## Risks

- 残リスク: Linux/macOS での実測は未実施（NotRun — Windows 11 x64
  のみ）。cross-repo Consumer gate（#281 AudioLens/Synvelis 実接続）
  は OBS-11 / #281 の責務で本 PR では not run。
- `ObservationBatch`/`MetricsSnapshot` は非 RT 所有物であり RT
  thread で保持・読み取りしない（crate doc の RT 契約どおり）。
- Histogram bucket 読取は複数 atomic の非同時観測で、トランザクション
  整合は保証しない（契約で明示）。
- Rollback: `metrics/`・`collector/` 2 ディレクトリ・core/source の
  `pub(crate)` アクセサ・lib.rs export/doc を除去すれば完全に元に
  戻る（既存公開 API 非変更・追加のみ）。

## Follow-ups

- OBS-03(#278): Collector の `ObservationBatch`/`MetricsSnapshot` から
  tracing 変換する consumer（本 Issue の非対象）。
- OBS-09(#291): `CollectorHealth` を diagnostics 面へ接続。
- OBS-11(#293)/#281: AudioLens/Synvelis 実接続・cross-repo 検証 lane。
- OBS-08(#290): RT producer 計測（collector 観測と独立）。
