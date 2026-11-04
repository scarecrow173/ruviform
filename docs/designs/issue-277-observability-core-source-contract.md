---
id: "DESIGN-034Rp1Um3ZXDWfRx8pCmuw"
type: "design"
status: "approved"
title: "Issue #277 OBS-02 軽量Primitive・Observation Source契約: Gauge/Histogram・Descriptor・SourceId/Generation・Registrar/Lease・寿命"
created: "2026-11-03"
updated: "2026-11-03"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/277"
    - "https://github.com/scarecrow173/ruviform/issues/275"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-277-observability-core-source-contract.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/foundation-observability-minimal-api.md"
    - "docs/designs/shared-foundations-and-modularization-policy.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-277-observability-core-source-contract.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-277-observability-core-source-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/289"
    - "https://github.com/scarecrow173/ruviform/issues/278"
    - "https://github.com/scarecrow173/ruviform/issues/121"
    - "https://github.com/scarecrow173/ruviform/issues/104"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/289"
  deferred-by: []
---

# Issue #277 OBS-02 軽量Primitive・Observation Source契約: Gauge/Histogram・Descriptor・SourceId/Generation・Registrar/Lease・寿命

## Context

`ruviform-observability` は現状 `counter() -> (Counter, CounterReader)` のみを持つ
最小クレート（依存ゼロ・std のみ）。Issue #275（Observability 統合設計正本）は
単一クレート内の論理構成を `core → collector → diagnostics/debug/tracing_export` と
定め、OBS-07 Collector（#289）は「OBS-02 の登録ポートと Lease で
Active→Retiring→最終Drain→Removed を調整する」と前提している。本設計は
`core`（軽量 Primitive）と `source`（登録/寿命契約）の**型だけ**を固定する。
Registry・Poll・集約・History は実装しない。

## Scope

### In Scope

- `src/core/`: 既存 counter（`lib.rs` から `core` へ移動し root re-export で
  公開 API を不変維持）、`gauge()`、`histogram()`。
- `src/source/`: Descriptor 群・`ObservationScope`・`SourceId`/`SourceGeneration`/
  `SourceIdentity`・`MetricSource`/`CounterSource`/`MetricReader`・
  `SourceLifecycle`/`SourceState`・`RegistrationLease`/`RetiredSource`・
  `ObservationRegistrar` trait・`RegistrationError`/`LifecycleError`。
- operation 別 RT 適性表・`rt_` レーン試験・寿命遷移の決定的試験・
  trait object safety / `Send + Sync` / 循環参照なしのコンパイル検証。
- crate README の Domain/Consumer 責務分離例。

### Out of Scope

- 実 Registry・Poll・Baseline/Rate・History・Snapshot・Health（全て OBS-07）。
- tracing/logging/export・RT Record bridge・diagnostics・debug・integration。
- f64 値・複数値 Gauge・厳密分位点・可変長 Label（cardinality は 'static key +
  enum scope で静的制限のみ）。

## Components and Boundaries

### モジュール構成

```text
src/lib.rs     — crate doc・`pub mod core; pub mod source;`・root re-export
                 (`counter`/`Counter`/`CounterReader` は root で不変維持)
src/core/mod.rs      — pub use counter/gauge/histogram
src/core/counter.rs  — 既存実装をそのまま移動（API・意味・テスト不変）
src/core/gauge.rs    — `gauge() -> (Gauge, GaugeReader)`
src/core/histogram.rs— `histogram(boundaries) -> Result<(Histogram, HistogramReader), HistogramError>`
src/source/mod.rs    — pub use 各型
src/source/descriptor.rs — CounterDescriptor/MetricDescriptor/MetricKind/
                       ObservationUnit/ReportingHint/ReportingImpact/
                       AggregationPolicy/SourceSemantics/SourceTemperature/
                       Confidentiality
src/source/scope.rs  — ObservationScope
src/source/identity.rs — SourceId/SourceGeneration/SourceIdentity
src/source/source.rs — MetricSource/CounterSource/MetricReader
src/source/lifecycle.rs — SourceLifecycle/SourceState/LifecycleError
src/source/lease.rs  — RegistrationLease/RetiredSource
src/source/registrar.rs — ObservationRegistrar/RegistrationError
```

### core: Gauge

- `gauge(initial: u64) -> (Gauge, GaugeReader)`。共有 `Arc<AtomicU64>`・`Relaxed`。
- `Gauge::set(u64)`/`Gauge::read()`/`Gauge::reader()`/`GaugeReader::read()`:
  wait-free・no-alloc・有界。`set` は `store(Relaxed)`。
- Counter と異なり**単調増加を約束しない**（状態値・増減あり）ことを doc で明示。
- `gauge()` 構築・`Clone`・`Drop` は非 RT（Arc の alloc/free）。
- f64 ビット詰め・複数 Atomic の合成値は提供しない — 単一 `AtomicU64` のみを
  「安全」と型で限定し、transactional consistency は約束しない。

### core: Histogram

- `histogram(boundaries: &[u64]) -> Result<(Histogram, HistogramReader), HistogramError>`。
  構築時に境界を検証（非空・厳密昇順）し `Box<[u64]>` に固定。Bucket 数は
  `boundaries.len() + 1`（bucket i は `boundaries[i-1] <= v < boundaries[i]`、
  先頭は下限なし・末尾は上限なし）。
- 共有部 `Arc<HistogramShared>`: `boundaries: Box<[u64]>` +
  `buckets: Box<[AtomicU64]>`。
- `Histogram::observe(v)`: `partition_point`（O(log N)・固定 N で有界）で
  Bucket index を決め `fetch_add(1, Relaxed)`。**RT-safe と宣言しない** —
  `rt_` レーンで no-alloc を実証するが、正式宣言は実機証跡待ちと doc に明記。
- `HistogramReader::snapshot() -> Box<[u64]>`: Bucket ごとの `Relaxed` load。
  alloc するため非 RT かつ**複数 Atomic の非同時観測**（一貫 snapshot 非保証）。
- `HistogramError::{EmptyBoundaries, UnsortedBoundaries}`（non_exhaustive）。
- 分位点・合計・再 baseline は提供しない（OBS-07 以降の責務）。

### source: Descriptor 群（不変・登録入力）

- `CounterDescriptor { key: &'static str, unit: ObservationUnit, reporting: ReportingHint }`
  — Issue 指定の出発点形。
- `MetricDescriptor { kind: MetricKind, key, unit, reporting, aggregation: AggregationPolicy,
  buckets: Option<&'static [u64]>, semantics: SourceSemantics }`
  — 全 kind 共通の不変メタデータ。`From<CounterDescriptor>`（kind=Counter・
  aggregation=Cumulative・semantics=Default）を提供。
- `MetricKind { Counter, Gauge, Histogram }`、`ObservationUnit { None, Count, Bytes,
  Nanoseconds, Samples, Frames }`、`AggregationPolicy { Cumulative, LatestValue,
  BucketCounts }`、`ReportingImpact { Low, Normal, High }`。
- `ReportingHint { SnapshotOnly, OnIncrease { min_delta: u64, min_interval: Duration,
  impact: ReportingImpact } }` — Domain 宣言の報告ヒントであり、Severity を
  増分から一律推論する機構は置かない。
- `SourceSemantics { meaning: &'static str, temperature: SourceTemperature,
  warmup: Option<Duration>, expected_range: Option<RangeInclusive<u64>>,
  origin: &'static str, confidentiality: Confidentiality }` + `Default`。
  `SourceTemperature { Hot, Warm, Cold }`、`Confidentiality { Public, Internal,
  Confidential }`（既定 Internal）。
- key・情報源・意味は `&'static str` で動的文字列の無制限 Series 化を型で防ぐ
  （cardinality 制限は OBS-07 の `max_sources`/`max_series` 設定が担う）。

### source: Scope / Identity / Source

- `ObservationScope { Process, Instance, Session }` — Source の帰属スコープ。
  enum で候補を静的に制限する。
- `SourceId(NonZeroU64)` — 観測登録の一意識別子。Domain Instance ID・Graph
  Revision・再 mint と混同しない。`SourceId::new(u64) -> Option<Self>`・`get()`。
- `SourceGeneration(u64)` — 同一論理 Source の再 mint（再登録）世代を区別する。
- `SourceIdentity { id, generation }` — Registrar が払い出す登録一意性の対。
- `CounterSource { descriptor: CounterDescriptor, scope: ObservationScope,
  reader: CounterReader }`（公開フィールドの出発点形）。
- `MetricReader { Counter(CounterReader), Gauge(GaugeReader),
  Histogram(HistogramReader) }`（`kind()` 付き）。
- `MetricSource { descriptor: MetricDescriptor, scope: ObservationScope,
  reader: MetricReader }` — 不整合は `MetricSource::new()` が
  `SourceBuildError::KindMismatch` で構築時に排除（フィールド private）。
  `From<CounterSource>` は構成的に不整合を生じ得ず infallible。

### source: 寿命（`SourceLifecycle`/`SourceState`）

- `SourceLifecycle { Active, Retiring, FinalDrain, Removed }`（non_exhaustive）。
- `SourceState` = `Arc` 共有される状態セル（`AtomicU8` lifecycle +
  `AtomicBool` abandoned）。Registrar が生成し Lease と共有する。
- 遷移権限をメソッドで分離:
  - `request_retire()`: `Active → Retiring` CAS（Lease.retire/Drop が呼ぶ）
  - `abandon()`: abandoned フラグを立て `Active → Retiring` を試行（Lease Drop）
  - `begin_final_drain()`: `Retiring → FinalDrain`（Collector のみ。Writer
    quiescence 宣言後でなければ開始できない = retire/abandon 未実施では Err）
  - `mark_removed()`: `FinalDrain → Removed`（Collector のみ）
  - 不正遷移は `LifecycleError::InvalidTransition { expected, actual }`。
- **Reader の解放は Removed 到達後に Registry 側で行う**。Reader cell は
  Registry が所有する `MetricSource` 内にあり、Lease は一切所有しない。

### source: Lease / Registrar

- `RegistrationLease { identity, state: Arc<SourceState> }`:
  - `identity()`/`lifecycle()`/`is_abandoned()`。
  - `retire(self) -> RetiredSource`: `request_retire()` 後に ticket を返す。
    self 消費時に Drop が走るが、state は既に Retiring のため abandon 化しない
    （Drop は「state==Active のまま」でのみ `abandon()` を呼ぶ）。
  - `Drop`（retire 未実施）: `abandon()` — 「黙って消す」のではなく abandoned
    として Collector に可視化する。未出力 Delta の cell は Registry 側所有で
    Lease Drop では消えない（型で保証）。
  - **`RegistrationLease::new(identity, state)` は Registrar 実装者向け**
    （doc 明記）。Domain は `register()` 経由でのみ取得する。
- `RetiredSource { identity, state }`: `identity()`/`lifecycle()` — 進行を観測する
  非ブロッキング ticket。Drop は何もしない（明示的 no-op）。
- `ObservationRegistrar: Send + Sync`:
  `fn register(&self, source: MetricSource) -> Result<RegistrationLease,
  RegistrationError>` + default `register_counter(CounterSource)`。
  object-safe（generic なし・具象戻り値）。
- `RegistrationError`（non_exhaustive）: `RegistrarStopped`（Collector 停止後）・
  `CapacityExceeded { limit: usize }`・`KeyConflict { key: &'static str }`
  （同名別 Instance）・`SourceReused`（同一 Source cell の再登録）・
  `FinalDrainInProgress`（最終 Drain 中の登録競合）・`InvalidDescriptor
  { reason: &'static str }`。`Display` + `std::error::Error` を実装。
  **登録失敗は `Result` で表現し、Audio callback 継続可否を暗黙に変更しない**。

## Data and Control Flow

- Domain（非 RT 初期化）: counter/gauge/histogram を構築 → `MetricSource`/`CounterSource`
  を `ObservationRegistrar::register*` へ → `RegistrationLease` + `SourceIdentity`
  を得て RT 稼働前に Writer handle（`Counter` 等）を RT 側へ渡す。
- RT 稼働中: Writer は事前準備済みの `increment`/`set`/`observe` のみ実行。
  Registrar・Lock・Reader 取得を一切触らない。
- 停止時: Domain が RT Writer quiescence を確認 → `lease.retire()`（Active→
  Retiring）→ Collector が `begin_final_drain()` で最終値を確定 →
  `mark_removed()` で Reader を解放。Lease 破棄忘れは `abandon()` として
  Collector に可視化される。

## Non-Functional Constraints

- 依存: std のみ（`[dependencies]` 空）。`assert_no_alloc = "1.1"` は
  dev-dependency のみ（rt_ レーン用・既存クレートと同パターン）。
- RT 適性（operation 別・doc に表で明記）:
  `Counter::{increment,read,reader}`・`CounterReader::read`・`Gauge::{set,read,reader}`・
  `GaugeReader::read` → wait-free/no-alloc/有界（既存契約と同根拠）。
  `Histogram::observe` → assert_no_alloc で no-alloc 実証済みだが正式 RT-safe
  宣言は保留。構築/Clone/Drop/登録/解除/遷移/`snapshot()` → 全て非 RT。
- 全公開 enum `#[non_exhaustive]`・`unsafe` 不使用・`Send + Sync` は
  `Arc<Atomic*>`/`NonZeroU64`/`&'static str` 構成から自動導出。

## Risks and Trade-offs

- `MetricSource::new` の kind 不整合検査を構築時に限定 → 公開フィールド化より
  型安全を優先。`CounterSource` は typed reader で不整合不能なため公開
  フィールドのまま（Issue の出発点形を維持）。
- Lease が Reader を所有しない設計 → 「Drop で Delta が消える」が構造的に
  不可能。Trade-off: 最終値確定の責務が Collector に集中する（#275 合意と整合）。
- `SourceState` の遷移メソッドを pub 化 → Collector 以外も呼べてしまうが、
  権限分離は doc 契約＋テストで担保（言語上の権限型化は過剰設計と判断）。
- Counter の `reader()` 派生で Arc cell が増える既存挙動は変更しない。

## Deferred Design Concerns

<!-- Intentionally deferred future work. Link the deferred draft doc via relations.defers. -->

- Registry 実装・poll・Baseline・History・Health | OBS-07 責務 | #289 | 本契約の型が実装可能かの証明は OBS-07 の最初のタスク。
- `observe` 正式 RT 宣言・Histogram 精度/分位点 | OBS-08 計測 | #290 | 過剰宣言を避ける。
- f64 Gauge・可変 Label | 実需要待ち | — | cardinality 制御を壊す恐れ。

## Verification Notes

- [ ] `mise run check`/`lint`/`test`/`test-rt`/`fmt-check` が全 pass。
- [ ] 既存 counter テストが変更なしで通る（API/意味回帰なし）。
- [ ] `ruviform-audio`/`ruviform-plugin-host` ビルドが回帰なし（workspace check）。
- [ ] rt_ レーンで gauge/histogram/counter 各操作の no-alloc 実証。
- [ ] 寿命遷移・失効 Error・Lease Drop の abandon・retire ticket の決定的試験。
- [ ] `ObservationRegistrar` の `Box<dyn>` 化・Send+Sync コンパイル検証。
