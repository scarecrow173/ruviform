---
id: "TSPEC-034UBsnivDah67dJlEQhYq"
type: "test-spec"
status: "approved"
title: "Issue #278 OBS-03 tracing_export slice 2 試験仕様"
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
    - "docs/designs/issue-278-observability-tracing-export-records.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-278-observability-tracing-export-tests.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-278-observability-tracing-export-records.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-278-observability-tracing-export-records.md"
    - "docs/designs/issue-278-observability-tracing-export-records.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/278"
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/293"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/293"
  deferred-by: []
---

# Issue #278 OBS-03 tracing_export slice 2 試験仕様

## Purpose

`tracing_export` slice 2（`EventRecord` export・`CorrelationBook`
unlinked・W3C Trace Context）の契約を固定する。固定 schema・
correlation 分類・token 寿命・clock field・損失計数・Dispatch 注入・
W3C parse の有効/不正/未信頼入力を、capture `Layer` を使った
contract test で検証する。本書の試験は
`#[cfg(all(test, feature = "tracing-export"))]` で gate する。

## Feature

- `tracing_export::TracingRecordExporter::emit_records(&[ReceivedRecord<
  EventRecord>], &CorrelationBook, Option<&TraceContext>)` が record を
  `event_code = "rt_record"` の `tracing` event に変換し
  `RecordEmitStats` を返す。
- `tracing_export::TraceContext` が W3C `traceparent` を検証・
  構築・再形式化し、`emit_records` に渡すと `rt_record` event に
  trace field を付与する。slice 1 の `TracingExporter` event
  schema は変更しない。

## Rules

- R-1 record schema: `rt_record` event は固定 key
  （`event_code`/`event_version`/`event_id`/`record_version`/
  `source_id`/`source_generation`/`domain_generation`/`value`/
  `dropped_before`/`confidentiality`/`correlation_status`/
  `correlation_token`/`correlation_epoch`/`unlinked_reason`/
  `clock_status`/`clock_domain`/`clock_epoch`/`clock_position`/
  `clock_timebase_num`/`clock_timebase_den`/`clock_provenance`/
  `observed_at_mono_us`）を持ち、値は有界。trace 指定時のみ
  `trace_id`/`parent_span_id`/`trace_flags`/`trace_sampled` が
  追加される。
- R-2 correlation: `header.correlation() == None` は
  `correlation_status="none"`。`Some(link)` は `CorrelationBook::
  classify` で `linked`（token/epoch field 付き）または
  `unlinked`+`unlinked_reason`（`unknown_token`/
  `superseded_epoch`/`expired`）に分類される。unlinked から
  span/parent 関係を生成しない。
- R-3 token 寿命: issue→linked、expire→`unlinked`/`expired`、
  remint→旧 link は `unlinked`/`superseded_epoch`・新 link は
  `linked`、book 未発行 token は `unlinked`/`unknown_token`。
  classify は `&self` のみで book を変更しない。
- R-4 clock: `Some(ClockContext)` は `clock_status="stamped"` で
  domain/epoch/position/timebase/provenance をそのまま保持し、
  `None` は `clock_status="unstamped"` で clock field は sentinel。
  `Estimated` provenance は `"estimated"` として区別され、
  `None`（unstamped）と混同しない。wall-clock への換算・捏造なし。
- R-5 severity: `Level` は `impact_level(impact)`（Low→DEBUG /
  Normal→INFO / High→WARN）を基底とし、`dropped_before > 0` の
  ときのみ WARN floor。ERROR は使わない。
- R-6 cap/loss: `max_events_per_drain` 超過分は emit せず
  `cap_dropped` 計数。`dropped_before` は record event の field
  として保持され、cap_dropped と混同しない。
- R-7 trace context: `TraceContext::parse` は有効な `traceparent`
  を `Some` で返し、version `ff`・全ゼロ trace-id/span-id・
  長さ/hex 違反・version `00` の 4 field 超過・空 field を `None`
  で拒否する。version `01`-`fe` は前方互換で先頭 4 field を読む。
  `to_traceparent` は parse と往復一致する。emit に `Some` を
  渡すと trace field が付き、`None`/parse 失敗時は trace field
  を付けずに emit される。
- R-8 未信頼入力: 構文上有効な任意の `traceparent` は相関
  field を付けるだけで、認可判定・親 Span 生成・特権昇格に
  使われない（emit は常に flat event）。
- R-9 分離: `Dispatch::none()`・subscriber なし・filter 全落とし
  でも `emit_records` は panic せず、Collector/ Metrics/
  Diagnostics/RT/Source の状態を変更しない。emit は渡された
  slice/book を読むだけ。
- R-10 span context: `with_default` scoped Dispatch 内で enter
  した `tracing` span の context が emit event に反映される
  （capture Layer の span 情報で検証）。Context 未設定でも
  panic しない。
- R-11 依存: default build（feature OFF）で `tracing_export`・
  `dep:tracing` は存在しない。`tracing-export` のみで `dep:tracing`
  が有効で `tracing-subscriber`/`tracing-appender` は必須に
  ならない。新規外部依存なし。
- R-12 非 RT: `emit_records` は alloc・scoped dispatch を伴う
  非 RT 操作であり、RT path・`RecordSink::drain` callback から
  `tracing` を呼ぶ設計にしない（doc 契約）。

## Examples

- EX-1 linked record: `CorrelationBook::issue` で得た link を持つ
  record を emit → `correlation_status="linked"`・token/epoch
  field・`event_id`/`record_version`/`source_id#gen`・clock
  stamped field が golden 一致。
- EX-2 unlinked record: book で `expire` 済み token の link を持つ
  record → `correlation_status="unlinked"`・
  `unlinked_reason="expired"`・span field なし。
- EX-3 clock: `ClockContext` なし record → `clock_status="unstamped"`・
  clock field sentinel。`Estimated` stamp → `clock_provenance=
  "estimated"`。
- EX-4 loss: `dropped_before=3` の record → `dropped_before=3`
  field・Level が WARN floor。`max_events_per_drain=1` で 2 件中
  1 件が `cap_dropped`。
- EX-5 trace: `TraceContext::parse("00-4bf92f3577b34da6a3ce929d0e0e4736-
  00f067aa0ba902b7-01")` が `Some` → emit に渡すと `trace_id`/
  `parent_span_id`/`trace_sampled=true` field。`"ff-..."` や
  全ゼロ id は `None` → trace field なしで emit。
- EX-6 span context: injected Dispatch 上で `info_span!` を
  entered してから `emit_records` → capture された event がその
  span を parent として記録する。`Dispatch::none()` では panic
  なし。

## Guarantees

- `emit_records` は非 RT（alloc・scoped dispatch）— RT path・
  drain callback から呼ばない。
- `rt_record`/trace field は固定 schema で golden test 可能。
  `observed_at_mono_us` は emit 呼出 `Instant` を exporter t0
  基準の相対 microsecond とし、wall-clock を捏造しない。
- correlation 分類は `CorrelationBook::classify` の結果をそのまま
  field に写し、独自の有効期限推論を行わない。

## Non-goals

- `tracestate`/Baggage の parse・伝播、span 生成・親子関係、
  OTLP/Tracy 等の外部 propagator — NotRun（別 slice/依存追加が
  必要）。
- `ObservationRunner` への record drain 配線 — #293/host 責務。
- async/task `tracing` Context の完全伝播 — host の
  `Instrument`/明示 dispatch 責務。
- 両 Consumer 同一 Key/Field/Severity・stdout/Privacy — #281
  cross-repo gate。
- `Confidentiality::Confidential` record の値秘匿 — host の認可
  呼出し。exporter は分類 field を付けて忠実に emit する。

## Risk

- `Instant` に epoch anchor がない → exporter 初回観測を t0 と
  する相対 `*_mono_us` に限定（slice 1 と同一規約）。
- W3C parse の version 解釈ミス → version `00` 厳密・`ff` 拒否・
  他 version 前方互換という規則を test で固定する。
- `CorrelationBook` の `#[non_exhaustive]` enum 追加 → wildcard
  arm で `"unknown"` に写像し、追加時に compile error ではなく
  安全側の field 値になる。
