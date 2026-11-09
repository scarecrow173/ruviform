---
id: "DESIGN-ZAfKHGovgBPKLRo8KY7Ss7"
type: "design"
status: "approved"
title: "Issue #290 OBS-08 RT Timing — observability 内完結 slice"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/290"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-290-observability-rt-timing.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-290-observability-rt-timing.md"
    - "docs/designs/issue-290-observability-allocation-probe.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-289-observability-collector-metrics.md"
    - "docs/designs/foundation-observability-minimal-api.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-290-observability-rt-timing.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-290-observability-rt-timing-tests.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/275"
    - "https://github.com/scarecrow173/ruviform/issues/280"
    - "https://github.com/scarecrow173/ruviform/issues/289"
    - "https://github.com/scarecrow173/ruviform/issues/281"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/293"
  deferred-by: []
---

# Issue #290 OBS-08 RT Timing — observability 内完結 slice

## Context

Issue #290（OBS-08）は profiling・RT Timing・開発/テスト用
CountingAllocator を扱う。allocation-probe slice は PR #506 で
`src/profiling/` として実装済み。audit の結果、crate 内に残る受入行は
「RT Timing の Clock/overrun/Callback budget 意味論と Histogram の
統合試験」のみである。

既存資産:

- `record`（#277/#280）: `EventRecord`/`RecordHeader`/`ClockContext`/
  `InputStamp`/`ClockDomainId`/`ClockEpochId`/`TimestampProvenance`/
  `EventId`/`SourceIdentity` の固定長 `Copy` 語彙。
- `rt_bridge`（#280）: RT→非 RT の bounded SPSC `RtRecorder<R>`/
  `RecordSink<R>`。`R: Copy + Send + 'static`。drop-on-full +
  `dropped_before` envelope + `dropped()`/`recorded()` カウンタ。
- `core::histogram`（#289 系）: 固定 bucket・atomic counter の
  `Histogram`（非 RT 構築/`observe`）と `HistogramReader`
  （OBS-02 `MetricSource` 登録可能、`MetricValue::Histogram`）。
- `collector`（#289）: 非 RT のみ。poll ごとに `MetricValue` を
  `ObservationBatch` へまとめる。
- `metrics`（#289）: `Measurement`/`MeasurementValidity`/`MetricValue`
  の型付き validity 意味論（`None` vs `0`・anomaly/degraded）。

本 slice はこれらを再利用し、新規 `src/rt_timing/` モジュールとして
実装する。tick の読み取り・budget 値の決定・計測点の選定はドメイン側
（`ruviform-audio`/`ruviform-rt`・各 Consumer）の責務であり、
observability は「計測済み区間の語彙・輸送・集計・報告」のみを持つ。

## Scope

### In Scope

- `TimingClock`: 記録準備時に固定する clock 同一性。
- `TimingSample` / `TimingRecord`: 呼出側入力と bridge レコード。
- `timing_bridge` / `TimingRecorder` / `TimingSink`: 既存
  `rt_bridge` 上の domain endpoint。
- `TimingWindow` / `TimingSnapshot` / `QuantileEstimate`: 非 RT 報告。
- `src/rt_timing/tests.rs`: 契約・意味論・統合試験。

### Out of Scope

- tick reader の concrete adapter、Consumer 組込み、#293 Host 統合。
- 既存モジュール（`rt_bridge`/`record`/`histogram`/`collector`/
  `metrics`/`diagnostics`/`debug`/`profiling`）の変更。
- feature gate・新規外部依存・`unsafe`・`#[global_allocator]`。
- CPU sampling/Tracy 実機検証・cross-repo 回帰（#281/#293）。

## Components and Boundaries

### `record.rs` — 語彙

```rust
/// Bridge record schema version stamped into RecordHeader::version.
pub const TIMING_SCHEMA_VERSION: u16 = 1;

/// Clock identity fixed at preparation time. The recorder stamps every
/// record with this domain/base/provenance/epoch; the sink echoes it in
/// snapshots. Foreign-clock records are unreachable by construction —
/// `TimingRecord` is built only by `TimingRecorder`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TimingClock {
    domain: ClockDomainId,
    base: TimeBase,
    provenance: TimestampProvenance,
    epoch: ClockEpochId,
}

impl TimingClock {
    pub const fn new(
        domain: ClockDomainId,
        base: TimeBase,
        provenance: TimestampProvenance,
        epoch: ClockEpochId,
    ) -> Self;
    // accessors: domain(), base(), provenance(), epoch()
}

/// One measured interval supplied by the caller. `budget_ticks == 0`
/// asserts no callback budget; `end_tick < start_tick` is a clock
/// anomaly and is counted, never interpreted as a duration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimingSample {
    pub point: EventId,        // measurement point identity (domain-allocated)
    pub generation: u64,       // domain-declared generation (plan/stream)
    pub budget_ticks: u64,     // asserted budget in clock ticks; 0 = none
    pub start_tick: u64,       // clock reading at interval start
    pub end_tick: u64,         // clock reading at interval end
}

/// Fixed-size record crossing the bridge (Copy, Send).
/// - header.event = sample.point
/// - header.version = TIMING_SCHEMA_VERSION
/// - header.source / header.generation from bridge/sample
/// - header.clock = ClockContext { position: start_tick, base, domain,
///   provenance } + epoch   (position marks interval START)
/// - header.confidentiality = Internal, header.impact = Normal (fixed;
///   timing metadata is not secret-bearing)
/// - end_tick, budget_ticks as plain fields
#[derive(Clone, Copy, Debug)]
pub struct TimingRecord {
    header: RecordHeader,
    end_tick: u64,
    budget_ticks: u64,
}
```

`TimingRecord` は private field + const accessor で公開し、
`RecordHeader` と同じく「構築は recorder のみ」の語彙とする。
`size_of::<TimingRecord>() <= 128` を試験で固定する。

### `bridge.rs` — endpoint 構築と RT 側

```rust
/// Non-RT construction point. `duration_boundaries` are the fixed
/// histogram bucket edges in clock ticks (must be non-empty, strictly
/// increasing — `Histogram::histogram` の契約に従う)。
/// Returns Err(HistogramError) on invalid boundaries.
pub fn timing_bridge(
    capacity: usize,
    clock: TimingClock,
    source: SourceIdentity,
    duration_boundaries: &[u64],
) -> Result<(TimingRecorder, TimingSink), HistogramError>;

/// RT-side endpoint. `Send` (inherits RtRecorder), `!Sync`.
/// RT contract: `record` performs const header construction +
/// `RtRecorder::try_record` only — no alloc, no lock, no panic path
/// beyond the inner bridge, no tracing/subscriber, no I/O.
pub struct TimingRecorder { /* RtRecorder<TimingRecord> + TimingClock
                               + SourceIdentity */ }

impl TimingRecorder {
    /// Records one measured interval. Returns `false` when the bridge
    /// is full (drop-on-full): the loss is counted by `dropped()` and
    /// surfaced to the sink via `dropped_before` on subsequent records.
    pub fn record(&mut self, sample: &TimingSample) -> bool;
    pub fn dropped(&self) -> u64;   // total records dropped (bridge)
    pub fn recorded(&self) -> u64;  // total records accepted
}
```

`record` の実装は `TimingRecord::new(clock, source, sample)`（const
構築）→ `self.inner.try_record(record)` のみ。`sample` は `Copy` の
ため参照渡しで十分。

### `sink.rs` — 非 RT 側 endpoint と報告

```rust
/// Non-RT endpoint. Owns the consumer, the duration Histogram writer,
/// and cumulative exact statistics.
pub struct TimingSink { /* RecordSink<TimingRecord> + Histogram +
                           stats + expected TimingClock */ }

impl TimingSink {
    /// Drains all queued records and returns the per-poll window
    /// accounting. Non-RT only.
    pub fn poll(&mut self) -> TimingWindow;

    /// Cumulative statistics since construction. Non-RT only, cheap
    /// (no heap data).
    pub fn snapshot(&self) -> TimingSnapshot;

    /// OBS-02/OBS-07 registration handles.
    pub fn histogram_reader(&self) -> HistogramReader;
    pub fn recorded_reader(&self) -> CounterReader;  // cumulative accepted
    pub fn dropped_reader(&self) -> CounterReader;   // cumulative dropped
}

/// Per-poll accounting. `dropped > 0` marks the window as carrying
/// loss evidence — `dropped` is the positional sum of
/// `ReceivedRecord::dropped_before` over the records consumed by this
/// poll (loss is attributed to the window where its marker arrived,
/// not necessarily the window where the drop happened). The
/// distribution fields describe only the surviving samples and must
/// not be presented as a complete distribution.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TimingWindow {
    pub index: u64,                    // poll generation (1-based)
    pub records: u64,                  // records consumed this poll
    pub samples: u64,                  // valid duration samples
    pub clock_anomalies: u64,          // end_tick < start_tick
    pub dropped: u64,                  // positional loss evidence
    pub over_budget: u64,              // duration > asserted budget
    pub min_ticks: Option<u64>,
    pub max_ticks: Option<u64>,
    pub mean_ticks: Option<u64>,       // sum/count; u128 intermediate
    pub max_utilization_permille: Option<u64>,
    pub min_generation: Option<u64>,
    pub max_generation: Option<u64>,
    pub generation_transitions: u64,   // gen changes within this window
}

/// Cumulative report. Exact scalar statistics only — the full
/// per-sample distribution is intentionally not retained (bounded
/// memory contract); histogram quantiles are bucket-resolution
/// interval estimates, structurally distinct from exact stats.
/// `clock` echoes the `TimingClock` every record was stamped with —
/// foreign-clock records are unreachable by construction
/// (`TimingRecord` is built only by `TimingRecorder`), so no
/// consistency check is needed at the sink.
#[derive(Clone, Debug)]
pub struct TimingSnapshot {
    pub clock: TimingClock,            // the fixed clock identity
    pub records: u64,
    pub samples: u64,
    pub clock_anomalies: u64,
    pub dropped_total: u64,            // authoritative bridge counter
                                       // (includes in-flight/pending
                                       // drops not yet carried by an
                                       // accepted record)
    pub over_budget: u64,
    pub min_generation: Option<u64>,
    pub max_generation: Option<u64>,
    pub generation_transitions: u64,   // generation changes between
                                       // consecutive accepted records
    pub min_ticks: Option<u64>,
    pub max_ticks: Option<u64>,
    pub mean_ticks: Option<u64>,
    pub max_utilization_permille: Option<u64>,
    histogram: HistogramReader,        // live cell for estimated_quantile
}

impl TimingSnapshot {
    /// Bucket-resolution interval estimate of the duration quantile.
    /// `q_permille` is 0..=1000 (p50 = 500). Returns `None` when no
    /// samples exist or `q_permille > 1000`.
    /// This is an ESTIMATE at bucket resolution — not an exact
    /// percentile; exact distribution is not retained.
    pub fn estimated_quantile(&self, q_permille: u16)
        -> Option<QuantileEstimate>;
}

/// Interval estimate from fixed-bucket histogram: the true quantile
/// lies within [lower, upper]; `upper` is `None` for the top bucket
/// (unbounded above the last boundary).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuantileEstimate {
    pub lower_ticks: u64,
    pub upper_ticks: Option<u64>,
}
```

### sink の集計規則

各 `ReceivedRecord<TimingRecord>` について:

1. `dropped_before` を当該 window の `dropped`（位置的損失証拠）に
   加算する。bridge 全体の権威ある累積損失は `RecordSink::dropped()`
   の共有カウンタが持ち、`TimingSnapshot::dropped_total` として
   公開する（受理済み record にまだ担ぎ手が乗っていない drop も
   含む）。
2. `end_tick < start_tick` → `clock_anomalies += 1`、duration 統計から
   除外（wrap-around を duration として解釈しない）。
3. それ以外 → `duration_ticks = end_tick - start_tick`:
   - `histogram.observe(duration_ticks)`（固定 bucket、非 RT 側で
     atomic counter 更新 — alloc/lock なしだが RT では呼ばない）
   - `samples += 1`、min/max/sum 更新
   - `budget_ticks > 0` のとき:
     - `utilization_permille = duration_ticks * 1000 / budget_ticks`
       （u128 中間、u64 飽和）
     - `duration_ticks > budget_ticks` → `over_budget += 1`
     - `max_utilization_permille` 更新
   - `budget_ticks == 0` → utilization/over_budget 会計なし
     （「budget 未主張」を 0% 使用と偽装しない）
4. generation は anomaly の有無に関わらず全 consumed record の
   header から min/max/transition を更新する。

clock 同一性の検査は行わない: `TimingRecord` は `TimingRecorder`
のみが構築し、clock は常に構築時の `TimingClock` から刻まれるため、
foreign clock の record は型レベルで到達不能。clock を変えるには
新しい bridge を構築する（epoch 更新・domain 切替は非 RT の
再構築で行う）。

`poll()` は consumer を排出しきるまで処理し、当該 poll の
`TimingWindow` を返す。`index` は 1 始まりの poll 世代。
window の分布系 field（min/max/mean/max_utilization）は
`samples == 0` のとき `None`（0 埋めしない — metrics の
None-vs-zero 方針に一致）。

### budget 意味論の明示

- `duration / budget` は **測定区間の budget 使用率**（permille）。
  callback の deadline overrun・XRUN・device scheduling 遅延とは
  同義ではない — それらは domain/engine 側の別概念。
- `budget_ticks == 0` は budget 未主張であり、overrun 判定を行わない。
- 単位は record の `TimeBase` tick。tick→秒/サンプル変換は
  `ClockAnchor`/domain 側の責務で、本モジュールは変換しない。

### 生成世代（Generation）

`TimingSample.generation` はドメイン宣言の世代（plan/stream/graph
incarnation 等、ドメインが決める）。snapshot は `min_generation`/
`max_generation`/`generation_transitions`（連続 record 間の世代
変化回数）を保持し、世代の混在を隠さない。`0` は「世代なし」の
 sentinel として扱わず、値をそのまま記録する（domain が 0 を
 使うかは domain の契約）。

## Data and Control Flow

```
[RT thread]                                  [non-RT thread]
TimingRecorder::record(sample)
  └─ TimingRecord::new(clock, source, sample)   // const, pure
  └─ RtRecorder::try_record(record)
       ├─ success → SPSC ring                    TimingSink::poll()
       └─ full    → dropped += 1, dropped_before └─ for each ReceivedRecord:
                     on next record                  ├─ dropped_before → loss evidence
                                                      ├─ end<start       → clock_anomaly
                                                      └─ else → histogram.observe + exact stats
                                                  TimingSink::snapshot()
                                                  TimingSink::histogram_reader()
                                                    → Collector MetricSource
                                                      → MetricValue::Histogram
```

## Non-Functional Constraints

- RT `record`: alloc なし・lock なし・panic なし（`try_record` の
  戻り値で失敗を返す）・blocking なし・tracing/subscriber 呼出なし・
  I/O なし。`assert_no_alloc` レーンで検証。
- 構築（`timing_bridge`・`TimingSink` 内部）は非 RT: ring buffer・
  histogram bucket 確保で alloc してよい。
- `TimingRecord` は `Copy + Send + 'static`、128 byte 以内
  （`EventRecord` と同じ予算感）。
- `unsafe` なし。新規外部依存なし。feature gate なし（常時
  コンパイル — 語彙と endpoint は常備し、計測点の有無は domain が
  `record` 呼出の選択で決める）。
- 公開 enum を新設しない（既存語彙の再利用のみ）→
  `#[non_exhaustive]` 追加は不要。

## Risks and Trade-offs

- **bucket 分解能**: 固定 bucket のため quantile は区間推定。
  `QuantileEstimate` を `{lower, upper}` の区間で返し、exact 統計と
  型で区別することで「推定を exact と偽装しない」を構造的に担保。
- **全件分布を保持しない**: bounded memory 契約上、正確な全件分布は
  保存しない。exact scalar（count/min/max/mean/over-budget/max-util）
  と bucket 推定の組合せで要件を満たす。真の全件 percentile が必要な
  場合は domain 側で別経路（offline 処理）を使う — 本 slice の
  Non-goal。
- **clock の固定**: `TimingClock` は bridge 構築時に固定され、
  clock re-incarnation（epoch 変化）や domain 切替は新しい bridge
  構築（非 RT）で行う。1 bridge = 1 clock domain = 1 分布であり、
  異種 clock の混在は型レベルで排除される。
- **mean の精度**: `sum` は u128 累積、`mean = sum / count`。
  飽和時は `u64::MAX` に clamp して `mean_ticks` に反映（異常値の
  破壊的影響を防ぐ）。

## Deferred Design Concerns

- tick reader adapter（`MasterClock::tick`・`StreamInstant`・sample
  clock 由来 tick の RT-safe 読み取り）— domain 側実装、#293 統合時。
- window 単位の histogram delta（bucket 差分）— 現状は累積
  `HistogramReader` を OBS-07 が poll ごとに読む方式で十分。
  per-window delta が必要になれば collector 側で差分を取る。
- Consumer 側の budget 値決定・計測点選定・回帰評価 — #281/#293。
- 実機 overhead 計測 — #281 gate。

## Verification Notes

- `assert_no_alloc`（dev-dep 既存）で `TimingRecorder::record` の
  no-alloc を検証。
- bridge drop-on-full・`dropped_before`・window `dropped` の整合を
  小 capacity で決定的に試験。
- clock anomaly・budget 未主張・over-budget・generation 遷移・
  損失会計を unit test で網羅。
- `HistogramReader` を `MetricSource` として `Collector` に登録し
  `MetricValue::Histogram` が得られる統合試験（既存
  `source::descriptor`/`collector` API を使用）。
- `size_of::<TimingRecord>() <= 128` の固定。
