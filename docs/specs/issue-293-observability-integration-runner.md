---
id: "SPEC-4IC1B6yvvsvwdDR85TSYNI"
type: "spec"
status: "approved"
title: "Issue #293 OBS-11 Host統合 Config + ObservationRunner 生命周期 slice（observability 内完結）"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/293"
    - "https://github.com/scarecrow173/ruviform/issues/275"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "https://github.com/scarecrow173/ruviform/issues/293"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-276-generic-logging-adapter.md"
    - "docs/specs/issue-277-observability-core-source-contract.md"
    - "docs/specs/issue-278-observability-tracing-export.md"
    - "docs/specs/issue-289-observability-collector-metrics.md"
    - "docs/specs/issue-290-observability-allocation-probe.md"
    - "docs/specs/issue-291-observability-diagnostics.md"
    - "docs/specs/issue-292-observability-debug-inspection.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/foundation-observability-minimal-api.md"
    - "docs/designs/shared-foundations-and-modularization-policy.md"
    - "docs/designs/issue-292-observability-debug-inspection.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-276-generic-logging-adapter.md"
    - "docs/specs/issue-278-observability-tracing-export.md"
    - "docs/specs/issue-289-observability-collector-metrics.md"
    - "docs/specs/issue-291-observability-diagnostics.md"
    - "docs/specs/issue-292-observability-debug-inspection.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-293-observability-integration-runner-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/275"
    - "https://github.com/scarecrow173/ruviform/issues/276"
    - "https://github.com/scarecrow173/ruviform/issues/277"
    - "https://github.com/scarecrow173/ruviform/issues/278"
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/289"
    - "https://github.com/scarecrow173/ruviform/issues/290"
    - "https://github.com/scarecrow173/ruviform/issues/291"
    - "https://github.com/scarecrow173/ruviform/issues/292"
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

# Issue #293 OBS-11 Host統合 Config + ObservationRunner 生命周期 slice（observability 内完結）

## What Should Be Built

既存 `ruviform-observability` クレート内に `src/integration/` モジュールを
新設し、Issue #293（OBS-11）のうち **本クレート内で完結する
config + runner 生命周期 slice** を提供する。#275 module map の
`integration: Host向けnonRT Runner / Config / Lifecycle支援` に対応する。
独立 Integration クレート・Application Framework・GUI/IPC 層は作らない。
具体的には:

- `ObservabilityConfig`: collection（`CollectorConfig` — poll 間隔・
  Source/Series/Batch 予算・History 予算・stale 閾値を含む OBS-07 正本）、
  diagnostics（`Option<DiagnosticsConfig>`）、debug
  （`Option<DebugSettings>` — `DebugConfig` + 任意の
  `Arc<DebugAuthority>`）、tracing-exposure / Export policy
  （`tracing-export` feature 時のみ `Option<ExportPolicy>` — Host 注入
  `Dispatch`・per-poll emission cap・diagnostics event 配線）の
  独立設定。`LoggingConfig`（OBS-01 #276 所有）は config に無条件統合
  せず、logging は `attach_logging` による明示的な任意接続とする。
- `ObservationRunner`: `Unconfigured → Prepared → Ready → Running →
  Draining → Stopped` のライフサイクルを持つ非 RT の poll 駆動体。
  `Collector::poll(now) -> ObservationBatch` を Host の非 RT 呼出しで
  駆動し、各 poll で `diagnostics.evaluate` と（構成時）
  `TracingExporter::emit(&batch)` を実行する。Collector 単体
  （diagnostics/debug/export なし）・Logging なし運用が可能。
  `registration_port()` で動作中の Source 登録、
  `collector_handle()`/`snapshot()`/`latest_batch()`/`series()`/
  `history()` で Snapshot 照会、`shutdown()` で停止/Drain レポートを
  提供する。
- `tracing::Dispatch` は Host が `ExportPolicy` 経由で明示注入する。
  library は `set_global_default`/`init` を一切呼ばず、exporter は
  scoped `dispatcher::with_default` で注入 dispatch に限定して emit
  する。global 既設定・scoped guard 寿命の試験を行う。
- 停止順序: `begin_shutdown()`（Draining 突入・poll/登録受付停止の
  宣言）→ domain 側の Source Writer 停止・quiescence 確認（Host 責務）
  → `finish_shutdown()` で Collector 最終 Poll → Collector
  `shutdown()`（終端 drain batch commit）→ Exporter へ終端 batch
  emit → logging `WriterGuard` drop（flush/close）。回収不能・
  quiescence 未確認は `Partial`/`Unknown` として正直に報告し、
  完全保存を主張しない。
- 動的設定: `reconfigure(config)` が Validate → Prepare → Activate →
  Retire を非 RT で実行する。`CollectorConfig` は構築時不変のため、
  収集間隔/History 予算の変更は新 Collector 構築 + 旧 Collector の
  drain retire で実現し、Source 再登録は domain 責務として報告する。
- `RunnerHealth`: Logger 非依存・自己再帰なしの固定サイズ Snapshot
  （poll 遅延・Source/Series 予算・export suppression・record loss・
  queue full・Sink Error・Report 失敗・Debug session 容量を各層の
  既存 health から合成）。
- `integration` Cargo feature の追加（`integration = []` —
  collector/diagnostics/debug は既存の無条件 module、tracing/logging
  は選択）。default build の軽量 core を維持する。

## Why Now

- OBS-01（#276）logging adapter、OBS-02（#277）source 契約、
  OBS-03（#278）tracing_export、OBS-05（#280）rt_bridge/record、
  OBS-07（#289）Collector、OBS-08（#290）allocation-probe、
  OBS-09（#291）Diagnostics、OBS-10（#292）debug が全て main に
  実装済みであり、統合層が組み立てる全構成要素が揃った。
- AudioLens（audio-lens#138）・Synvelis（synvelis#81）の Host が
  統一された非 RT 構成・起動/停止規則を必要とし、consumer 側の
  統合受入（#281/#219 gate）に先立って library 側の Runner/Config
  契約を固定する必要がある。
- `Collector::poll` は単一 poll owner・`&mut self` 保証済みだが、
  diagnostics evaluate・tracing emit・停止 drain を束ねる非 RT
  ライフサイクルは未定義だった。

## Users and Value

- **Consumer Host（AudioLens/Synvelis）**: `ObservabilityConfig` に
  collector/diagnostics/debug/export を独立設定し、
  `ObservationRunner` で poll を駆動・Source 登録・Snapshot 照会・
  段階的停止を統一規則で行える。Logging は `attach_logging` で
  任意接続でき、Logging OFF でも Collector/Diagnostics/Debug が
  成立する。
- **Domain Owner**: `registration_port()` から得た registrar で
  Source を登録し、lease の `retire` で quiescence を宣言する
  既存契約をそのまま使う。Runner は domain の writer を直接
  制御しない。
- **OBS-11 残件の実装者**: GUI 向け non-RT Snapshot/Subscription・
  concrete probe adapter・両 Consumer 受入の土台となる確定 API を
  得る。

## Scope

### In Scope

- `src/integration/` モジュール群（`mod.rs` / `config.rs` /
  `runner.rs` / `health.rs` / `tests.rs` を想定）と `lib.rs` への
  feature-gated module 登録・crate doc 更新。
- `ObservabilityConfig`/`DebugSettings`/`ExportPolicy`（
  `tracing-export` feature 時）/`ConfigError`: subsystem 独立設定と
  typed validation。
- `ObservationRunner`/`RunnerState`/`PollOutcome`/`RunnerError`/
  `ShutdownReport`/`ReconfigureReport`: 非 RT poll 駆動・6 状態
  ライフサイクル・段階的 drain・動的再構成。
- `attach_logging`（`logging` feature 時）: `WriterGuard`/`LoggingHealth`
  の任意接続と停止時 flush/close 順序への組込み。
- `RunnerHealth`/`ExportTotals`: Logger 非依存の合成 health snapshot。
- `integration` feature 追加と feature 組合せ検証
  （default / integration / +tracing-export / +logging /
  +allocation-probe / full）。
- 契約試験: lifecycle 全遷移・不正遷移拒否・poll 駆動・Source 登録・
  Snapshot 照会・停止 drain 順序・Partial/Unknown の正直な報告・
  reconfigure の Validate→Prepare→Activate→Retire・global/scoped
  Dispatch・Logging OFF・health 合成・no-alloc 非該当（非 RT）。
- README/rustdoc: 起動・通常停止・異常停止・Logging OFF・Debug
  Session の使用例と制約。

### Out of Scope

- AudioLens #138・Synvelis #81 の consumer 統合・両 Consumer 受入 —
  NotRun（#281/#219 cross-repo gate）。
- RT callback での Subscriber/allocator/lock/IO/非有界 wake 混入の
  実機計測・AudioLens f64/Synvelis 再生回帰 — NotRun（consumer gate）。
  本 module は RT 呼出可能 API を持たない構造で保証する。
- Linux/macOS 実機・MSRV lane — NotRun（#219/#223 gate）。
- concrete probe adapter の `ruviform-audio` 側実装 — 別 Issue/slice
  （#292 で port のみ確定済み）。
- OTLP/Prometheus 等の外部 exporter、独自 Logger/Span/IPC/UI
  Framework、GUI Subscription/IPC 転送、Host 認可機構そのもの、
  `LoggingConfig` の config 統合（#276 所有・無条件統合禁止）、
  既存 collector/diagnostics/debug/logging/tracing_export/source/
  metrics/record/rt_bridge/profiling の公開 API 変更、新規クレート、
  新規外部依存（`integration` feature は依存を追加しない）、`unsafe`。

## Requirements

- 全公開 API は非 RT only（alloc/lock/thread を伴う）。RT writer
  経路は一切変更せず、runner は RT thread・registry lock・writer
  cell に触れない。
- `ObservabilityConfig` の各 subsystem 設定は独立であり、
  `diagnostics: None`/`debug: None`/`export` 無しで Collector 単体
  運用が成立する。`LoggingConfig` は config に含めず、logging は
  `attach_logging` の明示接続のみ。
- lifecycle は `Unconfigured → Prepared → Ready → Running →
  Draining → Stopped` の一方向。各遷移は typed `RunnerError` で
  不正遷移を拒否し、放棄した要求を成功と報告しない。`Drop` は
  未完了の runner を best-effort で drain する。
- 停止順序は「`begin_shutdown`（Draining 宣言）→ domain quiescence
  → `finish_shutdown`（Collector 最終 Poll → Collector `shutdown`
  終端 batch → Exporter 終端 emit → WriterGuard drop）」を守る。
  quiescence 未確認の source は collector が `Partial` +
  `WriterNotQuiesced` で drain し、runner はそれを隠さない。
  「完全保存」は主張せず、`ShutdownReport` が回収結果を数値で
  報告する。
- `reconfigure` は Validate → Prepare → Activate → Retire を原子的
  に近い順序で非 RT 実行する。検証失敗・構築失敗時は旧構成を
  維持する。retire では旧 Collector を drain し終端 batch を旧
  Exporter で emit してから破棄し、diagnostics/debug の保持状態・
  debug session・source 再登録の非継承を report で正直に報告する。
- `tracing::Dispatch` は Host 注入のみ。library は global default を
  install せず、scoped `with_default` の外側 dispatch・guard 寿命に
  依存しない。global 既設定時も scoped 注入が優先されることを試験
  する。
- `RunnerHealth` は source に登録せず固定サイズカウンタ/スナップ
  ショット合成であり、logger/tracing/subscriber を一切経由しない
  （自己再帰なし）。loss は層ごとに区別する（collector
  discontinuity/clock anomaly、export suppression、rt record
  loss、writer queue full、sink error、report 失敗、debug session
  容量）。
- feature: `integration = []` を追加し default は変更しない。
  `tracing-export`/`logging`/`allocation-probe` との組合せを
  `--no-default-features`・各 feature・full で compile 検証する。
- 全公開 enum は `#[non_exhaustive]`。`unsafe` 不使用。新規外部
  依存なし。

## Acceptance Criteria

- [ ] AC-1: `ObservabilityConfig` が collection/diagnostics/debug/
  tracing-exposure・export policy を独立設定でき、`LoggingConfig`
  を無条件統合せず、`diagnostics: None`・`debug: None`・export 無し・
  Logging 未接続で Collector 単体運用できる試験がある。
- [ ] AC-2: `ObservationRunner` が `Collector::poll(now)` を非 RT で
  駆動し、Source 登録（`registration_port`）・Snapshot 照会
  （`snapshot`/`latest_batch`/`series`/`history`/`collector_handle`）・
  停止/Drain レポートを提供する試験がある。
- [ ] AC-3: `Unconfigured→Prepared→Ready→Running→Draining→Stopped`
  の全遷移と不正遷移の typed 拒否、`begin_shutdown`→`finish_shutdown`
  の drain 順序（最終 Poll→終端 batch→exporter emit→writer close）
  を試験する。quiescence 未確認 source が `Partial`+
  `WriterNotQuiesced` で報告され、完全保存を主張しない。
- [ ] AC-4: `tracing::Dispatch` の Host 明示注入・global 既設定下・
  scoped guard での emit が注入 dispatch に限定される試験がある
  （library が `set_global_default` を呼ばない）。
- [ ] AC-5: `reconfigure` が Validate→Prepare→Activate→Retire を
  非 RT で実行し、検証失敗時に旧構成を維持、retire で旧 Collector
  drain + 終端 emit + 非継承項目の報告を行う試験がある。
- [ ] AC-6: `RunnerHealth` が poll 遅延・Source/Series 予算・export
  suppression・record loss・queue full・Sink Error・Report 失敗・
  Debug session 容量を Logger 非依存で合成し、自己再帰しない
  試験がある。
- [ ] AC-7: `integration` feature 追加後も default build は軽量 core
  を維持し、`mise run fmt-check`/`check`/`lint`/`test`/`test-rt`
  および feature 組合せ build が pass する。
- [ ] AC-8: README/rustdoc に起動・通常停止・異常停止・Logging
  OFF・Debug Session の使用例と制約が記載される。
- [ ] AC-9: Issue #293 の残件（両 Consumer 受入・RT 実機計測・
  他 OS/MSRV・probe adapter・外部 exporter）は NotRun/deferred と
  して正直に記録され、Issue は Open のまま残る。

## Deferred Design Concerns

<!-- Intentionally deferred future work. Link the deferred draft doc via relations.defers. -->

- 両 Consumer（AudioLens #138 / Synvelis #81）での統合受入・
  RT noalloc/latency/音声同値回帰 | #281/#219 cross-repo gate |
  consumer 環境が必要 | worker 環境は単一 repo。
- GUI 向け non-RT Snapshot/Subscription・IPC 転送・Host 認可・
  設定優先順位（GUI/CLI） | consumer Host 側 | #293 残件 | library は
  pull snapshot と capability モデルまで。
- concrete probe adapter（`ruviform-audio` → `debug::ProbeSource`）|
  `ruviform-audio` 側 | 別 Issue | 依存循環のため本 crate に置けない。
- OTLP/Prometheus 等の外部 exporter・専用 Observability 基盤 |
  Issue 非対象 | 実需要 | 初期完成条件に混ぜない。
- `LoggingConfig` の `ObservabilityConfig` 統合・logging filter の
  runner 管理 | #276 所有・Host 責務 | 別 Issue/改訂 | 無条件統合
  禁止のユーザー指定。
- Linux/macOS 実機・MSRV lane | #219/#223 gate | CI 環境 | worker は
  Windows のみ。

## Sources and Relations

- Issue #293（本 spec の正本・slice 対応付け）、親設計 Issue #275
  （単一クレート `ruviform-observability`・module map の
  `integration`・feature 方針・安全/所有権の合意版）。
- `docs/specs|designs/issue-276-generic-logging-adapter.md`
  （`LoggingConfig`/`build_logging`/`WriterGuard`/`LoggingHealth` —
  logging は所有権を侵さない任意接続）、
  `docs/specs|designs/issue-278-observability-tracing-export.md`
  （`TracingExporter::emit(&batch)`/`TracingFindingSink`/Host 注入
  `Dispatch`）、
  `docs/specs|designs/issue-289-observability-collector-metrics.md`
  （`Collector::poll`/`CollectorConfig`/`ObservationBatch`/
  `MetricsSnapshot`/`CollectorHealth`）、
  `docs/specs|designs/issue-291-observability-diagnostics.md`
  （`Diagnostics`/`DiagnosticsConfig`/`DiagnosticsHealth`）、
  `docs/specs|designs/issue-292-observability-debug-inspection.md`
  （`DebugInspector`/`DebugConfig`/`DebugAuthority`/`SessionPermit`/
  `DebugHealth`/`SessionView`）。
- `docs/designs/foundation-observability-minimal-api.md`・
  `docs/designs/shared-foundations-and-modularization-policy.md`
  （責務区分の正本）。
