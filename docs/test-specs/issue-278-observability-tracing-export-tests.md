---
id: "TSPEC-034Uprl9YF50Z6pFk0lPWo"
type: "test-spec"
status: "approved"
title: "Issue #278 OBS-03 tracing_export 試験仕様"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/278"
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
    - "docs/designs/issue-278-observability-tracing-export.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-278-observability-tracing-export.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-278-observability-tracing-export.md"
    - "docs/designs/issue-278-observability-tracing-export.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/278"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/293"
  deferred-by: []
---

# Issue #278 OBS-03 tracing_export 試験仕様

## Purpose

`tracing_export` スライスの契約を固定する。固定 schema・severity 写像・
`ReportingHint` 解釈・損失計数・Dispatch 注入・`FindingSink` adapter・
feature 分離を、実 `Collector`/`Diagnostics` を通した contract test で
検証する。本書の試験は `#[cfg(feature = "tracing-export")]` で gate し、
`tracing-subscriber` の capture `Layer`（dev-dependency）で event を
観測する。

## Feature

- `tracing_export::TracingExporter::emit(&ObservationBatch)` が
  `ReportingHint` に従い `tracing` event を発行し `EmitStats` を返す。
- `tracing_export::TracingFindingSink` が `DiagnosticsEvent` を
  `tracing` event に写す `FindingSink` 実装である。
- feature `tracing-export` は `dep:tracing` のみを有効化し、
  `logging` feature・subscriber 構築とは独立である。

## Rules

- R-1 schema: measurement event は固定 key
  （`event_code`/`event_version`/`observation_key`/`source_id`/
  `source_generation`/`metric_key`/`scope`/`lifecycle`/`kind`/`unit`/
  `validity`/`issue`/`loss`/`loss_quantified`/`observed_at_mono_us`/
  `poll_generation` + kind 別値 field）を持ち、値は有界。
- R-2 SnapshotOnly: `ReportingHint::SnapshotOnly` の measurement は
  event を生成しない（`snapshot_suppressed` 計数）。
- R-3 OnIncrease: `min_delta` 未満の増分・`min_interval` 未経過の
  再発行は抑制される（`policy_suppressed` 計数）。初観測は baseline
  確立のみで emit しない。
- R-4 severity: `Level` は `ReportingImpact`（Low→DEBUG /
  Normal→INFO / High→WARN）と validity floor（Stale→INFO /
  Partial・Discontinuous→WARN）の大きい方。ERROR は使わない。
- R-5 window event: eligible measurement を処理した poll のみ
  `event_code="observation_window"` の集約 event を 1 件発行する。
- R-6 cap: `max_emissions_per_poll` 超過分は発行せず
  `cap_dropped` 計数（policy 抑制と区別）。
- R-7 valueless: `value == None` の measurement は event 化せず
  `valueless_skipped` 計数。
- R-8 dedup: 同一 batch の再 `emit` や変化のない連続 poll で
  measurement event を重複発行しない。
- R-9 sink: `HealthTransition` は `to` の `HealthState` 由来の
  Level（Degraded→WARN / Unhealthy→ERROR / Unknown・Healthy→INFO）
  と `domain`/`from_state`/`to_state` field で発行される。sink の
  panic は隔離され Diagnostics を巻き込まない。
- R-10 分離: Dispatch 未設定（`Dispatch::none()`）・subscriber なし・
  filter 全落としでも `emit` は panic せず Collector/Metrics/
  Diagnostics state を変更しない。`emit` は batch を消費するだけで
  `ObservationBatch`/`MetricsSnapshot` を変更しない。
- R-11 依存: default build（feature OFF）で `tracing_export` module・
  `dep:tracing` は存在しない。`tracing-export` のみで `dep:tracing`
  が有効で `tracing-subscriber`/`tracing-appender` は必須にならない。

## Examples

- EX-1 counter OnIncrease: `min_delta=2, min_interval=1s, impact=High`
  の counter が baseline 確立後、delta 蓄積 3・1.5s 経過で WARN event
  1 件（`delta`/`total`/window field 付き）。同 poll 再 emit・間隔
  未満の再 poll では追加 event なし。
- EX-2 SnapshotOnly counter: 増分があっても event 0 件、
  `snapshot_suppressed` が計数される。
- EX-3 discontinuity: counter 減少で `validity="discontinuous"`・
  `issue="counter_decreased"`・`delta_present=false` の event が
  validity floor で WARN 以上になる。
- EX-4 gauge OnIncrease: `value` を report-signal とし増分閾値で
  gate される。
- EX-5 sink: `Healthy → Degraded` で `event_code="diagnostics.health_transition"`・
  Level=WARN の event。sink panic は `panics` 計数のみで engine を
  落とさない。
- EX-6 Dispatch::none(): subscriber なしで `emit` が panic せず
  `EmitStats` を返す。

## Guarantees

- `emit`/`FindingSink::emit` は非 RT（alloc・lock・scoped dispatch
  を伴う）— RT path から呼ばない・tracing を呼ばない。
- `tracing` event の field は固定 schema で golden test 可能。
  `observed_at`/`window` 時刻は host monotonic `Instant` 基準の
  相対 microsecond であり wall-clock を捏造しない。
- `removed()` の identity は emit 処理後に policy state から除去し、
  state は登録数に比例して増え続けない。

## Non-goals

- `EventRecord`→event・`CorrelationBook` unlinked・W3C Trace
  Context・RT correlation token・thread/async span context 伝播・
  両 Consumer 検証 — NotRun（#281/#293 責務）。
- OTLP/metrics exporter・global subscriber 設置・Collector への
  tracing 結合 API。

## Risk

- `Instant` に epoch anchor がない → exporter 初回観測を t0 とする
  相対 `*_mono_us` に限定し、実時刻を装わない。
- `min_interval` は `Instant` 差で評価 — host clock domain 内で
  一貫。`clock_anomaly` window でも saturating で安全側に倒す。
