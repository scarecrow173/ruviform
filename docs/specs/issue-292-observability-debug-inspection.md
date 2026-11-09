---
id: "SPEC-tBdATMZtqYxn8MbVgZquYA"
type: "spec"
status: "approved"
title: "Issue #292 OBS-10 Debug Inspection・Invariant・Session・Probe連携（observability 内完結 slice）"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/292"
    - "https://github.com/scarecrow173/ruviform/issues/275"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "https://github.com/scarecrow173/ruviform/issues/292"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-277-observability-core-source-contract.md"
    - "docs/specs/issue-280-rt-record-bridge-clock-correlation.md"
    - "docs/specs/issue-289-observability-collector-metrics.md"
    - "docs/specs/issue-291-observability-diagnostics.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/foundation-observability-minimal-api.md"
    - "docs/designs/shared-foundations-and-modularization-policy.md"
    - "docs/designs/issue-291-observability-diagnostics.md"
    - "docs/designs/issue-309-probe-time-context-and-alignment.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
    - "docs/adr/obs-05-observability-leaf-primitive-deps.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-277-observability-core-source-contract.md"
    - "docs/specs/issue-280-rt-record-bridge-clock-correlation.md"
    - "docs/specs/issue-289-observability-collector-metrics.md"
    - "docs/specs/issue-291-observability-diagnostics.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-292-observability-debug-inspection-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/277"
    - "https://github.com/scarecrow173/ruviform/issues/280"
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/289"
    - "https://github.com/scarecrow173/ruviform/issues/293"
    - "https://github.com/scarecrow173/ruviform/issues/185"
    - "https://github.com/scarecrow173/ruviform/issues/308"
    - "https://github.com/scarecrow173/ruviform/issues/309"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/278"
    - "https://github.com/scarecrow173/ruviform/issues/293"
    - "https://github.com/scarecrow173/ruviform/issues/281"
  deferred-by: []
---

# Issue #292 OBS-10 Debug Inspection・Invariant・Session・Probe連携（observability 内完結 slice）

## What Should Be Built

既存 `ruviform-observability` クレート内に `src/debug/` モジュールを
新設し、Issue #292（OBS-10）のうち **本クレート内で完結する debug
inspection 基盤 slice** を提供する。独立 Debug クレート・Debugger
Engine・二重 Probe Engine は作らない。具体的には:

- `InspectionBudget { max_items, max_bytes, timeout }` と
  `InspectionRequest { targets, detail, disclosure }` を受ける非 RT
  `DebugInspector::inspect(request, inputs, now) -> InspectionSnapshot`。
  対象・詳細度・時間/容量/個数予算・整合性・切り詰めを型で明示し、
  任意の内部状態の `Debug` 実装を無制限に Serialize しない。
- 型付き `InspectionSection`（Metrics / Diagnostics / Invariants /
  Session / Probes）と `SectionStatus`（Ok / Truncated /
  Unavailable / Withheld）。要求された対象の入力が無い・期限超過・
  開示区分超過は、項目を黙って落とさず typed status で報告する。
- `InspectionConsistency`: collector 系入力（`MetricsSnapshot` /
  `ObservationBatch` / `DiagnosticReport.source_generation`）の世代
  整合と、probe deck 単位の generation/clock 整合を評価し、
  `Aligned` / `Skewed` / `Unknown` を最も保守的な側で報告する。
  単一時点 transactional snapshot は主張しない。
- `InvariantOutcome`（`Satisfied` / `Violated` / `Inconclusive`）と
  `InconclusiveReason`（観測欠落・世代不一致・未同期読取・check
  自体の評価不能等）。`InvariantCheck` port（`Box<dyn>` 注入）を通じて
  ドメイン所有の不変条件検査を登録し、評価結果を OBS-09
  `Diagnostics` へ Evidence/Finding として接続する
  （`publish_invariants`）。Inconclusive を原因確定と偽装しない。
- `ProbeSource` port trait: #185 Generic Probe の
  Configured/Observed/Unavailable/Gap/Loss・ProbeMode
  （Off/Monitor/Capture）・plan generation・clock mark
  （`ClockDomainId`+`ClockEpochId`）を observability 側の語彙で
  参照する `Box<dyn>` 注入点。concrete adapter は依存循環のため
  本 crate に置けず、#293 / `ruviform-audio` 側の責務として設計文書に
  明記する。
- `DebugSession` 状態機械 `Disabled → Prepared → Active → Draining →
  Stopped`。`DebugAuthority`/`SessionPermit` による capability モデルで
  権限ある Host のみが開始/停止し、RT 対象の Buffer/Writer は非 RT で
  事前準備する。`SessionDrain` port + `EventRecordDrain`
  （`rt_bridge::RecordSink<EventRecord>` への concrete adapter）で
  RT buffer の最終回収（`drain_final` 相当）を行い、writer quiescence
  未確認のまま Buffer/Arc を解放しない。停止競合・世代切替は typed
  error で明示し、放棄した要求を成功と報告しない。
- `DebugReport`: `InspectionSnapshot` と session/drain 会計から生成
  する非 RT の明示的 export 物。`Confidentiality` 開示区分と
  external export 認可（`allow_external_export`）で未認可要求を
  `Unauthorized` 拒否し、Secrets/Media/Path/Credential を運ぶ型は
  存在しない（`EventRecord` の固定長語彙・typed 値のみ）。
- `DebugHealth`: 固定サイズの自己観測カウンタ（source 非登録・非再帰、
  OBS-09 `DiagnosticsHealth` と同じパターン）。

Logging/Tracing OFF（`logging` feature 無効・外部依存ゼロ）でも
Inspection/Invariant/Session/Report が利用可能であること。新規
クレート・新規外部依存・`unsafe` は追加しない。

## Why Now

- OBS-02（#277）source 契約、OBS-05（#280）`record`/`rt_bridge`、
  OBS-07（#289）Collector、OBS-09（#291）Diagnostics が main に
  実装済みであり、debug 層が消費する全入力が揃った。#185 Probe・
  #308 Generation-safe Rebind・#309 Probe time alignment も実装済みで、
  port 語彙の写像元が確定している。
- OBS-11（#293）Host 統合・両 Consumer（#281/#219）が debug の
  Session/Inspection/Report を消費する前に、observability 内の契約を
  固定する必要がある。
- `ruviform-observability → ruviform-audio` の依存は循環になるため、
  probe 参照は port trait 注入に限定する境界を今確定しないと、
  concrete adapter の置き場所で設計が揺れる。

## Users and Value

- **OBS-11（#293）Host 統合実装者**: `DebugInspector`・
  `InspectionSnapshot`・`DebugSession`・`DebugReport` の確定 API と、
  `ProbeSource`/`InvariantCheck`/`SessionDrain` の port 契約を得る。
- **Domain Owner（graph/audio/plugin host 等）**: `InvariantCheck`・
  `ProbeSource` adapter を実装するだけで、予算・切り詰め・世代整合・
  Diagnostics 接続・Session 状態機械を共通実装に委ねられる。
  ドメインの `Error` 型・既存 probe 実装（#185 `ProbeBroker`/
  `ProbeDeck`）は置き換えない。
- **Consumer（AudioLens/Synvelis）**: Host 経由で型付き Inspection/
  Report を受け取り、secret/media 非含有・開示境界を契約として
  得る（受入自体は #281 gate）。

## Scope

### In Scope

- `src/debug/` モジュール群（`mod.rs` / `budget.rs` / `request.rs` /
  `snapshot.rs` / `invariant.rs` / `probe.rs` / `session.rs` /
  `report.rs` / `engine.rs` / `tests.rs` を想定）と `lib.rs` への
  module 登録・crate doc 更新。
- `InspectionBudget`/`InspectionDetail`/`InspectionTarget`/
  `InspectionRequest`/`InspectionInputs`: 非 RT inspect の予算・
  対象・詳細度・入力束。
- `InspectionSnapshot`/`InspectionSection`/`SectionStatus`/
  `InspectionTruncation`/`InspectionConsistency`: 有界・typed・
  非 transactional な inspection 結果。
- `InvariantOutcome`/`InconclusiveReason`/`InvariantVerdict`/
  `InvariantCheck`/`InvariantResult`: Satisfied/Violated/Inconclusive
  の型付き判定と `Box<dyn>` port、`publish_invariants` による
  OBS-09 `Evidence`/`DiagnosticFinding` への変換。
- `ProbeMode`/`ProbeAvailability`/`ProbeStatusKind`/`ProbeClockMark`/
  `ProbeReading`/`ProbeDeckSnapshot`/`ProbeSource`: #185 語彙を
  observability 内型で写した port 注入点（concrete adapter は除く）。
- `DebugSessionState`/`DebugAuthority`/`SessionPermit`/
  `SessionRequest`/`SessionError`/`SessionDrain`/`SessionRecord`/
  `DrainReport`/`EventRecordDrain`: Session 状態機械・権限・
  Draining・RT buffer 最終回収。
- `DebugReport`/`DebugReportRequest`/`ExportLimits`/`ExportError`:
  開示区分 + external export 認可 + 有界 truncation を持つ
  非 RT export 物。
- `DebugConfig`/`DebugInspector`/`DebugHealth`: 集約 engine と
  固定カウンタの自己観測。
- 契約試験: budget/Truncated/Unavailable/Discontinuous 相当
  （`MeasurementValidity::Discontinuous` の保持）/Generation 整合・
  Invariant 3 値・Session 全遷移・競合・最終回収・Report 認可拒否・
  非流出・Logging OFF。

### Out of Scope

- concrete probe adapter（`ruviform-audio::typed::probe` →
  `ProbeSource` の写像実装）— 依存循環のため #293 /
  `ruviform-audio` 側の責務。本 slice は port trait と語彙のみ。
- #185 Probe の OFF/Monitor/Capture 本体・`ProbeBroker`/`ProbeDeck`/
  `ProbeCollector` の再実装（重複実装禁止。port 経由で参照する
  のみ）。
- `debug-tools`/`allocation-probe` Cargo feature の追加 — #275 の
  feature 方針は「最終名称・依存は実装時にコンパイル検証」であり、
  OBS-07/09 と同様に無条件 module として着地させる先例に倣う
  （canonical `cargo test --workspace` が default feature で全
  module を検証できることを優先）。feature 分離は #293 統合時の
  検討事項として deferred に記録する。
- Host 認可機構そのもの（OBS-11 #293）— 本 Issue は
  `DebugAuthority`/`SessionPermit`/`allow_external_export` の
  capability パラメータとしてのモデル化まで。
- OBS-08（#290）CountingAllocator・profiling 計測結果の取り込み —
  port 注入で将来接続可能な形に留め、本 slice では allocator
  adapter を実装しない。
- Diagnostics への finding resolve 公開 API 追加 — `diagnostics/`
  は編集禁止範囲。debug 発 findings の Open→Resolved 完了は
  現行公開 API では表現できない既知制約として deferred 記録する。
- RT callback 上の alloc/lock/OS IO/Subscriber 非呼出し・ON/OFF
  DSP 結果同値の実機検証 — debug module は RT 呼出可能 API を
  持たない構造で保証し、DSP 同値は domain/consumer 側（#293/#281）
  の責務として NotRun 記録する。
- 両 Consumer（AudioLens/Synvelis）での互換性確認（#281 gate）、
  Synvelis 側 Debug 能力連携（synvelis#81）、Host 統合（#293）。
- 新規クレート・新規外部依存・`unsafe`・既存
  collector/source/metrics/record/rt_bridge/diagnostics/logging の
  公開 API 変更。

## Requirements

- `inspect` は非 RT only（alloc/lock を伴う）。RT writer 経路・
  既存 primitive の API・性能・ビルドを回帰させない。debug module
  の公開 API に RT-suitable と宣言する操作は存在しない
  （`SessionRecord`/`DrainedRecord`/`ProbeReading` 等の値型は
  `Copy` で RT 隣接データとして no-alloc を試験する）。
- `InspectionBudget` の `max_items`/`max_bytes`/`timeout` を
  全 section に適用し、超過は `InspectionTruncation`（items
  dropped・bytes estimated・deadline_exceeded・withheld）に
  構造化記録する。0 は項目ごとに文書化した「無制限/無効」意味を
  持つ（diagnostics `ReportLimits` と同じ規約）。
- 要求 target の入力欠落・deadline 超過・開示区分超過は
  `SectionStatus::Unavailable`/`Withheld`/`Truncated` で typed
  報告し、黙って section を落とさない。
- `InspectionSnapshot` は `consistency` で generation 整合を
  報告する: collector 系は `MetricsSnapshot.generation()` /
  `ObservationBatch.generation()` / `DiagnosticReport.
  source_generation` の一致を `Aligned`/`Skewed` とし、判定不能は
  `Unknown`。probe deck は deck generation と各 reading の bound
  generation の不一致・clock mark `UNKNOWN` を検出して同じく
  報告する。複数世代に跨る入力を単一時点 snapshot と主張しない。
- `InvariantOutcome` は `Satisfied`/`Violated`/`Inconclusive` を
  区別し、観測欠落・世代不一致（probe index/source identity
  不一致を含む）・未同期読取・check 評価不能を `Inconclusive`
  として原因確定と偽装しない。check の panic は `catch_unwind`
  で隔離し `Inconclusive` + `invariant_panics_total` 計数で
  評価を継続する（diagnostics sink と同じ隔離方針）。
- `publish_invariants` は outcome を OBS-09 語彙へ正直に写す:
  全評価 check を `Evidence`（basis 付き verdict は `Derived`、
  無ければ `Reported`）として記録し、`Violated` は
  `Confirmed` certainty の `DiagnosticFinding`（severity は
  check 宣言）、`Inconclusive` は `NotEvaluable`/`Info` の
  `DiagnosticFinding` とする。`Satisfied` は Evidence のみで
  finding を生成しない。finding の Open→Resolved 完了は現行
  `Diagnostics` 公開 API に resolve 経路がないため deferred
  制約として文書化する（Open findings は retention FIFO で
  bounded）。
- `DebugSession` は `Disabled → Prepared → Active → Draining →
  Stopped` の一方向遷移のみ許し、不正遷移・権限なし操作・
  Draining 中の新規 prepare は typed `SessionError` で拒否する。
  `SessionPermit` は `DebugAuthority` 経由でのみ発行される
  capability であり、prepare/stop は permit を消費する。
- `SessionDrain` endpoint は writer quiescence（`Arc<AtomicBool>`
  共有 flag — `SourceState` と同じ共有 cell 先例）を報告し、
  quiescence 未確認の endpoint が残る限り session は `Draining`
  に留まり Buffer/sink（ring の Arc）を解放しない。`drain_final`
  相当の回収は `EventRecordDrain`（`rt_bridge::bridge::<
  EventRecord>` の sink 側）が実装し、配信数・drop 累計・残留
  lag を `DrainReport` で会計する。`abandon` は明示的な typed
  経路でのみ Stopped に到達し、未確認 quiescence を成功と
  偽装しない。
- `DebugReport` は `Confidentiality` 開示区分を `DebugConfig.
  max_export_disclosure` で上限認可し、`external=true` は
  `allow_external_export` を要求する。超過は
  `ExportError::Unauthorized` で拒否（黙った downgrade なし）。
  report が運ぶのは `EventRecord` metadata・typed metric 値・
  finding 要約等の closed vocabulary のみで、Secrets/Media/
  Path/Credential/Prompt/stderr を運ぶフィールドは型として
  存在しない。
- `DebugHealth` は固定サイズカウンタで source に登録せず、
  自己観測が evidence/finding/report を再帰生成しない。
- Security: path・media bytes・prompt 全文・Credential/Grant
  Token・child stderr 原文は型として保持できず自動添付しない。
  外部 export は Host の Authorization 境界（capability
  パラメータとしてモデル化）。Synvelis 監査・Provenance を
  best-effort log 化しない。
- 全公開 enum は `#[non_exhaustive]`。`unsafe` 不使用。新規外部
  依存なし（std + 既存 `ruviform-media-core`/`ruviform-rt` 経由の
  既存 crate 内型のみ）。

## Acceptance Criteria

- [ ] AC-1: `InspectionBudget` の `max_items`/`max_bytes`/`timeout`
  が全 section に適用され、超過が `InspectionTruncation` に
  構造化記録される試験がある。`max_items=0` の意味（その section
  の item 収集無効）と `max_bytes=0`/`timeout=None`（無制限）の
  文書化された規約が試験で固定される。
- [ ] AC-2: 要求 target の入力欠落・deadline 超過・開示区分超過が
  `SectionStatus::Unavailable`/`Truncated`/`Withheld` として
  typed に現れ、section が黙って消えない試験がある。
  `MeasurementValidity::Discontinuous`/`Stale`/`Partial`/
  `Unavailable` が Metrics section で保存・表示される。
- [ ] AC-3: `InspectionConsistency` が collector 系世代
  （snapshot/batch/diagnostics source_generation）の一致・
  不一致を `Aligned`/`Skewed`/`Unknown` で報告し、probe deck の
  generation 不一致・clock `UNKNOWN` を検出する試験がある。
  異種 generation domain（collector poll 世代 vs plan 世代）を
  数値比較で混同せず、domain ごとに評価する。
- [ ] AC-4: `InvariantOutcome` の `Satisfied`/`Violated`/
  `Inconclusive` が区別され、観測欠落・世代不一致・未同期読取・
  check panic が `Inconclusive`（原因確定と偽装しない）になる
  試験がある。`publish_invariants` が Evidence/Finding を
  OBS-09 `Diagnostics` へ正直に接続し、`diagnostics.report()` で
  確認できる。
- [ ] AC-5: `DebugSession` の `Disabled→Prepared→Active→Draining→
  Stopped` 全遷移・権限なし拒否・Draining 中 prepare 拒否・
  stop 競合・`EventRecordDrain` 経由の RT buffer 最終回収
  （quiescence 未確認では Stopped にならず sink/ring を保持、
  quiescence + drain 完了で解放）を試験する。
- [ ] AC-6: `DebugReport` が `max_export_disclosure` 超過要求と
  `external` 未認可を `ExportError::Unauthorized` で拒否し、
  許可時に bounded truncation を `ExportTruncation` 相当で
  構造化記録する試験がある。report/snapshot の型に secret/
  media/path/token を運ぶフィールドが存在しないことを型で保証する。
- [ ] AC-7: `logging` feature OFF・外部依存ゼロの default build で
  `debug` module が compile・全 test が pass する
  （`mise run check`/`test` で実証）。
- [ ] AC-8: `mise run fmt-check`/`check`/`lint`/`test`/`test-rt`
  が pass し、既存 observability 試験が回帰しない。RT 隣接値型
  （`SessionRecord`/`DrainedRecord`/`ProbeReading`）の no-alloc を
  `rt_` レーンで試験する。
- [ ] AC-9: Issue #292 の残件（concrete probe adapter・#185 本体・
  Host 統合・両 Consumer 互換・DSP 同値実機検証）は NotRun/
  deferred として正直に記録され、Issue は Open のまま残る。

## Deferred Design Concerns

<!-- Intentionally deferred future work. Link the deferred draft doc via relations.defers. -->

- concrete probe adapter（`ruviform-audio::typed::probe` →
  `ProbeSource`/`InvariantCheck` 実装）・`debug-tools` feature 分離
  | 責務は OBS-11（#293）/`ruviform-audio` | #293 実装時 | 依存
  循環（`ruviform-audio → ruviform-observability`）のため本 crate
  には置けない。
- `Diagnostics` への finding resolve 公開 API | `diagnostics/`
  module の API 拡張 | 別 Issue/改訂 | `record_finding` は Open
  upsert のみで外部発 finding の Resolved 化経路がない。本 slice
  は Evidence+Open finding 接続に限定し、Open 残留は retention
  bounded の既知制約として記録する。
- Host 認可機構（capability 発行・失効・監査） | OBS-11（#293）|
  #293 実装時 | 本 Issue は `DebugAuthority`/`SessionPermit`/
  `allow_external_export` のモデル化まで。
- `FindingSink` → tracing adapter・RT Record → tracing 変換 |
  OBS-03（#278）| #278 実装時 | tracing 依存を本 module に
  持ち込まない。
- 両 Consumer での Inspection/Report 受入・Synvelis Debug 能力
  連携 | #281/#219・synvelis#81 | cross-repo gate | worker 環境は
  単一 repo。
- ON/OFF DSP 結果同値・RT callback alloc/lock/IO 非呼出しの実機
  検証 | domain/consumer 側 + RT verification lane | #293/#281 |
  debug module は RT 呼出 API を持たない構造保証まで。
- PCM/Signal Capture backend・Deterministic Replay・Crash
  Handler | Issue 非対象（#292 本文でも別設計） | 実需要 | lossy
  診断契約を lossless Recording 品質契約と混同しない。

## Sources and Relations

- Issue #292（本 spec の正本・完了条件の slice 対応付け）、親設計
  Issue #275（単一クレート `ruviform-observability`・module 構成・
  feature 方針・安全/所有権の合意版）。
- `docs/specs|designs/issue-277-observability-core-source-contract.md`
  （`SourceIdentity`/`SourceGeneration`/`Confidentiality`/
  `SourceLifecycle`/`SourceState`）、
  `docs/specs|designs/issue-280-rt-record-bridge-clock-correlation.md`
  （`rt_bridge`/`record`/`EventRecord`/`ClockContext`/`CorrelationBook`）、
  `docs/specs|designs/issue-289-observability-collector-metrics.md`
  （`MetricsSnapshot`/`ObservationBatch`/`Measurement`/
  `MeasurementValidity`）、
  `docs/specs|designs/issue-291-observability-diagnostics.md`
  （`Diagnostics`/`Evidence`/`DiagnosticFinding`/`Confidentiality`
  開示モデル）。
- `docs/designs/issue-309-probe-time-context-and-alignment.md` と
  `crates/ruviform-audio/src/typed/probe*.rs`（#185/#308/#309 の
  probe 語彙 — port 写像の正本。observability からは依存不可）。
- `docs/designs/foundation-observability-minimal-api.md`（`debug`
  候補モジュールの最小スケッチ・RT 契約 probe の責務域）、
  `docs/designs/shared-foundations-and-modularization-policy.md`
  （Diagnostics/Debug/Logging の責務区分）。
