# ruviform-observability

Ruviform の共有観測基盤 — Issue #101 `foundation-observability-minimal-api`
スケッチ起点で Issue #121 の L1 diagnostics が初消費者となった **counter pair**
に加え、Issue #277（OBS-02）で **core 軽量 Primitive**（Gauge/Histogram）と
**Observation Source の登録・寿命契約**を収める最小クレート。

実 Registry・Poll・Metrics 集約・History・Snapshot は OBS-07（Issue #289）の
責務であり、本クレートはその実装可能な**コンパイル済み契約**のみを提供する。
`tracing`/`tracing-subscriber`/外部 IO/GUI への依存は持たない（std のみ）。

## 責務

### core（軽量 Primitive — writer/reader 共有ペア）

- `counter() -> (Counter, CounterReader)` — 共有 `Arc<AtomicU64>` セル、
  `Relaxed`、単一 Counter は非減少（crate root でも re-export — 既存公開 API）。
- `core::gauge(u64) -> (Gauge, GaugeReader)` — 状態値（増減あり・単調増加は
  約束しない）。単一 `AtomicU64` の atomicity のみを保証し、浮動小数点や
  複数値の transactional consistency は提供しない。
- `core::histogram(&[u64]) -> Result<(Histogram, HistogramReader), HistogramError>`
  — 構築時に固定された厳密昇順 Bucket 境界（非空必須）と Bucket 別
  `AtomicU64`。`observe` は有界 `partition_point` + `fetch_add`。
  `HistogramReader::snapshot` は Bucket ごとの独立 `Relaxed` load で
  **一貫 snapshot を約束しない**。

### source（登録・寿命契約）

- Descriptor: `CounterDescriptor {key, unit, reporting}`（出発点形）と
  `MetricDescriptor {kind, key, unit, reporting, aggregation, buckets, semantics}`。
  `key`・`meaning`・`origin` は `&'static str` で、Source/Label の
  cardinality を静的に制限する。
- `ObservationScope {Process, Instance, Session}`、`SourceId`（`NonZeroU64`・
  登録一意）+ `SourceGeneration` + `SourceIdentity`。`SourceId` は Domain
  Instance ID・Graph Revision・再 mint と混同しない。
- `CounterSource`/`MetricSource`/`MetricReader` — 登録時に Registry へ
  Reader 所有権を移す束。`MetricSource::new` は descriptor.kind と
  reader.kind の不整合を `SourceBuildError` で構築時に排除する。
- `ObservationRegistrar`（object-safe・`Send + Sync`）+ `RegistrationError`
  （`RegistrarStopped`/`CapacityExceeded`/`KeyConflict`/`SourceReused`/
  `FinalDrainInProgress`/`InvalidDescriptor`）— 実装は OBS-07 の Registry。
- `SourceLifecycle {Active, Retiring, FinalDrain, Removed}` + `SourceState`
  （共有状態セル）+ `RegistrationLease`/`RetiredSource`:
  - `lease.retire()` が `Active → Retiring`（writer quiescence 宣言）を行い
    ticket を返す。Collector が `begin_final_drain()` → `mark_removed()` を
    駆動し、Removed 到達後にのみ Reader を解放する。
  - retire せず Drop した Lease は `abandoned` として観測可能 — Reader cell
    は Registry 側所有のため、未出力 Delta が Lease Drop で消えることは
    **構造的にない**。

## 層・許可依存

- 層: Primitives（最下位・相互依存なし）
- 許可依存: なし（std のみ）。依存なしの最小構成を維持する。
  `assert_no_alloc` は dev-dependency のみ（`rt_` 検証レーン用）。
- 消費方向: `ruviform-plugin-host → ruviform-observability`（Issue #121）、
  `ruviform-audio → ruviform-observability`（Issue #185）。
  `ruviform-rt` からの逆依存は追加しない（foundation DAG/ADR 問題は
  別 Issue — `docs/designs/foundation-observability-minimal-api.md`）。

## RT 契約（operation ごと）

| 操作 | RT 適性 |
| --- | --- |
| `Counter::{increment, read, reader}`・`CounterReader::read` | wait-free・no-alloc・有界（`rt_` レーン実証済み） |
| `Gauge::{set, read, reader}`・`GaugeReader::read` | wait-free・no-alloc・有界（`rt_` レーン実証済み） |
| `Histogram::{boundaries, bucket_count, reader}`・`HistogramReader::{boundaries, bucket_count}` | wait-free・no-alloc・有界 |
| `Histogram::observe` | `rt_` レーンで no-alloc・有界を実証済みだが **正式 RT-safe 宣言は保留**（実機証跡待ち） |
| `counter()`/`gauge()`/`histogram()`・`Clone`・`Drop`・`HistogramReader::snapshot`・`source::*` 全操作（登録・retire・遷移・Lease Drop） | 非 RT のみ（alloc/registry 制御） |

- RT Writer は Registry Lock・登録ポートを持たない。必要な Reader/metadata は
  RT 稼働前に準備する。最後の Drop を RT で実行しない。
- 複数 counter/bucket の逐次 `Relaxed` 読みは同一時点の整合 snapshot を
  **保証しない**。
- `u64` は modulo 2^64 wrap（現実的構成では到達不能 — saturating 措置なし）。

## Domain と Consumer の責務分離

登録時に **Domain（計装側）が意味を宣言**し、**Consumer（収集・出力側）は
変換しない**。Severity を Counter 増分から一律推論しない。Counter→Log
mapping は Common Exporter（OBS-03 以降）が一箇所で担い、Consumer 個別実装は
持たない。Grant/Audit 認可は通常観測経路とは別の正本が担う。

```rust
use std::time::Duration;
use ruviform_observability::{counter, source::*};

// ——— Domain 側（plugin host など計装コード）: 意味を宣言して登録する
let (counter, reader) = counter();
let source = CounterSource {
    descriptor: CounterDescriptor {
        key: "plugin_host.l1.dropped_submits",
        unit: ObservationUnit::Count,
        reporting: ReportingHint::OnIncrease {
            min_delta: 1,
            min_interval: Duration::from_secs(1),
            impact: ReportingImpact::High, // 宣言ヒント（推論された Severity ではない）
        },
    },
    scope: ObservationScope::Instance,
    reader,
};

// registrar: &dyn ObservationRegistrar（OBS-07 の Collector が渡すポート）
// let lease = registrar.register_counter(source)?;

// RT 稼働中: 事前構築済みの writer 操作のみ
// counter.increment();

// 停止時: writer quiescence 確認後に retire → collector が FinalDrain/Removed を駆動
// let ticket = lease.retire();
```

Consumer はこの `impact`/`meaning`/`confidentiality` を**読むだけ**で、
独自の Counter→Log 変換・Severity 推論・認可判断を行わない。

## 実装 Issue

- Issue #101: 候補・trigger（`foundation-observability-minimal-api`）
- Issue #104: `observability_poc` の `Counter`/`CounterReader` 参照形
- Issue #121: 初消費者 L1 diagnostics
- Issue #275: Observability 統合設計正本（単一クレート構成）
- **Issue #277（OBS-02）**: core Primitive・Source 登録/寿命契約
- Issue #289（OBS-07）: Collector/Registry 実装（本契約の実装先）
