---
id: "TSPEC-zq1tvuEsFrUjTnAjn3csSR"
type: "test-spec"
status: "approved"
title: "Issue #293 OBS-11 ObservabilityConfig/ObservationRunner 試験仕様"
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
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-293-observability-integration-runner.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-292-observability-debug-inspection-tests.md"
    - "docs/test-specs/issue-291-observability-diagnostics-tests.md"
    - "docs/test-specs/issue-289-observability-collector-metrics-tests.md"
    - "docs/test-specs/issue-278-observability-tracing-export-tests.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-293-observability-integration-runner.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-293-observability-integration-runner.md"
    - "docs/designs/issue-293-observability-integration-runner.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/275"
    - "https://github.com/scarecrow173/ruviform/issues/293"
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/audio-lens/issues/138"
    - "https://github.com/scarecrow173/synvelis/issues/81"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/audio-lens/issues/138"
    - "https://github.com/scarecrow173/synvelis/issues/81"
  deferred-by: []
---

# Issue #293 OBS-11 ObservabilityConfig/ObservationRunner 試験仕様

## Purpose

Issue #293（OBS-11）の `src/integration/` slice が、subsystem 独立
設定・6 状態ライフサイクル・非 RT poll 駆動・段階的 drain・動的
再構成・Host 注入 dispatch・Logger 非依存 health を契約通りに
提供することを保証する。実装が変わっても「何を保証すべきか」を
固定し、回帰検出の正本とする。

## Feature

`integration` feature で提供される `ObservabilityConfig`/
`ObservationRunner` は既存の `Collector`/`Diagnostics`/
`DebugInspector`/`TracingExporter`/logging adapter の公開 API のみを
消費し、内部を再実装しない。`ObservabilityConfig` は
collection（`CollectorConfig`）・diagnostics（`Option`）・debug
（`Option<DebugSettings>`）・export（`tracing-export` 時
`ExportPolicy`）を独立に持ち、`LoggingConfig` を含まない。
`ObservationRunner` は `Unconfigured→Prepared→Ready→Running→
Draining→Stopped` を一方向に遷移し、`poll_once(now)` で
`Collector::poll` → `diagnostics.evaluate` → `exporter.emit` を
束ねて駆動する。停止は `begin_shutdown`（Draining 宣言・domain
quiescence 猶予）→ `finish_shutdown`（最終 Poll→Collector
shutdown 終端 batch→Exporter emit→WriterGuard drop）の順序を守る。
`reconfigure` は Validate→Prepare→Activate→Retire を非 RT で行い、
retire で旧構成を drain する。`RunnerHealth` は全層の既存 health
を合成した固定サイズ snapshot で、logger/tracing を経由しない。

## Rules

- **Config 独立性**: `diagnostics: None`・`debug: None`・export 無し
  （`dispatch: None` または feature 無し）・logging 未接続の構成で
  Collector 単体 runner が全 lifecycle・poll・snapshot・drain を
  完遂する。`LoggingConfig` は `ObservabilityConfig` のフィールドに
  存在しない。
- **Lifecycle 一方向性**: `Unconfigured→Prepared→Ready→Running→
  Draining→Stopped` 以外の遷移は `RunnerError::InvalidState` で
  拒否される。`prepare` は Unconfigured/Stopped のみ、`activate`
  は Prepared のみ、`poll_once` は Ready/Running のみ、
  `begin_shutdown`/`finish_shutdown`/`reconfigure` の受理状態は
  設計通り。放棄した要求を成功と報告しない。
- **Poll 合成**: `poll_once(now)` は `collector.poll(now)` の
  `ObservationBatch` を返し、diagnostics 有効時は同 poll の
  `DiagnosticSnapshot` を `latest_diagnostics` に保持、export 有効
  時は `TracingExporter::emit(&batch)` を呼び `EmitStats` を累積する。
  poll は `&mut self` の単一 owner で直列化され、RT thread からは
  呼べない（型レベルで非 RT）。
- **Drain 順序**: `finish_shutdown`/`shutdown` は (1) Collector 最終
  Poll、(2) `collector.shutdown()` の終端 batch commit、(3) 終端
  batch の exporter emit、(4) logging `WriterGuard` の順次 drop
  をこの順で実行する。quiescence 未確認の Active source は
  `Partial`+`WriterNotQuiesced` として drain され、
  `ShutdownReport.completeness` は `PartialDrains{count}` を示す。
  `AllQuiesced`/`NoSources`/`PartialDrains` 以外の「完全保存」
  表現は存在しない。
- **Reconfigure 原子性**: `reconfigure` は Validate 失敗・構築失敗
  で旧構成を一切変更せずエラーを返す。成功時は新 components に
  swap（Activate）してから旧 components を drain（Retire）し、
  終端 batch を旧 exporter で emit する。source 再登録・
  diagnostics retained・debug session/records・exporter baseline
  の非継承が `ReconfigureReport` に記録される。
- **Dispatch 注入**: library は `set_global_default`/`init` を呼ば
  ない。`ExportPolicy.dispatch` の `Dispatch` のみが emit 先で、
  `TracingExporter` の scoped `with_default` が ambient/global
  dispatch より優先される。global 既設定下でも注入 dispatch に
  emit されること、scoped guard 破棄後も注入 dispatch が有効な
  ことを試験する。
- **Health 合成**: `RunnerHealth` は `state`・runner 累積
  counter・`CollectorHealth`・`DiagnosticsHealth`・`DebugHealth`・
  `SessionView`・`ExportTotals`・`HealthSnapshot`(logging)・
  `last_shutdown` を合成し、source 登録・tracing emit・logging
  書込みを一切行わない（自己再帰なし）。loss は層ごとの既存
  counter をそのまま保持し、runner 独自の集約で潰さない。
- **Logging 接続**: `attach_logging(guards, health)` は Prepared
  以降・Draining 前のみ受理し、二重呼出しは拒否する。guards は
  `finish_shutdown`/`shutdown`/`Drop` の最終段で drop され、
  `LoggingHealth` は guard 共有 counter 経由で shutdown 後も
  snapshot 可能。attach なしでは `RunnerHealth.logging` は `None`。

## Examples

### Lifecycle と drain 順序

- **Given** Ready 状態の runner に Active source が 1 件あり lease
  が retire されていない
- **When** `begin_shutdown(now)` → domain が writer 停止を確認せず
  `finish_shutdown(now)` を呼ぶ
- **Then** collector `shutdown` がその source を `Partial`+
  `WriterNotQuiesced` で drain し、`ShutdownReport.completeness` は
  `PartialDrains{count:1}`、`partial_drains >= 1` となり、
  `writers_closed` が logging attach 時 true になる。

- **Given** Ready 状態の runner で source が lease `retire` 済み
- **When** `shutdown(now)` を一括呼出し
- **Then** collector は quiesced drain で `AllQuiesced` を報告し、
  終端 batch が `latest_batch()` に commit され、`state()` は
  `Stopped` になる。

### Reconfigure

- **Given** Running 状態で `poll_interval` を変えた新
  `ObservabilityConfig`
- **When** `reconfigure(new_config)` を呼ぶ
- **Then** Validate→Prepare→Activate→Retire が実行され、新 collector
  で `poll_once` が成功し、旧 collector の終端 batch が旧 exporter
  で emit され、`ReconfigureReport.retired` に drain 結果と非継承
  項目（source 再登録要・diagnostics retained 数等）が記録される。

- **Given** Running 状態
- **When** `collection.poll_interval = Duration::ZERO` の無効 config
  で `reconfigure` を呼ぶ
- **Then** `InvalidConfig` が返り、`state()` は Running のまま・
  旧 components が維持される。

### Dispatch

- **Given** `tracing-export` feature で `ExportPolicy{dispatch: Some(d),
  ..}` を持つ config
- **When** global default に別の `Dispatch` が install 済みの状態で
  `poll_once` を呼ぶ
- **Then** batch/finding の tracing event は `d` にのみ emit され、
  global 側 subscriber には現れない。`d` が `Dispatch::none()` の
  場合は emit は no-op で `EmitStats` の dropped 系は増えない
  （none dispatch は有効な dispatch であり suppression ではない）。

### Health

- **Given** logging attach 済みで sink queue を溢れさせた runner
- **When** `health()` を呼ぶ
- **Then** `RunnerHealth.logging.sinks[0].dropped_queue_full > 0`・
  `export`/`collector`/`diagnostics`/`debug_session` の各 field が
  対応する既存 health 値を反映し、health 取得自体がいずれの sink
  にも event を書かない。

## Constraints

- 全 API は非 RT（alloc/lock/thread を伴う）。RT thread から呼ぶ
  経路は型・feature ともに存在しない。
- `tracing-export` feature 無しでは `ObservabilityConfig` に
  `export` field が存在せず、`ExportPolicy` 型自体が export され
  ない。`logging` feature 無しでは `attach_logging` が存在しない。
- `integration` feature 無しでは `integration` module 自体が
  存在しない（default build は軽量 core を維持）。
- 新規外部依存・`unsafe` なし。既存 module の公開 API は変更しない。

## Deferred

- 両 Consumer 受入・RT 実機計測・他 OS/MSRV lane・probe adapter・
  外部 exporter — spec の Deferred Design Concerns と同一
  （#281/#219 gate・別 Issue）。
