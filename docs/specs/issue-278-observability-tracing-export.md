---
id: "SPEC-034UpqR38pxxqJjjHLaFBG"
type: "spec"
status: "approved"
title: "Issue #278 OBS-03 tracing_export — ObservationBatch Exporter + FindingSink adapter slice"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/278"
    - "https://github.com/scarecrow173/ruviform/issues/275"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "https://github.com/scarecrow173/ruviform/issues/278"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-277-observability-core-source-contract.md"
    - "docs/specs/issue-289-observability-collector-metrics.md"
    - "docs/specs/issue-291-observability-diagnostics.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/foundation-observability-minimal-api.md"
    - "docs/designs/shared-foundations-and-modularization-policy.md"
    - "docs/designs/issue-289-observability-collector-metrics.md"
    - "docs/designs/issue-291-observability-diagnostics.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
    - "docs/adr/obs-05-observability-leaf-primitive-deps.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-277-observability-core-source-contract.md"
    - "docs/specs/issue-289-observability-collector-metrics.md"
    - "docs/specs/issue-291-observability-diagnostics.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-278-observability-tracing-export-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/275"
    - "https://github.com/scarecrow173/ruviform/issues/277"
    - "https://github.com/scarecrow173/ruviform/issues/279"
    - "https://github.com/scarecrow173/ruviform/issues/280"
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/289"
    - "https://github.com/scarecrow173/ruviform/issues/291"
    - "https://github.com/scarecrow173/ruviform/issues/293"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/293"
  deferred-by: []
---

# Issue #278 OBS-03 tracing_export — ObservationBatch Exporter + FindingSink adapter slice

## What Should Be Built

既存 `ruviform-observability` クレート内に `src/tracing_export/` モジュールを
新設し、feature `tracing-export`（既定 OFF・`logging` feature とは独立）で
ゲートする。Issue #289（OBS-07）の `Collector::poll(now) ->
ObservationBatch` が返す型付き batch と、Issue #291（OBS-09）の
`FindingSink`/`DiagnosticsEvent` port を消費して、標準 `tracing` への
**非 RT 変換層**を提供する。具体的には:

- `TracingExporter::emit(&ObservationBatch)`: batch の各 `Measurement` を
  固定 schema の `tracing` event へ変換する。Collector との境界は
  `ObservationBatch` のみで、Collector/Registry/Metrics 正本は
  `tracing` に一切依存しない。
- `TracingFindingSink`: `DiagnosticsEvent`（Health 遷移）を `tracing`
  event に写す `FindingSink` adapter。`Diagnostics::set_sink` に
  `Box::new(TracingFindingSink::new(dispatch))` としてそのまま渡せる。
- host が `tracing::Dispatch` を明示注入する。強制 Subscriber・独立
  クレート・global dispatcher 設置は行わない。

`EventRecord`→event 変換・`CorrelationBook` unlinked 処理・W3C Trace
Context adapter・runner 配線は本スライスの対象外（Issue #278 残件、
OBS-11 #293 の責務として記録）。

## Why Now

- OBS-02（#277）source 契約・OBS-05（#280）record/rt_bridge・OBS-07
  （#289）Collector・OBS-09（#291）Diagnostics が main に実装済み。
  `metrics/mod.rs` は tracing export を想定 consumer として明記しており、
  `diagnostics/sink.rs` は「The real `tracing` adapter is deferred to
  OBS-03 (#278)」と本 Issue を名指しで deferred している。
- OBS-04（#279）Plugin Host counter 登録が `OnIncrease` reporting hint
  を宣言済みであり、Exporter 側の解釈契約（SnapshotOnly は Event 化
  しない・min_delta/min_interval/impact の意味）を固定する段階にある。
- Consumer（AudioLens/Synvelis）に独自 Delta/Exporter を実装させない
  ため、Ruviform 提供 Telemetry の `tracing` 変換を本クレートが所有する
  （Issue #275 合意事項 3）。

## Users and Value

- **OBS-11（#293）integration runner 実装者**: `emit(&ObservationBatch)`
  と `TracingFindingSink` の確定 API・固定 schema・損失計数を得る。
- **Host（AudioLens/Synvelis）**: `logging` feature で構築した
  `Dispatch` をそのまま `TracingExporter`/`TracingFindingSink` に注入
  でき、Collector poll 結果と Diagnostics 遷移が同一 schema で
  `tracing` pipeline に流れる。Logging OFF でも Collector snapshot は
  OBS-07 のまま利用可能。
- **Domain 側**: `ReportingHint::OnIncrease { min_delta, min_interval,
  impact }` / `SnapshotOnly` の宣言がそのまま Exporter の抑制・severity
  規則として解釈され、retry・毎 poll・warmup を ERROR/WARN にしない。

## Scope

### In Scope

- `src/tracing_export/` モジュール群（`mod.rs` / `schema.rs` /
  `exporter.rs` / `sink.rs` / `tests.rs` を想定）と `lib.rs` への
  feature ゲート付き module 登録・crate doc 更新。
- `Cargo.toml`: `tracing-export = ["dep:tracing"]` feature 追加と、
  試験用 `tracing-subscriber` dev-dependency 追加のみ。新規外部依存
  なし（`tracing` optional dep は既存）。
- `ObservationBatch → tracing event` 変換: 固定 schema
  （`event_code`/`event_version`/`observation_key`/`source_id`/
  `source_generation`/`scope`/`lifecycle`/`validity`/`issue`/`loss`/
  `observed_at_mono_us`/`poll_generation` + kind 別値フィールド +
  poll 単位の window/batch 集約 event）。
- `ReportingHint` 解釈: `SnapshotOnly` は Event 化しない。
  `OnIncrease { min_delta, min_interval, impact }` は「最後の発行
  （または初観測 baseline）以降の report-signal 増分が `min_delta`
  以上、かつ前回発行から `min_interval` 経過」に加え、signal 増分
  または validity/issue/lifecycle の実変化がある観測のみ発行
  （無変化の再観測・同一 batch の再 emit は dedup で抑制）。
  report-signal は kind 別に `cumulative_since_registration`
  （Counter）/ `value`（Gauge）/ `total_approx`（Histogram）の
  単調信号。
- Severity: `MeasurementValidity` の floor（Partial/Discontinuous →
  WARN、Stale → INFO）と `ReportingImpact`（Low → DEBUG /
  Normal → INFO / High → WARN）の大きい方。ERROR は使わない
  （Counter 増分・Retry・毎 poll を ERROR にしない）。
- `max_emissions_per_poll` cap: 1 `emit` 呼出あたりの measurement
  event 数上限。超過分は `cap_dropped` として計数（policy 抑制と区別）。
- `DiagnosticsEvent::HealthTransition` → `tracing` event 変換
  （`TracingFindingSink`）。severity は `to` の `HealthState` 由来
  （OBS-09 finding severity 写像と整合: Degraded → WARN /
  Unhealthy → ERROR / Unknown・Healthy → INFO）。
- `tracing::Dispatch` の host 注入と `dispatcher::with_default`
  scoped 発行。Context 未設定・global subscriber 未設置でも安全
  （dispatch が捨てるだけで panic しない）。
- 契約試験: schema golden・baseline/delta/window/generation・
  severity・抑制/上限/損失計数・Dispatch 注入・sink 変換・
  feature OFF での非依存。

### Out of Scope

- `EventRecord`/`RecordHeader` → event 変換、`CorrelationBook` の
  unlinked 処理、`RtCorrelationRef`・Source Generation を使った RT
  相関 token 解決、W3C Trace Context adapter（有効/不正/未信頼入力の
  試験を含む）— 本スライスでは NotRun。runner 配線は OBS-11（#293）
  の責務。
- Thread/async Context（`tracing` span context）の完全な伝播保証 —
  本スライスは `with_default` scoped Dispatch のみ。未対応部分は
  NotRun として記録。
- 両 Consumer（AudioLens/Synvelis）での同一 Key/Field/Severity・
  stdout 保護・Privacy 検証 — #281 の cross-repo gate（NotRun）。
- RT Correlation token 寿命・Clock 不明/欠落/再生成/不正時の unlinked
  処理 — #293/consumer gate（NotRun）。
- Collector/`poll_and_emit`/`start_background(dispatch)` のような
  tracing 結合 API — Collector は `tracing` 非依存を維持し、統合は
  host/runner が `emit(&batch)` を呼ぶ形に限定。
- `tracing` 以外の exporter（OTLP/metrics backend 等）、独立
  `ruviform-observability-tracing` クレート、Ruviform 独自
  Span/Logger framework。

## Requirements

- Collector との境界は `ObservationBatch` のみ。`emit` は batch を
  消費するだけで Collector/Registry/Metrics/Source/Diagnostics の
  公開 API を変更せず、それらは `tracing` 型に依存しない
  （`tracing-export` feature 無効の default build で依存ゼロを維持）。
- Event schema は固定キー・固定型で golden test 可能とし、
  `event_version` で将来の互換性を版管理する。`observed_at` は
  Counter 読取時刻（`Instant`）であり実事象時刻ではない —
  wall-clock への無根拠な変換は行わず、host monotonic clock 上の
  相対 microsecond（`*_mono_us`）として正直に記録する。
- `SnapshotOnly` は Event 化しない。`OnIncrease` の `min_delta`/
  `min_interval`/`impact` は domain 宣言どおりに解釈し、Retry・
  毎 poll・正常 warmup を ERROR/WARN にしない。Source/Schema を
  無制限に Event へ展開しない（`max_emissions_per_poll`・kind 別の
  有界フィールド）。
- 失敗分離: `max_emissions_per_poll` 超過・policy 抑制・valueless
  measurement を別の損失/抑制種別として計数する。filter
  suppression・Sampling・Writer error は subscriber/logging 側の
  責務（`LoggingHealth`）であり Exporter は関与しない。
  Exporter/Subscriber が失敗・停止しても Collector/RT/Source/
  Diagnostics 正本に影響しない。logger 自己再帰なし。
- nonRT のみ: `emit`/sink `emit` は alloc・`HashMap`・`with_default`
  scoped dispatch を伴う非 RT 操作。RT writer 経路から `tracing`
  event・Subscriber・format・I/O・alloc・lock を呼ばない。
- Dispatch/Layer 選択は host が明示注入。強制 Subscriber・global
  `set_global_default`・独立クレートなし。`logging` feature との
  結合は host 側の `Dispatch` 共有のみで、feature 同士は独立。
- 全公開 enum は `#[non_exhaustive]`。`unsafe` 不使用。新規外部
  依存なし（`tracing` optional dep 流用 + dev-dep `tracing-subscriber`）。

## Acceptance Criteria

- [ ] AC-1: Collector との境界が `ObservationBatch` のみで、
  `tracing` 非依存をコンパイル検証（`tracing-export` feature 無効の
  default build で collector/diagnostics が `tracing` に依存しない
  こと、`--features tracing-export` でのみ `dep:tracing` が有効）。
- [ ] AC-2: Counter baseline/delta/window/schema/Severity/Source
  Generation の Golden test がある（登録 baseline・delta 蓄積・
  `OnIncrease` 抑制・min_interval pacing・impact/validity → Level・
  `SourceId`/`SourceGeneration` フィールド・poll window 集約 event）。
- [ ] AC-3: nonRT Dispatch 注入・Context 未設定でも安全な挙動
  （global subscriber なし・`with_default` scoped・panic なし）。
- [ ] AC-4: Logging OFF / Filter suppression / Exporter failure /
  cap 超過で Metric 正本（Collector/Diagnostics）への影響なし —
  Exporter は pull 型で batch/snapshot を変更せず、cap_dropped は
  計数のみ。
- [ ] AC-5: `tracing-export` feature だけで必要な標準依存
  （`dep:tracing`）が有効、`tracing-subscriber`/`tracing-appender` を
  必須にしない。強制 Subscriber・独立クレートなし。
- [ ] AC-6: `DiagnosticsEvent` → `tracing` event の `TracingFindingSink`
  が `FindingSink` trait を実装し、`Diagnostics::set_sink` にそのまま
  渡せる。Health 遷移の severity 写像が OBS-09 finding severity と
  整合する。
- [ ] AC-7: 本スライス対象外（EventRecord 変換・W3C Trace Context・
  RT correlation・Thread/async context 伝播・両 Consumer 検証）は
  PR 本文と impl record に NotRun/残件として明記し、Issue #278 は
  Open のまま残る。

## Deferred Design Concerns

<!-- Intentionally deferred future work. Link the deferred draft doc via relations.defers. -->

- `EventRecord`/`RecordHeader` → `tracing` event 変換・`CorrelationBook`
  unlinked 処理・RT 相関 token | 責務は OBS-11（#293）runner 配線 +
  consumer gate | #293 実装時 | 本スライスは `ObservationBatch` 境界と
  `FindingSink` adapter に限定。
- W3C Trace Context の nonRT 外部伝播 adapter（有効/不正/未信頼入力の
  試験） | 責務は OBS-03 残件・#293 | 実需要 | optional adapter であり
  必須 OTLP/Tracy ではない。
- Thread/async `tracing` Context 伝播の完全対応 | OBS-11/consumer
  gate | #293・#281 | `with_default` scoped Dispatch のみでは span
  context の task/worker 間伝播を保証しない。
- 両 Consumer での同一 Key/Field/Severity・stdout 保護・Privacy 検証
  | cross-repo gate | Issue #281 | worker 環境は単一 repo。
- RT Correlation token 寿命・Clock 不明/欠落/再生成の unlinked 処理
  | #293/consumer gate | 実機検証 | RT path から `tracing` を呼ばない
  契約の検証は別 gate。

## Sources and Relations

- Issue #278（本 spec の正本・スライス境界は coordinator Spec に従う）、
  親設計 Issue #275、先行 Issue #277（OBS-02）・#289（OBS-07）・
  #291（OBS-09）。
- `docs/specs|designs/issue-277-observability-core-source-contract.md`
  （`SourceIdentity`/`SourceGeneration`/`ReportingHint`/`ReportingImpact`/
  `ObservationScope`/`Confidentiality` の語彙）、
  `docs/specs|designs/issue-289-observability-collector-metrics.md`
  （`ObservationBatch`/`Measurement`/`MeasurementValidity`/
  `MeasurementIssue`/`MetricValue`/`ObservationWindow` の消費契約）、
  `docs/specs|designs/issue-291-observability-diagnostics.md`
  （`FindingSink`/`DiagnosticsEvent`/`SinkPolicy` port と deferred
  明記）、`docs/designs/foundation-observability-minimal-api.md`
  （`tracing_export` モジュールの位置づけ）、
  `docs/designs/shared-foundations-and-modularization-policy.md`
  （Observability/Diagnostics/Logging の責務区分）。
