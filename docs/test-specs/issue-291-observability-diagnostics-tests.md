---
id: "TSPEC-034UnxESRAtEpUTP5RInqW"
type: "test-spec"
status: "approved"
title: "Issue #291 OBS-09 Diagnostics/Evidence/Finding/Health/Report 試験仕様"
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
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-291-observability-diagnostics.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-289-observability-collector-metrics.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-291-observability-diagnostics.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-291-observability-diagnostics.md"
    - "docs/designs/issue-291-observability-diagnostics.md"
  verified-by: []
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

# Issue #291 OBS-09 Diagnostics/Evidence/Finding/Health/Report 試験仕様

## Purpose

Issue #291（OBS-09）の `src/diagnostics/` 実装が、検証可能な事実と
推定を型で区別し、Source Generation・clock domain・Evidence 寿命・
Health 遷移・Report 上限/開示境界を契約通りに提供することを保証する。
実装が変わっても「何を保証すべきか」を固定し、回帰検出の正本とする。

## Feature

Diagnostics 層は Collector の `MetricsSnapshot`/`Measurement` を消費し、
Evidence（Observed/Derived/Reported/Inferred）・DiagnosticFinding・
HealthState（Healthy/Degraded/Unhealthy/Unknown）・DiagnosticSnapshot・
DiagnosticReport を非 RT で提供する。Logging/Tracing OFF で全機能が
動作し、重要な Health 遷移のみ bounded `FindingSink` 経由で外部通知
可能。

## Rules

- **Evidence 分類の不変条件**: `EvidenceKind` の 4 値は相互に昇格
  しない。Derived/Inferred は空でない `basis`（`EvidenceRef` 列）を
  必須とし、空 basis は `EvidenceError::MissingBasis` で拒否される。
  Observed/Reported は basis を持たず、`Evidence::from_measurement`
  は必ず `kind=Observed`・`source=m.identity`・
  `time=Monotonic(observed_at)`・`validity=m.validity`・
  `confidentiality` を descriptor から引き継ぐ。
- **Evidence/Finding の非流出性**: `EvidencePayload`/`summary` 等の
  型に任意バイト列・path・token・media bytes・prompt 全文・child
  stderr を運ぶフィールドは存在しない。`EvidenceRef`/`EvidenceEntry`
  は engine 採番の単調 id であり、retire/Removed/世代交代した source
  の evidence は `superseded=true` となり別 source へ再結合されない。
- **Finding 確度**: `FindingCertainty::Confirmed` は閾値違反等の
  実測+宣言規則に基づく。根拠なしの原因断定は型/検証で防がれ、
  Inferred は非空 `evidence_refs` を要求する。`DiagnosticCode` は
  `name: &'static str` + `schema_version` で、中央 enum への統合や
  severity の暗黙変更を行わない（domain 所有）。
- **HealthState 遷移**: `Unknown` は Healthy/Degraded/Unhealthy の
  いずれでもない第 4 状態。rule 評価は measurement 不在 →
  `missing_is`、value=None → `valueless_is`、validity!=Valid →
  `reduced_validity_is`、signal 抽出不能 → `Unknown`、threshold 比較
  → Healthy/Degraded/Unhealthy の固定順序。hysteresis は
  `enter_after`/`recover_after` の連続評価カウントで、level 変化で
  streak をリセットし、`current` は escalate のみ（Unknown で
  悪い状態を帳消しにしない）。Source id/generation 変化で rule 状態を
  リセットする。
- **Domain rollup**: 既知（非 Unknown）が 0 → `Unknown`、全て
  Unhealthy → `Unhealthy`、Unhealthy が一部 → `Degraded`、Degraded
  あり → `Degraded`、全て Healthy + Unknown 混在 → `Degraded`、全て
  Healthy → `Healthy`。
- **Report 境界**: `report(request, now)` は
  `request.disclosure > config.max_export_disclosure` で
  `ReportError::Unauthorized`。許可時、`confidentiality >
  request.disclosure` の evidence/finding は除外し
  `withheld_confidential` に計数。findings/bytes/deadline の上限超過は
  `ReportTruncation` に構造化記録され、黙って切らない。report には
  schema version・generated_at・source generation・notes
  （非 transactional 注記）を含む。
- **Logging OFF**: `logging` feature 無効・外部依存ゼロの default
  build で `Diagnostics::new`/`record_evidence`/`add_health_rule`/
  `evaluate`/`report`/`health` が全て動作する。
- **Sink 抑制**: `FindingSink` は Health 遷移のみを通知し、
  `SinkPolicy{max_events, window}` の leaky bucket で window あたり
  `max_events` 件まで。超過は `DiagnosticsHealth.sink_suppressed_total`
  に計数。sink panic は `catch_unwind` で隔離し
  `sink_panics_total` に計数して評価を継続する。
- **非再帰**: `DiagnosticsHealth` は固定カウンタで source に登録
  せず、自身の観測が evidence/finding を再帰生成しない。

## Examples

- **Golden Evidence**: `Evidence::from_measurement` で作った evidence
  が `kind()==Observed`・`source()==Some(m.identity)`・
  `validity()==m.validity`・`confidentiality()==descriptor.semantics.
  confidentiality`・`basis().is_empty()` であることを assert。
  `Evidence::derived(&[], ..)` は `Err(MissingBasis)`、
  `Evidence::inferred(&[r1], ..)` は `Ok` で `basis()==[r1]`。
- **Health 遷移**: `HealthRule{ direction: Above, degrade_at: Some(5.0),
  unhealthy_at: Some(9.0), hysteresis: {enter_after:2,
  recover_after:2} }` に gauge=6.0 を 1 回 evaluate → state は
  Healthy のまま（streak 1 < enter_after 2）。2 回目 → Degraded。
  gauge=0.0 を 1 回 → Degraded 維持、2 回連続 → Healthy（回復）。
  source generation を +1 して再 evaluate → hysteresis 状態リセットで
  streak やり直し。
- **Unknown/stale**: measurement 不在の rule → `missing_is` に従う
  （例: Unknown）。`validity=Stale` → `reduced_validity_is` に従う。
  `value=None` → `valueless_is`。
- **部分障害**: domain に 2 rule（片方 Healthy・片方 Unhealthy）→
  `DomainHealth.state == Degraded`。
- **Report truncation**: `ReportLimits{max_findings:1, max_bytes:大,
  deadline:None}` で findings 3 件 → report.findings.len()==1、
  `truncation.findings_dropped==2`。
- **Unauthorized**: `max_export_disclosure=Internal` の engine に
  `ReportRequest{disclosure: Confidential}` → `Err(Unauthorized)`。
  `disclosure=Public` で Confidential evidence を含む →
  evidence は除外・`withheld_confidential>=1`。
- **Sink 抑制**: `SinkPolicy{max_events:2, window:60s}` で 5 回の
  Health 遷移を発生 → sink には 2 件のみ、
  `sink_suppressed_total==3`。
- **Logging OFF**: `cargo build -p ruviform-observability`（default
  features）で `diagnostics` module が compile・全 test が pass。

## Guarantees

- 全公開 enum に `#[non_exhaustive]`。`unsafe` 不使用。新規外部依存・
  Cargo.lock 変更なし。`Cargo.toml` feature 追加なし。
- `DiagnosticSnapshot`/`DiagnosticReport` は `consistency=
  IndependentReads` を保持し、単一時点 transactional snapshot と
  主張しない。
- `Diagnostics::health()` は固定サイズの `DiagnosticsHealth` を返し、
  source 非登録・非再帰。
- 既存 `collector`/`source`/`metrics`/`record`/`rt_bridge`/`logging`
  の公開 API・振る舞いは無改変。

## Non-goals

- tracing adapter の実装（OBS-03 #278 deferred）。実 `tracing::Event`
  への変換は本 spec では検証しない。
- Host 認可機構の実装（OBS-11 #293 deferred）。capability パラメータ
  としての `max_export_disclosure` モデル化のみ。
- 両 Consumer（AudioLens/Synvelis）での実機 Evidence/Health 受入
  （#281 cross-repo gate・NotRun）。
- Evidence/Finding の永続化・外部 export 形式・UI 表示。
- f64 以外の signal 型・複合 rule（AND/OR）・rule 動的 reload。
- collector 自身の `CollectorHealth` と本 `HealthState` の統合
  （別概念として維持）。

## Risk

- Evidence の basis 必須性を緩めると「根拠なき推論」が型を通り、
  AC-1 の golden 保証が崩れる。
- hysteresis を連続カウントでなく単発閾値にすると flapping を誤検出
  し、AC-3 の明示的状態遷移が保証できない。
- 開示フィルタを「値を伏せる」方式に変えると secret が report に
  残り、AC-6 の非流出保証が崩れる（項目ごと除外が必須）。
- sink 抑制を外すと診断障害時に外部へ無制限 emit し、AC-5 の bounded
  保証が崩れる。
- `DiagnosticCode` を中央 enum に統合すると domain error との二重
  管理・互換性破壊が起き、AC-7 を満たせない。
