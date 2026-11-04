---
id: "PLAN-034Rp216xT1c4Dc6YM6xqa"
type: "plan"
status: "approved"
title: "Implement Issue #277 OBS-02 observability core/source contract"
created: "2026-11-03"
updated: "2026-11-03"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-277-observability-core-source-contract.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-277-observability-core-source-contract.md"
    - "docs/designs/issue-277-observability-core-source-contract.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-277-observability-core-source-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/289"
  defers: []
  deferred-by: []
---

# Implement Issue #277 OBS-02 observability core/source contract Implementation Plan

## Goal

design `DESIGN-034Rp1Um3ZXDWfRx8pCmuw` どおりに `ruviform-observability` へ
`core`（Gauge/Histogram・既存 Counter は移動のみで API 不変）と `source`
（Descriptor/Scope/SourceId+Generation/Registrar/Lease/寿命）を実装し、
spec AC-1〜AC-7 を `mise` 検証と決定的試験で証明する。

## Scope

1 本の実装ストリーム（core primitive → source 契約 → 試験/README は同一
critical path）。実 Registry・Poll・History・tracing/logging/diagnostics、
新規クレート、外部依存追加は含まない。

## Upstream Documents

- `docs/specs/issue-277-observability-core-source-contract.md`（approved・AC-1〜AC-7）
- Issue #277（完了条件の正本）・親 Issue #275（統合設計正本）

## Design Inputs

- `docs/designs/issue-277-observability-core-source-contract.md`（approved）

## File Map

- `crates/ruviform-observability/src/lib.rs` — crate doc・`pub mod core`/`pub mod source`・
  root re-export（`counter`/`Counter`/`CounterReader` 不変維持）
- `crates/ruviform-observability/src/core/mod.rs` — re-export
- `crates/ruviform-observability/src/core/counter.rs` — 既存 counter 実装を移動（不変）
- `crates/ruviform-observability/src/core/gauge.rs` — `gauge()`/`Gauge`/`GaugeReader`
- `crates/ruviform-observability/src/core/histogram.rs` — `histogram()`/`Histogram`/`HistogramReader`/`HistogramError`
- `crates/ruviform-observability/src/source/mod.rs` — re-export
- `crates/ruviform-observability/src/source/descriptor.rs` — CounterDescriptor/MetricDescriptor/MetricKind/ObservationUnit/ReportingHint/ReportingImpact/AggregationPolicy/SourceSemantics/SourceTemperature/Confidentiality
- `crates/ruviform-observability/src/source/scope.rs` — ObservationScope
- `crates/ruviform-observability/src/source/identity.rs` — SourceId/SourceGeneration/SourceIdentity
- `crates/ruviform-observability/src/source/source.rs` — MetricSource/CounterSource/MetricReader/SourceBuildError
- `crates/ruviform-observability/src/source/lifecycle.rs` — SourceLifecycle/SourceState/LifecycleError
- `crates/ruviform-observability/src/source/lease.rs` — RegistrationLease/RetiredSource
- `crates/ruviform-observability/src/source/registrar.rs` — ObservationRegistrar/RegistrationError
- `crates/ruviform-observability/Cargo.toml` — dev-dependency `assert_no_alloc` 追加（理由: rt_ レーン）
- `crates/ruviform-observability/README.md` — 責務・RT 契約表・Domain/Consumer 分離例
- `crates/README.md` — ruviform-observability 行の更新

## Implementation Sequence

- [ ] T-1: `src/core/` へ counter を移動（root re-export で API 不変）+ gauge/histogram 実装 + rt_ 試験
- [ ] T-2: `src/source/` 契約型群 + 寿命状態機械 + registrar port + 試験 + README

## Dependencies and Risks

- #276（logging feature）が同一クレートを編集中の可能性 → 最終検証前に
  `git merge origin/main` で共有ファイル（lib.rs/Cargo.toml/README）を統合する。
- 同一クレート内 feature 分離だけでは RT 安全性は保証されない（#275）→
  RT 適性は operation 単位の doc+試験で担保する。

## Task Breakdown

- `docs/tasks/p277-t1-core-primitives-gauge-histogram.md`
- `docs/tasks/p277-t2-source-contract-registration-lifecycle.md`

## Verification

- [ ] `mise run fmt-check` / `mise run check` / `mise run lint` / `mise run test` / `mise run test-rt`
- [ ] 既存 counter 4 テストが無改変で pass（回帰証明）
- [ ] `git merge origin/main` 後の最終検証（共有ファイル競合の統合確認）

## Open Questions / Gaps

- なし（Issue 本文・親 #275・#289 の要求は設計で確定済み）。
