---
id: "SPEC-034UBnaDfEZMAipNtQoTeu"
type: "spec"
status: "approved"
title: "Issue #278 OBS-03 tracing_export slice 2 — EventRecord export + Correlation unlinked + W3C Trace Context"
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
    - "docs/specs/issue-280-rt-record-bridge-clock-correlation.md"
    - "docs/specs/issue-278-observability-tracing-export.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/foundation-observability-minimal-api.md"
    - "docs/designs/shared-foundations-and-modularization-policy.md"
    - "docs/designs/issue-278-observability-tracing-export.md"
    - "docs/designs/issue-280-rt-record-bridge-clock-correlation.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-278-observability-tracing-export.md"
    - "docs/specs/issue-280-rt-record-bridge-clock-correlation.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-278-observability-tracing-export-records-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/275"
    - "https://github.com/scarecrow173/ruviform/issues/277"
    - "https://github.com/scarecrow173/ruviform/issues/279"
    - "https://github.com/scarecrow173/ruviform/issues/280"
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/289"
    - "https://github.com/scarecrow173/ruviform/issues/293"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/293"
  deferred-by: []
---

# Issue #278 OBS-03 tracing_export slice 2 — EventRecord export + Correlation unlinked + W3C Trace Context

## What Should Be Built

既存 `ruviform-observability::tracing_export`（feature
`tracing-export`）に、slice 1（PR #507）で deferred された残件のうち
本リポジトリ内で完結する変換層を追加する:

- `TracingRecordExporter`: `rt_bridge::RecordSink` が drain した
  `ReceivedRecord<EventRecord>` のスライスを、固定 schema の
  `tracing` event（`event_code = "rt_record"`）へ変換する非 RT
  exporter。`RecordHeader` の `EventId`/`version`/`SourceIdentity`/
  domain `generation`/`confidentiality`/`impact`、`value`、および
  `ReceivedRecord::dropped_before`（位置付き欠落）を field として
  保持する。`ClockContext` は clock domain/epoch/position/timebase/
  provenance をそのまま field 化し、wall-clock への換算・捏造は
  行わない。
- `CorrelationBook` 連携: record header の `CorrelationLink` を
  `CorrelationBook::classify` で非 RT 解決し、`linked` /
  `unlinked`（`unknown_token`/`superseded_epoch`/`expired`）/
  `none`（意図的非相関）を区別して field に記録する。失効・誤世代
  の token は unlinked として報告し、偽の親子 Span・trace 関係を
  生成しない。record event は常に flat な event であり、correlation
  は field データに留まる。
- `TraceContext`: W3C Trace Context `traceparent` の検証・構築・
  再形式化を行う値型。`TracingRecordExporter::emit_records` に
  `Option<&TraceContext>` を渡すと `rt_record` event に
  `trace_id`/`parent_span_id`/`trace_flags`/`trace_sampled` field
  が付与される。parse は構文検証のみを行い、得られた context は
  相関 metadata であって認可の証拠ではない。親 Span の生成・
  Baggage の伝播・tracestate の引き継ぎは行わない。
  `metric_observation`/`observation_window`/`diagnostics.health_
  transition` event への trace field 付与は本 slice では行わず、
  schema version を据え置く（Deferred Design Concerns 参照）。
- feature `tracing-export` のみで有効化（新規外部依存なし。
  `tracing` optional dep のみ）。

## Why Now

- Issue #278 の完了条件のうち、`EventRecord`→event 変換・
  `CorrelationBook` unlinked 処理・W3C Trace Context（有効/不正/
  未信頼入力）・RT Correlation token 寿命の試験が未実装のまま
  残っている（slice 1 の impl record / Issue コメント記載の残件）。
- OBS-05（#280）の `record`/`rt_bridge` は main に実装済みで、
  「tracing span/event への変換・export フォーマット → OBS-03
  （#278）」と責務が明記されている。OBS-11（#293）の runner は
  `ObservationBatch`/`DiagnosticsEvent` の emit まで配線済みだが、
  record drain の export 変換は未接続であり、本 slice がその
  変換契約を固定する。
- Consumer（AudioLens/Synvelis）に独自 mapping を要求しないため、
  Ruviform 提供 Telemetry の `tracing` 変換を本クレートが所有する
  （Issue #275 合意事項 3）。

## Users and Value

- **OBS-11（#293）integration runner / Host 実装者**: `RecordSink`
  の drain 結果をそのまま `TracingRecordExporter::emit_records` に
  渡せる確定 API・固定 schema・損失/相関計数を得る。drain callback
  内で `Vec` に貯めてから呼ぶ形で ring capacity に比例した有界
  batch になる。
- **Host（AudioLens/Synvelis）**: `tracing-export` feature で構築した
  `Dispatch` を `TracingRecordExporter` に注入し、record event が
  measurement/window/health event と同一 target・同一 `*_mono_us`
  規約・同一 severity 規則で `tracing` pipeline に流れる。
- **Domain 側**: `RecordHeader` の `impact`/`confidentiality` 宣言が
  そのまま severity field/分類 field として解釈され、record の
  Clock Domain 情報が失われずに export される。

## Scope

### In Scope

- `src/tracing_export/` への追加: `trace_context.rs`（`TraceContext`
  値型・`traceparent` parse/format）、`records.rs`
  （`TracingRecordExporter`/`RecordEmitStats`）、`schema.rs` への
  `rt_record` event code・record 用 field 写像の追加、
  `tests.rs` への contract/golden test 追加。新規外部依存なし。
  `exporter.rs`（`TracingExporter`）は変更しない — slice 1 の
  event schema/version を凍結する。
- `rt_record` event の固定 schema: `event_code`/`event_version`/
  `event_id`/`record_version`/`source_id`/`source_generation`/
  `domain_generation`/`value`/`dropped_before`/`confidentiality`/
  `correlation_status`/`correlation_token`/`correlation_epoch`/
  `unlinked_reason`/`clock_status`/`clock_domain`/`clock_epoch`/
  `clock_position`/`clock_timebase_num`/`clock_timebase_den`/
  `clock_provenance`/`observed_at_mono_us` + 任意の trace field
  （`trace_id`/`parent_span_id`/`trace_flags`/`trace_sampled`）。
- `CorrelationBook::classify` による link 解決と unlinked reason の
  field 記録。`CorrelationStatus`/`UnlinkedReason` は
  `#[non_exhaustive]` のため未知 variant は安全側の写像
  （`"unknown"`）に倒す。
- severity: `impact_level(header.impact())` を基底とし、
  `dropped_before > 0`（橋の drop-on-full による位置付き欠落が
  観測された）場合のみ WARN floor を適用する。ERROR は使わない。
  unlinked correlation・`Estimated` provenance は field で報告し
  severity を上げない（失効は正常な lifecycle、Estimate は record
  自身の宣言）。
- `max_events_per_drain` cap: 1 `emit_records` 呼出あたりの event 数
  上限。超過分は `cap_dropped` として計数し state を持たない
  （record は状態を持たない pull 変換のため baseline 更新は不要）。
- `TraceContext` の W3C `traceparent` 準拠 parse（version・
  trace-id・parent-id・flags の検証、version `00` は 4 field 固定・
  `ff` は不正・それ以外の version は前方互換で先頭 4 field を読む）
  と `to_traceparent` による再形式化。hex は小文字のみ受理する
  厳密側の選択とし、逸脱は `None`（context なし）として扱う。
- 契約試験: record schema golden・correlation linked/unlinked/none・
  token 寿命（expire/remint/unknown）・clock stamped/unstamped/
  Estimated・dropped_before loss・cap・Dispatch::none 安全性・
  W3C 有効/不正/未信頼入力・trace field 付与・feature OFF 非依存。

### Out of Scope

- `tracestate`/Baggage の解析・伝播（vendor 拡張・機微情報を含み
  得るため本 slice では扱わない。必要になれば別 slice）。
- `metric_observation`/`observation_window`/`diagnostics.health_
  transition` event への trace field 付与 — slice 1 の event
  schema/version を変更しないため本 slice では `rt_record` のみ。
  必要になれば `event_version` bump とセットの別 slice で扱う。
- `tracing` span の生成・親子関係の構築、`tracing-opentelemetry`
  等の外部 propagator 依存、OTLP/Tracy exporter — Issue が「必須
  OTLP/Tracy なし」と明記。
- Thread/async `tracing` Context（span）の完全な伝播保証 — 本
  slice は `with_default` scoped Dispatch 内の span context が
  event に反映されることの検証まで。task/worker 間伝播は host の
  `Instrument`/明示 dispatch 責務（#293/consumer gate、NotRun）。
- `RecordSink` drain を `ObservationRunner` に配線する統合 —
  runner 側の record drain 接続は OBS-11 残件/host 責務。本 slice
  は変換 API のみを提供し、runner の公開 API は変更しない。
- 両 Consumer（AudioLens/Synvelis）での同一 Key/Field/Severity・
  stdout 保護・Privacy 検証 — #281 の cross-repo gate（NotRun）。
- `Confidentiality::Confidential` record の値秘匿・除外 —
  export 判断は host の認可呼出しであり、本 exporter は分類 field
  を付けて忠実に emit する（filter suppression は subscriber 側
  責務）。

## Requirements

- Collector/Registry/Metrics/Diagnostics/Source/RT の公開 API・
  状態を変更しない。`emit_records` は `&[ReceivedRecord<EventRecord>]`
  と `&CorrelationBook` を読むだけで、book の `issue`/`remint`/
  `expire`（`&mut`）は呼ばない。
- record event の時刻表現は 2 系統を混同しない: `ClockContext` は
  record 自身の clock domain/ticks/timebase/epoch/provenance を
  field としてそのまま保持し、`observed_at_mono_us` は emit 呼出の
  `Instant`（非 RT 観測時刻）を exporter t0 基準の相対値として
  記録する。異なる時刻系を無根拠に変換・合成しない。
- `correlation: None`（意図的非相関）と unlinked（失効/誤世代/
  未知 token）は別の `correlation_status`/`unlinked_reason` で
  区別する。`Unlinked` の link から親子関係・span 関連 field を
  生成しない。
- `TraceContext` は相関 metadata であり認可の証拠ではない。
  parse は構文のみを検証し、得られた trace-id/span-id を認可・
  権限判定に使う API を提供しない。Baggage/tracestate/credential
  系 field を event に載せない。
- 失敗分離: `cap_dropped`・correlation 分類・record 自体の内容は
  全て field/計数で報告し、subscriber/filter/Writer の失敗は
  `LoggingHealth` 側の責務。`Dispatch::none()`・global subscriber
  未設置でも panic しない。record export の失敗が
  Collector/RT/Source/Diagnostics 正本に影響しない。
- nonRT のみ: `emit_records` は alloc・`Vec` 反復・
  `with_default` scoped dispatch を伴う非 RT 操作。RT writer
  経路・`RecordSink::drain` の callback 内から `tracing` を
  呼ぶ設計にはしない（host が drain → emit を分離）。
- `tracing-export` feature 無効の default build で `dep:tracing`
  は有効化されず、`tracing_export` module は存在しない。新規外部
  依存・強制 Subscriber・global `set_global_default`・独立
  クレートなし。`unsafe` 不使用。

## Acceptance Criteria

- [ ] AC-1: `rt_record` event が固定 schema（上記 In Scope の key
  集合）で golden test 可能に emit され、`ClockContext` の
  domain/epoch/position/timebase/provenance・`dropped_before`・
  `domain_generation`・`confidentiality` が field に保持される。
- [ ] AC-2: `CorrelationBook` 連携で linked/unlinked
  （`unknown_token`/`superseded_epoch`/`expired`）/none が区別され、
  token の expire・remint・再生成・未知・誤世代の各寿命状態で
  unlinked が正しく報告され、偽の親子関係が生成されない。
- [ ] AC-3: `TraceContext` が W3C `traceparent` の有効入力を parse
  し、`emit_records` に渡すと `rt_record` event に trace field が
  付き、不正入力（version `ff`・全ゼロ id・長さ/hex 違反・
  version `00` の余分 field）は `None` として context なしで
  安全に emit される。未信頼入力があっても認可判定・親 Span
  生成に使われない。
- [ ] AC-4: `Dispatch::none()`・subscriber なし・filter 全落としで
  `emit_records` が panic せず、Collector/ Metrics/Diagnostics/
  RT/Source 正本に影響しない。cap 超過は `cap_dropped` 計数のみ。
- [ ] AC-5: feature `tracing-export` のみで `dep:tracing` が有効化
  され、新規外部依存・`tracing-subscriber`/`tracing-appender`
  必須化・強制 Subscriber・独立クレートがない。default build で
  `tracing` 非依存を維持する。
- [ ] AC-6: scoped Dispatch 内の `tracing` span context が emit
  event に反映されることを検証し、Context 未設定でも安全な挙動を
  確認する。async/task 間伝播の完全対応は NotRun として記録する。
- [ ] AC-7: 本スリス対象外（tracestate/Baggage・span 生成・
  runner への record drain 配線・両 Consumer 検証・Confidential
  値秘匿）は PR 本文と impl record に NotRun/残件として明記し、
  Issue #278 は Open のまま残る。

## Deferred Design Concerns

<!-- Intentionally deferred future work. Link the deferred draft doc via relations.defers. -->

- `tracestate`/Baggage の parse・伝播 | vendor 拡張で機微情報を
  含み得るため privacy 検証（#281）とセットで扱う | 実需要 +
  #281 gate | baggage 経由の情報漏洩。
- `ObservationRunner` への record drain 配線 | runner の公開 API
  変更を伴うため OBS-11 側の別 slice | host/runner 要求 | 本 slice
  は変換 API のみ。
- `metric_observation`/`observation_window`/`diagnostics.health_
  transition` event への trace field 付与 | slice 1 schema の
  version bump を伴う別変更 | host 要求 | `rt_record` で W3C
  伝播の契約は確立済み。
- async/task `tracing` Context 伝播の完全対応 | `Instrument`/
  明示 dispatch は host 責務 | #293・#281 | scoped dispatch のみ
  では task 間 span context を保証しない。
- 両 Consumer 同一 Key/Field/Severity・stdout/Privacy 検証 |
  cross-repo gate | Issue #281 | worker 環境は単一 repo。

## Sources and Relations

- Issue #278（本 spec の正本・slice 境界は coordinator Spec に従う）、
  slice 1 spec `docs/specs/issue-278-observability-tracing-export.md`
  （ObservationBatch Exporter + FindingSink adapter・deferred 明記）、
  OBS-05 spec `docs/specs/issue-280-rt-record-bridge-clock-correlation.md`
  （`EventRecord`/`RecordHeader`/`ClockContext`/`CorrelationBook`/
  `CorrelationStatus`/`UnlinkedReason` の語彙と unlinked 契約）、
  OBS-02 spec `docs/specs/issue-277-observability-core-source-contract.md`
  （`SourceIdentity`/`SourceGeneration`/`Confidentiality`/
  `ReportingImpact` の語彙）。
- W3C Trace Context（`traceparent` version-trace-id-parent-id-flags）
  の構文規則。OTLP/Tracy 等の外部 propagator は依存に含めない。
