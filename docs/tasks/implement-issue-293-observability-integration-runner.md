---
id: "TASK-SJPi9nUIVUE9wH6ysAKEGp"
type: "task"
status: "done"
title: "Issue #293 OBS-11 ObservabilityConfig/ObservationRunner 実装タスク"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/293"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-293-observability-integration-runner.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-293-observability-integration-runner.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-293-observability-integration-runner.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-293-observability-integration-runner-tests.md"
  verified-by:
    - "docs/test-specs/issue-293-observability-integration-runner-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/275"
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/audio-lens/issues/138"
    - "https://github.com/scarecrow173/synvelis/issues/81"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/audio-lens/issues/138"
    - "https://github.com/scarecrow173/synvelis/issues/81"
  deferred-by: []
---

# Issue #293 OBS-11 ObservabilityConfig/ObservationRunner 実装タスク

## Work

`crates/ruviform-observability/src/integration/` を新設し、
`ObservabilityConfig`（collection/diagnostics/debug/export の独立
設定・`LoggingConfig` 非統合）と `ObservationRunner`
（`Unconfigured→Prepared→Ready→Running→Draining→Stopped`・非 RT
`poll_once` 駆動・段階的 drain・`reconfigure` Validate→Prepare→
Activate→Retire・Host 注入 `tracing::Dispatch`・Logger 非依存
`RunnerHealth` 合成・`attach_logging` 任意接続）を実装する。
`Cargo.toml` に `integration = []` feature と `[[test]]` を追加し、
`lib.rs` に feature-gated module 登録と crate doc 更新を行う。
既存 collector/metrics/source/record/rt_bridge/diagnostics/
debug/profiling/tracing_export/logging の公開 API は変更しない。
他 crate・CI・生成物への変更なし。新規外部依存・unsafe なし。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-293-observability-integration-runner.md`
- `docs/specs/issue-293-observability-integration-runner.md`
- `docs/designs/issue-293-observability-integration-runner.md`
- `docs/test-specs/issue-293-observability-integration-runner-tests.md`

## Dependencies

- Issue #289（OBS-07 Collector — `Collector::poll`/`CollectorConfig`/
  `CollectorHealth`/`PollError`/`CollectorHandle`/`CollectorRegistrar`）
- Issue #291（OBS-09 Diagnostics — `Diagnostics`/`DiagnosticsConfig`/
  `DiagnosticsHealth`/`DiagnosticSnapshot`/`FindingSink`）
- Issue #292（OBS-10 Debug — `DebugInspector`/`DebugConfig`/
  `DebugAuthority`/`SessionPermit`/`DebugHealth`/`SessionView`）
- Issue #278（OBS-03 tracing_export — `TracingExporter`/`EmitStats`/
  `TracingFindingSink`・Host 注入 `Dispatch`）
- Issue #276（OBS-01 logging — `WriterGuard`/`LoggingHealth`/
  `LoggingBuildResult`・Host 所有 dispatch）
- Issue #277（OBS-02 source 契約 — lease retire/quiescence）

## Acceptance Checks

- [ ] `ObservabilityConfig` の独立設定（Logging 無条件統合なし）
- [ ] `ObservationRunner` の非 RT poll・Source 登録・Snapshot・
  停止/Drain レポート・Collector 単体/Logging なし運用
- [ ] `tracing::Dispatch` 明示注入・global/scoped・Guard 寿命の試験
- [ ] 生命周期 Unconfigured→…→Stopped・Drain 順序・
  Partial/Unknown の正直な扱い
- [ ] 動的設定 Validate→Prepare→Activate→Retire（非 RT 変更）
- [ ] 自身の Health Snapshot（Logger 非依存・自己再帰なし）
- [ ] Feature DAG（`integration` 追加・default 軽量 core 維持）
- [ ] README/rustdoc に起動・通常停止・異常停止・Logging OFF・
  Debug Session の使用例
- [ ] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` pass・
  feature 組合せ build pass

## Deferred

- AudioLens #138・Synvelis #81 の consumer 統合・両 Consumer 受入 —
  NotRun（#281/#219 cross-repo gate）
- RT callback での Subscriber/allocator/lock/IO/非有界 wake 混入の
  実機計測・AudioLens f64/Synvelis 再生回帰 — NotRun（consumer gate）
- Linux/macOS 実機・MSRV lane — NotRun（#219/#223 gate）
- concrete probe adapter の ruviform-audio 側実装 — NotRun（別
  Issue/slice）
- OTLP 等の外部 exporter — 対象外（初期完成条件に混ぜない）
