---
id: "PLAN-034UBtEBpkW4bYquguyXAB"
type: "plan"
status: "approved"
title: "Issue #278 OBS-03 tracing_export slice 2 実装計画"
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
    - "docs/designs/issue-278-observability-tracing-export-records.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-278-observability-tracing-export-records-tests.md"
    - "docs/tasks/implement-issue-278-observability-tracing-export-records.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-278-observability-tracing-export-records.md"
    - "docs/designs/issue-278-observability-tracing-export-records.md"
  derived-by:
    - "docs/tasks/implement-issue-278-observability-tracing-export-records.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-278-observability-tracing-export-records-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/278"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/293"
  deferred-by: []
---

# Issue #278 OBS-03 tracing_export slice 2 実装計画 Implementation Plan

## Goal

`ruviform-observability::tracing_export` に `EventRecord` →
`tracing` event の `TracingRecordExporter`、`CorrelationBook`
unlinked 分類、W3C `traceparent` の `TraceContext` adapter を追加する。
Collector/Registry/Metrics/Diagnostics/RT 正本は `tracing`
非依存を維持し、slice 1 の `TracingExporter` event schema は
凍結する（trace field の他 event への展開は Deferred）。

## Scope

In scope は spec の In Scope 節どおり（`src/tracing_export/` への
`trace_context.rs`/`records.rs` 追加、`schema.rs`/`exporter.rs`/
`mod.rs`/`tests.rs` の additive 変更）。Out of scope は spec の
Out of Scope 節どおり（tracestate/Baggage・span 生成・runner 配線・
両 Consumer 検証・Confidential 値秘匿は NotRun、#281/#293 責務）。
`Cargo.toml` の feature/依存は変更しない（`tracing-export` =
`dep:tracing` のみで既に十分、dev-dep `tracing-subscriber` も
slice 1 で追加済み）。

## Upstream Documents

- spec: `docs/specs/issue-278-observability-tracing-export-records.md`
- design: `docs/designs/issue-278-observability-tracing-export-records.md`
- test-spec: `docs/test-specs/issue-278-observability-tracing-export-records-tests.md`

## Design Inputs

- `EventRecord`/`RecordHeader`/`EventId`（`src/record/event.rs`）
- `ClockContext`（`src/record/clock.rs`）— domain/epoch/position/
  timebase/provenance
- `CorrelationBook`/`CorrelationLink`/`CorrelationStatus`/
  `UnlinkedReason`（`src/record/correlation.rs`）— `classify(&self)`
- `ReceivedRecord<R>`（`src/rt_bridge.rs`）— `record()`/
  `dropped_before()`
- `SourceIdentity`/`Confidentiality`/`ReportingImpact`（`src/source/`）
- `TimestampProvenance`/`ClockDomainId`/`ClockEpochId`/`TimeBase`
  （`ruviform-media-core`）
- 既存 `tracing_export`（`schema.rs` の写像/severity/mono_us、
  `exporter.rs` の `with_default` scoped dispatch・cap・EmitStats
  規約、`sink.rs` の panic 隔離）

## File Map

- `crates/ruviform-observability/src/tracing_export/trace_context.rs`
  — `TraceContext` 値型・`traceparent` parse/`to_traceparent`/
  `is_sampled`・hex 変換の単一正本。
- `crates/ruviform-observability/src/tracing_export/records.rs`
  — `TracingRecordExporter`/`RecordEmitStats`/`emit_records`。
- `crates/ruviform-observability/src/tracing_export/schema.rs` —
  `EVENT_CODE_RECORD`・record 用 enum→&str 写像・trace field key。
- `crates/ruviform-observability/src/tracing_export/mod.rs` —
  module 追加・re-export・module doc 更新。`exporter.rs` は
  無変更（slice 1 schema 凍結）。
- `crates/ruviform-observability/src/tracing_export/tests.rs` —
  contract/golden tests 追加（既存 capture Layer 基盤を再利用）。

## Implementation Sequence

1. `trace_context.rs`（parse/format/hex）→ unit test。
2. `schema.rs` 拡張（event code・写像）→ `records.rs`
   （`TracingRecordExporter`/`RecordEmitStats`）。
3. `mod.rs` 登録・re-export、`tests.rs` に contract/golden test
   追加 → doctest。
5. review-protocol → mise 検証 → impl record 更新 → commit/PR。

## Dependencies and Risks

- 依存: `tracing`（optional 既存）+ dev `tracing-subscriber`
  （既存）。新規外部依存なし。
- Risk: `Instant` に epoch なし → record exporter も独自 t0 を持ち
  `observed_at_mono_us` は emit 引数 `Instant` 基準の相対値。
  record の一次時刻は `ClockContext` field であり mono_us と混同
  しない。
- Risk: `with_default` は thread-local scoped — async/task 間伝播は
  NotRun。
- Risk: W3C version 解釈 — `00` 厳密 4 field・`ff` 拒否・他 version
  前方互換を test で固定。

## Task Breakdown

単一 task（`docs/tasks/implement-issue-278-observability-tracing-export-records.md`）
— module 規模が小さく分割不要。

## Verification

- `mise run fmt-check` / `mise run check` / `mise run lint` /
  `mise run test`（canonical）。
- 補助証拠: `cargo check|test -p ruviform-observability --features
  tracing-export`、default build での非依存確認、`cargo tree` で
  feature 分離の確認。

## Open Questions / Gaps

- record event の severity floor は `dropped_before > 0` → WARN の
  み。unlinked correlation・Estimated provenance は field 報告で
  severity 不変（design で固定済み）。
- `emit_records` の入力は `&[ReceivedRecord<EventRecord>]` slice —
  host が drain callback で有界 `Vec` に集める前提。callback 直
  下での emit は非 RT 契約上想定しない。

## Self-Review

- spec の全 In Scope 項目が file map/sequence に対応。
- Out of Scope は spec と一致し NotRun 記録先（impl record/PR）を
  明記。
- 公開 enum の新規追加なし（`TraceContext`/stats は struct）→
  `#[non_exhaustive]` 対象は発生しない。

## Implementation Handoff

- impl record:
  `docs/impl/ir/issue-278-obs-03-tracing-export-records.md`
  （実装前に `in-progress` で作成）。

## Review Handoff

- review-protocol で spec/design/plan/task + 実装 diff をレビューし、
  CONFIRMED P0/P1 を解消してから承認相当とする。
