---
id: "PLAN-4afZ4yYXHh9Vey7rTu5wFt"
type: "plan"
status: "approved"
title: "Issue #292 OBS-10 Debug Inspection 実装計画"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/292"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-292-observability-debug-inspection.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-292-observability-debug-inspection.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/implement-issue-291-observability-diagnostics.md"
    - "docs/plans/implement-issue-289-observability-collector-metrics.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-292-observability-debug-inspection.md"
    - "docs/designs/issue-292-observability-debug-inspection.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-292-observability-debug-inspection-tests.md"
  verified-by:
    - "docs/test-specs/issue-292-observability-debug-inspection-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/277"
    - "https://github.com/scarecrow173/ruviform/issues/280"
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/289"
    - "https://github.com/scarecrow173/ruviform/issues/291"
    - "https://github.com/scarecrow173/ruviform/issues/293"
    - "https://github.com/scarecrow173/ruviform/issues/185"
    - "https://github.com/scarecrow173/ruviform/issues/308"
    - "https://github.com/scarecrow173/ruviform/issues/309"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/278"
    - "https://github.com/scarecrow173/ruviform/issues/293"
    - "https://github.com/scarecrow173/ruviform/issues/281"
  deferred-by: []
---

# Issue #292 OBS-10 Debug Inspection 実装計画 Implementation Plan

## Goal

`crates/ruviform-observability/src/debug/` に Inspection budget/
Truncated/Unavailable/Discontinuous/Generation 整合の型付き契約、
Invariant `Satisfied`/`Violated`/`Inconclusive` 判定と Diagnostics
接続、Debug Session 状態機械（開始/権限/切替/Draining/停止競合/
最終回収）、Debug Report の secrets/media 非含有・未認可 export
拒否を実装し、Issue #292 の observability 内完結 slice の受入
条件を満たす。

## Scope

単一実装 stream。`src/debug/` 新設 + `lib.rs` module 登録 + crate
doc 更新のみ。既存 collector/source/metrics/record/rt_bridge/
diagnostics/logging の公開 API は消費のみで変更しない。
`ruviform-audio` 等への依存追加は禁止（依存循環）。新規外部
依存・Cargo feature・unsafe なし。

## Upstream Documents

- `docs/specs/issue-292-observability-debug-inspection.md`

## Design Inputs

- `docs/designs/issue-292-observability-debug-inspection.md`

## File Map

- `crates/ruviform-observability/src/debug/mod.rs` - module doc +
  re-export。
- `crates/ruviform-observability/src/debug/budget.rs` -
  `InspectionBudget`（max_items/max_bytes/timeout の 0/None 規約）。
- `crates/ruviform-observability/src/debug/request.rs` -
  `InspectionTarget`/`InspectionDetail`/`InspectionRequest`/
  `InspectionInputs`/`RequestEcho`/`InspectionError`。
- `crates/ruviform-observability/src/debug/snapshot.rs` -
  `InspectionSnapshot`/`InspectionSection`/`SectionStatus`/
  `UnavailableReason`/`InspectionTruncation`/
  `InspectionConsistency`/`ConsistencyLevel`/`CollectorGenerations`/
  `ProbeDeckAlignment` + Metrics/Diagnostics/Invariants/Session/
  Probes 各 payload 型。
- `crates/ruviform-observability/src/debug/invariant.rs` -
  `InvariantOutcome`/`InconclusiveReason`/`InvariantVerdict`/
  `InvariantCheck`/`InspectionContext`/`InvariantResult`/
  `InvariantPublishReport`/`InvariantError`。
- `crates/ruviform-observability/src/debug/probe.rs` -
  `ProbeMode`/`ProbeAvailability`/`ProbeStatusKind`/
  `ProbeClockMark`/`ProbeReading`/`ProbeDeckSnapshot`/`ProbeSource`
  （#185 語彙の observability 側写像・port のみ）。
- `crates/ruviform-observability/src/debug/session.rs` -
  `DebugSessionState`/`DebugAuthority`/`SessionPermit`/
  `SessionRequest`/`SessionError`/`SessionDrain`/`SessionRecord`/
  `DrainReport`/`SessionDrainStatus`/`EventRecordDrain`。
- `crates/ruviform-observability/src/debug/report.rs` -
  `DebugReport`/`DebugReportRequest`/`ExportLimits`/
  `ExportError`/`ExportTruncation`/`InspectionEcho`/
  `SessionReportView`/`RecordSummary`。
- `crates/ruviform-observability/src/debug/engine.rs` -
  `DebugConfig`/`DebugInspector`/`DebugHealth`。
- `crates/ruviform-observability/src/debug/tests.rs` - unit/golden
  試験（`#[cfg(test)] mod tests`）。
- `crates/ruviform-observability/src/lib.rs` - `pub mod debug`
  登録 + crate doc。
- `docs/impl/ir/issue-292-*.md` - 実装記録。

## Implementation Sequence

- [ ] `budget.rs` + `request.rs`（予算・対象・detail・入力束の
      語彙層）
- [ ] `probe.rs`（#185 語彙写像 + `ProbeSource` port）
- [ ] `snapshot.rs`（section payload・status・consistency・
      truncation）
- [ ] `invariant.rs`（3 値 outcome・check port・publish 型）
- [ ] `session.rs`（状態機械・capability・drain port・
      `EventRecordDrain`）
- [ ] `report.rs`（DebugReport・export 認可・truncation）
- [ ] `engine.rs`（`DebugInspector`/`DebugConfig`/`DebugHealth` 集約）
- [ ] `lib.rs` 登録 + crate doc + `tests.rs` 試験整備
- [ ] 全試験 green → mise 検証 → review-protocol → merge main →
      再検証 → commit/push/PR

## Dependencies and Risks

- 依存: OBS-02（`SourceIdentity`/`SourceGeneration`/
  `Confidentiality`/`SourceLifecycle`/`SourceState` 先例）、
  OBS-05（`record::EventRecord`/`ClockContext`/`ClockDomainId`/
  `rt_bridge::RecordSink`/`drain_final` 契約）、OBS-07
  （`MetricsSnapshot`/`Measurement`/`MeasurementValidity`/
  `MetricValue`/`ObservationBatch`）、OBS-09
  （`Diagnostics`/`Evidence`/`EvidenceRef`/`DiagnosticFinding`/
  `DiagnosticCode`/`FindingSeverity`/`FindingCertainty`/
  `FindingStatus`/`DomainHealth`/`ReportLimits` 規約）。全て main
  実装済み・変更不可。
- risk: `MetricsSnapshot::measurements()`/`ObservationBatch`/
  `DiagnosticReport` の正確な accessor シグネチャは実装時に source
  確認（OBS-09 plan と同じ既知 gap）。
- risk: `RecordSink::drain_final` の正確なシグネチャ
  （`impl FnMut(DrainedRecord<T>)` 系）は実装時に source 確認。
- risk: `Instant`/`Duration` の timeout 判定は section 境界でのみ
  行い、section 内 item 走査は deadline を items cap で近似する
  （長い単一 section は deadline 超過を次 section の
  `Unavailable{DeadlineExceeded}` で報告 — 契約として文書化済み）。
- `catch_unwind` を `InvariantCheck::evaluate`/`ProbeSource::decks`
  に使う（port panic 隔離・diagnostics sink と同じ方針）。

## Task Breakdown

- T1: budget.rs + request.rs（予算/対象/detail/入力束）
- T2: probe.rs（port 語彙）
- T3: snapshot.rs（section/consistency/truncation 型）
- T4: invariant.rs（outcome/check/publish 型）
- T5: session.rs（状態機械/drain/EventRecordDrain）
- T6: report.rs（export 認可）
- T7: engine.rs + lib.rs + tests.rs + 検証

## Verification

- [ ] `mise run fmt-check` / `mise run check` / `mise run lint` /
      `mise run test` / `mise run test-rt` 全 pass。
- [ ] golden 試験: budget 各項・Truncated/Unavailable/Withheld・
      Discontinuous/validity 保持・Generation 整合
      （Aligned/Skewed/Unknown・collector domain 分離・probe
      mismatch/clock UNKNOWN）・Invariant 3 値+理由・panic 隔離・
      Diagnostics 接続（Evidence/Finding が report に現れる）・
      Session 全遷移+権限拒否+切替+Draining 競合+最終回収+
      abandon・Report Unauthorized+truncation+非流出・Logging OFF・
      RT 隣接値型 no-alloc。
- [ ] 既存 observability 試験の回帰なし。

## Open Questions / Gaps

- `MetricsSnapshot::measurements()`/`ObservationBatch`/
  `DiagnosticReport`/`RecordSink::drain_final` の正確な accessor
  シグネチャ（実装時に source 確認 — OBS-09 plan と同じ扱い）。
- concrete probe adapter・`debug-tools` feature・Host 認可・両
  Consumer 検証・DSP 同値実機検証は deferred（spec/design と同一）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution
- Independent work streams: none
- User approval required before dispatch: yes（review-protocol で代替）
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- Assumptions: `MetricsSnapshot::measurements()` が read view を返す。
  `RecordSink::drain_final` が bounded drain + writer quiescence
  契約を持つ。`EventRecord` が Copy/`Send` で `u64` payload のみ。
- Reviewer focus: budget 0/None 規約の一貫性・consistency の domain
  分離（collector gen vs plan gen を数値比較しない）・Invariant
  Inconclusive の原因確定偽装防止・Session の quiescence 未確認で
  buffer を解放しない不変条件・Report の非流出（項目除外 vs 値伏字）。
