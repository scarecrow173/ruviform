# Issue #275 OBS-01〜11 受入照合台帳（公開 API/Feature/Owner・OS-L1 Fixture・未対応 Case）と Consumer Gate 割当

- 監査日: 2026-11-14
- 対象 Revision（監査基準）: `20d8e7c23bc365e26060a4ef82d463bb7eed38bc`
  （監査時点の origin/main HEAD = worktree HEAD、clean tree。
  merge 後 SHA の統合検証は Daily/手動 Full CI の対象 SHA 証拠が正本）
- 前提正本: PR #521（`1d12dfef`、統合設計正本）・PR #547
  （`8ff39973`、契約 Fixture・実装照合）・`issue-275-obs-owner-map`
  slice（`574a9dc0`/`ad586423`、Owner/API/Feature/CI lane 対応表）。
  実測根拠の補助 snapshot: `evidence/rf275/obs-owner-map-audit.md`。
- 方法: `gh issue view`（Issue #275/#281・AL #127・SV #28）、
  `gh pr view`/`gh run list`/`gh run view --job --log`
  （CI run・job・test 件数）、`git diff`（SHA 間 crate 同一性）、
  `crates/` 走査（module/feature/test target）、
  `mise run test <package>`（ローカル補助証拠。canonical = mise、
  ローカル実行は統合受入と区別）。
- 語彙（POLICIES §2/§3）: 要件状態 `Required`/`Deferred`/
  `NotApplicable`、実行結果 `Pass`/`Fail`/`NotRun`/`Unsupported`。
  job 定義・green・skipped・fake/headless 成功から個別 Case の
  Pass を推論しない。検証レベルは OS-L1 / GUI-L2 / EXT-L3。

## 0. 証拠一覧（本台帳が引用する実測）

| ID | 種別 | 対象 | 内容 |
| --- | --- | --- | --- |
| E-CI-1 | CI run | ruviform `365c74ac`（run [43439434737](https://github.com/scarecrow173/ruviform/actions/runs/43439434737)、2026-11-13T14:05Z） | `verify`（ubuntu-latest job 131932892523・windows-latest job 131932892775）・`verify-macos`（job 131932892563）・`rt-verify`（job 131932892476）・`msrv-verify`（job 131932892455）全て success。job log で observability の実行件数を確認（§2） |
| E-CI-2 | consumer-verify run | ruviform `3692c6fa`（run [43347745016](https://github.com/scarecrow173/ruviform/actions/runs/43347745016)、2026-11-13T01:49Z） | tuple = ruviform `3692c6fa` + audio-lens `d6bf7221` + synvelis `802c6bf6`。`verify-audiolens`/`verify-synvelis`/`summary` 全て success（job-level pass。lane 別 pass/not_run/unsupported は artifact が正本） |
| E-LOCAL-1 | ローカル実行 | 対象 Revision `20d8e7c2`・Windows 11 x64・rust 1.99.1（dev toolchain） | `mise run test ruviform-observability --all-features`（= `cargo test --locked -p ruviform-observability --all-features`）exit 0。lib 298・tests/integration.rs 15・tests/logging.rs 28・tests/rt_boundary.rs 3・doctest 15+1 ignored、全 Pass |
| E-LOCAL-2 | ローカル実行 | 同上 | `mise run test ruviform-plugin-host`（= `cargo test --locked -p ruviform-plugin-host`）— lib 226・tests 各 binary 計 254 件 Pass。初回 `l1_proxy_e2e` の 2 件失敗は `ruviform-plugin-l1-runner` bin 未 build の環境起因で、`cargo build -p ruviform-plugin-export --bins` 後の再実行で Pass（OBS-04 とは無関係の runner_bin fixture） |
| E-DIFF | git diff | `365c74ac..20d8e7c2` | `ruviform-observability`/`ruviform-rt`/`ruviform-media-core`/`ruviform-audio-core`/`ruviform-video-core` は diff ゼロ。`ruviform-plugin-host`/`ruviform-plugin-core` は diff あり（OBS-04 の plugin-host 側証拠は `365c74ac` 時点で stale） |
| E-NONE | — | `20d8e7c2` | `gh run list --commit 20d8e7c2` で run 0 件。daily-ci 最終 run は `3692c6fa`（2026-11-13）。対象 Revision の CI・consumer-verify 証拠は未発生（次回 daily-ci が対象 SHA を dispatch するまで `NotRun`） |
| E-LOCAL-3 | ローカル実行 | branch `feat/281-obs-feature-matrix`・Windows 11 x64・rust dev toolchain（§6 追記時点） | `mise run check-observability` / `lint-observability` / `test-observability` 全て exit 0。6 組合せ（default・`logging`・`tracing-export`・`allocation-probe`・`integration`・`--all-features`）の `cargo check/clippy --all-targets`/`cargo test` が Pass。`--all-features` では lib 298・integration.rs 15・logging.rs 28・rt_boundary.rs 3・doctest 15+1 ignored。crate の Rust コードは `20d8e7c2` から diff ゼロ（lane 定義のみの追加）。ローカル補助証拠であり CI 証拠ではない |

## 1. OBS-01〜11 受入照合表

各 OBS 1 行。Fixture の詳細・lane/OS 別結果は §2、
Consumer Gate 割当は §3。`状態` 列は要件状態（Required/
Deferred/NotApplicable）を示し、実行結果は `結果` 列に
対象 Revision 付きで記す。

| OBS | Owner Issue | Module / crate | Feature | 公開 API（代表） | OS-L1 Fixture（test target・必要 feature） | Case | 結果 @`20d8e7c2` | 未対応 Case |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| OBS-01 | [#276](https://github.com/scarecrow173/ruviform/issues/276) CLOSED（実装）。検証 #281 | `ruviform-observability::logging` | `logging` | `LoggingConfig`/`SinkConfig`/`SinkDestination`/`LogFormat`/`RotationPeriod`/`SinkFailureMode`/`build_logging`/`build_logging_layers`/`LoggingHealth`/`SinkHealthSnapshot`/`HealthSnapshot` | `tests/logging.rs` 28 test + `src/logging/*` unit 9 + example `logging_host`（compile）。全て `logging` feature 必須 | OBS-LOG（host subscriber・sink・rotation・health・failure mode）・G1/G7 | Required。Linux: `msrv-verify`/`test-observability-msrv`（rust@1.89.0 `--all-features`）で 28+9 Pass @`365c74ac`（E-CI-1、crate diff 空）。Windows: E-LOCAL-1 で 28+9 Pass @HEAD（ローカル補助証拠）。Win/macOS native の feature 付き CI lane: `NotRun`（lane なし）。HEAD CI: `NotRun`（E-NONE） | feature compile matrix 単独組合せ（#281 G0）`NotRun`。AL Logging parity/Privacy の独立 Oracle → AL #127 `NotRun`。実 sink/外部 Provider 故障の実機再現 → Deferred（owner-map Deferred 表） |
| OBS-02 | [#277](https://github.com/scarecrow173/ruviform/issues/277) CLOSED | `ruviform-observability::{core, source}` | なし（default） | `counter()`/`Counter`/`CounterReader`、`gauge()`/`Gauge`/`GaugeReader`、`histogram()`/`Histogram`/`HistogramReader`/`HistogramError`；`MetricDescriptor`/`SourceId`/`SourceGeneration`/`ObservationRegistrar`/`RegistrationLease`/`SourceLifecycle`/`ObservationScope`/`MetricSource`/`MetricReader` | `src/core/*` unit 15 + `src/source/*` unit 20（lib binary 内、`rt_*` no-alloc 含む） | OBS-RT（no-alloc）・OBS-SRC（lease/generation/lifecycle/drain）・G-2/G-4 | Required。Win/Linux/macOS: `verify`/`verify-macos` `test-full` で Pass @`365c74ac`（E-CI-1、crate diff 空）。Linux: `rt-verify`/`test-rt` で `rt_counter_ops_allocate_nothing` 他 Pass @`365c74ac`。Windows: E-LOCAL-1 で Pass @HEAD（ローカル）。HEAD CI: `NotRun`（E-NONE） | feature matrix `NotRun`（#281 G0）。実機負荷 RT → #281 実機 Gate `NotRun`/Deferred |
| OBS-03 | [#278](https://github.com/scarecrow173/ruviform/issues/278) CLOSED | `ruviform-observability::tracing_export` | `tracing-export` | `TracingExporter`/`TracingRecordExporter`/`TracingFindingSink`/`TraceContext`/`EmitStats`/`RecordEmitStats`/`DEFAULT_MAX_EMISSIONS_PER_POLL`/`DEFAULT_MAX_EVENTS_PER_DRAIN` | `src/tracing_export/tests.rs` 26 + `src/tracing_export/trace_context.rs` 8 + `tests/rt_boundary.rs` 3（`tracing-export` 必須・RT 非発行契約の共有 fixture） | G1 Tracing（Source Generation/Severity/Window/Clock/Loss/Correlation/Context）・G-2 tracing 非発行半面 | Required。Linux: `msrv-verify` で 34+3 Pass @`365c74ac`（E-CI-1）。Windows: E-LOCAL-1 で 34+3 Pass @HEAD（ローカル）。Win/macOS feature 付き CI: `NotRun`。HEAD CI: `NotRun` | W3C Context の実環境伝播・外部 backend 実接続 → Deferred。AL/SV 側 tracing 利用 Oracle → #281/#294 経由 `NotRun` |
| OBS-04 | [#279](https://github.com/scarecrow173/ruviform/issues/279) CLOSED | `ruviform-plugin-host`（`l1`/`composite` — observability 側 feature なし） | なし | `ObservationRegistrar` port 実装・`set_observation_registrar`・4 Counter（`dropped_submits`/`passthrough_submits`/`results_failed_observed`/`results_dropped`）の Source 登録 | plugin-host lib unit（`l1::sources::tests` 7・`composite::tests` registrar 系・`l1::tests::audio_counters_*`/`create_instance_without_registrar_*`）+ `tests/l1_proxy_e2e.rs` の `audio_counters_track_*` 3 件。default features | OBS-SRC（plugin host counter source 登録・lease・identity） | Required。Windows: E-LOCAL-2 で全 test Pass @HEAD（ローカル・crate diff ありのため `365c74ac` 証拠は stale）。Win/Linux/macOS CI @HEAD: `NotRun`（E-NONE）。`365c74ac` の verify 3 OS Pass は plugin-host/plugin-core diff により現行コードへ引き継がない | plugin-host の feature/OS 別 fault Case（crash/timeout 経路の counter 観測）は test-spec 範囲内だが consumer 文脈の故障注入 → #281 G4 `NotRun`。実 Plugin 環境 → EXT-L3 Deferred |
| OBS-05 | [#280](https://github.com/scarecrow173/ruviform/issues/280) CLOSED | `ruviform-observability::{record, rt_bridge}`（leaf deps: `ruviform-rt`・`ruviform-media-core` — ADR obs-05） | なし | `rt_bridge::bridge`/`RtRecorder`/`RecordSink`/`ReceivedRecord`；`ClockContext`/`CorrelationBook`/`CorrelationToken`/`CorrelationLink`/`CorrelationStatus`/`EventRecord`/`RecordHeader`/`EventId` | `src/record/*` unit 14 + `src/rt_bridge.rs` unit 11（`rt_*` no-alloc 含む） | OBS-RT（record/bridge no-alloc・clock・correlation・欠落非偽装）・G-2/G-4 | Required。Win/Linux/macOS: `test-full` Pass @`365c74ac`（E-CI-1、crate+leaf deps diff 空）。Linux `rt-verify`: `rt_try_record_and_counters_allocate_nothing`・`rt_record_ops_allocate_nothing`・`rt_clock_context_ops_allocate_nothing` Pass @`365c74ac`。Windows: E-LOCAL-1 Pass @HEAD（ローカル）。HEAD CI: `NotRun` | 実機負荷下 latency/欠落計測 → #281 実機 Gate `NotRun`（EXT-L3 Deferred）。SV 側 Record/Plugin/Project Oracle → SV #28 `NotRun` |
| OBS-06 | [#281](https://github.com/scarecrow173/ruviform/issues/281) OPEN（gate Owner）+ [#219](https://github.com/scarecrow173/ruviform/issues/219)・[#294](https://github.com/scarecrow173/ruviform/issues/294)（cross-repo 機構） | なし（gate — module を持たない） | — | —（gate 証拠の収集・判定を所有。Case 組: OBS-RT/OBS-LOG/OBS-SRC/OBS-PRIV/OBS-PROF/OBS-AL/OBS-SV、G0〜G4） | gate 自体に test fixture なし。証拠源 lane: `consumer-verify.yml`（`verify-audiolens`/`verify-synvelis`/`summary`）・`rt-verify`・`msrv-verify`・Daily/手動 Full CI | 全 Case 組（§3 の割当表が正本） | Required（継続中）。対象 Revision での gate 実施: `NotRun`（E-NONE・E-CI-2 は `3692c6fa` の stale tuple）。個別 Case の Pass は本台帳から推論しない | 全 Case 組が未実施のまま — Owner/Trigger は §3・owner-map Deferred 表どおり |
| OBS-07 | [#289](https://github.com/scarecrow173/ruviform/issues/289) CLOSED | `ruviform-observability::{collector, metrics}` | なし | `Collector`/`CollectorRegistrar`/`CollectorHandle`/`PollRunner`/`ObservationBatch`/`MetricsSnapshot`/`CollectorHealth`/`CollectorState`/`CollectorConfig`/`PollError`/`ObservationWindow`；`Measurement`/`MeasurementValidity`/`MeasurementIssue`/`MetricValue` | `src/collector/tests.rs` 51（PR #547 追加の `abandoned_leases_racing_shutdown_still_drain_partial` 含む）+ `src/metrics` unit 0 | OBS-SRC（quiescence→final drain→unregister 競合安全・loss/stale/clock/epoch 非偽装）・G-3/G-4 | Required。Win/Linux/macOS: `test-full` Pass @`365c74ac`（E-CI-1、crate diff 空）。Windows: E-LOCAL-1 Pass @HEAD（ローカル）。HEAD CI: `NotRun` | feature matrix `NotRun`。外部 Provider/Exporter 故障の実環境再現 → Deferred（G7 の一部は in-repo 契約で被覆済み） |
| OBS-08 | [#290](https://github.com/scarecrow173/ruviform/issues/290) CLOSED | `ruviform-observability::rt_timing`（default）+ `ruviform-observability::profiling`（`allocation-probe`） | なし / `allocation-probe` | rt_timing: `timing_bridge`/`TimingRecorder`/`TimingSink`/`TimingWindow`/`TimingSnapshot`/`TimingClock`/`TimingRecord`/`TimingSample`/`QuantileEstimate`/`TIMING_SCHEMA_VERSION`；profiling: `CountingAllocator`/`AllocationScope`/`AllocationWindow`/`AllocationStats`/`OpStats` | rt_timing: `src/rt_timing/tests.rs` 15 + inline（lib 内 `rt_*` 含む）。profiling: `src/profiling/*` unit 34。profiling は `allocation-probe` 必須 | OBS-PROF（RT timing・allocator scope/stats・dev-only）・G-2/G-3 | Required。rt_timing: Win/Linux/macOS `test-full` + Linux `rt-verify` Pass @`365c74ac`（E-CI-1）。profiling: Linux `msrv-verify` `--all-features` で 34 Pass @`365c74ac`。Windows: E-LOCAL-1 で rt_timing+profiling Pass @HEAD（ローカル・`--all-features`）。Win/macOS の `allocation-probe` feature CI lane: `NotRun`。HEAD CI: `NotRun` | 長時間 profiler（samply 等）→ Deferred（owner-map 表）。TLS scope の他スレッド非観測・実負荷計測 → #281 `NotRun` |
| OBS-09 | [#291](https://github.com/scarecrow173/ruviform/issues/291) CLOSED | `ruviform-observability::diagnostics` | なし | `Diagnostics`/`DiagnosticsHealth`/`Evidence*`/`DiagnosticFinding`/`FindingSeverity`/`FindingCertainty`/`FindingStatus`/`HealthState`/`HealthEvaluator`/`HealthRule`/`Hysteresis`/`DiagnosticSnapshot`/`DiagnosticReport`/`FindingSink`/`SinkPolicy`/`DiagnosticsEvent` | `src/diagnostics/tests.rs` 32（lib binary 内） | OBS-PRIV（secret/PII redaction・Evidence 区分）・G-3/G-4/G-7 | Required。Win/Linux/macOS: `test-full` Pass @`365c74ac`（E-CI-1、crate diff 空）。Windows: E-LOCAL-1 Pass @HEAD（ローカル）。HEAD CI: `NotRun` | Report Export の実認可経路（Host 側）→ #293/#292 境界・consumer Oracle `NotRun`。実 sink panic の実環境再現 → G7 `NotRun` |
| OBS-10 | [#292](https://github.com/scarecrow173/ruviform/issues/292) CLOSED | `ruviform-observability::debug` | なし | `DebugInspector`/`DebugConfig`/`DebugHealth`/`InspectionBudget`/`InspectionRequest`/`InspectionSnapshot`/`InvariantCheck`/`InvariantOutcome`/`InvariantVerdict`/`ProbeSource`/`DebugSessionState`/`DebugReport`/`ExportLimits`/`DEBUG_REPORT_SCHEMA_VERSION` | `src/debug/tests.rs` 45 + `rt_debug_value_types_allocate_nothing`（rt lane） | G-3（Inspection/Invariant/Session/Probe・budget・権限） | Required。Win/Linux/macOS: `test-full` Pass @`365c74ac`（E-CI-1）。Linux `rt-verify`: rt 関連 test Pass @`365c74ac`。Windows: E-LOCAL-1 Pass @HEAD（ローカル）。HEAD CI: `NotRun` | 実 OS GUI 上の debug 表示・実機 Probe → GUI-L2/EXT-L3 Deferred（owner-map 表: RF #293 + AL #127/SV #28 Windows L2 + RF #313/#314 lane） |
| OBS-11 | [#293](https://github.com/scarecrow173/ruviform/issues/293) CLOSED | `ruviform-observability::integration` | `integration` | `ObservabilityConfig`/`ObservationRunner`/`RunnerState`/`RunnerHealth`/`PollOutcome`/`DrainCompleteness`/`ShutdownReport`/`ReconfigureReport`/`ConfigError`/`ExportPolicy`/`DebugSettings`/`NonMigrated`/`RunnerError`/`ExportTotals` | `src/integration/tests.rs` 12 + `tests/integration.rs` 15（`--all-features`）/ 9（default+`integration` unify — `test-full` 実効値）。`integration` feature 必須 | G-3/G-7（lifecycle・drain・dynamic config・health・error isolation）・GUI 購読境界（API のみ） | Required。Win/Linux/macOS: `test-full`（feature unify で `integration` 有効）Pass @`365c74ac`（E-CI-1）。Linux `msrv-verify`: 15 test Pass @`365c74ac`。Windows: E-LOCAL-1 で 12+15 Pass @HEAD（ローカル・`--all-features`）。HEAD CI: `NotRun` | GUI 購読の実画面検証 → GUI-L2 Deferred。real Provider/Exporter 接続 → Deferred。feature 組合せ matrix → #281 G0 `NotRun` |

## 2. Fixture × lane 証拠行列

凡例: `✓` = 当該 lane/OS で実行され Pass を観測、`—` = compile/
実行されない、`NR` = lane は存在するが対象 Revision で未実走、
`local` = ローカル補助実行（統合受入ではない）。

| Fixture（test target / test 群） | 必要 feature | `verify` ubuntu | `verify` windows | `verify-macos` | `rt-verify` ubuntu | `msrv-verify` ubuntu (rust@1.89 `--all-features`) | ローカル Windows @HEAD |
| --- | --- | --- | --- | --- | --- | --- | --- |
| lib unittests default+integration（core/source/collector/diagnostics/debug/record/rt_bridge/rt_timing/metrics = 221 test @365c74ac Win 観測） | default+`integration`（unify） | ✓ @365c74ac | ✓ @365c74ac | ✓ @365c74ac | ✓ @365c74ac（`rt_` substring 選択で 57 test） | ✓ @365c74ac（298 test — 全 module） | ✓ @HEAD（298 test・`--all-features`、E-LOCAL-1） |
| `tests/integration.rs` | `integration` | ✓ 9 test @365c74ac | ✓ 9 test @365c74ac | ✓ @365c74ac | —（`rt_` 非該当） | ✓ 15 test @365c74ac | ✓ 15 test @HEAD（E-LOCAL-1） |
| `tests/logging.rs` 28 | `logging` | — | — | — | — | ✓ @365c74ac | ✓ @HEAD（E-LOCAL-1） |
| `src/logging/*` unit 9 | `logging` | — | — | — | — | ✓ @365c74ac（lib 298 に含む） | ✓ @HEAD（E-LOCAL-1） |
| `tests/rt_boundary.rs` 3 | `tracing-export` | — | — | — | — | ✓ @365c74ac | ✓ @HEAD（E-LOCAL-1） |
| `src/tracing_export/*` unit 34 | `tracing-export` | — | — | — | — | ✓ @365c74ac | ✓ @HEAD（E-LOCAL-1） |
| `src/profiling/*` unit 34 | `allocation-probe` | — | — | — | — | ✓ @365c74ac | ✓ @HEAD（E-LOCAL-1） |
| `examples/logging_host.rs` | `logging` | — | — | — | — | ✓ compile @365c74ac | ✓ compile @HEAD（E-LOCAL-1 の `--all-features` で build） |
| plugin-host OBS-04 fixtures（lib unit + `tests/l1_proxy_e2e.rs` audio_counters 系） | なし | NR @HEAD（`365c74ac` は Pass だが crate diff ありで stale） | NR @HEAD（同左） | NR @HEAD（同左） | —（`rt_` 非該当） | —（対象外 package） | ✓ @HEAD（E-LOCAL-2、全 test Pass） |
| doctest 15+1 ignored | default+feature 組合せ | ✓ @365c74ac | ✓ @365c74ac | ✓ @365c74ac | — | ✓ @365c74ac | ✓ @HEAD（E-LOCAL-1） |

- `test-full` の observability lib 件数は feature unify により
  default+`integration`（Win 観測 221 test）。`--all-features`
  では feature 付き module の test が加わり 298 test。
- `rt_boundary.rs` の `rt_*` 名 test は `tracing-export`
  必須のため `test-rt`（feature 無し）では compile されず、
  `msrv-verify`/`--all-features` 系でのみ実行される。
- `ci.yml` は dispatch-only。E-CI-1 は daily-ci または手動
  dispatch による `365c74ac` 実行であり、`20d8e7c2` の run は
  未発生（E-NONE）。上表 `✓ @365c74ac` は「同一コードへの
  証拠」（E-DIFF で crate diff 空を確認した範囲）であり、
  対象 Revision での CI 実施を意味しない。
- `365c74ac..20d8e7c2` の `ci.yml` 差分は lint 系 step の
  Linux 限定化と Windows/macOS への `check-full`/
  `build-full` 追加であり、`test-full`（verify/
  verify-macos 全 OS）・`rt-verify`・`msrv-verify` の
  observability fixture 実行経路は不変。`mise.toml` は
  task 追加のみで既存 task 定義は不変。`Cargo.toml`/
  `Cargo.lock` も diff ゼロ。

## 3. Consumer Gate 割当表

Issue #275・#281 現行正本（2026-11-13 Stage 2/3・Windows
L2/L3 条件付き必須）に基づく割当。`状態 @20d8e7c2` は
監査時点の実行結果。

| Gate | Case 組（#281） | RF 側 Owner | Consumer Owner | 実行経路 | 状態 @`20d8e7c2` |
| --- | --- | --- | --- | --- | --- |
| AL Logging/Privacy/f64 独立 Consumer Gate | OBS-LOG（Logging parity: 既定 OFF/`AL_LOG`/stderr・json/inspect stdout/daily 7 世代/失敗 degrade）・OBS-PRIV（機密漏出）・OBS-AL（f64/NaN/gap/PTS/長尺/Dataset/CLI/Export 精度） | #281（gate 判定・PASS 証跡書式）・#294（candidate tuple 集約・fail-closed summary）・#219（Cross-Repo 運用契約） | AL #127（継続追従 Owner。AL #138 は CLOSED 履歴で再 Open しない） | consumer-verify `verify-audiolens` lanes: `check-ruviform-lock`/`check-full`/`smoke`/`check-cli-headless`/`test-full`/`sync-ui-deps`/`setup-ui`/`check-ui`/`test-ui`/`extra tasks`/GUI-CONTRACT/GUI-SEMANTIC + AL repo 側独立 Oracle（f64/CLI/Job/Dataset/GUI semantic） | `NotRun`（最終 tuple 証拠 E-CI-2 @`3692c6fa` は job-level pass だが stale。AL #127 側の OBS 固有 Oracle 実施記録は consumer repo が正本） |
| SV RT/DAW/GUI 独立 Consumer Gate | OBS-RT（callback noalloc/no block/no unexpected drop）・OBS-SV（Record/Plugin/Project/Undo/Save/RT）・GUI（DAW 画面・利用機能の L2） | 同上（#281/#294/#219） | SV #28（継続追従 Owner。SV #81 は CLOSED 履歴で再 Open しない） | consumer-verify `verify-synvelis` lanes: `check-ruviform-rev`/`check-full`/`test-full`/`test-rt`/`check-headless-deps`/`sync-ui-deps`/`setup-ui`/`check-ui`/`test-ui`/`build-ui`/GUI-CONTRACT/GUI-SEMANTIC/extra + SV repo 側独立 Oracle（Project/PCM/RT/Gateway/Plugin） | `NotRun`（E-CI-2 @`3692c6fa` stale。SV #28 側の OBS 固有 Oracle 実施記録は consumer repo が正本） |
| Windows 利用機能 L2/L3 故障・RT/Privacy Gate | OBS-LOG/OBS-PRIV/OBS-RT のうち Windows で**実際に提供・利用する**機能に限定（Logging/Tracing/Privacy/RT 非干渉・故障後継続） | #281/#294（集約・判定）・#293（Host 統合/API 境界）・#313/#314 系 real-window/real-scenario lane 設計 | AL #127・SV #28（各製品機能の Owner Issue） | 各 Consumer repo の Windows L2/L3 Case（実 Window/実 Plugin/実 Device 結合時）+ RF real-window lanes。headless RT/Privacy は必須だが実 Host 受入の代用にしない | `NotRun`/`Deferred`（利用機能が確定するまで Required にしない。POLICIES: Windows 利用機能のみ Required、macOS/Linux L2/L3 は Deferred） |
| Feature compile matrix（`--no-default-features`・各 feature 単独・full consumer 組合せ） | G-1/G-0 | #281 G0 | — | `mise run check/lint/test-observability`（6 組合せ）+ CI `observability-verify`/`observability-verify-macos`（branch `feat/281-obs-feature-matrix` で追加、§6） | `NotRun` @`20d8e7c2`（監査時点では lane なし — §6 で lane 追加を追記。target-SHA CI 実走は依然 `NotRun`。Required のまま維持。Deferred ではない） |
| Cross-repo tuple 集約 | 全 Case 組の candidate SHA 照合 | #294（tuple/Result/Owner routing）・#219（runbook） | 両 Consumer repo | consumer-verify `summary`（fail-closed: not_run/unsupported を pass にしない）+ Daily/手動 dispatch | `NotRun` @HEAD（E-CI-2 @`3692c6fa` が直近証拠） |

補足:

- AL #138・SV #81 は CLOSED の導入履歴 Issue であり継続受入
  Owner ではない。本台帳・後続 gate で再 Open しない。
- 一方の Consumer の Pass を他方へ転用しない。AL/SV それぞれの
  repo 側 task・fixture・Oracle が受入の正本。
- consumer-verify の job-level `success`/`pass` は個別 OBS
  Case の Pass を意味しない。lane 記録（pass/not_run/
  unsupported 区別）と `consumer-evidence-*.json` artifact が
  正本（POLICIES §3）。

## 4. SHA 証拠引き継ぎ規則と NotRun 棚卸し

### 4.1 引き継ぎ規則（本台帳で適用した判定）

- `git diff 365c74ac..20d8e7c2 -- <paths>` が空の crate に
  ついてのみ、E-CI-1（`365c74ac` の Full CI 成功）の証拠を
  「同一コードへの証拠」として引用する。
- diff 結果（E-DIFF）: `ruviform-observability`・
  `ruviform-rt`・`ruviform-media-core`・`ruviform-audio-core`・
  `ruviform-video-core` = diff ゼロ → OBS-01/02/03/05/07/08
  (rt_timing)/09/10/11 の in-repo fixture 証拠は `365c74ac`
  から引き継ぐ。`ruviform-plugin-host`・`ruviform-plugin-core`
  = diff あり → OBS-04 の `365c74ac` 証拠は stale。
- 対象 Revision `20d8e7c2` 自体の CI/consumer-verify run は
  未発生（E-NONE）のため、HEAD での統合検証は全て `NotRun`
  （Pending ではなく監査時点の事実）。merge 後 SHA の
  Daily/手動 Full CI が統合受入の証拠となる。

### 4.2 未対応 Case 棚卸し（既存 lane/fixture がない、または対象 Revision 未実走）

| 未対応 Case | 要件状態 | Owner | 現行の扱い |
| --- | --- | --- | --- |
| 対象 Revision での CI 実走（verify/rt-verify/msrv-verify/test-full 全 lane） | Required | Daily/手動 Full CI（#294 dispatch 経路） | `NotRun` — E-NONE。次回 daily-ci が `20d8e7c2` 以降の main SHA を対象に dispatch |
| 対象 Revision での consumer-verify tuple 実走 | Required | #294/#219 | `NotRun` — 直近 E-CI-2 @`3692c6fa` は stale |
| OBS-04 plugin-host の HEAD CI 証拠 | Required | Daily/手動 Full CI | `NotRun` — crate diff ありで `365c74ac` 証拠は stale。ローカル E-LOCAL-2 のみ |
| Feature compile matrix（`--no-default-features`・各 feature 単独・full consumer 組合せ） | Required | #281 G0 | `NotRun` — 監査時点では既存 lane なし。§6 で `check/lint/test-observability` + `observability-verify`/`observability-verify-macos` の追加を追記（lane 定義済み・target-SHA CI 未実走） |
| Win/macOS native の feature 付き build/link/test（`logging`/`tracing-export`/`allocation-probe`） | Required（OS-L1: feature 有効時） | #281 G0 + CI lane 補完 | `NotRun` — 監査時点では CI lane なし。§6 で 3 OS lane 追加を追記。Windows ローカル matrix 実行（E-LOCAL-3）は補助証拠であって CI 証拠ではない |
| AL Logging/Privacy/f64 独立 Oracle | Required | AL #127（#281/#294 経由） | `NotRun` — consumer repo 側実施記録が正本 |
| SV RT/DAW/GUI 独立 Oracle | Required | SV #28（#281/#294 経由） | `NotRun` — 同上 |
| Windows 利用機能 L2/L3（故障・RT/Privacy） | Deferred→利用決定時 Required | AL #127・SV #28・RF #293/#313/#314 | `NotRun`/`Deferred` — 利用機能未確定 |
| 実 OS GUI（Observability 購読/表示） | Deferred | RF #293 + AL #127/SV #28 Windows L2 | `NotRun`（GUI-L2、利用要件未発生） |
| 実 Device・実機負荷 RT | Deferred | #281 + device 利用機能 Owner | `NotRun`（EXT-L3、利用要件未発生） |
| 実外部 Provider/Exporter | Deferred | 新規 Issue（利用決定時）+ #281 | `NotRun`/`Unsupported` ではなく Deferred — in-repo 契約のみ検証済み |
| 長時間 Profiler | Deferred | #281 + 新規 Issue（必要時） | `NotRun` — 有界計測のみ |

## 5. 判定メモ

- 本台帳の `✓ @365c74ac` は E-DIFF で crate diff 空を確認した
  範囲の「同一コードへの CI 証拠」であり、対象 Revision
  `20d8e7c2` での実行済みを意味しない。対象 Revision の
  統合検証は merge 後 main SHA の Daily/手動 Full CI・
  consumer-verify の結果が証拠となる。
- ローカル実行（E-LOCAL-1/2）は Windows・dev toolchain での
  補助証拠であり、CI lane の Pass・統合受入の Pass を
  意味しない。`--all-features` 単独実行は feature compile
  matrix（分離組合せ）の代替にもならない。
- `NotRun`/`Fail`/`Unsupported`/`Deferred` を `Pass` に
  置き換えない。Deferred 項目の Owner/Trigger は
  `docs/designs/issue-275-observability-integration-design.md`
  の「Deferred 範囲の Owner/Trigger」節が正本。
- per-module の詳細契約・test 規則は per-issue 正本
  （`docs/designs/issue-2NN-*.md`・`docs/test-specs/
  issue-2NN-*.md`）が所有し、本台帳は索引・照合・状態
  記録のみを所有する。

## 6. 追記: G0 feature matrix lane 追加（Issue #281 slice、2026-11-14）

branch `feat/281-obs-feature-matrix`（PR 未 merge）で G0 の
feature compile matrix lane を新設した。本節は §1〜§5 の
`20d8e7c2` 監査記録を改変せず、lane の現況のみを追記する。

- 追加物: `mise.toml` の `check-observability`/
  `lint-observability`/`test-observability`（default・
  `logging`・`tracing-export`・`allocation-probe`・
  `integration`・`--all-features` の 6 組合せを `&&` 連結）、
  `.github/workflows/ci.yml` の `observability-verify`
  （ubuntu-latest/windows-latest matrix・`fail-fast: false`）と
  `observability-verify-macos`（macos-latest 別 job）。
  両 job は checkout SHA 証跡・`ci-summary` `needs` 登録を
  既存 lane と同一規約で実装し、`ci/summary/summary.mjs` の
  `REQUIRED_JOBS`（15→17）/`REQUIRED_CHECKOUT_LEGS`（19→22）を
  同期更新した。`docs/development-environment.md` の lane
  inventory も更新済み。正本文書:
  `docs/specs|designs|plans|tasks|test-specs|
  impl/ir/issue-281-obs-feature-matrix.md`。
- ローカル実行（E-LOCAL-3）: Windows 11 x64・dev toolchain で
  3 タスク全て exit 0。6 組合せの check/lint/test が Pass。
  これは lane 定義のローカル補助証拠であり、CI lane の Pass・
  統合受入の Pass を意味しない。
- CI 実走: `NotRun`。ci.yml は `workflow_dispatch` 専用のため
  push/PR では起動しない。対象 SHA（merge 後 main）での
  `observability-verify`/`observability-verify-macos` の実走と
  `ci-summary` verdict が G0 の CI 証拠となる。それまでは
  §3「Feature compile matrix」行・§4.2 の 2 行は `NotRun`
  のまま維持する（lane 定義の存在は `Pass` の根拠にしない）。
- 範囲外（本 slice で扱わない）: pairwise 2 feature 組合せ、
  `--no-default-features` 明示（`default = []` で default と
  等価のため省略）、AL #127/SV #28 の独立 Oracle、#294 tuple
  への結果投入、Windows 利用機能 L2/L3、実機・実 Provider。
  これらは §3 の各 Gate 行・§4.2 の棚卸しどおり `NotRun`/
  `Deferred` のまま。
