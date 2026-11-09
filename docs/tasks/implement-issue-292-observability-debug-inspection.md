---
id: "TASK-I1LfNQN5Dh3eVjMCa0yvUi"
type: "task"
status: "done"
title: "Issue #292 OBS-10 Debug Inspection 実装タスク"
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
    - "docs/plans/implement-issue-292-observability-debug-inspection.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-292-observability-debug-inspection.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-292-observability-debug-inspection.md"
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

# Issue #292 OBS-10 Debug Inspection 実装タスク

## Work

`crates/ruviform-observability/src/debug/` を新設し、Inspection
budget/Truncated/Unavailable/Discontinuous/Generation 整合の型付き
契約、Invariant `Satisfied`/`Violated`/`Inconclusive` 判定と
Diagnostics 接続、Debug Session 状態機械（開始/権限/切替/
Draining/停止競合/最終回収）、Debug Report の secrets/media
非含有・未認可 export 拒否を実装する。`lib.rs` に
`pub mod debug` を登録し、crate doc を更新する。既存
collector/source/metrics/record/rt_bridge/diagnostics/logging の
公開 API は変更しない。`ruviform-audio` 等への依存追加は禁止
（依存循環）。新規外部依存・feature・unsafe なし。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-292-observability-debug-inspection.md`
- `docs/specs/issue-292-observability-debug-inspection.md`
- `docs/designs/issue-292-observability-debug-inspection.md`
- `docs/test-specs/issue-292-observability-debug-inspection-tests.md`

## Dependencies

- OBS-02（#277）`SourceIdentity`/`SourceGeneration`/`Confidentiality`/
  `SourceLifecycle` — main 実装済み。
- OBS-05（#280）`record::EventRecord`/`ClockContext`/`ClockDomainId`/
  `rt_bridge::RecordSink`/`drain_final` — main 実装済み。
- OBS-07（#289）`MetricsSnapshot`/`Measurement`/`MeasurementValidity`/
  `MetricValue`/`ObservationBatch` — main 実装済み。
- OBS-09（#291）`Diagnostics`/`Evidence`/`EvidenceRef`/
  `DiagnosticFinding`/`DiagnosticCode`/`FindingSeverity`/
  `FindingCertainty`/`FindingStatus`/`DomainHealth` — main 実装済み。
- #185/#308/#309 probe 語彙（`ruviform-audio::typed::probe`）— port
  写像の正本。observability からは依存不可。

## Affected Files

- `crates/ruviform-observability/src/debug/mod.rs`（新規）
- `crates/ruviform-observability/src/debug/budget.rs`（新規）
- `crates/ruviform-observability/src/debug/request.rs`（新規）
- `crates/ruviform-observability/src/debug/snapshot.rs`（新規）
- `crates/ruviform-observability/src/debug/invariant.rs`（新規）
- `crates/ruviform-observability/src/debug/probe.rs`（新規）
- `crates/ruviform-observability/src/debug/session.rs`（新規）
- `crates/ruviform-observability/src/debug/report.rs`（新規）
- `crates/ruviform-observability/src/debug/engine.rs`（新規）
- `crates/ruviform-observability/src/debug/tests.rs`（新規）
- `crates/ruviform-observability/src/lib.rs`（module 登録 + doc）
- `docs/impl/ir/issue-292-*.md`（実装記録）

## Done When

- [ ] `src/debug/` が実装され、`lib.rs` に登録される。
- [ ] budget 各項・Truncated/Unavailable/Withheld・
      Discontinuous/validity 保持・Generation 整合
      （Aligned/Skewed/Unknown・domain 分離・probe mismatch/clock
      UNKNOWN）・Invariant 3 値+理由+panic 隔離・Diagnostics 接続・
      Session 全遷移+権限拒否+切替+Draining 競合+最終回収+abandon・
      Report Unauthorized+truncation+非流出・Logging OFF・RT 隣接
      値型 no-alloc の試験が pass。
- [ ] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` が pass。
- [ ] 実装記録（`docs/impl/ir/`）が作成・更新される。

## Verification

- [ ] `mise run fmt-check` / `mise run check` / `mise run lint` /
      `mise run test` / `mise run test-rt`（exit 0）
- [ ] golden 試験が `mise run test` 経由で pass
- [ ] `git fetch origin main && git merge origin/main` 後に再検証
