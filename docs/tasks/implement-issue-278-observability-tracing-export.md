---
id: "TASK-034UprmEYoMJcy1j0tHLHy"
type: "task"
status: "in-progress"
title: "Issue #278 OBS-03 tracing_export 実装タスク"
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
    - "docs/plans/implement-issue-278-observability-tracing-export.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-278-observability-tracing-export.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-278-observability-tracing-export.md"
    - "docs/designs/issue-278-observability-tracing-export.md"
    - "docs/test-specs/issue-278-observability-tracing-export-tests.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-278-observability-tracing-export.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-278-observability-tracing-export-tests.md"
  verified-by: []
  references: []
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/293"
  deferred-by: []
---

# Issue #278 OBS-03 tracing_export 実装タスク

## Work

- [ ] `Cargo.toml`: `tracing-export = ["dep:tracing"]` feature と
  `tracing-subscriber` dev-dependency（registry feature）追加。
- [ ] `lib.rs`: `#[cfg(feature = "tracing-export")] pub mod
  tracing_export;` + doc bullet（additive のみ）。
- [ ] `src/tracing_export/mod.rs` / `schema.rs` / `exporter.rs` /
  `sink.rs` 実装。
- [ ] `src/tracing_export/tests.rs` contract/golden tests +
  public API doctest。
- [ ] impl record `docs/impl/ir/issue-278-obs-03-tracing-export.md`
  作成・更新、docs index 再生成。
- [ ] review-protocol レビュー、mise 検証、`origin/main` merge、
  commit/push/PR、worker_done 報告。

## Classification

feature / OBS-03 slice（Issue #278 の一部。Issue は Open のまま）。

## Upstream Documents

- spec: `docs/specs/issue-278-observability-tracing-export.md`
- design: `docs/designs/issue-278-observability-tracing-export.md`
- plan: `docs/plans/implement-issue-278-observability-tracing-export.md`
- test-spec: `docs/test-specs/issue-278-observability-tracing-export-tests.md`

## Dependencies

- OBS-07 `ObservationBatch`/`Measurement`（main 実装済み）
- OBS-09 `FindingSink`/`DiagnosticsEvent`/`HealthState`（main 実装済み）
- OBS-02 `SourceIdentity`/`ReportingHint`/`ReportingImpact` 語彙
- `tracing` optional dep（既存）

## Affected Files

- `crates/ruviform-observability/Cargo.toml`
- `crates/ruviform-observability/src/lib.rs`
- `crates/ruviform-observability/src/tracing_export/**`（新規）
- `docs/**`（本チェーン + impl record + index）

## Done When

- test-spec R-1〜R-11 を満たす test が `--features tracing-export`
  で pass し、default build が `tracing` 非依存で compile する。
- `mise run fmt-check`/`check`/`lint`/`test` が pass。
- impl record が検証結果と NotRun/Unsupported を記録する。

## Verification

- canonical: `mise run fmt-check` / `mise run check` /
  `mise run lint` / `mise run test`
- 補助証拠: `cargo check|test -p ruviform-observability --features
  tracing-export`、default build compile、`cargo tree` で
  `tracing-export` のみ `dep:tracing` を有効化することの確認。
