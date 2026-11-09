---
id: "PLAN-034UnAUkBnCas6lEsFTewD"
type: "plan"
status: "approved"
title: "Issue #291 OBS-09 Diagnostics 実装計画"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/291"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-291-observability-diagnostics.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-291-observability-diagnostics.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/implement-issue-289-observability-collector-metrics.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-291-observability-diagnostics.md"
    - "docs/designs/issue-291-observability-diagnostics.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-291-observability-diagnostics-tests.md"
  verified-by:
    - "docs/test-specs/issue-291-observability-diagnostics-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/277"
    - "https://github.com/scarecrow173/ruviform/issues/278"
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/289"
    - "https://github.com/scarecrow173/ruviform/issues/293"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/278"
    - "https://github.com/scarecrow173/ruviform/issues/293"
  deferred-by: []
---

# Issue #291 OBS-09 Diagnostics 実装計画 Implementation Plan

## Goal

`crates/ruviform-observability/src/diagnostics/` に Evidence/Finding/
Health/Snapshot/Report/Sink の型語彙と `Diagnostics` 集約 engine を
実装し、Issue #291 の受入条件（事実/推定の型分離・generation/
lifetime・hysteresis・Logging OFF・開示拒否・非流出）を満たす。

## Scope

単一実装 stream。`src/diagnostics/` 新設 + `lib.rs` module 登録 +
crate doc 更新のみ。collector/source/metrics/record/rt_bridge/logging
の既存 API は消費のみで変更しない。新規外部依存・feature・unsafe なし。

## Upstream Documents

- `docs/specs/issue-291-observability-diagnostics.md`

## Design Inputs

- `docs/designs/issue-291-observability-diagnostics.md`

## File Map

- `crates/ruviform-observability/src/diagnostics/mod.rs` - module doc +
  re-export。
- `crates/ruviform-observability/src/diagnostics/evidence.rs` -
  `Evidence`/`EvidenceKind`/`EvidenceTime`/`EvidencePayload`/
  `EvidenceRef`/`EvidenceEntry`/`EvidenceError`。
- `crates/ruviform-observability/src/diagnostics/finding.rs` -
  `DiagnosticCode`/`FindingSeverity`/`FindingCertainty`/
  `FindingStatus`/`DiagnosticFinding`/`FindingError`。
- `crates/ruviform-observability/src/diagnostics/health.rs` -
  `HealthState`/`SignalKind`/`SignalDirection`/`Hysteresis`/
  `HealthRule`/`HealthEvaluator`/`DomainHealth`/`RuleHealth`/
  `RuleInput`/`RuleError`/`RuleTransition`。
- `crates/ruviform-observability/src/diagnostics/report.rs` -
  `SnapshotConsistency`/`DiagnosticSnapshot`/`ReportLimits`/
  `DiagnosticsConfig`/`ReportRequest`/`ReportError`/
  `ReportTruncation`/`DiagnosticReport`。
- `crates/ruviform-observability/src/diagnostics/sink.rs` -
  `FindingSink`/`DiagnosticsEvent`/`SinkPolicy`。
- `crates/ruviform-observability/src/diagnostics/engine.rs` -
  `Diagnostics`/`DiagnosticsHealth`/`DiagnosticsConfigError`。
- `crates/ruviform-observability/src/diagnostics/tests.rs` - unit/golden
  試験（`#[cfg(test)] mod tests`）。
- `crates/ruviform-observability/src/lib.rs` - `pub mod diagnostics`
  登録 + crate doc。
- `docs/impl/ir/issue-291-*.md` - 実装記録。

## Implementation Sequence

- [ ] `evidence.rs` + `finding.rs`（語彙・不変条件・golden test 骨格）
- [ ] `health.rs`（HealthEvaluator・hysteresis・rollup・遷移検出）
- [ ] `report.rs`（Snapshot/Report/開示/truncation）
- [ ] `sink.rs`（FindingSink port + 抑制 policy）
- [ ] `engine.rs`（Diagnostics 集約・evaluate/report/health）
- [ ] `lib.rs` 登録 + crate doc
- [ ] 全試験 green → mise 検証 → review-protocol → merge main →
      再検証 → commit/push/PR

## Dependencies and Risks

- 依存: OBS-02（`SourceIdentity`/`SourceGeneration`/`Confidentiality`/
  `SourceSemantics`）、OBS-07（`MetricsSnapshot`/`Measurement`/
  `MeasurementValidity`/`MetricValue`）、`record::ClockContext`。全て
  main 実装済み・変更不可。
- `MetricsSnapshot` は `Arc` 共有・`measurements()` は read snapshot。
  evaluate は非 RT の `&mut self` で hysteresis/evidence/findings を
  更新する。
- `catch_unwind` を sink emit に使う（sink panic 隔離）。
- risk: `MetricsSnapshot::measurements()` の lookup API の正確な
  シグネチャは実装時に確認（`by_key`/iterator 系）。

## Task Breakdown

- T1: evidence.rs + finding.rs（語彙層）
- T2: health.rs（評価層）
- T3: report.rs + sink.rs（出力層）
- T4: engine.rs（集約層）
- T5: lib.rs 登録 + 試験整備 + 検証

## Verification

- [ ] `mise run fmt-check` / `mise run check` / `mise run lint` /
      `mise run test` / `mise run test-rt` 全 pass。
- [ ] golden 試験: Evidence 4 種・basis 必須・Finding certainty・
      Health 遷移（threshold/hysteresis/recovery/Unknown/stale/
      partial）・generation リセット・superseded・snapshot/report
      truncation・Unauthorized・sink 抑制/panic 隔離・Logging OFF。
- [ ] 既存 observability 試験の回帰なし。

## Open Questions / Gaps

- `MetricsSnapshot` の lookup API 詳細（実装時に source 確認）。
- `Measurement.value`/`delta`/`rate` の正確なフィールド名（同上）。
- 両 Consumer 検証（#281）・tracing adapter（#278）・Host 認可
  （#293）は deferred。

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

- Assumptions: `MetricsSnapshot::measurements()` が `&[Measurement]`
  または反復可能な read view を返す。`Measurement.value` が
  `Option<MetricValue>`。
- Reviewer focus: Evidence の basis 不変条件・hysteresis の連続カウント
  意味論・Unknown rollup・開示フィルタの非流出性・sink 抑制。
