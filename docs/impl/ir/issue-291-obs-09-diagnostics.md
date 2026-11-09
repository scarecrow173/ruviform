---
id: "IMPL-034UnDVtaBTUOTvc3aTj8m"
type: "impl"
status: "completed"
title: "Issue #291 OBS-09 Diagnostics 実装記録"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/291"
  changes:
    added:
      - "crates/ruviform-observability/src/diagnostics/"
    modified:
      - "crates/ruviform-observability/src/lib.rs"
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
