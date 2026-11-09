---
id: "TASK-034UBtKswl6dq50oYiJ9H8"
type: "task"
status: "in-progress"
title: "Issue #278 OBS-03 tracing_export slice 2 実装タスク"
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
    - "docs/plans/implement-issue-278-observability-tracing-export-records.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-278-observability-tracing-export-records.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-278-observability-tracing-export-records.md"
    - "docs/designs/issue-278-observability-tracing-export-records.md"
    - "docs/test-specs/issue-278-observability-tracing-export-records-tests.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-278-observability-tracing-export-records.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-278-observability-tracing-export-records-tests.md"
  verified-by: []
  references: []
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/293"
  deferred-by: []
---

# Issue #278 OBS-03 tracing_export slice 2 実装タスク

## Work

- [ ] `src/tracing_export/trace_context.rs` 実装（`TraceContext`・
  `traceparent` parse/`to_traceparent`/hex）。
- [ ] `src/tracing_export/schema.rs` 拡張（`EVENT_CODE_RECORD`・
  record 用 enum→&str 写像・trace field key）。
- [ ] `src/tracing_export/records.rs` 実装
  （`TracingRecordExporter`/`RecordEmitStats`/`emit_records`）。
  `exporter.rs` は無変更（slice 1 schema 凍結）。
- [ ] `src/tracing_export/mod.rs` module 登録・re-export・doc 更新。
- [ ] `src/tracing_export/tests.rs` に contract/golden tests 追加
  （test-spec R-1〜R-12）+ public API doctest。
- [ ] impl record `docs/impl/ir/issue-278-obs-03-tracing-export-records.md`
  作成・更新、docs index 再生成。
- [ ] review-protocol レビュー、mise 検証、`origin/main` merge、
  commit/push/PR、worker_done 報告。

## Classification

feature / OBS-03 slice 2（Issue #278 の残件スライス。Issue は
Open のまま）。

## Upstream Documents

- spec: `docs/specs/issue-278-observability-tracing-export-records.md`
- design: `docs/designs/issue-278-observability-tracing-export-records.md`
- plan: `docs/plans/implement-issue-278-observability-tracing-export-records.md`
- test-spec: `docs/test-specs/issue-278-observability-tracing-export-records-tests.md`

## Dependencies

- OBS-05 `EventRecord`/`RecordHeader`/`ClockContext`/
  `CorrelationBook`/`CorrelationStatus`/`UnlinkedReason`（main
  実装済み）
- OBS-05 `rt_bridge::ReceivedRecord`（main 実装済み）
- OBS-02 `SourceIdentity`/`Confidentiality`/`ReportingImpact`
  語彙（main 実装済み）
- slice 1 `tracing_export` schema/severity/mono_us 規約・
  capture Layer テスト基盤（main 実装済み）
- `tracing` optional dep（既存）

## Affected Files

- `crates/ruviform-observability/src/tracing_export/trace_context.rs`
  （新規）
- `crates/ruviform-observability/src/tracing_export/records.rs`
  （新規）
- `crates/ruviform-observability/src/tracing_export/schema.rs`
- `crates/ruviform-observability/src/tracing_export/mod.rs`
- `crates/ruviform-observability/src/tracing_export/tests.rs`
- `docs/**`（本チェーン + impl record + index）

## Done When

- test-spec R-1〜R-12 を満たす test が `--features
  tracing-export` で pass し、default build が `tracing` 非依存で
  compile する。
- `mise run fmt-check`/`check`/`lint`/`test` が pass。
- impl record が検証結果と NotRun/Unsupported を記録する。

## Verification

- canonical: `mise run fmt-check` / `mise run check` /
  `mise run lint` / `mise run test`
- 補助証拠: `cargo check|test -p ruviform-observability --features
  tracing-export`、default build compile、`cargo tree` で
  `tracing-export` のみ `dep:tracing` を有効化することの確認。
