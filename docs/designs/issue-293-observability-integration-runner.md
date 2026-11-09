---
id: "DESIGN-DQg7hyyzqx49OBMHYx8lv8"
type: "design"
status: "approved"
title: "Issue #293 OBS-11 ObservabilityConfig + ObservationRunner 設計"
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
    - "docs/specs/issue-293-observability-integration-runner.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-289-observability-collector-metrics.md"
    - "docs/designs/issue-291-observability-diagnostics.md"
    - "docs/designs/issue-292-observability-debug-inspection.md"
    - "docs/designs/issue-278-observability-tracing-export.md"
    - "docs/designs/issue-276-generic-logging-adapter.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-292-observability-debug-inspection.md"
    - "docs/designs/foundation-observability-minimal-api.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-293-observability-integration-runner.md"
    - "docs/designs/issue-289-observability-collector-metrics.md"
    - "docs/designs/issue-291-observability-diagnostics.md"
    - "docs/designs/issue-292-observability-debug-inspection.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-293-observability-integration-runner-tests.md"
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

# Issue #293 OBS-11 ObservabilityConfig + ObservationRunner 設計

## Overview

`src/integration/` に Host 向け non-RT 統合層を追加する。役割は
「構成の検証・構築」「ライフサイクル状態機械」「poll の束ね駆動
（collect → diagnostics evaluate → tracing emit）」「段階的 drain」
「動的再構成」「合成 health snapshot」の 6 点に限定し、各 subsystem
（collector/diagnostics/debug/tracing_export/logging）の内部実装は
一切再実装せず公開 API のみを消費する。

feature は `integration = []`（新規依存なし）。`tracing-export`・
`logging`・`allocation-probe` との組合せは `#[cfg(feature)]` で
選択的に有効化し、`integration` 単体でも Collector-only runner が
成立する。

## Module Layout

- `src/integration/mod.rs` — module doc（責務・非 RT 契約・feature
  構成・drain 順序の要約）と re-export。
- `src/integration/config.rs` — `ObservabilityConfig`/
  `DebugSettings`/`ExportPolicy`/`ConfigError`。
- `src/integration/runner.rs` — `ObservationRunner`/`RunnerState`/
  `PollOutcome`/`RunnerError`/`ShutdownReport`/`ReconfigureReport`。
- `src/integration/health.rs` — `RunnerHealth`/`ExportTotals`。
- `src/integration/tests.rs` — module 内 unit test。
- `tests/integration.rs` — `required-features = ["integration"]` の
  契約 test（内部で `tracing-export`/`logging` を `#[cfg]` 分岐）。

## Config Design

```rust
pub struct ObservabilityConfig {
    /// OBS-07 正本。poll 間隔・Source/Series/Batch 予算・History
    /// 予算（metrics retention）・stale 閾値を含む。metrics 側の
    /// 独立設定はこの history/series 項目が担う（metrics は
    /// Collector の出力であり別 engine を持たないため、第二の
    /// metrics config は作らない — authoritative config の複製を
    /// 避ける）。
    pub collection: CollectorConfig,
    /// Diagnostics engine。`None` で subsystem 無効。
    pub diagnostics: Option<DiagnosticsConfig>,
    /// Debug inspector。`None` で subsystem 無効。
    pub debug: Option<DebugSettings>,
    /// Tracing exposure / export policy。`tracing-export` feature
    /// 時のみ存在。
    #[cfg(feature = "tracing-export")]
    pub export: ExportPolicy,
}
```

- `DebugSettings { config: DebugConfig, authority: Option<Arc<DebugAuthority>> }`。
  authority を `Arc` で持つのは、`DebugInspector::with_authority` が
  construction 時に token を clone するため、Host が permit 発行用の
  `DebugAuthority` を保持し続けられるようにするため。`authority: None`
  の inspector は session 操作が `NoAuthority` で拒否される
  （inspection/report 機能は動く）。
- `ExportPolicy { dispatch: Option<Dispatch>, max_emissions_per_poll: usize,
  diagnostics_events: bool }`（`tracing-export` 時）。`dispatch: None`
  は export 無し（runner は `TracingExporter` を構築しない）。
  `diagnostics_events: true` かつ diagnostics 有効かつ dispatch 有りの
  とき、`prepare` が `TracingFindingSink` を diagnostics の sink に
  設定する（Host は `diagnostics_mut().set_sink(..)` で後から上書き
  可能 — 自動配線は既定であり強制ではない）。
- `ConfigError { Collector(CollectorConfigError),
  Diagnostics(DiagnosticsConfigError) }` — `validate()` は
  `CollectorConfig::validate`/`DiagnosticsConfig::validate` に委譲し、
  新しい検証規則を発明しない。`DebugConfig`/`ExportPolicy` は構築時に
  不正値を取り得ない（`max_emissions_per_poll` は 0 = emit なしを
  許容）。

## Runner Design

```rust
pub struct ObservationRunner {
    state: RunnerState,
    components: Option<Components>,
    // runner-level accumulators（reconfigure を跨いで保持）
    polls_total: u64,
    poll_errors_total: u64,
    reconfigures_total: u64,
    shutdowns_total: u64,
    last_shutdown: Option<ShutdownReport>,
    #[cfg(feature = "tracing-export")] export_totals: ExportTotals,
    #[cfg(feature = "logging")] logging: Option<LoggingAttachment>,
}

struct Components {
    collector: Collector,
    diagnostics: Option<Diagnostics>,
    debug: Option<DebugInspector>,
    #[cfg(feature = "tracing-export")] exporter: Option<TracingExporter>,
    latest_diagnostics: Option<DiagnosticSnapshot>,
}
```

### Lifecycle

`RunnerState { Unconfigured, Prepared, Ready, Running, Draining,
Stopped }` — `#[non_exhaustive]`・`Copy`。一方向遷移のみ:

| 遷移 | API | 条件 |
| --- | --- | --- |
| → Unconfigured | `ObservationRunner::new()` | 常に |
| Unconfigured/Stopped → Prepared | `prepare(config)` | Validate→構築成功 |
| Prepared → Ready | `activate()` | components 存在 |
| Ready → Running | `poll_once(now)` 初回成功 | components 存在 |
| Ready/Running → Draining | `begin_shutdown()` | components 存在 |
| Draining → Stopped | `finish_shutdown(now)` | drain 完了 |
| Ready/Running → Stopped | `shutdown(now)` | begin+finish 一括 |
| Ready/Running → （同状態維持） | `reconfigure(config)` | Validate→Prepare→Activate→Retire |

- `Ready` は「構成済み・activate 宣言済み・まだ poll 未実施」、
  `Running` は「少なくとも 1 回の `poll_once` が commit 済み」。
  poll 前の Ready でも snapshot/health/登録は有効。
- `prepare` は Unconfigured と Stopped（再起動 = 全構成要素を新規
  構築）からのみ許可。Prepared/Ready/Running/Draining からの
  `prepare` は `InvalidState` で拒否（Running 中の変更は
  `reconfigure` のみ）。
- `activate` は Prepared のみ。Ready/Running での再 `activate` は
  拒否（冪等成功にしない — 状態遷移を成功と偽らない）。
- `poll_once` は Ready/Running のみ受理。`now` は Host が供給する
  `Instant`（RT thread の clock mark を渡してもよいが、本 API は
  非 RT のみ）。各 poll で:
  1. `collector.poll(now)` → `ObservationBatch`
  2. `diagnostics.evaluate(&snapshot, now)`（存在時）→
     `DiagnosticSnapshot` を `latest_diagnostics` に保持
  3. `exporter.emit(&batch)`（存在時）→ `EmitStats` を累積
  4. `PollOutcome { batch, diagnostics, emit }` を返す
- `begin_shutdown` は Draining に遷移し、以降の `poll_once`/
  `reconfigure`/`activate`/`prepare` を拒否する。collector への
  影響はまだ与えない — domain が writer 停止・lease `retire`・
  quiescence 確認を行う猶予を Host が持つ（runner は writer を
  直接制御しない）。
- `finish_shutdown(now)`（または `shutdown(now)` 一括）は順序を
  守って実行:
  1. Collector 最終 Poll（`collector.poll(now)` — 失敗は無視せず
     `poll_errors_total` に計上した上で継続。CollectorStopped は
     既に drain 済みの意で続行可）
  2. `collector.shutdown()` → 終端 drain batch（`latest_batch()` に
     commit 済み）
  3. 終端 batch を `exporter.emit`（存在時 — Removed/Partial の
     最終観測を export 経路へ届ける "Exporter drain"）
  4. `logging` の `WriterGuard` を順に drop（flush/close —
     `LoggingHealth` snapshot は guard 共有カウンタ経由で
     shutdown 後も読める）
  5. `ShutdownReport` を生成して Stopped へ。

  quiescence 未確認のまま残った Active source は collector の
  `shutdown` が `Partial`+`WriterNotQuiesced` で強制 drain する
  — runner はこれを `ShutdownReport` の数値でそのまま報告し、
  「完全回収」とは表示しない。

### Drain Report

```rust
pub struct ShutdownReport {
    pub polls_completed: u64,          // この構成での poll_once 成功数
    pub poll_errors: u64,
    pub final_poll_generation: Option<u64>,   // 最終 Poll の batch generation
    pub terminal_generation: Option<u64>,     // collector.shutdown() の終端 batch generation
    pub measurements_drained: u64,     // 終端 batch の drained 計数
    pub partial_drains: u64,           // Partial + WriterNotQuiesced の計数
    pub removed_total: u64,            // 終端 batch の Removed 計数
    pub sources_active_at_drain: usize,// shutdown 時点で Active だった source 数
    pub emit: Option<EmitStats>,       // 終端 emit の結果
    pub writers_closed: bool,          // logging guards を drop 済みか
    pub debug_session_abandoned: bool, // session が非 Disabled だったか
    pub diagnostics_evidence_lost: usize,
    pub diagnostics_findings_lost: usize,
    pub completeness: DrainCompleteness,
}

#[non_exhaustive]
pub enum DrainCompleteness {
    /// drain 対象の全 source が quiescence 確認済み（retire 済み）。
    AllQuiesced,
    /// quiescence 未確認 source があり Partial drain が発生。
    PartialDrains { count: u64 },
    /// source が存在しなかった（drain 対象なし）。
    NoSources,
}
```

「完全保存」は主張しない: `completeness` は回収の確度を型で示し、
Host crash 等で runner 自体が死んだ場合の `Unknown` は consumer 側
（#281 gate）が扱う — runner は生きている限り実測値のみ報告する。

### Dynamic Reconfigure

`reconfigure(config) -> Result<ReconfigureReport, RunnerError>`
（Ready/Running のみ）:

1. **Validate**: `ObservabilityConfig::validate()`。失敗時は旧構成
   を一切変更せず `InvalidConfig` を返す。
2. **Prepare**: 新 `Components` を構築（新 `Collector`・新
   `Diagnostics`・新 `DebugInspector`・新 `TracingExporter`）。
   構築失敗時も旧構成を維持（Collector/Diagnostics の `new` は
   validate 済み config では失敗しないが、型として失敗経路を残す）。
3. **Activate**: `components` を新構成に swap。`state` は Running
   のまま（Ready なら Ready のまま）。polls/health 累積は runner
   レベルなので継続。
4. **Retire**: 旧 components を drain — `collector.shutdown()` で
   終端 batch → 旧 exporter で emit → diagnostics の保持
   evidence/findings 数・debug session 状態・Active source 数を
   `ReconfigureReport.retired`（`ShutdownReport` 再利用）と
   非継承項目に記録してから drop。

非継承を正直に報告する項目: source registration（新 collector は
`SourceId` を 1 から再採番 — domain は新 `registration_port()` で
再登録）、diagnostics の retained evidence/findings、debug の
active session・retained records、tracing exporter の emission
baseline（新 exporter は初回 poll で再 baseline — OnIncrease は
emit されない）。`ReconfigureReport` は `retired: ShutdownReport`
+ `non_migrated` 集計を持つ。

### Poll 駆動モデル

runner はスレッドを持たない。Host が `poll_interval()`（config の
`collection.poll_interval`）に従い `poll_once(Instant::now())` を
非 RT thread から呼ぶ。collector の `PollRunner`（専用 thread）は
runner とは別経路として残り、runner はそれを内包しない — 統一
境界は「Host 駆動の poll + 構成 + drain」であり、thread 管理は
Host 責務（#275 の責務区分に一致）。

### Accessors

- `state() -> RunnerState`
- `collector_handle() -> Option<CollectorHandle>`（Ready 以降）
- `registration_port() -> Option<CollectorRegistrar>`
- `snapshot() -> Option<MetricsSnapshot>`（`latest_batch()`/
  `latest_batch_generation()`/`series()`/`history()`/`poll_interval()`
  も同様に Option 委譲）
- `diagnostics() -> Option<&Diagnostics>` / `diagnostics_mut()`
- `debug() -> Option<&DebugInspector>` / `debug_mut()`
- `health() -> RunnerHealth`
- `shutdown_report() -> Option<&ShutdownReport>`

`CollectorHandle`/`CollectorRegistrar` は `Clone + Send + Sync` で
GUI/他 thread の pull 境界としてそのまま使える。

## Health Design

```rust
pub struct RunnerHealth {
    pub state: RunnerState,
    pub polls_completed: u64,
    pub poll_errors_total: u64,
    pub reconfigures_total: u64,
    pub shutdowns_total: u64,
    pub collector: Option<CollectorHealth>,       // poll 遅延・source/series 予算・loss
    pub diagnostics: Option<DiagnosticsHealth>,   // report 失敗・sink panic/suppression
    pub debug: Option<DebugHealth>,               // session drain/report 失敗
    pub debug_session: Option<SessionView>,       // session 容量（records retained/overflowed 等）
    pub export: Option<ExportTotals>,             // suppression/cap/loss 累積
    pub logging: Option<HealthSnapshot>,          // queue full・sink error・disconnect
    pub last_shutdown: Option<ShutdownReport>,    // 直近 drain の実測
}
```

- 全フィールドは既存型の Clone/Snapshot 合成であり、runner 自身を
  source として登録しない・tracing/logging を経由しない（自己再帰
  なし）。`health()` は固定サイズ値を返し、内部で emit/log を行わ
  ない。
- loss taxonomy は層ごとに区別: collector の `discontinuities_total`/
  `clock_anomalies_total`/`histogram_inconsistencies_total`、
  export の `snapshot_suppressed`/`policy_suppressed`/
  `valueless_skipped`/`cap_dropped`、rt record loss は debug session
  の `records_overflowed`/`pending_records`/drain accounting
  （SessionView 経由）と collector batch の `Measurement.loss`、
  logging の `dropped_queue_full`/`write_failures`/`flush_failures`/
  `channel_disconnected`、report 失敗は diagnostics
  `reports_rejected_total`・debug `reports_rejected_total`、debug
  session 容量は SessionView の retained/overflowed/pending。
- `ExportTotals` は `EmitStats` を runner 生存期間で累積した
  固定サイズ struct（reconfigure でもリセットしない — health は
  runner の通算値、構成ごとの値は `PollOutcome.emit` が持つ）。

## Logging Integration（`logging` feature）

`ObservabilityConfig` には logging を含めない（#276 所有・無条件
統合禁止）。代わりに:

```rust
#[cfg(feature = "logging")]
pub fn attach_logging(&mut self, guards: Vec<WriterGuard>, health: LoggingHealth)
    -> Result<(), RunnerError>
```

- Host が `build_logging(&LoggingConfig)` で得た `guards`/`health`
  を Prepared 以降・Draining 前に接続する。`dispatch` は Host が
  `set_global_default`/scoped で自ら install する責務のまま
  （runner は dispatch を受け取らない・install しない）。
- `finish_shutdown`/`shutdown` の最終段で guards を drop し、
  writer flush/close を drain 順序に組み込む。guard drop は各
  worker の shutdown（drain→flush→join、timeout 時 abandon）を
  実行し、結果は `LoggingHealth` の `shutdown_abandoned`/
  `flush_failures`/`channel_disconnected` に残る。
- Logging OFF（attach なし）では `RunnerHealth.logging` が `None`
  であり他の全機能が成立する。

## Error Design

- `RunnerError`（`#[non_exhaustive]`）:
  - `InvalidState { state: RunnerState, operation: &'static str }`
  - `Unconfigured`（`activate`/`poll_once`/`shutdown` 等で
    components 不在 — `InvalidState` に統合してもよいが、
    「まだ prepare していない」は頻出のため独立 variant とする）
  - `InvalidConfig(ConfigError)`
  - `Collector(PollError)`（`poll_once` 内の collector エラー）
  - `AlreadyAttached`（`attach_logging` 二重呼出し）
- `ConfigError`（`#[non_exhaustive]`）: `Collector(CollectorConfigError)`
  / `Diagnostics(DiagnosticsConfigError)`。
- `prepare`/`activate`/`poll_once`/`begin_shutdown`/`finish_shutdown`/
  `shutdown`/`reconfigure`/`attach_logging` は全て `Result` を返し、
  状態違反・構築失敗を成功と偽らない。

## Non-Goals / Constraints

- runner は thread・timer・subscriber install・global dispatch・
  writer 直接制御を持たない。RT thread から呼べる API は存在しない。
- `unsafe` なし・新規外部依存なし・既存 module の内部変更なし。
- `integration` feature は依存 feature を強制しない
  （`integration = []`）。feature 組合せは additive に検証する。
- crate 全体の公開 enum は `#[non_exhaustive]`（workspace lint）。

## Testing Strategy（test-spec へ展開）

- module 内 unit test: lifecycle 全遷移・不正遷移・poll_once の
  batch/diagnostics/emit 合成・health 合成・reconfigure の
  Validate→Prepare→Activate→Retire・Partial drain の正直な報告。
- `tests/integration.rs`: 実 `tracing_subscriber` を使った scoped/
  global dispatch 注入試験・`WriterGuard` を伴う logging attach・
  Logging OFF での Collector-only E2E・Debug Session を通した
  session 容量の health 反映。
- feature 組合せ: `cargo check/test` を default /
  `--features integration` / `+tracing-export` / `+logging` /
  `+allocation-probe` / `--all-features` / `--no-default-features
  --features integration` で実行（mise タスク経由を優先、cargo は
  補助証拠）。
