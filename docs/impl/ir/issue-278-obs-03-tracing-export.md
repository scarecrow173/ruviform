---
id: "IMPL-034Upz8rpFOqIgQe87GGYp"
type: "impl"
status: "completed"
title: "Issue #278 OBS-03 tracing_export 実装記録"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/278"
  changes:
    added:
      - type: "directory"
        path: "crates/ruviform-observability/src/tracing_export/"
    modified:
      - type: "file"
        path: "crates/ruviform-observability/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-observability/src/lib.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/implement-issue-278-observability-tracing-export.md"
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
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/293"
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #278 OBS-03 tracing_export 実装記録

## Summary

Issue #278（OBS-03）のうち `ObservationBatch` → `tracing` event
Exporter と `DiagnosticsEvent` → `tracing` event の `FindingSink`
adapter のスライスを `src/tracing_export/` に実装する。feature
`tracing-export`（既定 OFF・`logging` とは独立）で gate し、host が
`tracing::Dispatch` を注入する。Collector/Diagnostics 正本は
`tracing` 非依存を維持する。

## Context

`docs/tasks/implement-issue-278-observability-tracing-export.md` の
実装スライス。上流: `docs/specs|designs|test-specs|
plans/issue-278-observability-tracing-export*`。消費する既存契約:
OBS-07 `ObservationBatch`/`Measurement`/`MetricValue`/
`MeasurementValidity`/`MeasurementIssue`/`ObservationWindow`、
OBS-09 `FindingSink`/`DiagnosticsEvent`/`HealthState`、OBS-02
`SourceIdentity`/`ReportingHint`/`ReportingImpact`/`MetricKind`/
`ObservationUnit`/`ObservationScope`/`SourceLifecycle`。

## Implementation

### Changes

- `crates/ruviform-observability/Cargo.toml`: `tracing-export =
  ["dep:tracing"]` feature 追加（既定 OFF・`logging` と独立）と、
  試験用 `tracing-subscriber` dev-dependency 追加のみ。新規外部
  依存なし（`tracing` optional dep は既存）。
- `crates/ruviform-observability/src/lib.rs`: feature ゲート付き
  `pub mod tracing_export;` と crate doc bullet の additive 変更
  のみ。
- `crates/ruviform-observability/src/tracing_export/`（新規）:
  - `mod.rs` — 公開 re-export（`TracingExporter`/`EmitStats`/
    `DEFAULT_MAX_EMISSIONS_PER_POLL`/`TracingFindingSink`）と
    module/RT/失敗分離 contract doc。
  - `schema.rs` — event 名（`metric_observation`/
    `observation_window`/`diagnostics.health_transition`）、
    `event_version = 1`、target `ruviform.observability`、enum→
    文字列写像、severity 写像、`*_mono_us` 相対時刻変換の単一正本。
  - `exporter.rs` — `TracingExporter`: `emit(&ObservationBatch)
    -> EmitStats`。`SnapshotOnly`/valueless/cap/policy 抑制を別計数、
    `OnIncrease` は kind 別 report-signal の baseline 差分 +
    `min_interval` pacing + 無変化 dedup で gate、`max_emissions_
    per_poll` cap、`batch.removed()` で policy state を解放、
    eligible work のある poll のみ window 集約 event（cap 損失は
    WARN 昇格）。`dispatcher::with_default` scoped dispatch。
  - `sink.rs` — `TracingFindingSink`: `FindingSink` 実装。
    `HealthTransition` を `to` state 由来 Level（Degraded→WARN /
    Unhealthy→ERROR / 他→INFO）で発行。`catch_unwind` で panic を
    隔離し `panics()` に計数（再帰ログなし）。
  - `tests.rs` — 15 件の contract/golden test。実 `Collector::poll`
    を使い JSON fmt layer + shared buffer で event を capture。
- docs チェーン: spec/design/test-spec/plan/task/impl-record。

### Decisions During Implementation

- **severity は `tracing::Level` の verbosity 順に注意**:
  `Level::Ord` は TRACE > DEBUG > INFO > WARN > ERROR（severity
  とは逆順）のため、「大きい方の severity」は `Ord::min` で取る。
  `tracing::event!` の callsite level は static 必須のため、計算
  した Level は DEBUG/INFO/WARN（sink は +ERROR）の callsite 分岐
  で発行する。
- **OnIncrease dedup（test-spec R-8）**: `min_delta == 0` /
  `min_interval == ZERO` でも無変化の再観測・同一 batch 再 emit で
  重複 event を出さないよう、「signal 増分なし かつ
  validity/issue/lifecycle 不変」を抑制条件に追加。signal が平坦な
  validity 劣化（Counter 減少時の `cumulative_since_registration`
  等）は遷移検出で拾い WARN floor を維持する。baseline/発行時に
  観測状態（validity/issue/lifecycle）も `SourcePolicy` に保持。
- **window event の発行条件**: SnapshotOnly でなく `value` を持つ
  measurement が 1 件以上あった poll のみ（design の eligible
  定義どおり）。全抑制 poll でも window は出るが cap 損失が
  観測可能。空・valueless・SnapshotOnly のみの poll では出さない。
- **時刻**: `Instant` は wall-clock 変換不可のため、exporter 初回
  `emit` の `window.started`（sink は初回 event の `at`）を t0 と
  した `*_mono_us` 相対 microsecond。捏造 wall-clock なし。
- **cap_dropped の baseline 非更新**: cap 超過分は state を進めず
  次 poll で再評価される（損失を pending のまま残す）。
- **panic の扱い（exporter）**: subscriber panic は catch せず
  呼出側へ伝播（host が subscriber を所有）。baseline は emit 前に
  更新するため panic 時に同一増分が二重報告されない（イベント
  1 件喪失と引換え — panic 時は host 側が既に異常）。sink は逆に
  `catch_unwind` で隔離・計数（engine を巻き込まないため）。

## Related Experiments

### Adopted

（なし）

### Rejected

（なし）

## Validation

canonical（`mise run`、merge `origin/main` = 7816a22a 取り込み後）:

- `mise run fmt-check` — exit 0
- `mise run check`（`cargo check --workspace --all-targets`）— exit 0
- `mise run lint`（`cargo clippy --workspace --all-targets --
  -D warnings`）— exit 0
- `mise run test`（`cargo test --workspace`）— exit 0
- `mise run test-rt`（`cargo test --workspace rt_`）— exit 0

補助証拠（canonical = mise、直接 cargo）:

- `cargo check -p ruviform-observability --features
  tracing-export` — exit 0
- `cargo test -p ruviform-observability --features
  tracing-export` — 156 unit + 8 doc tests 全 pass（`tracing_export`
  15 件を含む）
- `cargo check -p ruviform-observability`（feature OFF）— exit 0、
  `cargo tree -e normal` に `tracing` 非含有（`tracing-export` のみ
  `dep:tracing` を有効化）
- review-protocol: CONFIRMED P1×2 を修正済み — (1) `tracing::Level`
  の `Ord` が verbosity 順（TRACE 最大）で severity 比較に `max`
  が使えず全 Level が逆転していた → `min` に修正、(2) test-spec
  R-8 dedup 未充足（`min_delta=0`/`min_interval=0` で無変化 poll・
  同一 batch 再 emit が重複発行し得た）→ 無変化観測の抑制 gate を
  追加。P2（clippy pedantic・test fixture の lease 落とし）も修正。
  残 P3: exporter は subscriber panic を呼出側へ伝播（baseline 先行
  更新で二重報告は防止）、dedup fingerprint に `loss` を含めない
  （loss 変化は validity/issue 遷移に随伴する前提）。

### NotRun / Unsupported

- `EventRecord`→event 変換・`CorrelationBook` unlinked 処理・W3C
  Trace Context adapter（有効/不正/未信頼入力）— NotRun（#293
  runner 配線の責務）。
- Thread/async `tracing` Context（span）伝播の完全対応 — NotRun
  （本スライスは `dispatcher::with_default` scoped Dispatch のみ）。
- 両 Consumer（AudioLens/Synvelis）での同一 Key/Field/Severity・
  stdout 保護・Privacy 検証 — NotRun（#281 cross-repo gate）。
- RT Correlation token 寿命・Clock 不明/欠落/再生成の unlinked 処理
  — NotRun（#293/consumer gate）。
- 実 subscriber への end-to-end 出力検証（JSON/fmt）— 本スライスは
  capture `Layer` による field/level 検証まで。stdout 保護は
  #281 gate。

## Risks

- **exporter の subscriber panic は呼出側へ伝播する設計** — baseline を emit 前に更新するため panic 時の同一増分の二重報告は防げるが、イベント 1 件の喪失と引換え（panic 時は host 側が既に異常という前提。残 P3 として記録済み）
- **OnIncrease dedup の fingerprint に `loss` を含めない前提** — loss 変化は validity/issue 遷移に随伴するという前提に依存（残 P3。前提が崩れる入力では dedup が効かない可能性）
- **`*_mono_us` は exporter 初回 `emit`/sink 初回 event 基準の相対時刻** — wall-clock 捏造はしないが、絶対時刻との相関は consumer 側の責務
- **Thread/async span 伝播は scoped `Dispatch` のみ** — span context を跨ぐ event の親子関係は本スライスの範囲外

## Follow-ups

- Issue #293: `EventRecord`→event 変換・`CorrelationBook` unlinked 処理・W3C Trace Context adapter・RT Correlation token 寿命と Clock 不明/欠落/再生成の unlinked 処理（runner 配線の責務）
- Issue #281: 両 Consumer（AudioLens/Synvelis）での同一 Key/Field/Severity・stdout 保護・Privacy 検証（cross-repo gate）
- Thread/async `tracing` Context（span）伝播の完全対応と、実 subscriber への end-to-end 出力検証（JSON/fmt）は上記 NotRun 項目の続きとして別途対応
