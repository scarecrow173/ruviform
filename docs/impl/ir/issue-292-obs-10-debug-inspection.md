---
id: "IMPL-034UqAG1z1lwcerRfda1x1"
type: "impl"
status: "in-progress"
title: "Issue #292 OBS-10 Debug Inspection 実装記録"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/292"
  changes:
    added:
      - "crates/ruviform-observability/src/debug/"
    modified:
      - "crates/ruviform-observability/src/lib.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/implement-issue-292-observability-debug-inspection.md"
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

# Issue #292 OBS-10 Debug Inspection 実装記録

## Summary

Issue #292（OBS-10）の `src/debug/` モジュールを実装する。
bounded な Inspection（budget/Truncated/Unavailable/Discontinuous/
Generation 整合）・Invariant 判定（Satisfied/Violated/Inconclusive）と
Diagnostics 接続・Debug Session 状態機械・disclosure 認可付き
Debug Report を非 RT で提供する。probe 依存は port trait 注入に
留め、concrete adapter は #293 / ruviform-audio 側の責務とする
（依存循環回避）。

## Context

`docs/tasks/implement-issue-292-observability-debug-inspection.md` の
実装スライス。上流: `docs/specs/issue-292-observability-debug-inspection.md`、
`docs/designs/issue-292-observability-debug-inspection.md`、
`docs/test-specs/issue-292-observability-debug-inspection-tests.md`。
消費する既存契約: OBS-05 `rt_bridge`（SPSC record transport）、
`record::`（`ClockContext`/`RecordHeader`/`EventRecord`/generation）、
OBS-07 `collector`/`metrics`（`MetricsSnapshot`/`Measurement`）、
OBS-09 `diagnostics`（`DiagnosticFinding`/`Evidence`/`DiagnosticReport`/
`Confidentiality` ranking）、OBS-02 `source`、#308/#309 の probe
vocabulary（`ruviform-media-core` の `ClockDomainId`/`ClockEpochId`）。

## Implementation

### Changes

- `crates/ruviform-observability/src/debug/` 新設（10 ファイル、
  `mod.rs` の公開 re-export を含む）。
  - `budget.rs`: `InspectionBudget`（`max_items`/`max_bytes`/optional
    whole-inspection `timeout`）、`InspectionDetail`
    （Summary/Standard/Verbose）、`InspectionTruncation`/
    `SectionTruncation`（drop は常に計数・silent truncation なし）。
    `max_items == 0` は意図的な no-entry mode であり budget drop
    とは区別する。
  - `probe.rs`: #185/#308/#309 の probe 契約を observability 側で
    再宣言した port vocabulary（`ProbeMode`/`ProbeAvailability`/
    `ProbeStopReason`/`ProbeUnobservedReason`/`ProbeClockMark`/
    `ProbeReading`/`ProbeDeckSnapshot`/`ProbeSource`）。
    `ProbeSource` は object-safe port trait で、concrete adapter は
    `ruviform-audio`/#293 側が実装する（本 crate → audio 依存は
    循環になるため置かない）。
  - `request.rs`: `InspectionRequest`/`InspectionInputs`/
    `InspectionTarget`（Metrics/Diagnostics/Invariants/Session/
    Probes — 重複は canonicalize・dedup）。
  - `snapshot.rs`: `InspectionSnapshot`/`InspectionSection`/
    `SectionStatus`（Ok/Truncated/Unavailable{reason}/Withheld）/
    `UnavailableReason`/`InspectionConsistency`
    （Aligned/Skewed/Unknown — 別 generation domain は数値比較
    しない）/`ConsistencyLevel`/`INSPECTION_SCHEMA_VERSION`。
  - `invariant.rs`: `InvariantCheck`（domain/code/evaluate +
    declared diagnostic mapping）、`InvariantVerdict`/
    `InvariantOutcome`（Satisfied/Violated/Inconclusive の
    three-valued）、`InconclusiveReason`、`InvariantSummary`/
    `InvariantResult`。
  - `session.rs`: `DebugAuthority`/`SessionPermit`（Arc token の
    capability — 提示で消費）、`SessionRequest`、
    `DebugSessionState`（Disabled/Prepared/Active/Draining/
    Stopped/Abandoned）、`SessionCore` 状態機械、
    `SessionDrain`/`SessionRecord` port、`SessionDrainStatus`、
    単調増加の `drain_totals()`（generation 跨ぎで underflow
    しない累積 delivered/overflow）。
  - `report.rs`: `DebugReport`/`ReportRequest`/`RecordSummary`
    （payload を持たない metadata-only 要約）/`ReportTruncation`/
    `REPORT_SCHEMA_VERSION`。
  - `engine.rs`: `DebugConfig`/`DebugHealth`（固定カウンタ —
    自己観測のため source 登録しない）/`DebugInspector` 集約。
    inspect/report/publish_invariants/session ops/probe attach。
    panic は `catch_unwind` で invariant check と probe port に
    隔離し `inconclusive`/`Unavailable{PortFailure}` に写す。
  - `tests.rs`: 35 件の契約テスト。
- `crates/ruviform-observability/src/lib.rs`: `pub mod debug;`
  登録と crate doc への責務・非 RT 注記の追記。
- docs chain（spec/design/test-spec/plan/task/ir）と各 index README。

### Decisions During Implementation

- Section 単位の truncation は snapshot-global ledger の delta で
  計上する（先行 section の drop を後続 section に誤計上しない）。
- Probe deck 収集中に deadline が来ても、収集済みの部分 deck は
  破棄せず保持して `Truncated` とする（admitted entry の silent
  loss を防ぐ）。残りの未評価 deck は reading 単位（
  `deck.probes.len()` 合計）で drop を計数する。
- `max_items == 0` は「意図的に entry を入れない」明示モードで
  あり、budget 超過 drop とは区別する（`items_dropped` に
  計上しない）。
- Session drain の累積計数は `SessionCore` 生存期間全体の
  `delivered_total`/`overflow_total` で保持し、`drain_totals()`
  経由で generation 跨ぎでも単調増加を保証する（delta が
  underflow しない）。
- Drain timeout は `deadline_exceeded` を立てて報告するが
  `Stopped` を強制しない（quiesce しない drain port を
  偽って完了扱いしない）。
- Missing input は空の成功 section ではなく
  `Unavailable{MissingInput}` を返す。probe port 未接続は
  `Unavailable{PortAbsent}`、port panic/失敗は
  `Unavailable{PortFailure}`。
- Consistency は section 成功とは独立に raw generation/clock
  stamp から計算し、probe clock mark が無い場合は fail-closed で
  `Unknown` にする。
- Report の認可は `max_export_disclosure` + request disclosure
  の policy parameter でモデル化し、超過は `Unauthorized`。
  record は payload を持たない `RecordSummary`（identity/
  classification/loss metadata のみ）に限定し、secrets/media
  を構造的に排除する。
- `DebugHealth` は固定カウンタのみで `Source` 登録しない
  （自己観測の再帰を防ぐ — OBS-09 と同じ方針）。
- Public enum は全て `#[non_exhaustive]`（公開 enum ポリシー）。

## Related Experiments

### Adopted

（なし）

### Rejected

（なし）

## Validation

- `mise run fmt-check` — PASS（exit 0、`cargo fmt --all -- --check`）。
- `mise run check` — PASS（exit 0、`cargo check --workspace
  --all-targets`）。
- `mise run lint` — PASS（exit 0、clippy `-D warnings` ゲート。
  初回 19 件の指摘 — unused_self/single_match_else/
  manual_let_else/must_use/needless_pass_by_value/collapsible_if/
  needless_lifetimes/field_reassign_with_default/assert_is_empty/
  redundant_closure — を全て修正して再実行で緑）。
- `mise run test` — PASS（exit 0、`cargo test --workspace`。
  debug 39 テストを含む全クレート緑）。
- `mise run test-rt` — PASS（exit 0、`cargo test --workspace rt_`）。
- 補助証拠（canonical ではない）: `cargo test -p
  ruviform-observability --lib debug` 39 件。
- review-protocol: 実装レビュー 1 回（cooperative + adversarial
  の分離パス、Validator=証拠確認、Judge=重要度判定）。CONFIRMED
  を 4 件検出・全て修正:
  1. P1 `ExportLimits::max_items == 0` が文書の「summaries only
     (no record entries)」に反して無制限収集していた → 0 の場合は
     record 収集自体を skip（deliberate mode、drop 計数なし）。
     回帰テスト `report_max_items_zero_is_summaries_only`。
  2. P1 `DiagnosticReport::source_generation == 0`（never
     evaluated sentinel）を実 generation として比較し、評価済み
     入力との間に偽の Skewed を生んでいた → 0 は absent 扱い。
     回帰テスト `unevaluated_diagnostics_report_is_not_a_generation`。
  3. P2 consistency で Unknown（probe clock 不明）が確定済み
     Skewed（collector generation 不一致）を上書きし得た →
     Skewed > Unknown > Aligned の優先順位に修正（OBS-09 の
     「Unknown は known-bad を消さない」と同じ規則）。回帰テスト
     `confirmed_skew_outranks_unknown_probe_clock`。
  4. P2 `DiagnosticsInspection::findings_total` が domain-health
     entry 数を含んで水増しされていた → findings のみ計数に修正。
     回帰テスト `diagnostics_findings_total_excludes_domain_health`。
  P3 non-blocking（報告のみ）: probe source の部分失敗は
  `probe_source_failures_total` にのみ記録され snapshot には
  現れない（全滅時のみ `Unavailable{PortFailure}`）；deadline で
  Unavailable になった Session section は `state: Disabled` の
  placeholder を持つ（status=Unavailable が payload 非信頼を
  示す）；`publish_invariants` は budget で採用された result のみ
  publish する（health counter は評価全件を計数）。blocker なし
  = 承認扱い。

## Risks

- `InspectionBudget.timeout` は `Instant` deadline の cooperative
  check であり、preemptive ではない（budget 超過の hard guarantee
  ではなく bounded-effort の近似 — 設計どおり文書化済み）。
- `ProbeSource` port の concrete 実装は #293 / ruviform-audio 側に
  委譲されるため、本 slice では port 契約と mock による検証に
  留まる（実 probe adapter との結合は NotRun）。
- `catch_unwind` は panic 隔離の標準手段だが、panic = abort 構成
  では機能しない（workspace は unwind 前提）。
- Session permit は Arc token の capability であり、暗号学的認可
  ではない（host 側の認可機構は #293 の責務）。

## Follow-ups

- #293 OBS-11: concrete probe adapter（`ruviform-audio` 側）と
  Host 統合・外部認可。
- #185: Probe OFF/Monitor/Capture 本体（本 module は port 接続のみ）。
- #281: AudioLens/Synvelis 両 Consumer での互換性確認
  （cross-repo gate・NotRun）。
- synvelis#81: Synvelis 側 Debug 能力連携。
