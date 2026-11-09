---
id: "DESIGN-034UBq1SnAHmlvoFnohCoS"
type: "design"
status: "approved"
title: "Issue #278 OBS-03 tracing_export slice 2: EventRecord export + Correlation unlinked + W3C Trace Context"
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
  implements:
    - "docs/specs/issue-278-observability-tracing-export-records.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-277-observability-core-source-contract.md"
    - "docs/specs/issue-280-rt-record-bridge-clock-correlation.md"
    - "docs/specs/issue-278-observability-tracing-export.md"
  blocks:
    - "docs/plans/implement-issue-278-observability-tracing-export-records.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-278-observability-tracing-export.md"
    - "docs/designs/issue-280-rt-record-bridge-clock-correlation.md"
    - "docs/designs/foundation-observability-minimal-api.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-278-observability-tracing-export-records.md"
  derived-by:
    - "docs/test-specs/issue-278-observability-tracing-export-records-tests.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-278-observability-tracing-export-records-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/275"
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/293"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/293"
  deferred-by: []
---

# Issue #278 OBS-03 tracing_export slice 2: EventRecord export + Correlation unlinked + W3C Trace Context

## Context

`tracing_export`（slice 1, PR #507）は `ObservationBatch` →
`tracing` event と `DiagnosticsEvent` → `tracing` event の 2 経路を
確定した。Issue #278 の残件として、OBS-05（#280）が供給する
`EventRecord`/`RecordHeader`/`ClockContext`/`CorrelationBook`
語彙を `tracing` event へ変換する経路と、W3C Trace Context の
nonRT adapter が未実装である。OBS-05 spec は「tracing への変換・
export フォーマット → OBS-03」と責務を明記しており、本 slice が
その契約を固定する。

## Scope

### In Scope

- `tracing_export::trace_context`: `TraceContext` 値型
  （trace-id/parent-span-id/flags/version）と W3C `traceparent`
  の parse/`to_traceparent` 再形式化。
- `tracing_export::records`: `TracingRecordExporter` —
  `&[ReceivedRecord<EventRecord>]` + `&CorrelationBook` +
  `Option<&TraceContext>` を `rt_record` event へ変換する非 RT
  pull exporter。`RecordEmitStats` で損失/相関を別計数。
- `tracing_export::schema`: `EVENT_CODE_RECORD`・record 用
  enum→文字列写像（`confidentiality`/`correlation_status`/
  `unlinked_reason`/`clock_status`/`clock_provenance`）・
  trace field key の追加。
- `tracing_export::mod`/`lib.rs`: re-export と module doc 更新
  （additive のみ）。`exporter.rs`（`TracingExporter`）は変更
  しない — slice 1 の event schema/`event_version` を凍結し、
  measurement/window/health event への trace field 付与は
  Deferred（別 slice で version bump とセット）。
- contract/golden tests（`tests.rs` 拡張）。

### Out of Scope

- `tracestate`/Baggage、span 生成・親子関係、外部 propagator
  （OTLP/Tracy）、runner への record drain 配線、両 Consumer
  検証、`Confidential` 値秘匿 — spec の Out of Scope 節どおり
  （#281/#293/別 slice）。

## Components and Boundaries

- `tracing_export::trace_context::TraceContext` — `Copy` 値型。
  `trace_id: u128`・`parent_span_id: u64`・`flags: u8`・
  `version: u8` を保持。`new` は全ゼロ id・version `0xff` を
  `None` で拒否、`parse(&str) -> Option<Self>` は W3C
  `traceparent` の構文検証のみ（認可・信頼判定はしない）、
  `to_traceparent(&self) -> String` で再形式化（version 保持）。
  `is_sampled()` は `flags & 0x01`。hex は小文字のみ受理。
- `tracing_export::records::TracingRecordExporter` —
  `{ dispatch, t0: Option<Instant>, max_events_per_drain: u64 }`。
  `emit_records(&mut self, observed_at: Instant,
  records: &[ReceivedRecord<EventRecord>], book: &CorrelationBook,
  trace: Option<&TraceContext>) -> RecordEmitStats`。
  record ごとに `book.classify`（`&self` のみ）で correlation を
  解決し、cap 超過は `cap_dropped` 計数で残りを打ち切る
  （baseline state を持たないため未処理 record の再評価は不要 —
  次回 drain で同じ record は来ない）。
- `tracing_export::records::RecordEmitStats` — `emitted`/
  `cap_dropped`/`correlation_linked`/`correlation_unlinked`/
  `correlation_none` の別計数（`EmitStats` と同じ「損失種別を
  混同しない」規約）。
- `schema.rs` — `EVENT_CODE_RECORD = "rt_record"`、
  `confidentiality_str`/`correlation_status_str`/
  `unlinked_reason_str`/`clock_status_str`/`clock_provenance_str`
  の単一正本。`#[non_exhaustive]` enum の未知 variant は
  `"unknown"` sentinel に写像。
- 既存 `TracingExporter`/`TracingFindingSink`/`logging`/
  `collector`/`diagnostics`/`record`/`rt_bridge` は公開 API を
  変更しない（`emit` の内部委譲化のみ）。

## Data and Control Flow

1. host/domain が RT 側で `EventRecord` を生成し `RtRecorder` に
   積む（本 slice の対象外・既存契約）。
2. 非 RT 側で host が `RecordSink::drain`/`drain_final` の
   callback で `ReceivedRecord<EventRecord>` を有界 `Vec` に
   集める（ring capacity が上限）。
3. host が `CorrelationBook`（issue/remint/expire の権限を持つ
   非 RT registry）と、必要なら `TraceContext` を用意し、
   `TracingRecordExporter::emit_records(now, &records, &book,
   trace)` を呼ぶ。
4. exporter は各 record について:
   - `header.confidentiality()`/`impact()`/`source()`/
     `generation()`/`event()`/`version()`/`value`/`dropped_before`
     を field 化。
   - `header.clock()`: `Some(ctx)` → `clock_status="stamped"` +
     domain/epoch/position/timebase/provenance field、
     `None` → `clock_status="unstamped"` で clock field は
     sentinel（`clock_domain=0` 等）— 捏造しない。
   - `header.correlation()`: `None` → `correlation_status="none"`、
     `Some(link)` → `book.classify(link)`:
     `Linked` → `"linked"` + token/epoch field、
     `Unlinked(r)` → `"unlinked"` + `unlinked_reason` field。
     いずれの場合も span/parent field は生成しない。
   - severity = `min(impact_level(impact),
     dropped_before>0 ? WARN : TRACE)`（`Level::Ord` は verbosity
     順のため `min` が高 severity — slice 1 の知見を踏襲）。
   - `cap_dropped` 超過分は emit せず計数のみ。
   - trace が `Some` なら `trace_id`（32 hex）/
     `parent_span_id`（16 hex）/`trace_flags`/`trace_sampled` を
     `rt_record` event に付与（`Option` field — `None` では
     field 自体を出さない）。
5. `TracingExporter`（measurement/window/health event）は
   変更しない。trace field の他 event への展開は slice 1 schema
   の version bump を伴う別 slice とする。

## Non-Functional Constraints

- nonRT only: `emit_records` は alloc（`String` hex 化・slice
  反復）と scoped dispatch を伴う。`RecordSink::drain` の
  callback 内で直接 `tracing` を呼ばず、drain → emit を分離する
  形を doc で固定する。
- 失敗分離: exporter は `&[ReceivedRecord]`/`&CorrelationBook`/
  `&ObservationBatch` を読むだけ。`CorrelationBook` の変更系
  （`issue`/`remint`/`expire`）は `&mut self` が必要なため型上
  呼べない。subscriber 失敗は host pipeline の責務。
- 依存: `tracing`（optional 既存）のみ。W3C parse は自前実装で
  新規 crate なし。`unsafe` なし。公開 enum は新規に作らない
  （`TraceContext` は struct、stats は struct）→
  `#[non_exhaustive]` 追加対象なし。
- `#[non_exhaustive]` enum（`CorrelationStatus` 系・
  `TimestampProvenance`・`Confidentiality`・`ReportingImpact`）の
  未知 variant は wildcard arm で `"unknown"`/安全側に写像し、
  将来 variant 追加で exhaustive match が壊れないようにする。

## Risks and Trade-offs

- **hex 文字列化の alloc**: `trace_id`/`parent_span_id` を event
  field に載せるため `String` 化が必要。非 RT・1 record あたり
  高々 2 alloc で許容範囲。`tracing::field::display` で
  `TraceContext` に `fmt::Display` を実装すれば alloc を
  subscriber 側に寄せられるが、field 値として hex 文字列が
  schema 上明確なため `String` を選ぶ。
- **`to_traceparent` の version 保持**: parse した version を
  保持して再形式化する。version `00` 以外で parse した context
  を `00` に潰すと将来 field との整合が崩れるため version を
  field に保持する。
- **小文字 hex 限定**: W3C は小文字を規定。大文字混じりを受理
  する実装もあるが、厳密側（reject → `None`）を選び、consumer
  側の寛容さに委ねる。test で明示する。
- **`dropped_before` の意味**: 「この record の直前に落ちた件数」
  であり record 自体の欠落ではない。WARN floor は「欠落の存在を
  観測した最初の生存 record」にだけ掛かり、連続損失のたびに
  WARN が出るのは意図どおり（損失を黙らせない）。
- **cap の再評価なし**: record は状態を持たない pull 変換のため
  `cap_dropped` 分を次回に持ち越さない（drain 済み record は
  二度と来ない）。損失は `cap_dropped` 計数と `dropped_before`
  field で観測可能。

## Deferred Design Concerns

- `tracestate`/Baggage、span 生成、外部 propagator — spec の
  Deferred Design Concerns 節どおり（#281/#293/別 slice）。
- runner への record drain 配線 — OBS-11 残件/host 責務。
- async/task span context 伝播 — host の `Instrument` 責務。

## Verification Notes

- [ ] test-spec の R-1〜R-12 を `#[cfg(all(test,
  feature = "tracing-export"))]` の contract test で検証。
- [ ] `TraceContext` parse/format は `trace_context.rs` 内の
  unit test + emit 経由の field 検証。
- [ ] feature OFF build で `dep:tracing` 非含有・
  `tracing_export` module 非存在を確認（slice 1 と同じ手法）。
