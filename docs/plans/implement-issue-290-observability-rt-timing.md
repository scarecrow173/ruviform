---
id: "PLAN-k3qysbE3jkIfMxFWa8dlGX"
type: "plan"
status: "approved"
title: "Issue #290 OBS-08 RT Timing 実装計画"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/290"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-290-observability-rt-timing.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-290-observability-rt-timing.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/implement-issue-290-observability-allocation-probe.md"
    - "docs/plans/implement-issue-289-observability-collector-metrics.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-290-observability-rt-timing.md"
    - "docs/designs/issue-290-observability-rt-timing.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-290-observability-rt-timing-tests.md"
  verified-by:
    - "docs/test-specs/issue-290-observability-rt-timing-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/275"
    - "https://github.com/scarecrow173/ruviform/issues/280"
    - "https://github.com/scarecrow173/ruviform/issues/289"
    - "https://github.com/scarecrow173/ruviform/issues/281"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/293"
  deferred-by: []
---

# Issue #290 OBS-08 RT Timing 実装計画 Implementation Plan

## Goal

Issue #290（OBS-08）の残件「RT Timing の Clock/overrun/Callback
budget 意味論と Histogram の統合試験」を、既存 `ruviform-
observability` 内の `src/rt_timing/` モジュールとして最小差分で
実装する。新規依存・feature gate・unsafe・既存モジュール変更なし。

## Scope

- 新規: `crates/ruviform-observability/src/rt_timing/{mod,record,
  bridge,sink,tests}.rs`
- 変更: `crates/ruviform-observability/src/lib.rs`（module 登録 +
  crate doc 追記）
- docs: 本 chain（spec/design/test-spec/plan/task/IR）+ Issue
  #290/#291/#292 audit 証跡

## Upstream Documents

- `docs/specs/issue-290-observability-rt-timing.md`
- `docs/designs/issue-290-observability-rt-timing.md`
- `docs/test-specs/issue-290-observability-rt-timing-tests.md`

## Design Inputs

- `record`: `RecordHeader`/`ClockContext`/`InputStamp`/`ClockDomainId`/
  `ClockEpochId`/`TimestampProvenance`/`EventId`/`SourceIdentity`/
  `Confidentiality`/`ReportImpact`
- `rt_bridge`: `bridge::<R>()`/`RtRecorder`/`RecordSink`/
  `ReceivedRecord`/`CounterReader`
- `core::histogram`: `histogram()`/`Histogram`/`HistogramReader`/
  `HistogramError`
- `source`: `CounterSource`/`HistogramSource`/`MetricSource`/
  `ObservationRegistrar`
- `collector`: `Collector`/`CollectorConfig`/`ObservationBatch`/
  `MetricValue::Histogram`

## File Map

| File | Responsibility |
| --- | --- |
| `src/rt_timing/mod.rs` | module doc・re-export |
| `src/rt_timing/record.rs` | `TIMING_SCHEMA_VERSION`/`TimingClock`/`TimingSample`/`TimingRecord` |
| `src/rt_timing/bridge.rs` | `timing_bridge()`/`TimingRecorder` |
| `src/rt_timing/sink.rs` | `TimingSink`/`TimingWindow`/`TimingSnapshot`/`QuantileEstimate` |
| `src/rt_timing/tests.rs` | 契約・意味論・統合試験 |
| `src/lib.rs` | `pub mod rt_timing` + doc |

## Implementation Sequence

1. `record.rs`: `TimingClock`/`TimingSample`/`TimingRecord` + accessor。
2. `bridge.rs`: `timing_bridge()` + `TimingRecorder::record`。
3. `sink.rs`: `TimingSink::poll`/`snapshot`/reader 群 + 報告型。
4. `tests.rs`: test-spec R1–R10 に対応する試験。
5. `lib.rs` 登録・crate doc 追記。
6. mise 検証 → IR → index 再生成 → audit 証跡 → commit → PR。

## Dependencies and Risks

- 依存: 全て既存 crate 内 module。外部依存追加なし。
- `HistogramError` を `timing_bridge` の戻り値に使うため
  `core::histogram` の error 型を確認（`HistogramError` が公開か
  確認してから実装）。
- foreign clock 試験は sink 直接構築が必要 — `RecordSink` の
  crate 内可視性を確認。
- `assert_no_alloc` は crate dev-dependency 既存を使用。

## Task Breakdown

単一タスク（実装 + 試験 + docs）として扱う。詳細は
`docs/tasks/implement-issue-290-observability-rt-timing.md`。

## Verification

- `mise run fmt-check` / `check` / `lint` / `test` / `test-rt`
  （存在するもの全て）。
- 補助（非 canonical）: `cargo test -p ruviform-observability` で
  rt_timing module のみ先行確認。

## Open Questions / Gaps

- なし（設計は spec/design で確定済み）。

## Self-Review

- spec/design/test-spec/plan/task の relations が相互に整合。
- 既存モジュールへの変更がないことを File Map で確認。
- Issue #290 の残件のみを対象とし、#291/#292 の充足済み項目は
  audit 証跡で記録するに留める。

## Implementation Handoff

`docs/tasks/implement-issue-290-observability-rt-timing.md` を正本
として実装する。

## Review Handoff

実装完了後 `review-protocol` で差分レビューを行い、blocker なしを
もって承認とみなす。
