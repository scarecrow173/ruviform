---
id: "IMPL-034UBve3q5ki4mCV8cLprd"
type: "impl"
status: "in-progress"
title: "Issue #278 OBS-03 tracing_export slice 2 実装記録"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source: []
  changes:
    added:
      - "crates/ruviform-observability/src/tracing_export/trace_context.rs"
      - "crates/ruviform-observability/src/tracing_export/records.rs"
    modified:
      - "crates/ruviform-observability/src/tracing_export/schema.rs"
      - "crates/ruviform-observability/src/tracing_export/mod.rs"
      - "crates/ruviform-observability/src/tracing_export/tests.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/implement-issue-278-observability-tracing-export-records.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-278-observability-tracing-export-records.md"
    - "docs/designs/issue-278-observability-tracing-export-records.md"
    - "docs/test-specs/issue-278-observability-tracing-export-records-tests.md"
    - "docs/plans/implement-issue-278-observability-tracing-export-records.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/test-specs/issue-278-observability-tracing-export-records-tests.md"
  verified-by: []
  references: []
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/293"
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #278 OBS-03 tracing_export slice 2 実装記録

## Summary

`tracing_export` に `EventRecord` → `tracing` event の
`TracingRecordExporter`（`rt_record` 固定 schema）、
`CorrelationBook::classify` による linked/unlinked 分類、W3C
`traceparent` の `TraceContext` adapter を追加した。slice 1 の
`TracingExporter`/`TracingFindingSink` とその event schema・
`event_version` は無変更で凍結し、trace field は `rt_record`
のみに限定した。

## Context

Issue #278（OBS-03）の第 2 slice。第 1 slice（`ObservationBatch`/
`DiagnosticsEvent` → tracing、PR #507 系）は main 入り済みで、
本 slice は残る in-repo 受入範囲 — RT record export・
Correlation unlinked 処理・W3C Trace Context — を担う。
spec → design → test-spec → plan → task の文書チェーンは
`docs/{specs,designs,test-specs,plans,tasks}/` の
`issue-278-observability-tracing-export-records*` 各ファイル。

## Implementation

### Changes

- `src/tracing_export/trace_context.rs`（新規）: `TraceContext`
  値型。`new`/`parse`/`to_traceparent`/`is_sampled`/hex accessor。
  W3C `traceparent` の構文検証のみ（小文字 hex・version `ff`
  拒否・全ゼロ id 拒否・version `00` は 4 field 固定・他
  version は先頭 4 field を前方互換で読む）。認可・信頼判定なし。
- `src/tracing_export/records.rs`（新規）:
  `TracingRecordExporter`/`RecordEmitStats`/
  `DEFAULT_MAX_EVENTS_PER_DRAIN`(=256)/`emit_records`。
  `&[ReceivedRecord<EventRecord>]` + `&CorrelationBook` +
  `Option<&TraceContext>` を読み、record ごとに `classify` で
  correlation を解決して `rt_record` event を scoped dispatch で
  emit。cap 超過は `cap_dropped` 計数のみ（record は状態を持た
  ない pull 変換のため baseline 更新なし）。
- `src/tracing_export/schema.rs`: `EVENT_CODE_RECORD`、
  `correlation_status`/`clock_status` 定数、
  `confidentiality_str`/`unlinked_reason_str`/
  `clock_provenance_str`/`record_level` を追加。
- `src/tracing_export/mod.rs`: module 登録・re-export・doc 更新。
- `src/tracing_export/tests.rs`: contract/golden test 9 件 +
  fixture 追加（実 `bridge` 経由で `ReceivedRecord` を生成し
  `dropped_before` を実契約から取得）。

### Decisions During Implementation

- **slice 1 schema 凍結**: 当初 plan では `TracingExporter` に
  `emit_with_trace` を足し measurement/window event にも trace
  field を付ける案だったが、`event_version` bump を伴う破壊的
  schema 変更になるため断念し、trace field は `rt_record` のみ
  とした（spec/design/test-spec/plan/task を整合済み）。
  measurement/window への trace 付与は Deferred。
- **correlation field の値**: link が存在する限り（linked /
  unlinked いずれも）`correlation_token`/`correlation_epoch` は
  record が主張した handle として emit し、分類は
  `correlation_status`/`unlinked_reason` が担う。link 非存在時は
  `0` sentinel。span/parent 関係 field は schema に存在しない。
- **clock sentinel**: unstamped record は `clock_status="unstamped"`
  + `clock_*=0`/`clock_provenance="none"`。`Estimated` stamp は
  `"estimated"` のまま明示し昇格しない。`observed_at_mono_us` は
  emit 呼出 `Instant` を exporter t0 基準の相対値とし、record 自身の
  clock domain とは別系統として混同しない。
- **severity**: `min(impact_level(impact), dropped_before>0 ?
  WARN : TRACE)`（`Level::Ord` は verbosity 順のため `min` が高
  severity — slice 1 と同規約）。unlinked correlation・Estimated
  provenance は field 報告のみで severity を上げない。ERROR 不使用。
- **`Option<T>: tracing::Value`**: `trace_*` field は `Option`
  field として emit し、`None` では field が出力されない
  （JSON では absent または null — test は両方を許容）。
- **`#[non_exhaustive]` enum の写像**: `TimestampProvenance`
  （外部 crate）は未知 variant を `"unknown"` に写像する
  wildcard を持つ。同 crate の `Confidentiality`/
  `UnlinkedReason`/`CorrelationStatus` は exhaustive match とし、
  将来 variant 追加時は compile error で写像更新を強制する
  （spec の「安全側写像」意図をより強い compile 時保証で実現）。
- **cap 計数の順序**: correlation 分類は cap 判定より先に全
  record へ適用し、`correlation_*` は drain 内全 record を
  計数する（cap 圧力下でも相関健全性が観測可能）。

## Related Experiments

### Adopted

<!-- 該当なし -->

### Rejected

<!-- 該当なし -->

## Validation

補助証拠（canonical = mise、最終検証は別途記録）:

- `cargo check -p ruviform-observability --features tracing-export`
  — pass（exit 0）。
- `cargo check -p ruviform-observability`（default features）—
  pass。`tracing_export` module は feature OFF で非存在。
- `cargo check -p ruviform-observability --all-features` — pass。
- `cargo test -p ruviform-observability --features tracing-export`
  — 233 lib tests + 11 doctests 全 pass（exit 0）。
  `tracing_export` 配下 32 tests（slice 1 の 15 + 本 slice の
  record 9 + trace_context unit 8）全 pass。
- `cargo clippy -p ruviform-observability --features
  tracing-export --all-targets -- -D warnings` — pass。
- `cargo fmt --check`（crate 内）— pass。
- `cargo tree -e normal`（default）: `tracing` 系 dependency
  なし。`--features tracing-export`: `tracing`/`tracing-core`/
  `tracing-attributes` のみ（`tracing-subscriber`/
  `tracing-appender` は normal dep に出ない）。

## Risks

- `emit_records` は `Vec<ReceivedRecord>` 規模の alloc を伴う
  非 RT 操作。ring capacity が大きい drain では event 数が
  `max_events_per_drain`(256) で頭打ちになるが、分類計数は
  全 record 分行う（意図的）。
- trace field の `Option` 出力は subscriber 実装依存で absent/
  null のいずれかになり得る — test は両方を許容する形で記録。
- `TraceContext` は構文検証のみ。version `01` 以降の将来
  semantics（tracestate 等）は本 slice の対象外。

## Follow-ups

- `metric_observation`/`observation_window`/
  `diagnostics.health_transition` への trace field 付与は
  `event_version` bump とセットの別 slice（Deferred）。
- `RecordSink` drain → `TracingRecordExporter` の runner 配線は
  OBS-11/host 責務（本 slice は変換 API のみ）。
- 両 Consumer での同一 Key/Field/Severity・stdout 保護・
  Privacy 検証は #281 cross-repo gate（NotRun）。
- task/worker 間の `tracing` Context 伝播は host の
  `Instrument`/明示 dispatch 責務（#293、NotRun）。
