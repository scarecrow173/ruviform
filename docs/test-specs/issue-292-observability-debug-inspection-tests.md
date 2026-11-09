---
id: "TSPEC-a7qCcQ6m0LyKTiq4IswgJo"
type: "test-spec"
status: "approved"
title: "Issue #292 OBS-10 Debug Inspection/Invariant/Session/Report 試験仕様"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/292"
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
    - "docs/designs/issue-292-observability-debug-inspection.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-291-observability-diagnostics-tests.md"
    - "docs/designs/issue-291-observability-diagnostics.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-292-observability-debug-inspection.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-292-observability-debug-inspection.md"
    - "docs/designs/issue-292-observability-debug-inspection.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/277"
    - "https://github.com/scarecrow173/ruviform/issues/280"
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/289"
    - "https://github.com/scarecrow173/ruviform/issues/291"
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

# Issue #292 OBS-10 Debug Inspection/Invariant/Session/Report 試験仕様

## Purpose

Issue #292（OBS-10）の `src/debug/` 実装が、Inspection の予算・
切り詰め・整合性を型で報告し、Invariant の 3 値判定を原因確定と
偽装せず Diagnostics へ接続し、Debug Session の権限・状態遷移・
Draining・RT buffer 最終回収・Report の開示/認可境界を契約通りに
提供することを保証する。実装が変わっても「何を保証すべきか」を
固定し、回帰検出の正本とする。

## Feature

Debug 層は OBS-02/05/07/09 の既存契約（`SourceIdentity`/
`Confidentiality`/`EventRecord`/`rt_bridge`/`MetricsSnapshot`/
`ObservationBatch`/`Diagnostics`）を消費し、非 RT の
`DebugInspector` を提供する。`inspect` は `InspectionBudget`
（max_items/max_bytes/timeout）と `InspectionRequest`
（targets/detail/disclosure）に従い型付き `InspectionSnapshot` を
返し、`InvariantCheck`/`ProbeSource`/`SessionDrain` の `Box<dyn>`
port 経由で domain 所有の検査・観測・回収を注入する。
`DebugSession` は `Disabled→Prepared→Active→Draining→Stopped`
の一方向状態機械で、`DebugAuthority`/`SessionPermit` capability
により権限ある Host のみが制御する。`DebugReport` は開示区分と
external export 認可で未認可要求を拒否する。Logging/Tracing OFF
（`logging` feature 無効・外部依存ゼロ）でも全機能が動作する。

## Rules

- **Budget 規約**: `InspectionBudget{max_items, max_bytes,
  timeout}` の `0`/`None` は項目ごとに文書化した意味を持つ
  （`max_items=0` は全 section の item 収集無効・集計は継続、
  `max_bytes=0`/`timeout=None` は無制限）。超過は
  `InspectionTruncation{items_dropped, bytes_estimated,
  deadline_exceeded, withheld_confidential}` に構造化記録され、
  黙って項目を落とさない。
- **Section status**: 要求 target の入力欠落・deadline 超過・
  port 失敗は `SectionStatus::Unavailable{reason}`、全項目が
  開示区分超過は `Withheld`、予算打切りは `Truncated` として
  typed に現れ、section は消えない。`MeasurementValidity` の
  `Discontinuous`/`Stale`/`Partial`/`Unavailable` は Metrics
  section の entry/集計で潰さず保持される。
- **Consistency**: `InspectionConsistency` は collector 系
  （`MetricsSnapshot.generation`/`ObservationBatch.generation`/
  `DiagnosticReport.source_generation`）を同一 domain として
  評価し、一致 `Aligned`・不一致 `Skewed`・判定不能 `Unknown`
  を最も保守的な側で報告する。probe deck は deck generation と
  各 `ProbeReading.generation` の不一致・clock mark `UNKNOWN`
  を検出し同じく報告する。異種 generation domain（collector
  poll 世代 vs plan 世代）は数値比較で混同せず domain ごとに
  評価する。単一時点 transactional snapshot は主張しない。
- **Invariant 3 値**: `InvariantOutcome` は `Satisfied`/
  `Violated`/`Inconclusive` を区別する。観測欠落・世代不一致
  （probe index/source identity 不一致を含む）・未同期読取・
  check 評価不能・check panic は `Inconclusive` であり、
  `InconclusiveReason` が原因を型で保持する（原因確定と
  偽装しない）。`InvariantVerdict` は `Violated` に severity
  必須・`Inconclusive` に reason 必須の構築時不変条件を持ち、
  違反は `InvariantError` で拒否される。check panic は
  `catch_unwind` で隔離し `invariant_panics_total` に計数して
  評価を継続する。
- **Diagnostics 接続**: `publish_invariants` は全評価 check を
  `Evidence` として記録し（`basis` 非空は `Derived`、空は
  `Reported`）、`Violated` は `Confirmed` certainty の
  `DiagnosticFinding`（severity は check 宣言）、
  `Inconclusive` は `NotEvaluable`/`Info` の `DiagnosticFinding`
  とする。`Satisfied` は Evidence のみで finding を生成しない。
  finding は `(domain, code)` upsert で bounded。retention 無効
  （max=0）は `skipped` 計数で報告し握り潰さない。
- **Session 状態機械**: `Disabled→Prepared→Active→Draining→
  Stopped` のみ許可し、`Stopped→Prepared` は新 generation の
  再 prepare。`prepare`/`request_stop`/`abandon` は
  `SessionPermit` を消費し、`SessionPermit` は
  `DebugAuthority::permit()` 経由でのみ発行される。
  非 Stopped での `prepare` は `SessionBusy`、権限なしは
  `NoAuthority`/`PermitDenied`、不正遷移は `InvalidTransition`
  で typed 拒否する（放棄した要求を成功と報告しない）。
- **Draining/最終回収**: `request_stop` は `Draining` に遷移し
  新規 prepare/attach を拒否する。`poll_drain` は
  `writer_quiesced()==false` の endpoint に `drain_final` を
  呼ばず pending quiescence に計上し、quiesced endpoint のみ
  bounded `drain_final` を実行する。全 endpoint が quiesced かつ
  `lag==0` の時点で初めて `drains` を drop して ring/Arc を解放
  し `Stopped` に到達する。`drain_timeout` 超過でも強制
  Stopped せず `deadline_exceeded=true` で Draining を継続報告
  する。`abandon_session` は明示的 typed 経路でのみ未確認
  quiescence のまま Stopped に到達し
  `sessions_abandoned_total` に計数する。
- **切替**: single-slot session のため `prepare` は非 Stopped
  で `SessionBusy`。切替は `request_stop` → `poll_drain` で
  `Stopped` を待ってから `prepare`（新 generation）。暗黙
  queue は作らない。
- **Report 境界**: `report(request, snapshot, now)` は
  `request.disclosure > config.max_export_disclosure` または
  `request.external && !config.allow_external_export` で
  `ExportError::Unauthorized`（`reports_rejected_total` 計数）。
  許可時、`confidentiality > request.disclosure` の項目は項目ごと
  除外し `withheld_confidential` に計数（値を伏せる方式ではない）。
  `ExportLimits` の items/bytes/deadline 超過は
  `ExportTruncation` に構造化記録される。`DebugReport`/
  `InspectionSnapshot`/`SessionRecord` が運ぶ型は closed
  vocabulary（`EventRecord` metadata・`MetricValue`・finding 要約・
  `&'static str`）のみで、Secrets/Media/Path/Credential/Prompt/
  stderr を運ぶフィールドは型として存在しない。
- **非 RT only**: debug の公開操作は全て非 RT（alloc/lock/
  vector 操作を伴う）。RT callback 経路に触れる API は存在
  しない。RT 隣接値型（`SessionRecord`/`DrainedRecord`/
  `ProbeReading`/`ProbeClockMark`/`ProbeStatusKind`）は `Copy`
  で no-alloc を `rt_` レーンで検証する。
- **Logging OFF・非再帰**: `logging` feature 無効・外部依存ゼロ
  の default build で `debug` module が compile・全機能が動作
  する。`DebugHealth` は固定カウンタで source に登録せず、
  自己観測が evidence/finding/report を再帰生成しない。
- **Probe port の非重複**: `ProbeSource` は #185 `ProbeBroker`/
  `ProbeDeck` の OFF/Monitor/Capture・capture・rebind を再実装
  しない。observability 側は `ProbeMode`/`ProbeAvailability`/
  `ProbeStatusKind`/generation/clock mark の *読み取り語彙* のみ
  を持ち、concrete adapter は #293/`ruviform-audio` 側の責務。
- **Debug OFF**: debug module 非使用時に既存 Live/Recording
  Router/Bypass・本番 Invariant（Invalid Graph/Plugin ABI 等の
  domain 側検査）の振る舞いは無改変。debug は追加コストのある
  opt-in 層であり、既定経路を変更しない。

## Examples

- **Budget truncation**: `InspectionBudget{max_items:2,
  max_bytes:0, timeout:None}` で Metrics section に 5 series →
  `entries.len()==2`、`truncation.items_dropped==3`、
  `status==Truncated`。`max_items:0` → `entries` 空・
  `total_series==5`（集計のみ）。
- **Unavailable**: `InspectionInputs{metrics:None, ..}` で
  `targets:[Metrics]` → `SectionStatus::Unavailable{
  reason:InputNotProvided}`。`timeout:Some(0)` 相当で deadline
  超過 → 後続 section が `Unavailable{DeadlineExceeded}`。
- **Withheld**: `request.disclosure=Public` で Confidential の
  measurement のみ → `SectionStatus::Withheld`・
  `withheld_confidential>=1`。
- **Consistency**: snapshot.generation=3・batch.generation=3・
  diagnostics.source_generation=4 → `level==Skewed`・
  `collector.aligned==false`。全て 3 → `Aligned`。
  `diagnostics` 入力なし → その field は `None` で、残りが一致
  なら `Aligned`（判定不能は `Unknown` にしない — 欠落は
  欠落として報告）。probe deck generation=5 で reading
  generation=4 → `ProbeDeckView.aligned==false`・
  `mismatched>=1`・`ConsistencyLevel::Skewed`。clock
  `domain==ClockDomainId::UNKNOWN` → `clock_known==false`・
  `level==Unknown`。
- **Invariant 3 値**: check が `InvariantVerdict::satisfied()` →
  outcome `Satisfied`・finding なし・Evidence `Reported`。
  `violated(Warning, "..")` → outcome `Violated`・finding
  `Confirmed`/`Warning`。`inconclusive(MissingObservation, "..")`
  → outcome `Inconclusive`・finding `NotEvaluable`/`Info`・
  `inconclusive==Some(MissingObservation)`。check panic →
  `Inconclusive{CheckPanicked}`・`invariant_panics_total==1`・
  他 check の評価は継続。
- **Diagnostics 接続**: `publish_invariants(&mut diag, &snap,
  now)` 後に `diag.report(&ReportRequest{disclosure:Internal,
  ..}, now)` で当該 `DiagnosticCode` の finding が Open で
  現れ、Evidence が `evidence_total` に計上される。`Satisfied`
  のみの場合 finding は増えない。
- **Session lifecycle**: `authority.permit()` →
  `prepare_session` → `attach_drain(EventRecordDrain)` →
  `activate_session` → `request_stop` → `poll_drain` で quiesced
  flag 未設定 → `Draining{pending_quiescence:1}`・sink 保持。
  flag 設定 + records 消化 → `Stopped{clean:true}`・sink/ring
  解放（`Arc::strong_count` で検証可能）。`drain_timeout` 超過 →
  `Draining{deadline_exceeded:true}` 継続。`abandon_session` →
  `Stopped`・`sessions_abandoned_total==1`・report に abandoned
  表示。
- **権限/競合**: authority なし `prepare_session` →
  `Err(NoAuthority)`。`prepare` 後に `prepare` → `Err(SessionBusy
  {state:Prepared})`。`Draining` 中 `attach_drain` →
  `Err(InvalidTransition)`。`activate` で drains 空 →
  `Err(NoDrains)`。
- **Report**: `max_export_disclosure=Internal` で
  `disclosure:Confidential` → `Err(Unauthorized)`。
  `external:true` で `allow_external_export:false` →
  `Err(Unauthorized)`。許可時 Confidential 項目は除外・
  `withheld_confidential` 計数。`ExportLimits{max_items:1}` で
  records 3 件 → `records.len()==1`・`truncation.items_dropped==2`。
- **Logging OFF**: `cargo build -p ruviform-observability`（default
  features）で `debug` module が compile・全 test が pass。
- **RT 値型 no-alloc**: `SessionRecord`/`ProbeReading`/
  `ProbeClockMark`/`ProbeStatusKind` の構築・copy・読み取りが
  `assert_no_alloc`（rt レーン）で pass。

## Guarantees

- 全公開 enum に `#[non_exhaustive]`。`unsafe` 不使用。新規外部
  依存・Cargo.lock 変更なし。`Cargo.toml` feature 追加なし。
- `InspectionSnapshot`/`DebugReport` は非 transactional である
  ことを `notes`/型で明示し、単一時点 snapshot を主張しない。
- `DebugHealth` は固定サイズカウンタで source 非登録・非再帰。
- `EventRecordDrain` は `rt_bridge::RecordSink<EventRecord>` を
  再利用し、独自の record 経路・第二 probe engine を持たない。
- 既存 `collector`/`source`/`metrics`/`record`/`rt_bridge`/
  `diagnostics`/`logging` の公開 API・振る舞いは無改変。
- concrete probe adapter は本 module に存在しない（port trait
  のみ）。`ruviform-observability` から `ruviform-audio` への
  依存は追加されない。

## Non-goals

- concrete probe adapter（`ruviform-audio::typed::probe` →
  `ProbeSource`/`InvariantCheck`）— #293/`ruviform-audio` 側の
  責務（依存循環）。本 spec では port 契約のみ検証する。
- #185 Probe の OFF/Monitor/Capture 本体・`ProbeBroker`/`ProbeDeck`
  の再実装（重複実装禁止）。
- Host 認可機構の実装（OBS-11 #293 deferred）。capability
  パラメータとしての `DebugAuthority`/`SessionPermit`/
  `allow_external_export` モデル化のみ。
- Diagnostics への finding resolve 公開 API（`diagnostics/` は
  編集禁止範囲。Open 残留は retention bounded の既知制約）。
- 両 Consumer（AudioLens/Synvelis）での実機 Inspection/Report
  受入（#281 cross-repo gate・NotRun）。
- ON/OFF DSP 結果同値・RT callback alloc/lock/IO 非呼出しの実機
  検証（domain/consumer 側 + RT verification lane。debug module
  は RT 呼出 API を持たない構造保証まで）。
- PCM/Signal Capture backend・Deterministic Replay・Crash
  Handler・任意内部状態の Serialize。

## Risk

- budget の `0`/`None` 規約を項目ごとに曖昧にすると、caller が
  「無制限」と「無効」を取り違え、AC-1 の予算保証が崩れる。
- consistency で collector poll 世代と plan 世代を同一 domain
  として数値比較すると、無関係な世代差を `Skewed` と誤報し、
  AC-3 の domain 分離保証が崩れる。
- `Inconclusive` を `Violated`/`Satisfied` に潰す、または check
  panic を catch せず伝播させると、観測欠落を原因確定と偽装し、
  AC-4 の 3 値保証が崩れる。
- quiescence 未確認で `drains`/sink/ring を drop すると、RT
  writer がまだ record を書き得る状態で buffer を解放し、
  AC-5 の最終回収保証が崩れる（`abandon` のみが明示的脱出）。
- 開示フィルタを「値を伏せる」方式に変えると secret が report
  に残り、AC-6 の非流出保証が崩れる（項目ごと除外が必須）。
- `ProbeSource`/`InvariantCheck`/`SessionDrain` port を
  `ruviform-audio` 型に直結すると依存循環で compile 不能に
  なり、port 語彙の observability 側写像という境界が崩れる。
