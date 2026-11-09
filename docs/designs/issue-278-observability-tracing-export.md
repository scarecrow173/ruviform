---
id: "DESIGN-034UprcXe8LevfKFoKevlN"
type: "design"
status: "approved"
title: "Issue #278 OBS-03 tracing_export: ObservationBatch Exporter + FindingSink adapter"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/278"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-278-observability-tracing-export.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-277-observability-core-source-contract.md"
    - "docs/specs/issue-289-observability-collector-metrics.md"
    - "docs/specs/issue-291-observability-diagnostics.md"
  blocks:
    - "docs/plans/implement-issue-278-observability-tracing-export.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/foundation-observability-minimal-api.md"
    - "docs/designs/issue-289-observability-collector-metrics.md"
    - "docs/designs/issue-291-observability-diagnostics.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-278-observability-tracing-export.md"
  derived-by:
    - "docs/test-specs/issue-278-observability-tracing-export-tests.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-278-observability-tracing-export-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/275"
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/293"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/293"
  deferred-by: []
---

# Issue #278 OBS-03 tracing_export: ObservationBatch Exporter + FindingSink adapter

## Context

`tracing_export` は #275 module map（`core → collector → diagnostics/debug/tracing_export`）
で定義された非 RT の Ruviform 固有変換層である。本スライスは OBS-07
（`ObservationBatch`）と OBS-09（`FindingSink`）の公開 API を消費し、
標準 `tracing` event への変換と固定 schema・severity 規則を確定する。
Collector は `tracing` に依存せず、統合は host/runner が
`TracingExporter::emit(&batch)` を呼ぶ形に限定する（`poll_and_emit`
は作らない）。

## Scope

### In Scope

- `src/tracing_export/` モジュール（`mod.rs`・`schema.rs`・
  `exporter.rs`・`sink.rs`・`tests.rs`）と feature
  `tracing-export = ["dep:tracing"]`、試験用 `tracing-subscriber`
  dev-dependency。
- `ObservationBatch` → `tracing` event の固定 schema 変換、
  `ReportingHint` 解釈（SnapshotOnly 抑制・OnIncrease の
  min_delta/min_interval/impact）、kind 別 report-signal による
  増分判定、poll 単位の window 集約 event。
- severity 写像（validity floor + impact の max、ERROR 不使用）、
  `max_emissions_per_poll` cap と `cap_dropped` 計数。
- `TracingFindingSink`（`FindingSink` 実装、HealthTransition →
  `tracing` event、OBS-09 severity 写像と整合）。
- Dispatch host 注入・`dispatcher::with_default` scoped 発行・
  契約試験。

### Out of Scope

- `EventRecord`/`RecordHeader` 変換・`CorrelationBook` unlinked・
  W3C Trace Context・RT correlation token・Thread/async context
  伝播・両 Consumer gate — NotRun として impl record/PR に記録
  （責務は #281/#293）。
- Collector への tracing 結合 API・global subscriber 強制・
  独立クレート・OTLP/metrics exporter。

## Components and Boundaries

- `tracing_export::schema` — event 名・`event_version`・固定 field
  key・severity 写像・mono time → `*_mono_us` 変換の単一正本。
- `tracing_export::exporter` — `TracingExporter { dispatch,
  policy-state: HashMap<SourceId, SourcePolicy> }`。
  `emit(&ObservationBatch) -> EmitStats`。pull 型・非 RT・alloc 可。
- `tracing_export::sink` — `TracingFindingSink { dispatch }`、
  `FindingSink` trait 実装。
- `tracing_export::mod` — 公開 re-export と crate doc。
- `logging` module は変更しない。`Dispatch` を host が両者に共有
  するだけで feature 間の依存は作らない。

## Data and Control Flow

1. host/runner が `Collector::poll(now)` で `ObservationBatch` を得る。
2. host が `tracing::Dispatch`（`logging` で構築したもの等）を
   `TracingExporter::new(dispatch)` に注入して `emit(&batch)` を呼ぶ。
3. Exporter は batch の各 `Measurement` について:
   - `reporting == SnapshotOnly` → skip（`snapshot_suppressed` 計数）。
   - `reporting == OnIncrease{min_delta,min_interval,impact}` →
     kind 別 report-signal（Counter: `cumulative_since_registration`
     / Gauge: `value` / Histogram: `total_approx`）の前回 baseline
     との差分 `delta` と前回発行からの経過 `elapsed` を評価。
     `delta >= min_delta` かつ `elapsed >= min_interval`（初観測は
     baseline 確立のみで emit しない）なら event 発行、発行時に
     baseline/発行時刻を更新。さらに「前回発行（または baseline）
     から signal 増分も validity/issue/lifecycle 遷移もない」観測は
     dedup として抑制する — `min_delta == 0`/`min_interval == ZERO`
     は「変化のある観測を全て報告」の意味であり、無変化 poll や
     同一 batch の再 emit で重複 event を出さない（test-spec R-8）。
     signal が平坦な validity 劣化（Counter 減少時の
     `cumulative_since_registration` 等）もこの遷移検出で拾う。
   - valueless measurement（`value == None`）→ event 化せず
     `valueless_skipped` 計数（validity/loss 情報は batch/snapshot
     側が保持）。
   - cap 超過 → `cap_dropped` 計数。
4. `dispatcher::with_default(&dispatch, || {
   tracing::event!(level, target:"ruviform.observability",
   event_code=..., ...) })` で scoped 発行。
5. 各 emit 後に poll 単位の window 集約 event
   （`event_code="observation_window"`、window の generation/
   started/ended/duration/counters）を発行するかどうかは設計で固定:
   eligible（`SnapshotOnly` でなく `value` を持つ）measurement が
   1 件以上あった poll のみ — 空 batch・全抑制 poll で event を
   量産しないが、cap 超過の損失は window event の `cap_dropped`
   field で観測可能にする。
6. `TracingFindingSink::emit(DiagnosticsEvent)` は engine から
   呼ばれ、`HealthTransition` を `tracing::event!` に写す。
   panic は `catch_unwind` で隔離し `sink_panics` を計数（engine 側
   `SinkPolicy` と二重防御。sink 自体も失敗を自己再帰ログしない）。

## Non-Functional Constraints

- nonRT: `emit`・sink `emit` は `HashMap`/alloc/`with_default` を伴う。
  RT path から呼ばない・tracing を呼ばない。doc に明記。
- 失敗分離: Exporter は `&ObservationBatch` を読むだけで batch/
  snapshot/Collector/Diagnostics state を変更しない。subscriber/
  filter/Writer の失敗は `LoggingHealth` 側の責務で Exporter は
  関与しない。`Dispatch` が捨てる場合（subscriber なし）は何もしない
  だけで panic しない。
- 依存: `tracing` のみ（optional dep 既存）。`tracing-subscriber` は
  dev-dependency のみ。`unsafe` なし。公開 enum は
  `#[non_exhaustive]`。

## Risks and Trade-offs

- **`Instant` wall-clock 変換不可**: `observed_at` は `Instant` で
  epoch anchor がない → `*_mono_us`（`Instant` の `duration_since`
  基準に依存しない生の相対表現は取れないため、exporter 内部の
  最初の観測を t0 とした相対 microsecond）とし、実時刻を捏造しない。
  `ObservationWindow.started/ended` も同様に相対値として記録する。
- **`min_interval` の時計**: `Instant` 同士の差で評価（monotonic、
  host clock domain 内で一貫）。
- **`with_default` scoped dispatch は thread-local**: async/task 間の
  span context 伝播は本スライスでは保証しない → NotRun 記録。
- **cap の決定性**: `measurements()` の順序は Collector の
  registry 順で決定的。cap 超過は後続を落とす（先勝ち）方針を固定。

## Deferred Design Concerns

- `EventRecord`→event・CorrelationBook・W3C Trace Context・RT token —
  #293/consumer gate。本スライスは `ObservationBatch` と
  `DiagnosticsEvent` のみ。
- Thread/async span context 伝播 — #293・#281。
- 両 Consumer 同一性検証・stdout/Privacy — #281。

## Verification Notes

- [ ] schema golden・baseline/delta/window/generation・severity・
  抑制/cap/損失計数・Dispatch 注入・sink 変換・feature OFF での
  非依存を contract test で検証（test-spec 参照）。
