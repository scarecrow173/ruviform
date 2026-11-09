---
id: "SPEC-034UnxeRRp2vX35gnordF9"
type: "spec"
status: "approved"
title: "Issue #291 OBS-09 Diagnostics・Evidence・Finding・Health・Report"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/291"
    - "https://github.com/scarecrow173/ruviform/issues/275"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "https://github.com/scarecrow173/ruviform/issues/291"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-277-observability-core-source-contract.md"
    - "docs/specs/issue-289-observability-collector-metrics.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/foundation-observability-minimal-api.md"
    - "docs/designs/shared-foundations-and-modularization-policy.md"
    - "docs/designs/issue-289-observability-collector-metrics.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
    - "docs/adr/obs-05-observability-leaf-primitive-deps.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-277-observability-core-source-contract.md"
    - "docs/specs/issue-289-observability-collector-metrics.md"
  derived-by: []
  verifies: []
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

# Issue #291 OBS-09 Diagnostics・Evidence・Finding・Health・Report

## What Should Be Built

既存 `ruviform-observability` クレート内に `src/diagnostics/` モジュールを
新設し、OBS-07（Issue #289）Collector が生成する型付き
`Measurement`/`MetricsSnapshot` と OBS-02（Issue #277）の
`SourceIdentity`/`SourceGeneration` 契約を消費して、**検証可能な事実と
推定を区別した診断**を提供する。具体的には:

- `Evidence`: Observed / Derived / Reported / Inferred を型で区別し、
  source ID + generation・clock domain・validity・confidentiality・
  説明可能な根拠参照を持つ証拠語彙。
- `DiagnosticFinding`: 安定した `DiagnosticCode` + schema version・
  domain/source・severity・`EvidenceRef`・確定/推定/評価不能の区別を持つ
  発見語彙。
- `HealthState`（Healthy / Degraded / Unhealthy / Unknown）と
  `HealthEvaluator`: ドメインが明示した評価規則・閾値・Hysteresis・
  状態遷移を非 RT で評価する共通実装。
- `DiagnosticSnapshot`: 現在値・Source 世代・収集時刻・欠落・整合性
  レベルを有界で提供する pull 型ビュー。
- `DiagnosticReport`: 根拠参照・Finding・Health・制限事項・schema
  version・開示区分を含む非 RT の明示的生成物。max findings / max bytes /
  deadline 上限と切り詰め表示、Host 認可をモデル化した開示ポリシーを持つ。
- `FindingSink`: 重要な状態変化のみを非 RT から外部（将来は標準
  tracing）へ接続する bounded・抑制上限付きの hook。subscriber には
  依存しない。

Logging/Tracing OFF（`logging` feature 無効・外部依存ゼロ）でも
Snapshot/Report/Health が利用可能であること。新規クレート・独立
Collector は作らない。

## Why Now

- OBS-02（#277）で source 登録/寿命契約、OBS-07（#289）で Collector の
  `ObservationBatch`/`MetricsSnapshot`/`MeasurementValidity` が main に
  実装済み。`metrics/mod.rs` は diagnostics を consumer として想定済みで
  あり、下流の診断契約を固定する段階にある。
- OBS-03（#278）Tracing 変換・OBS-10（#292）Debug・OBS-11（#293）
  Integration が本モジュールの Snapshot/Report/Health/FindingSink を
  消費する前提であり、API 契約を先に固定する必要がある。
- Diagnostics 不在のままだと各 Consumer（AudioLens/Synvelis）が観測値の
  意味づけ・health 判定・レポート生成を独自に再実装し、「実測を推定と
  断定する」「Unknown を正常扱いする」等の診断誤りが分散する。

## Users and Value

- **OBS-03/10/11 実装者**: `FindingSink`・`DiagnosticSnapshot`・
  `DiagnosticReport` の確定 API と、Evidence/Finding/Health の型付き
  語彙を得る。
- **Domain 側（plugin host・audio engine 等）**: `HealthRule` を宣言し
  `Evidence`/`DiagnosticFinding` を記録するだけで、閾値・Hysteresis・
  回復・stale/Unknown の扱いを共通実装に委ねられる。ドメインの
  `Error`/`Result` 型や plugin-host `DiagRegistry` 等の既存診断正本は
  置き換えない。
- **Consumer（AudioLens/Synvelis）**: Logging OFF でも同一の型付き
  診断結果を Pull で読み、秘密情報・Media・Grant の非開示境界を
  レポート契約として受け取る。

## Scope

### In Scope

- `src/diagnostics/` モジュール群（`mod.rs` / `evidence.rs` /
  `finding.rs` / `health.rs` / `report.rs` / `sink.rs` / `engine.rs` /
  `tests.rs` を想定）と `lib.rs` への module 登録・crate doc 更新。
- `Evidence`/`EvidenceKind`/`EvidenceRef`/`EvidenceEntry`:
  Observed/Derived/Reported/Inferred の区別、`SourceIdentity`
  （id + generation）・clock domain（`Instant` 系と
  `record::ClockContext` の両方を正直に表現）・validity・
  confidentiality・basis（根拠参照）の保持。Observed/Reported は
  basis を持たず、Derived/Inferred は空でない basis を必須とする
  （根拠なき推論を型で防ぐ）。
- `DiagnosticFinding`/`DiagnosticCode`/`FindingSeverity`/
  `FindingCertainty`/`FindingStatus`: 安定 code + schema version、
  domain/source 帰属、severity、EvidenceRef 列、確定/推定/評価不能、
  Open/Resolved の寿命を持つ。summary は `String`（非 RT のみ）で、
  任意バイト列・path・token を運ぶフィールドは型として存在しない。
- `HealthState`/`HealthRule`/`Hysteresis`/`HealthEvaluator`/
  `DomainHealth`/`RuleHealth`: 閾値（above/below 両方向）・
  連続評価 hysteresis・validity 低下/stale/missing/valueless の
  ドメイン宣言写像・Source generation 変化時の状態リセット・
  ドメイン集約（部分障害 → Degraded）の明示的状態遷移。
- `DiagnosticSnapshot`: collector `MetricsSnapshot` 世代を引き継ぐ
  有界ビュー。`SnapshotConsistency` で「複数 atomic の独立読取であり
  単一時点 transactional snapshot ではない」ことを型で明示。
- `DiagnosticReport`: schema version・generated_at・適用された開示
  区分・findings/evidence/health・構造化 truncation 情報・
  limitations を持つ非 RT 生成物。`max_findings`/`max_bytes`/
  `deadline` 上限と、要求開示区分が engine の許可上限を超える場合の
  `ReportError::Unauthorized` 拒否、許可区分を超える evidence/finding
  の除外（非流出）を実装。
- `FindingSink`/`DiagnosticsEvent`/`SinkPolicy`: Health 状態遷移のみを
  通知する object-safe port と、window あたり発行数の抑制上限。
  実 tracing adapter は本 Issue では実装しない（OBS-03 deferred）。
- `Diagnostics` engine: evidence/finding の有界保持（FIFO evict 計数）、
  per-domain `HealthEvaluator` 管理、evaluate/report API、
  `DiagnosticsHealth` 自己観測（source 非登録・非再帰）。
- 契約試験: Evidence/Finding/Health/Report/Sink の golden・状態遷移・
  世代/lifetime・上限/切り詰め・Logging OFF・認可拒否・非流出。

### Out of Scope

- tracing/`tracing::Dispatch`/Subscriber への依存・発行、および実
  tracing adapter の実装（OBS-03 #278 の責務。本 Issue は bounded
  `FindingSink` port の提供まで）。
- Host 認可機構そのもの（OBS-11 #293 の責務）。本 Issue は
  capability/policy パラメータとしてのモデル化まで。
- 両 Consumer（AudioLens/Synvelis）での Evidence/Health ユースケース
  実機検証（#281 の cross-repo gate。本 worker 環境では NotRun）。
- ドメイン `Error`/`Result` 型・plugin-host `DiagRegistry`・Synvelis
  監査/Gateway Audit 正本の置き換え・二重管理。AI root-cause 診断・
  未確証の原因断定・永続化・UI 実装。
- 新規クレート・新規外部依存・`unsafe`・既存 collector/source/metrics/
  record/rt_bridge/logging の公開 API 変更。

## Requirements

- Evidence は Observed/Derived/Reported/Inferred を区別し、実測である
  ことを理由に無条件に完全・正確とはみなさない（validity・
  confidentiality・basis を保持する）。Derived/Inferred は空でない
  basis を必須とし、根拠なき推論を構築できない。
- Finding は安定 `DiagnosticCode`（name + schema version）と
  Confirmed/Inferred/NotEvaluable を持ち、原因推定の根拠がなければ
  断定（Confirmed）しない。コード再利用や severity の勝手な変更で
  互換性を壊さない（code はドメイン所有の `&'static str` であり中央
  enum に統合しない）。
- HealthState の Unknown は正常でも異常でもない第 4 状態として扱い、
  欠損・観測古さ（stale）・Source 未対応を明示する。閾値・Hysteresis・
  回復はドメイン宣言規則に基づき非 RT で評価し、全 Counter 増分を
  WARN にしない（plugin warmup passthrough 正常等のドメイン意味は
  Domain Owner が規則で宣言する）。
- `DiagnosticSnapshot` は現在値・Source 世代・収集時刻・欠落・整合性
  レベルを有界で提供し、複数 atomic 読取を単一時点 transactional
  snapshot と主張しない。
- `DiagnosticReport` は非 RT の明示的生成で、max findings / max bytes /
  deadline 上限と切り詰め表示を持つ。詳細・秘密情報の開示は Host 認可
  （engine の `max_export_disclosure` ポリシー）を超える要求を
  `ReportError::Unauthorized` で拒否し、許可区分を超える内容は
  レポートへ含めない。
- 収集済み Evidence の寿命は Source の retire/generation に固定し、
  Retired Reference を別 Source へ再結合しない（id は再利用しない・
  generation 不一致は superseded として可視化する）。Snapshot/Report
  はサンプリング品質（truncation/欠落/世代）を表示可能にする。
- 重要な状態変化（Health 遷移）のみを `FindingSink` 経由で非 RT から
  外部へ接続可能とし、発行数に抑制上限を持つ。Diagnostics の評価自体は
  Subscriber/logging backend に依存しない（Logging OFF で全機能動作）。
- 診断失敗を再帰的にログ出力しない。Collector 自身の health
  （`CollectorHealth`）と本モジュールの `HealthState` は別概念として
  明確に区別する。`DiagnosticsHealth` は固定サイズで source に登録
  しない（自己観測の無限再帰を構造的に排除）。
- Security: path・media bytes・prompt 全文・Credential/Grant Token・
  child stderr 原文は型として保持できず自動添付しない。外部
  Agent/Gateway への開示は Host の Authorization 境界（本モジュールは
  capability パラメータとしてモデル化）。Synvelis 監査と Provenance を
  best-effort log 化しない。
- Ruviform は Plugin/Device/Graph の Evidence 契約を提供し、Synvelis の
  DAW 影響判定と AudioLens の解析原因説明は Consumer の責務とする。
- 全公開 enum は `#[non_exhaustive]`。`unsafe` 不使用。新規外部依存
  なし（std + 既存 `ruviform-media-core` の clock 語彙のみ）。
- 全操作は非 RT only（alloc/lock を伴う）。RT writer 経路・既存
  primitive の API・性能・ビルドを回帰させない。

## Acceptance Criteria

- [ ] AC-1: 事実（Observed）/Derived/Reported/推論（Inferred）/
  Unknown を混同しない型と golden 試験がある。Derived/Inferred の
  basis 必須性・Observed/Reported の basis 不在が型で強制される。
- [ ] AC-2: Source Generation/Clock/Evidence lifetime/Report 上限/
  Report 切り詰めが試験される（generation 変化での hysteresis 状態
  リセット・superseded evidence の可視化・max findings/bytes/deadline
  による切り詰めと truncation 情報）。
- [ ] AC-3: Health 閾値/Hysteresis/回復/Unknown/stale/部分障害が
  明示的状態遷移として試験される（連続評価カウント・recover streak・
  Unknown の第 4 状態・stale/missing/valueless の宣言写像・部分障害の
  Degraded 集約）。
- [ ] AC-4: `logging` feature OFF・外部依存ゼロで
  Snapshot/Report/Health/evaluate が利用可能であることがビルド・試験で
  示される。
- [ ] AC-5: 重要状態変化のみが bounded `FindingSink` へ非 RT から通知
  され、抑制上限（window あたり発行数）超過が計数される。実 tracing
  adapter は deferred として記録される。
- [ ] AC-6: Secret/Media/Gateway Grant の非流出（型に任意バイト列・
  path・token を運ぶフィールドが存在しない・許可区分超過内容の除外）
  と未認可 Report 拒否（`ReportError::Unauthorized`）が試験される。
- [ ] AC-7: Domain Error との二重管理を行わない（`DiagnosticCode` は
  ドメイン所有の `&'static str` で中央 enum に統合しない）こと、
  plugin-host `DiagRegistry` 等の既存診断正本を置き換えないことが
  設計・文書で明示される。
- [ ] AC-8: `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` が
  pass し、既存 observability 試験が回帰しない。
- [ ] AC-9: 両 Consumer の Evidence/Health ユースケース検証（#281）は
  cross-repo gate のため本スライスでは NotRun と記録し、Issue #291 は
  Open のまま残る。

## Deferred Design Concerns

<!-- Intentionally deferred future work. Link the deferred draft doc via relations.defers. -->

- `FindingSink` → 標準 `tracing` Event への実 adapter（span/event
  フィールド設計・rate limit 連携） | 責務は OBS-03（#278） | Issue
  #278 実装時 | tracing 依存を本モジュールへ持ち込むと Logging OFF
  契約が壊れる。
- Host 認可機構（capability 発行・失効・監査） | 責務は OBS-11（#293）
  | #293 実装時 | 本 Issue はポリシーパラメータのモデル化まで。
- 両 Consumer での Evidence/Health ユースケース受入 | cross-repo gate
  | Issue #281 | worker 環境は単一 repo のみ。
- Evidence の永続化・downsample・外部 export 形式 | Issue 非対象 |
  実需要 | 有界 in-process 保持で現行要件は満たせる。
- f64 以外の signal 型・複合 rule（AND/OR）・rule の動的 reload |
  OBS-02/07 と同じ保留判断 | 実需要 | 未定義の意味論を仮固定しない。

## Sources and Relations

- Issue #291（本 spec の正本・完了条件）、親設計 Issue #275、先行
  Issue #277（OBS-02 source 契約）・#289（OBS-07 Collector/Metrics）。
- `docs/specs|designs/issue-277-observability-core-source-contract.md`
  （`SourceIdentity`/`SourceGeneration`/`Confidentiality` 等の語彙）、
  `docs/specs|designs/issue-289-observability-collector-metrics.md`
  （`Measurement`/`MetricsSnapshot`/`MeasurementValidity` の消費契約）、
  `docs/designs/shared-foundations-and-modularization-policy.md`
  （Diagnostics と Error/Metrics/Logging の責務区分）、
  `docs/designs/foundation-observability-minimal-api.md`
  （diagnostics モジュールの最小 API スケッチ）。
