---
id: "TASK-034UnBJEQ9hvQiyzg4UMHk"
type: "task"
status: "in-progress"
title: "Issue #291 OBS-09 Diagnostics 実装タスク"
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
    - "docs/plans/implement-issue-291-observability-diagnostics.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-291-observability-diagnostics.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-291-observability-diagnostics.md"
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

# Issue #291 OBS-09 Diagnostics 実装タスク

## Work

`crates/ruviform-observability/src/diagnostics/` を新設し、
Evidence/Finding/Health/Snapshot/Report/Sink の型語彙と `Diagnostics`
集約 engine を実装する。`lib.rs` に `pub mod diagnostics` を登録し、
crate doc を更新する。既存 collector/source/metrics/record/rt_bridge/
logging の公開 API は変更しない。新規外部依存・feature・unsafe なし。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-291-observability-diagnostics.md`
- `docs/specs/issue-291-observability-diagnostics.md`
- `docs/designs/issue-291-observability-diagnostics.md`
- `docs/test-specs/issue-291-observability-diagnostics-tests.md`

## Dependencies

- OBS-02（#277）`SourceIdentity`/`SourceGeneration`/`Confidentiality`/
  `SourceSemantics` — main 実装済み。
- OBS-07（#289）`MetricsSnapshot`/`Measurement`/`MeasurementValidity`/
  `MetricValue` — main 実装済み。
- `record::ClockContext`/`ClockDomainId` — main 実装済み。

## Affected Files

- `crates/ruviform-observability/src/diagnostics/mod.rs`（新規）
- `crates/ruviform-observability/src/diagnostics/evidence.rs`（新規）
- `crates/ruviform-observability/src/diagnostics/finding.rs`（新規）
- `crates/ruviform-observability/src/diagnostics/health.rs`（新規）
- `crates/ruviform-observability/src/diagnostics/report.rs`（新規）
- `crates/ruviform-observability/src/diagnostics/sink.rs`（新規）
- `crates/ruviform-observability/src/diagnostics/engine.rs`（新規）
- `crates/ruviform-observability/src/diagnostics/tests.rs`（新規）
- `crates/ruviform-observability/src/lib.rs`（module 登録 + doc）
- `docs/impl/ir/issue-291-*.md`（実装記録）

## Done When

- [ ] `src/diagnostics/` が実装され、`lib.rs` に登録される。
- [ ] Evidence 4 種・basis 必須・Finding certainty・Health 遷移
      （threshold/hysteresis/recovery/Unknown/stale/partial）・
      generation リセット・superseded・snapshot/report truncation・
      Unauthorized・sink 抑制/panic 隔離・Logging OFF の試験が pass。
- [ ] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` が pass。
- [ ] 実装記録（`docs/impl/ir/`）が作成・更新される。

## Verification

- [ ] `mise run fmt-check` / `mise run check` / `mise run lint` /
      `mise run test` / `mise run test-rt`
- [ ] golden 試験が `cargo test -p ruviform-observability`（mise test
      経由）で pass
- [ ] `git fetch origin main && git merge origin/main` 後に再検証
