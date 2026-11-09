# ruviform-observability

Ruviform の共有観測基盤 — Issue #101 `foundation-observability-minimal-api`
スケッチ起点で Issue #121 の L1 diagnostics が初消費者となった **counter pair**
に加え、Issue #277（OBS-02）で **core 軽量 Primitive**（Gauge/Histogram）と
**Observation Source の登録・寿命契約**、Issue #280（OBS-05）で
**固定長 record 語彙**（ClockContext・Correlation・EventRecord）と
**RT → 非 RT Record Bridge**（`rt_bridge`）を収めるクレート。

実 Registry・Poll・Metrics 集約・History・Snapshot は OBS-07（Issue #289）の
責務であり、本クレートはその実装可能な**コンパイル済み契約**のみを提供する。
`tracing`/`tracing-subscriber`/外部 IO/GUI への依存は持たない（default build は
外部依存ゼロ — `ruviform-rt`・`ruviform-media-core` の葉 primitive への
一方向依存のみ、ADR `docs/adr/obs-05-observability-leaf-primitive-deps.md`）。

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

### record（固定長 record 語彙 — Issue #280 / OBS-05）

- `ClockContext` — record が運ぶ clock-domain stamp。
  `ruviform_media_core::InputStamp`（domain + `ticks` + `TimeBase` +
  `TimestampProvenance`）+ `ClockEpochId` の構成で、device stamp・
  `EngineFrame` と同一 identity 空間。wall/monotonic への変換 API は
  **持たない**（変換 authority は `map_stamp_to_frame` のみ）。
- `CorrelationToken`（`NonZeroU64`）・`CorrelationLink{token, epoch}`・
  `CorrelationBook`（非 RT issuer/registry — `issue`/`remint`/`expire`/
  `classify`）。link は `Linked`/`Unlinked(UnknownToken|SupersededEpoch|
  Expired)` に分類され、Trace ID・Grant ID・`SourceId`/`SourceGeneration`
  とは別軸。`correlation: None` は「意図的に link しない」で、unlinked と
  混同しない。
- `EventId`・`RecordHeader`（event/version/`SourceIdentity`/domain generation/
  correlation/clock/`Confidentiality`/`ReportingImpact`）・`EventRecord`
  （header + `value: u64` + `reserved: [u64; 2]`）。固定長 `Copy` —
  raw PCM/signal buffer は構造的に入らない。

### rt_bridge（RT → 非 RT Record Bridge — Issue #280 / OBS-05）

- `bridge(capacity) -> (RtRecorder<R>, RecordSink<R>)` — 既存
  `ruviform_rt::channel` の有界 SPSC ring を再利用（第 2 の ring 実装なし）。
  構築が唯一の確保点。`capacity == 0` は panic。
- `RtRecorder::try_record` — bounded・non-blocking・no-alloc（`try_push` 1 回 +
  `Relaxed` counter 高々 2 回）。満杯時は **drop-on-full**: record は回復
  不能に捨てられ、`dropped` counter と次に受理された record の
  `dropped_before` envelope で欠落を可視化する。
- `RecordSink::{drain, drain_final}` — FIFO で `ReceivedRecord{record,
  dropped_before}` を delivery（bounded by capacity）。`drain_final` は
  writer quiescence（recorder が RT パスを離れた/破棄された）後に限る。
- 計測: `recorded`（受理総数）・`dropped`（欠落総数）・`drained_total`・
  `lag() = recorded - drained_total`（consumer lag = 遅延であり欠落ではない）。
  `recorded_reader`/`dropped_reader` は `CounterReader` clone で OBS-02 の
  `CounterSource` 登録にそのまま使える。
- セッション境界: `Prepare`(bridge) → `Publish`(recorder を RT へ move) →
  `ON`(RT 実行中は `try_record`/counter 読みのみ) → `OFF`(move 戻し) →
  `Retire`(recorder drop + `drain_final`) → `FinalDestroy`(sink drop で
  ring 解放)。両 endpoint の Drop は非 RT のみ。

## 層・許可依存

- 層: Primitives
- 許可依存: `ruviform-rt`・`ruviform-media-core`（葉 primitive への
  **一方向**依存 — Issue #280 ADR）。外部 crates.io 依存はなし（default
  build）。`assert_no_alloc` は dev-dependency のみ（`rt_` 検証レーン用）。
- 消費方向: `ruviform-plugin-host → ruviform-observability`（Issue #121）、
  `ruviform-audio → ruviform-observability`（Issue #185）。
  `ruviform-rt → ruviform-observability` 等の逆依存は作らない。

## RT 契約（operation ごと）

| 操作 | RT 適性 |
| --- | --- |
| `Counter::{increment, read, reader}`・`CounterReader::read` | wait-free・no-alloc・有界（`rt_` レーン実証済み） |
| `Gauge::{set, read, reader}`・`GaugeReader::read` | wait-free・no-alloc・有界（`rt_` レーン実証済み） |
| `Histogram::{boundaries, bucket_count, reader}`・`HistogramReader::{boundaries, bucket_count}` | wait-free・no-alloc・有界 |
| `Histogram::observe` | `rt_` レーンで no-alloc・有界を実証済みだが **正式 RT-safe 宣言は保留**（実機証跡待ち） |
| `RtRecorder::{try_record, dropped, recorded}` | 有界・non-blocking・no-alloc（`rt_` レーン実証済み） |
| `record::*` の `Copy` 値型・`const` アクセサ | RT 適合（pure） |
| `counter()`/`gauge()`/`histogram()`・`Clone`・`Drop`・`HistogramReader::snapshot`・`bridge`・`RecordSink` 全操作（`drain`/`drain_final`/readers・sink Drop）・`CorrelationBook` 全操作・`source::*` 全操作（登録・retire・遷移・Lease Drop）・両 endpoint の Drop | 非 RT のみ（alloc/registry 制御） |

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
- **Issue #276（OBS-01）**: `logging` feature — 汎用 Logging Adapter
- **Issue #280（OBS-05）**: `record` 語彙・`rt_bridge` RT Record Bridge
- Issue #289（OBS-07）: Collector/Registry 実装（本契約の実装先）

## `logging` feature（Issue #276 / OBS-01）

標準 `tracing` / `tracing-subscriber` / `tracing-appender` 基盤の
**Host 所有**ロギングアダプタ。Ruviform 独自の Logger/Span API は
作らず、nonRT コードは `tracing` の Event/Span を直接発行する。

```toml
ruviform-observability = { path = "...", features = ["logging"] }
```

### 依存追加の理由（feature 最小・既定 off）

`logging` を有効にした時だけ `tracing` / `tracing-subscriber`（env-filter・
fmt・json）/ `tracing-appender` を optional 依存として引く。
既定（`--no-default-features`）では core counter pair のみで外部依存ゼロを
維持する。独自 logger を実装しないのは受入要件であり、標準エコシステムの
filter/format/rolling 実装を再利用するため。

### 所有権契約

- **Subscriber は Host が所有する。** `build_logging` は `Dispatch` を、
  `build_logging_layers` は `registry().with(..)` 合成可能な `Layer` 群を
  返すだけで、ライブラリは `set_global_default` / `init` を一切呼ばない。
  global 化は Host がプロセスごと 1 回だけ行う判断であり、
  `set_global_default` は既設定時に error を返す。GUI/headless/マルチ
  ウィンドウの scoped 運用には `tracing::dispatcher::with_default` を使う。
- **stdout には出さない。** `SinkDestination` は `Stderr` と `RollingFile`
  のみ — AudioLens `inspect` のような stdout の構造化出力を構造的に保護する。
- **製品固有の設定読み取りはしない。** filter 文字列は EnvFilter 構文だが
  `SinkConfig` で明示供給する。環境変数・CLI・GUI 設定から
  `LoggingConfig` への写像は Host の責務（AudioLens `AL_LOG`/flag/GUI
  優先順位も Host 側で解決する）。

### Sink 構成

- `SinkConfig { destination, format, span_events, filter, failure_mode }`
  — sink ごとに独立。filter は `Layer::with_filter` で sink 個別に
  適用され、同一 event を各 sink が独自にルーティングする。
- `LogFormat::{Text, JsonLines}` — Text は ANSI off の人間可読 1 行、
  JsonLines は event ごと 1 JSON object。
- `SpanEvents::{NONE, NEW, ENTER, EXIT, CLOSE, ACTIVE, FULL}` — sink ごとの
  span lifecycle record 出力（`tracing_subscriber::FmtSpan` の crate 固有
  対応物。`|` で合成可能）。既定 `NONE` は pre-#335 の record なし挙動を
  維持。`CLOSE` は内部 event を持たない leaf span を、span field と
  `time.busy`/`time.idle` を持つ `close` record として残す — AudioLens
  AL-OBS-01 の leaf span 観測が要求する契約（Issue #335）。lifecycle
  record は span 自身の level/target を継承するため sink の `filter` を
  通常 event と同様に通る。
- `SinkDestination::RollingFile { directory, filename_prefix,
  filename_suffix, rotation, max_files, queue_capacity }` — ファイル名は
  `{prefix}.{rotation-date}`（daily なら `app.2026-11-04`）。
  `filename_suffix: Some("log")` 指定時は
  `{prefix}.{rotation-date}.{suffix}`（`app.2026-11-04.log`）、`None` は
  従来どおり。retention は `tracing-appender` の `max_log_files` に委譲し、
  構築時・rotation 時に prefix+suffix 一致ファイルを古い順に削除する
  （AudioLens の `al.YYYY-MM-DD.log`・7 daily 世代は `Daily` +
  `Some("log")` + `max_files=7` — Issue #334）。
- `filter` が不正なら `LoggingBuildError::InvalidFilter` で build 失敗。
- `RotationPeriod::{Daily, Hourly, Minutely, Never}`。

### 失敗モデル

- `SinkFailureMode::Required` の sink 初期化失敗 →
  `LoggingBuildError::RequiredSinkFailed`（build 全体失敗、構築済み sink は
  先に shutdown）。
- `SinkFailureMode::BestEffort` の失敗 → `LoggingBuildResult::degraded` に
  明示記録し、残 sink は動作を継続。
- 全 sink 失敗（全て BestEffort）→ `LoggingBuildError::AllSinksFailed`。
  全滅構成を成功扱いしない。空 `sinks` も `NoSinks` で明示失敗。
- 実行時失敗は error ではなく `LoggingHealth::snapshot()` の per-sink
  カウンタで区別して観測する: `dropped_queue_full`（有界 queue 満杯
  drop）・`partial_writes`（`WriteZero`）・`write_failures`（その他の
  write/rotation 失敗）・`flush_failures`・`channel_disconnected`
  （worker 死亡/shutdown 後の emit）・`shutdown_abandoned`。
  Logger 自身の失敗を同じ Logger で再帰出力しない — `snapshot()` は
  atomic load のみで event を一切発行しない（非再帰計測）。
- 上游注意: `tracing-appender` は rotation 時のファイル作成失敗を自身で
  stderr に報告して旧ファイルへの書込を継続する。その経路は本 adapter の
  カウンタに到達しない。`Write` 経由で表面化した write 失敗は
  `write_failures` に計測される。

### Writer とライフサイクル

各 sink は専用 background worker スレッドが drain する有界非ブロック
writer（`std::sync::mpsc::sync_channel` + `queue_capacity`）。発行側は
`try_send` のみで決してブロックせず、満杯時は drop を計数する。
`tracing_appender::non_blocking` ではなく自前実装なのは、queue-full /
partial write / flush 失敗 / disconnect を区別する観測性が
`non_blocking` に存在しないため（ADR
`docs/adr/issue-276-logging-adapter-bounded-writer-ownership-and-build-time-sink-resilienc.md`）。

`WriterGuard` の drop が shutdown を開始する: worker は shutdown marker
以前に queue された event を全て drain・flush してから終了し、guard は
`SHUTDOWN_TIMEOUT`（5s）で join を待つ。timeout 時は worker を殺さず
abandon し `shutdown_abandoned` に記録する。marker 以後に queue された
event・abandon 後・プロセス crash 時の未 flush 分は失われる（契約として
rustdoc に明記）。`Dispatch`・guard は使用期間中保持すること — guard を
先に drop するとそれ以降の event は `channel_disconnected` として捨てられる。

### RT 契約

**RT コールバックから通常の `tracing` を発行しない。** Subscriber の
layer は発行スレッド上で同期実行され、本 adapter の writer 経路は event
ごとに `Vec<u8>` へ format 用 alloc を行うため RT-safe ではない。
RT 側の観測は RT-safe な専用機構（counter pair 等）を使う。

### センシティブデータ

adapter は event の field をそのまま sink に流すだけで、内容の検査・
redact・rate-limit は行わない。raw PCM・prompt・path・credential を
無制限に出力しない責務は発行側にある（本モジュール自身はそのような
event を一切生成しない）。

### Host sample

```sh
cargo run -p ruviform-observability --features logging --example logging_host
```

`examples/logging_host.rs` — Host 側 config 写像 → `build_logging` →
scoped dispatch → health snapshot → guard shutdown の一通りを示す。

## `integration` feature（Issue #293 / OBS-11）

Host 所有の非 RT 統合境界 — `ObservabilityConfig`（Collector / Metrics /
Diagnostics / Debug / tracing-export の独立設定）と `ObservationRunner`
（Collector poll の駆動・Source 登録・Snapshot 照会・停止/Drain
レポート）を提供する。`integration` は additive feature で既定 off
かつ外部依存を追加しない（`logging` / `tracing-export` は独立に
選択する）。runner はスレッド・タイマーを所有せず、すべての操作は
Host が非 RT コンテキストから呼ぶ。

```toml
ruviform-observability = { path = "...", features = ["integration"] }
```

ライブラリは `tracing::Dispatch` の global install を一切行わない。
`tracing-export` 使用時は Host が構築した `Dispatch` を
`ExportPolicy.dispatch` に注入し、exporter は emit ごとに
`tracing::dispatcher::with_default` でスコープする。

### 起動（通常運用）

```rust
use ruviform_observability::integration::{
    ObservabilityConfig, ObservationRunner,
};
use std::time::{Duration, Instant};

let config = ObservabilityConfig {
    collection: /* CollectorConfig::default() 相当 */ todo!(),
    diagnostics: None, // Diagnostics 無効でも Collector 単体で動く
    debug: None,
    // export は tracing-export feature 時のみ存在
};

let mut runner = ObservationRunner::new();
runner.prepare(config)?;   // Validate → Prepare（構成要素を完全構築）
runner.activate()?;        // Prepared → Ready
// Host の非 RT loop で poll を駆動。初回 poll で Ready → Running。
while running {
    runner.poll_once(Instant::now())?;
    std::thread::sleep(Duration::from_millis(10));
}
```

Source の登録は `runner.registration_port()`（`Collector` の
`ObservationRegistrar`）経由で、収集中でも追加できる。Snapshot 照会は
`runner.snapshot()` / `latest_batch()` / `history()`、Diagnostics は
`diagnostics_snapshot()`、Debug inspector は `debug()` から辿る。

### 通常停止

```rust
// 1) Draining へ — domain writer に quiescence を宣言させる猶予
runner.begin_shutdown()?;              // Running → Draining
// 2) writer 停止後に lease.retire() → Collector が FinalDrain を駆動
// 3) 非 RT の最終 drain（順序固定）:
let report = runner.finish_shutdown(Instant::now())?;
//    a. Collector 最終 poll → b. Collector::shutdown（terminal drain）
//    c. terminal batch の tracing export → d. logging writer guard drop
//       （flush + join、bounded wait 後 abandon は health に記録）
```

`runner.shutdown(now)` は上記を一括で行う one-shot 版。
`ShutdownReport` は `DrainCompleteness`（`Complete` / `Partial` /
`Unknown`）・quiesced/abandoned source 数・export 件数・
`writers_closed` を正直に報告する — 回収不能・writer 未静止を
「完全保存」とは主張しない。

### 異常停止（writer 未静止）

`begin_shutdown` 後に writer が quiesce しないまま `finish_shutdown`
を呼ぶと、Collector は active source を強制 drain し、観測は
`Partial`（writer-not-quiesced）として報告される。Host crash 等で
最終回収が走らなかった場合は `Unknown` — いずれも完全性を偽らない。

### Logging OFF

`attach_logging` を呼ばなければ logging は完全に無効のまま動く。
`build_logging` / `WriterGuard` は Host が構築し、
`attach_logging(guards, health)` で明示的に取り付ける
（Prepared/Ready/Running のみ受理）。guard の drop が shutdown 時の
flush/close を担い、`RunnerHealth.logging` は atomic のみで読む
非再帰 snapshot — 自身の health 計測が logging sink に再帰しない。

### Debug Session

`ObservabilityConfig.debug = Some(DebugSettings::new(debug_config))`
で debug inspector を有効化する。`SessionPermit` による disclosure
認可は Host が `DebugAuthority` 経由で行い、session 容量・probe 状態は
`RunnerHealth.debug` / `debug_session` に現れる。再構成
（`reconfigure`）では debug session・retention・exporter baseline は
**移行しない** — `ReconfigureReport.non_migrated` が明示する。
