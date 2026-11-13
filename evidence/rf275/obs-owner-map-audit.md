# Issue #275 OBS-01〜11 Owner/API/Feature/CI Case 監査証拠

- 監査日: 2026-11-14
- 対象 SHA: `512caca911d34e0196a87ea3565342c7ec4405ee`（作業開始時の
  worktree HEAD。merge 後の最終 SHA は PR 本文に記録）
- 目的: Issue #275 の残件「OBS-01〜11 の実装 Owner/公開 API/Feature
  Matrix/既存 CI Case の対応付け」「AL #127・SV #28 の独立 Headless
  受入の #281/#294 への引き渡し」「Deferred 範囲の Owner/Trigger 明記」
  の裏付けとなる実測スナップショット。
- 方法: `gh issue view`（Issue 状態）、`cargo tree`/`Cargo.toml`/
  `src/` 走査（crate・feature・公開 API）、`.github/workflows/*.yml`
  と `mise.toml` 走査（既存 CI lane）。canonical 検証は `mise.toml`
  タスク；`cargo tree` は補助証拠（canonical = mise）。

## 1. Issue 状態 snapshot（`gh issue view`、2026-11-14）

| Issue | 状態 | タイトル（要約） |
| --- | --- | --- |
| scarecrow173/ruviform#275 | OPEN | Observability統合設計正本 |
| ruviform#276 (OBS-01) | CLOSED | observability::logging 汎用 Logging Adapter |
| ruviform#277 (OBS-02) | CLOSED | 軽量 Primitive・Source 型・登録/寿命契約 |
| ruviform#278 (OBS-03) | CLOSED | tracing_export — Counter/RT Record 変換 |
| ruviform#279 (OBS-04) | CLOSED | Plugin Host 既存 4 Counter の Source 登録 |
| ruviform#280 (OBS-05) | CLOSED | RT 固定長 Record Bridge・Clock・欠落・Correlation |
| ruviform#281 (OBS-06) | OPEN | 両 Consumer 互換/RT/精度検証 Gate |
| ruviform#289 (OBS-07) | CLOSED | Collector・Metrics・履歴・Snapshot |
| ruviform#290 (OBS-08) | CLOSED | Profiling・RT Timing・CountingAllocator |
| ruviform#291 (OBS-09) | CLOSED | Diagnostics・Evidence・Finding・Health・Report |
| ruviform#292 (OBS-10) | CLOSED | Debug Inspection・Invariant・Session・Probe 連携 |
| ruviform#293 (OBS-11) | CLOSED | Host 統合・Lifecycle・設定・Export・GUI 購読境界 |
| ruviform#294 | OPEN | consumer-verify を UI/Golden/RT 横断 Gate へ拡張 |
| ruviform#219 | OPEN | 両 Consumer 並行開発の互換契約と Cross-Repo CI Gate |
| audio-lens#127 | OPEN | Ruviform 継続追従・高精度解析/GUI/CLI Consumer 回帰 |
| audio-lens#138 (AL-OBS-01) | CLOSED | al-observability 移行・Logging parity（履歴 Issue） |
| synvelis#28 | OPEN | Ruviform 継続追従・DAW Consumer 回帰 |
| synvelis#81 (SYN-OBS-01) | CLOSED | Rust Host 統合・Logging/Collector/Diagnostics（履歴 Issue） |

- OBS-01〜11 の実装 Issue は OBS-06（#281・検証 Gate、継続）を除き
  全て CLOSED。未着手の実装 Issue は存在しない — 残件は全て
  検証/Gate 領域であり、新規実装 Issue の起票は不要と判断した。
- AL #138・SV #81 は CLOSED の導入履歴 Issue であり、継続受入
  Owner は AL #127・SV #28（両者とも OPEN）。Closed 履歴 Issue を
  再 Open する記述は行わない。

## 2. crate / Feature / 公開 API 棚卸し

`crates/ruviform-observability`（workspace member、`Cargo.toml`
`default = []`）。

| feature | 状態 | 有効化されるもの |
| --- | --- | --- |
| （なし） | default | `core`/`source`/`collector`/`metrics`/`diagnostics`/`debug`/`record`/`rt_bridge`/`rt_timing`（無条件モジュール） |
| `logging` | opt-in | `logging` module + optional deps `tracing`/`tracing-subscriber`/`tracing-appender` |
| `tracing-export` | opt-in | `tracing_export` module + `dep:tracing` のみ |
| `allocation-probe` | opt-in | `profiling` module（依存追加なし・std のみ） |
| `integration` | opt-in | `integration` module（依存追加なし） |

workspace 内の feature unification 実測（補助証拠:
`cargo tree --workspace -e features -i ruviform-observability`）:
`ruviform-audio` の dev-dependency が `features = ["integration"]`
を要求するため、`cargo test --workspace` / `cargo check
--workspace --all-targets` 系では `default + integration` が有効化
される。`logging`/`tracing-export`/`allocation-probe` を有効化する
workspace member は存在しない。

### モジュール別 公開 API（代表型）と test 所在

| OBS | module(s) | feature | 代表公開 API | test 所在 |
| --- | --- | --- | --- | --- |
| OBS-02 | `core`, `source` | なし | `counter()`/`Counter`/`CounterReader`、`gauge()`/`Gauge`/`GaugeReader`、`histogram()`/`Histogram`/`HistogramReader`/`HistogramError`；`MetricDescriptor`/`CounterDescriptor`/`SourceId`/`SourceGeneration`/`SourceIdentity`/`ObservationRegistrar`/`RegistrationLease`/`SourceLifecycle`/`ObservationScope`/`CounterSource`/`MetricSource`/`MetricReader` | `src/core/*.rs`・`src/source/*.rs` inline unit test |
| OBS-07 | `collector`, `metrics` | なし | `Collector`/`CollectorRegistrar`/`CollectorHandle`/`PollRunner`/`ObservationBatch`/`MetricsSnapshot`/`CollectorHealth`/`CollectorState`/`CollectorConfig`/`PollError`/`ObservationWindow`；`Measurement`/`MeasurementValidity`/`MeasurementIssue`/`MetricValue` | `src/collector/tests.rs`（51 test）ほか inline |
| OBS-09 | `diagnostics` | なし | `Diagnostics`/`DiagnosticsHealth`/`Evidence*`/`DiagnosticFinding`/`FindingSeverity`/`FindingCertainty`/`FindingStatus`/`HealthState`/`HealthEvaluator`/`HealthRule`/`Hysteresis`/`DiagnosticSnapshot`/`DiagnosticReport`/`FindingSink`/`SinkPolicy`/`DiagnosticsEvent` | `src/diagnostics/tests.rs`（32 test） |
| OBS-10 | `debug` | なし | `DebugInspector`/`DebugConfig`/`DebugHealth`/`InspectionBudget`/`InspectionRequest`/`InspectionSnapshot`/`InvariantCheck`/`InvariantOutcome`/`InvariantVerdict`/`ProbeSource`/`DebugSessionState`/`DebugReport`/`ExportLimits`/`DEBUG_REPORT_SCHEMA_VERSION` | `src/debug/tests.rs`（45 test） |
| OBS-05 | `record`, `rt_bridge` | なし | `rt_bridge::bridge`/`RtRecorder`/`RecordSink`/`ReceivedRecord`；`ClockContext`/`CorrelationBook`/`CorrelationToken`/`CorrelationLink`/`CorrelationStatus`/`EventRecord`/`RecordHeader`/`EventId`。leaf 依存: `ruviform-rt`・`ruviform-media-core`（ADR obs-05） | `src/rt_bridge.rs`・`src/record/*.rs` inline unit test |
| OBS-08a | `rt_timing` | なし | `timing_bridge`/`TimingRecorder`/`TimingSink`/`TimingWindow`/`TimingSnapshot`/`TimingClock`/`TimingRecord`/`TimingSample`/`QuantileEstimate`/`TIMING_SCHEMA_VERSION` | `src/rt_timing/tests.rs`（15）+ inline |
| OBS-08b | `profiling` | `allocation-probe` | `CountingAllocator`/`AllocationScope`/`AllocationWindow`/`AllocationStats`/`OpStats` | `src/profiling/*.rs` inline unit test |
| OBS-01 | `logging` | `logging` | `LoggingConfig`/`SinkConfig`/`SinkDestination`/`LogFormat`/`RotationPeriod`/`SinkFailureMode`/`build_logging`/`build_logging_layers`/`LoggingHealth`/`SinkHealthSnapshot`/`HealthSnapshot`/`LoggingBuildResult`/`LoggingLayersResult`/`DEFAULT_STDERR_QUEUE_CAPACITY` | `tests/logging.rs`（28、`required-features=["logging"]`）+ `src/logging/writer.rs` inline（9）+ example `logging_host` |
| OBS-03 | `tracing_export` | `tracing-export` | `TracingExporter`/`TracingRecordExporter`/`TracingFindingSink`/`TraceContext`/`EmitStats`/`RecordEmitStats`/`DEFAULT_MAX_EMISSIONS_PER_POLL`/`DEFAULT_MAX_EVENTS_PER_DRAIN` | `src/tracing_export/tests.rs`（26）+ `trace_context.rs` inline（8）+ `tests/rt_boundary.rs`（3、`required-features=["tracing-export"]`） |
| OBS-11 | `integration` | `integration` | `ObservabilityConfig`/`ObservationRunner`/`RunnerState`/`RunnerHealth`/`PollOutcome`/`DrainCompleteness`/`ShutdownReport`/`ReconfigureReport`/`ConfigError`/`ExportPolicy`/`DebugSettings`/`NonMigrated`/`RunnerError`/`ExportTotals` | `src/integration/tests.rs`（12）+ `tests/integration.rs`（15、`required-features=["integration"]`） |
| OBS-04 | （`ruviform-plugin-host` 側） | なし（observability 側 feature なし） | `ObservationRegistrar` port の実装・`set_observation_registrar`・4 Counter（`dropped_submits`/`passthrough_submits`/`results_failed_observed`/`results_dropped`）の Source 登録 | `crates/ruviform-plugin-host` の test（workspace member として verify lane で実行） |
| OBS-06 | （検証 Gate — module なし） | — | Issue #281 本文の G0〜G4・Case 組（OBS-RT/OBS-LOG/OBS-SRC/OBS-PRIV/OBS-PROF/OBS-AL/OBS-SV） | #281/#219/#294 が Owner |

### RT 契約 test 名（`rt_` prefix、`mise run test-rt` = `cargo test
--workspace rt_` で選択）

- `core`: `rt_counter_ops_allocate_nothing`、
  `rt_gauge_ops_allocate_nothing`、
  `rt_histogram_observe_allocates_nothing`
- `debug`: `rt_debug_value_types_allocate_nothing`
- `record`: `rt_clock_context_ops_allocate_nothing`、
  `rt_record_ops_allocate_nothing`
- `rt_bridge`: `rt_try_record_and_counters_allocate_nothing`
- `rt_timing`: `rt_record_ops_allocate_nothing`（record.rs）、
  `rt_record_allocates_nothing`（tests.rs）
- `tracing_export`（feature `tracing-export` でのみ compile）:
  `rt_record_emits_fixed_schema` 他 `rt_record_*` 11 件
- `tests/rt_boundary.rs`（feature `tracing-export`）:
  `rt_core_primitive_ops_emit_no_tracing_events`、
  `rt_bridge_and_record_ops_emit_no_tracing_events`、
  `rt_timing_ops_emit_no_tracing_events`

## 3. 既存 CI lane 棚卸し

`ci.yml` は `workflow_dispatch` のみ（Issue #365/#366 — PR/Push では
起動しない）。`daily-ci.yml` が main SHA 変更検知で `ci.yml` を、
consumer tuple 変化で `consumer-verify.yml` を dispatch する。

| lane | OS runner | 内容（ observability 関連） |
| --- | --- | --- |
| `verify` | ubuntu-latest + windows-latest | Linux: `prek run --all-files`・`fmt-check`・`lint-full`（clippy -D warnings）。Windows: `check-full`・`build-full`（native build/link）。両 OS: `test-full`（`cargo test --workspace` — default+`integration` unify） |
| `verify-macos` | macos-latest | `check-full`・`build-full`・`test-full` |
| `rt-verify` | ubuntu-latest | strict `rt_` test 選択確認（0 件拒否）+ `mise run test-rt` |
| `msrv-verify` | ubuntu-latest | `test-observability-msrv` = `mise exec rust@1.89.0 -- cargo test -p ruviform-observability --all-features`（logging/tracing-export/allocation-probe/integration の test target を含む唯一の lane）+ ofx/ui-test/io の msrv |
| `consumer-verify.yml` `verify-audiolens` | ubuntu-latest | consumer 側 `check-ruviform-lock`/`check-full`/`check-cli-headless`/`test-full`/`sync-ui-deps`/`check-ruviform-rev`/`setup-ui`/`check-ui`/`test-ui` + ruviform 候補 tree の GUI-CONTRACT/GUI-SEMANTIC lane + extra tasks。**test-rt lane なし** |
| `consumer-verify.yml` `verify-synvelis` | ubuntu-latest | 上記に加え `test-rt`（`mise run test-rt`）・`check-headless-deps`・`build-ui` lane あり |
| `consumer-verify.yml` `summary` | ubuntu-latest | fail-closed 集約（not_run/unsupported を pass にしない） |

### Case 別の到達範囲（観測 crate 視点）

- **Native Build/Link（default feature）**: `verify`（Win/Linux）・
  `verify-macos` の check/build/test-full — 3 OS で到達。
- **Native Build/Link（feature 付き）**: `logging`/`tracing-export`/
  `allocation-probe` を個別または組合せで compile する既存 lane は
  **なし**。`msrv-verify` の `--all-features`（Linux・MSRV
  1.89 toolchain）のみがこれら module を compile+test する。
  Windows/macOS native での feature 付き build/link/test は
  **NotRun**。
- **Headless/契約 test**: default+`integration` は `test-full` で
  3 OS 到達。`logging`/`tracing-export`/`allocation-probe` 配下の
  test（`tests/logging.rs`・`tests/rt_boundary.rs`・
  `src/tracing_export/tests.rs`・`src/profiling/*`）は
  `--all-features` MSRV lane のみ。
- **RT lane**: `rt-verify`（ubuntu）の `test-rt` は workspace 全体の
  `rt_*` 名 test を選択 — default module の rt_* test と
  plugin-host/audio 系を含む。`rt_boundary.rs`・
  `tracing_export` の `rt_record_*` test は feature gate により
  本 lane では compile されず、`--all-features` MSRV lane でのみ
  実行される。
- **Feature matrix（`--no-default-features`・各 feature 単独・
  full consumer 組合せ）**: 既存 lane になし → **NotRun**
  （#281 G0 の gate 証拠項目）。
- **Consumer 側 headless Oracle**: `consumer-verify.yml` が
  候補 tuple（ruviform SHA + AL/SV resolved SHA）で consumer 側
  check/test/UI headless lane を実行。OBS 固有の Logging/
  Privacy/RT/精度の独立 Oracle は consumer repo 側 task
  （AL #127・SV #28 が Owner）に依存し、workflow の lane 記録は
  pass/not_run/unsupported を区別する。

## 4. Deferred 範囲の棚卸し（Issue #275 残件 3）

| Deferred 項目 | 現行の扱い | 再開 Owner | Activation Trigger |
| --- | --- | --- | --- |
| 実外部 Provider/Exporter（実 OTLP endpoint・外部 tracing backend 等） | Issue #275 非対象（必須 OTLP/Tracy/Prometheus を作らない）。in-repo の変換・health 契約のみ検証済み範囲 | Ruviform Observability owner — 利用決定時に新規 Issue を起票し #281 で gate 証拠を収集 | Host/Consumer が実 exporter 接続を製品機能として採用すると決定した時点 |
| 実 OS GUI（実 Window 上の Observability 表示/購読） | `integration` の GUI 向け non-RT Snapshot/Subscription は API/契約のみ。実画面検証は GUI-L2 | RF #293（API 境界）+ 利用機能の Owner Issue（AL #127/SV #28 の Windows L2 Case、RF #313/#314 real-window lane） | Host が Observability の GUI 診断表示を実際に提供・利用すると決定し Windows 利用機能として Required 化された時点 |
| 実 Device（物理 Audio/MIDI 機器） | RT Timing/Bridge の契約試験は headless のみ。実機負荷下の latency/欠落は未計測 | RF #281（OBS-06 gate の実機負荷段階）+ device 利用機能の Owner Issue | 実機 Device を使う製品機能の利用が決定した時点（POLICIES EXT-L3） |
| 長時間 Profiler（samply 等の長時間計測） | `allocation-probe`/`rt_timing` は開発・テスト用の有界計測のみ。長時間・実負荷 profiling は未実施 | RF #281 + 利用要件を起票する新規 Issue（必要時） | 長時間・実負荷での性能/確保時間計測要件が具体化した時点 |

いずれも `NotRun` を `Pass`/`Unsupported` に置き換えない。既存 CI
（Daily Full/手動 Full）の対象 SHA 証拠が揃うまで統合受入は
`NotRun`/Pending のまま。

## 5. 判定メモ

- 「Job が定義されている・green である」ことは個別 Case の Pass を
  示さない（POLICIES §3）。本表の CI 列は「どの lane がその Case を
  実行し得るか」の対応付けであり、特定 SHA での実測 Pass を意味
  しない。実測証拠は各 run の artifact/summary が正本。
- `ci.yml` は dispatch-only のため、PR ブランチ上の本変更に対して
  CI は自動実行されない。統合検証は merge 後 main SHA の Daily/
  手動 Full CI 結果が証拠となる。
