---
id: "IMPL-034UnDVtaBTUOTvc3aTj8m"
type: "impl"
status: "completed"
title: "Issue #291 OBS-09 Diagnostics 実装記録"
created: "2026-11-09"
updated: "2026-11-10"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/291"
  changes:
    added:
      - type: "directory"
        path: "crates/ruviform-observability/src/diagnostics/"
    modified:
      - type: "file"
        path: "crates/ruviform-observability/src/lib.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/implement-issue-291-observability-diagnostics.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #291 OBS-09 Diagnostics 実装記録

## Summary

Issue #291（OBS-09）の `src/diagnostics/` モジュールを実装する。
Evidence（Observed/Derived/Reported/Inferred）・DiagnosticFinding・
HealthState/HealthEvaluator・DiagnosticSnapshot/DiagnosticReport・
bounded FindingSink を非 RT で提供し、Logging OFF で動作する。

## Context

`docs/tasks/implement-issue-291-observability-diagnostics.md` の
実装スライス。上流: `docs/specs/issue-291-observability-diagnostics.md`、
`docs/designs/issue-291-observability-diagnostics.md`、
`docs/test-specs/issue-291-observability-diagnostics-tests.md`。
消費する既存契約: OBS-02 `SourceIdentity`/`SourceGeneration`/
`Confidentiality`/`SourceSemantics`、OBS-07 `MetricsSnapshot`/
`Measurement`/`MeasurementValidity`/`MetricValue`、`record::
ClockContext`。

## Implementation

### Changes

- `crates/ruviform-observability/src/diagnostics/` 新設（8 ファイル）。
  - `evidence.rs`: `EvidenceKind`（Observed/Derived/Reported/Inferred）、
    `EvidenceTime`（Monotonic/Clock/Unstamped）、closed `EvidencePayload`
    （Metric/Claim/None — 任意バイト列を持たない）、`Evidence`、
    `EvidenceMeta`、`EvidenceRef`/`EvidenceEntry`、`EvidenceError`。
    `Evidence::from_measurement` は `Measurement` の identity・validity・
    confidentiality・observed_at を欠損なく写す。
  - `finding.rs`: `DiagnosticCode`（domain 所有の `name` +
    `schema_version`）、`FindingSeverity`/`FindingCertainty`/
    `FindingStatus`（いずれも `#[non_exhaustive]`）、
    `DiagnosticFinding`、`FindingError`。Inferred は evidence 参照必須、
    upsert は worst severity・highest certainty・strictest
    confidentiality を保持し first_seen を維持する。
  - `health.rs`: `HealthState`（Healthy/Degraded/Unhealthy/Unknown）、
    `SignalKind`/`SignalDirection`/`Hysteresis`/`HealthRule`/
    `RuleInput`/`RuleHealth`/`DomainHealth`/`RuleTransition`/
    `HealthEvaluator`。閾値上下方向・gauge/counter delta/counter rate・
    `enter_after`/`recover_after` hysteresis・generation 変更時の状態
    リセット・missing/stale/valueless/unevaluable の宣言済み写像・
    domain rollup（部分障害は Degraded）。
  - `report.rs`: `SnapshotConsistency`、`DiagnosticSnapshot`、
    `ReportLimits`/`DiagnosticsConfig`/`ReportRequest`/
    `ReportError`/`ReportTruncation`/`DiagnosticReport`、
    `REPORT_SCHEMA_VERSION`、サイズ推定ヘルパー。
  - `sink.rs`: object-safe `FindingSink`、`DiagnosticsEvent`、
    `SinkPolicy`（window + max_per_window 抑制）。
  - `engine.rs`: `Diagnostics` 集約（bounded evidence/findings FIFO、
    per-domain evaluator 管理、evaluate/report、sink panic 隔離、
    `DiagnosticsHealth` 固定カウンタ — 自己診断は source 登録しない）。
  - `tests.rs`: 30 件の契約テスト。
- `crates/ruviform-observability/src/lib.rs`: `pub mod diagnostics;`
  登録と crate doc への責務・非 RT 注記の追記。
- docs chain（spec/design/test-spec/plan/task/ir）と各 index README。

### Decisions During Implementation

- `EvidencePayload` は closed enum（Metric 値は typed `MetricValue`
  参照ではなく所有コピー、Claim は `&'static str`、None）とし、任意
  バイト列・path・token・media・prompt・raw stderr を構造的に排除した。
- Health の signal は `MetricValue` の u64 フィールドから `f64` へ
  写像（gauge `value`/`previous`、counter `delta`/`rate_per_sec`）。
  counter delta/rate が `None` の場合は `RuleInput::Unevaluable` →
  `Unknown` であり、0 扱いや warning 化はしない。
- `Unknown` は「正常でも異常でもない」状態として、既知の bad 状態を
  上書きしない（stale/missing 入力時に Degraded/Unhealthy を消さない）。
- Report の認可は runtime 依存を持たず `max_export_disclosure`
  （`Confidentiality` 上限）+ `ReportRequest.disclosure` の policy
  parameter でモデル化。超過要求は `ReportError::Unauthorized`。
  開示超過 item は redact ではなく entry 単位で除外し、
  `withheld_confidential` で計数する。
- tracing adapter は本 issue では実装せず、object-safe `FindingSink` +
  `SinkPolicy` の抽象接続点のみ提供（OBS-03 #278 へ defer）。
- Snapshot は `SnapshotConsistency::IndependentReads` を型レベルで
  明示し、transactional 単一時点 snapshot ではないことを保証する。

## Related Experiments

### Adopted

（なし）

### Rejected

（なし）

## Validation

- `mise run fmt-check` — PASS（exit 0、Windows 11 x64 / Git Bash、
  rust 1.99.0 via mise）。
- `mise run check` — PASS（exit 0、`cargo check --workspace
  --all-targets`）。
- `mise run lint` — PASS（exit 0、clippy `-D warnings` ゲート）。
- `mise run test` — PASS（exit 0、`cargo test --workspace`。
  diagnostics 31 テストを含む全クレート緑）。
- `mise run test-rt` — PASS（exit 0、`cargo test --workspace rt_`）。
- 補助証拠（canonical ではない）: `cargo test --lib diagnostics`
  31 件、`cargo clippy --all-targets`、`cargo check --all-features`
  （logging feature 有効時も diagnostics は独立してコンパイル）。
- review-protocol: 実装レビュー 1 回。CONFIRMED P1 を 1 件検出・修正
  （source generation 変更時に confirmed state が transition なしで
  Healthy に reset される欠陥 → streak のみ reset に修正し回帰テスト
  `generation_change_never_fabricates_healthy_over_confirmed_bad`
  を追加）。残りは P3 non-blocking（finding upsert が source を
  含まない、`observe_again` の refs 全置換、初回 evaluate の
  Unknown→X domain transition、report byte budget が findings を
  個別 drop しない）。blocker なし = 承認扱い。

## Audit（2026-11-09、chore/obs-08-09-10-audit、base `0d07471d`）

Issue #291 受入条件の行ごとの照合結果。実装は main（PR #504
`70375e06`）の `src/diagnostics/`（`evidence.rs`/`finding.rs`/
`health.rs`/`report.rs`/`sink.rs`/`engine.rs`、31 tests）による。

| 受入行 | 判定 | 証跡 |
| --- | --- | --- |
| 事実/Derived/Reported/推論/Unknown を混同しない型・Golden test | 充足（既存） | `evidence.rs` 分類型 + `tests.rs` golden/分類試験 |
| Source Generation/Clock/Evidence lifetime/Report 上限/切り詰め試験 | 充足（既存） | `tests.rs`（generation 変更・lifetime・byte budget・truncation） |
| Health 閾値/Hysteresis/回復/Unknown/stale/部分障害の明示的状態遷移 | 充足（既存） | `health.rs` + 遷移試験（review 修正の generation reset 回帰含む） |
| Logging OFF でも Snapshot/Report/Health を利用可能 | 充足（既存） | `mod.rs`/`sink.rs` doc + default feature で `tracing` 非依存を確認 |
| 重要状態変化のみ非 RT から標準 tracing へ接続・抑制上限 | 充足（port 水準） | `FindingSink` object-safe port + `SinkPolicy`（`max_events`/`window` 抑制・`sink_suppressed_total` 計数）。具象 `tracing` adapter は OBS-03 #278 の責務として分離済み |
| Secret/Media/Gateway Grant 非流出・未認可 Report 拒否試験 | 充足（既存） | `report.rs` の confidentiality/認可検査 + 試験 |
| Domain Error との二重管理禁止・Plugin 既存 DiagRegistry 尊重 | 充足（既存） | 設計境界どおり domain error を複製しない構成（design/test-spec 参照） |
| 両 Consumer の Evidence/Health ユースケースを #281 で検証 | NotRun | cross-repo gate・本タスク対象外 |

## Audit（2026-11-10、rf291-obs09 残件確認 slice、base `1dc235b9`）

Issue #291 受入条件の再照合。実装は main の `src/diagnostics/`
（PR #504 `70375e06` 以降変更なし）に加え、OBS-03 の具象 adapter
`tracing_export::TracingFindingSink`（PR #507 `d134f95f`・slice 2
`b36f6de2`）が main に着地済み。本 slice の差分は
`diagnostics/tests.rs` への `EvidenceTime::Clock` golden 試験
`evidence_clock_domain_stamp_is_preserved_verbatim` 1 件追加のみ
（受入行 2 の Clock 要素を diagnostics 層で明示検証）。

| 受入行 | 判定 | 証跡 |
| --- | --- | --- |
| 事実/Derived/Reported/推論/Unknown を混同しない型・Golden test | 充足 | `evidence.rs` `EvidenceKind` 4 分類 + basis 不変条件 + `tests.rs` golden/分類試験 |
| Source Generation/Clock/Evidence lifetime/Report 上限/切り詰め試験 | 充足 | generation リセット 2 試験・superseded/lifetime・byte/deadline/count truncation 試験 + 今回追加の `EvidenceTime::Clock` verbatim 保存 golden |
| Health 閾値/Hysteresis/回復/Unknown/stale/部分障害の明示的状態遷移 | 充足 | `health.rs` 固定評価順序 + hysteresis + rollup + 遷移試験一式 |
| Logging OFF でも Snapshot/Report/Health を利用可能 | 充足 | default feature（`logging` 無効）で `diagnostics` 全 API が `tracing` 非依存に動作 — `cargo test -p ruviform-observability --lib` が default features で pass |
| 重要状態変化のみ非 RT から標準 tracing へ接続・抑制上限 | 充足 | `DiagnosticsEvent::HealthTransition`（domain rollup 遷移のみ）→ `FindingSink` port + `SinkPolicy` 抑制（`sink_suppressed_total` 計数）+ 具象 `tracing_export::TracingFindingSink`（`tracing-export` feature、panic 隔離・`catch_unwind`）。adapter 側試験: `sink_emits_health_transition_with_stable_fields`/`sink_panic_is_isolated_and_counted`/`diagnostics_set_sink_accepts_tracing_sink` |
| Secret/Media/Gateway Grant 非流出・未認可 Report 拒否試験 | 充足 | closed `EvidencePayload`（任意バイト列を型で排除）+ confidentiality 超過 item の wholesale 除外 + `ReportError::Unauthorized` 拒否試験 |
| Domain Error との二重管理禁止・Plugin 既存 DiagRegistry 尊重 | 充足 | `DiagnosticCode` は domain 所有 `name`+`schema_version`（中央 enum なし）。`REPORT_NOTES`/module doc が domain `Error`/`Result`・`DiagRegistry` の authoritative 境界を明記 |
| 両 Consumer の Evidence/Health ユースケースを #281 で検証 | NotRun | cross-repo gate・別 Gate（#281）。本 slice 対象外 |

## Risks

- `MetricValue` が u64 ベースのため、health signal の `f64` 写像は
  概算になる（閾値評価用途としては十分だが、将来 float metric が
  入れば写像を拡張する余地あり）。
- Report の byte 推定は `size_of` + 可変長近似であり、実シリアライズ
  サイズとは一致しない（boundedness の保守的近似として文書化済み）。
- `catch_unwind` は panic 隔離の標準手段だが、sink が panic を
  繰り返す場合は `sink_panics_total` のみが記録される（再帰診断なし
  は設計どおり）。

## Follow-ups

- OBS-03（#278）: `FindingSink` → `tracing` adapter。
- OBS-11（#293）: Host 認可機構。
- #281: 両 Consumer での Evidence/Health 受入（cross-repo gate・NotRun）。
