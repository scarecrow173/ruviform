---
id: "PLAN-034UprlQzHK8jUy6ATqC0u"
type: "plan"
status: "approved"
title: "Issue #278 OBS-03 tracing_export 実装計画"
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
    - "docs/specs/issue-278-observability-tracing-export.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-278-observability-tracing-export.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-278-observability-tracing-export-tests.md"
    - "docs/tasks/implement-issue-278-observability-tracing-export.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-278-observability-tracing-export.md"
    - "docs/designs/issue-278-observability-tracing-export.md"
  derived-by:
    - "docs/tasks/implement-issue-278-observability-tracing-export.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-278-observability-tracing-export-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/278"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/293"
  deferred-by: []
---

# Issue #278 OBS-03 tracing_export 実装計画 Implementation Plan

## Goal

`ruviform-observability` に feature `tracing-export` で gate された
`tracing_export` module を追加し、`ObservationBatch` → `tracing`
event の `TracingExporter` と `DiagnosticsEvent` → `tracing` event の
`TracingFindingSink` を実装する。Collector/Diagnostics 正本は
`tracing` 非依存を維持する。

## Scope

In scope は spec の In Scope 節どおり（`src/tracing_export/`、
`lib.rs` additive、`Cargo.toml` feature + dev-dep、docs チェーン）。
Out of scope は spec の Out of Scope 節どおり（EventRecord/Correlation/
W3C/RT token/thread-async context/consumer gate は NotRun、
#281/#293 責務）。

## Upstream Documents

- spec: `docs/specs/issue-278-observability-tracing-export.md`
- design: `docs/designs/issue-278-observability-tracing-export.md`
- test-spec: `docs/test-specs/issue-278-observability-tracing-export-tests.md`

## Design Inputs

- `ObservationBatch`/`Measurement`/`MetricValue`/`MeasurementValidity`/
  `MeasurementIssue`（`src/collector/batch.rs`, `src/metrics/mod.rs`）
- `MetricDescriptor`/`ReportingHint`/`ReportingImpact`/`MetricKind`/
  `ObservationUnit`/`ObservationScope`/`SourceIdentity`/
  `SourceLifecycle`（`src/source/`）
- `ObservationWindow`（`src/collector/window.rs`）
- `FindingSink`/`DiagnosticsEvent`/`HealthState`（`src/diagnostics/`）
- `tracing 0.1` `Dispatch`/`dispatcher::with_default`/`event!`（
  optional dep 既存、`Dispatch::none()` で no-subscriber 安全）

## File Map

- `crates/ruviform-observability/Cargo.toml` —
  `tracing-export = ["dep:tracing"]` feature、
  `tracing-subscriber` dev-dependency（`registry` feature、
  capture Layer 用）。
- `crates/ruviform-observability/src/lib.rs` —
  `#[cfg(feature = "tracing-export")] pub mod tracing_export;` +
  crate doc bullet（additive のみ）。
- `crates/ruviform-observability/src/tracing_export/mod.rs` —
  module doc・re-export。
- `crates/ruviform-observability/src/tracing_export/schema.rs` —
  event 名/`event_version`/field key/enum→&str 写像/severity 写像/
  mono-us 変換の単一正本。
- `crates/ruviform-observability/src/tracing_export/exporter.rs` —
  `TracingExporter`/`EmitStats`/`SourcePolicyState`。
- `crates/ruviform-observability/src/tracing_export/sink.rs` —
  `TracingFindingSink`。
- `crates/ruviform-observability/src/tracing_export/tests.rs` —
  contract/golden tests（`#[cfg(all(test, feature = "tracing-export"))]`）。

## Implementation Sequence

1. `Cargo.toml` feature + dev-dep、`lib.rs` module 登録（compile gate）。
2. `schema.rs`（写像・severity・mono-us）→ `exporter.rs` → `sink.rs`。
3. `tests.rs`（capture Layer + 実 `Collector`/`Diagnostics` 経由の
   contract test）→ doctest。
4. review-protocol → mise 検証 → impl record 更新 → commit/PR。

## Dependencies and Risks

- 依存: `tracing`（optional 既存）+ dev `tracing-subscriber`。
  新規外部依存なし。
- Risk: `Instant` に epoch なし → exporter t0 起点の相対
  `*_mono_us`。`min_interval` は `Instant` 差。
- Risk: `with_default` は thread-local scoped — async/task 間伝播は
  NotRun。

## Task Breakdown

単一 task（`docs/tasks/implement-issue-278-observability-tracing-export.md`）
— module 規模が小さく分割不要。

## Verification

- `mise run fmt-check` / `mise run check` / `mise run lint` /
  `mise run test`（canonical）。
- 補助証拠: `cargo check -p ruviform-observability --features
  tracing-export`、`cargo test -p ruviform-observability --features
  tracing-export`、default build での非依存確認。
- feature OFF で `tracing_export` が存在しないこと・
  `--features tracing-export` のみで `dep:tracing` が有効なことを
  Cargo.toml/`cargo tree` で確認。

## Open Questions / Gaps

- window 集約 event の発行条件は「eligible measurement を処理した
  poll のみ」（test-spec R-5）に固定。
- `observation_key` は `SourceIdentity` Display（`"id#gen"`）の
  `String`（`tracing::Value` 実装済み）。

## Self-Review

- spec の全 In Scope 項目が file map/sequence に対応。
- Out of Scope は spec と一致し NotRun 記録先（impl record/PR）を明記。
- 公開 enum なし（`EmitStats` は struct）→ `#[non_exhaustive]` 対象は
  新規 enum を作らない限り不要。`Level` は `tracing` 側の型。

## Implementation Handoff

- impl record: `docs/impl/ir/issue-278-obs-03-tracing-export.md`
  （実装前に `in-progress` で作成）。

## Review Handoff

- review-protocol で spec/design/plan/task + 実装 diff をレビューし、
  CONFIRMED P0/P1 を解消してから承認相当とする。
