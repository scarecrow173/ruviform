---
id: "IMPL-034UXQ38rN5TtHZ9AGp6EW"
type: "impl"
status: "completed"
title: "Issue #278 OBS-03 tracing_export acceptance verification slice"
created: "2026-11-10"
updated: "2026-11-10"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/278"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-278-obs-03-tracing-export-acceptance-verification-slice.md"
    modified:
      - type: "file"
        path: "crates/ruviform-observability/src/tracing_export/tests.rs"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-278-obs-03-tracing-export.md"
    - "docs/impl/ir/issue-278-obs-03-tracing-export-records.md"
    - "docs/test-specs/issue-278-observability-tracing-export-tests.md"
    - "docs/test-specs/issue-278-observability-tracing-export-records-tests.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/test-specs/issue-278-observability-tracing-export-records-tests.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-278-observability-tracing-export-tests.md"
    - "docs/test-specs/issue-278-observability-tracing-export-records-tests.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/278"
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/293"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/293"
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #278 OBS-03 tracing_export acceptance verification slice

## Summary

Issue #278（OBS-03）の残件確認スライス。main 入り済みの slice 1
（`ObservationBatch`/`DiagnosticsEvent` → tracing）と slice 2
（`EventRecord` → tracing・`CorrelationBook` unlinked・W3C
`TraceContext`）が Issue 受入 8 行をどこまで満たすかをコードと
試験で照合し、in-repo で検証可能な未充足分のみ contract test を
追加した。実装 API・schema・`event_version` は無変更。

## Context

前提 Issue #289（Collector/Metrics）・#293（ObservationRunner）は
close 済みで、`src/tracing_export/`（exporter/records/schema/sink/
trace_context/tests）と `integration` runner の選択的接続
（`ExportPolicy.dispatch` → `TracingExporter`、feature
`tracing-export` ゲート）は main に存在する。本スライスは新規
実装ではなく受入照合 + 残件試験の追加。

## Implementation

### Changes

- `crates/ruviform-observability/src/tracing_export/tests.rs`:
  contract test 2 件を追加（既存 test の fixture・helper を再利用、
  実装コード無変更）。
  - `rt_record_trace_context_is_metadata_not_correlation` — 構文上
    有効な任意の `traceparent` と expired correlation link を同時に
    持つ record で、trace field が付与されても
    `correlation_status="unlinked"`/`unlinked_reason="expired"`・
    span/parent 関係 field 非存在が維持されることを固定
    （test-spec R-7/R-8「権限と無関係な相関」の直接証拠）。
  - `rt_record_confidentiality_is_reported_not_enforced` —
    `Confidentiality::Internal`/`Confidential` の record が分類
    field を忠実に emit し value を秘匿しないことを固定
    （exporter は分類報告のみ、値秘匿は host の認可呼出し —
    test-spec Non-goals どおり）。
- docs: 本 IR のみ。spec/design/test-spec/plan/task の chain は
  既存のまま（契約・Rule の追加・変更なし）。

### Decisions During Implementation

- **照合の結果、API/schema 実装の追加は不要と判断**: 受入 8 行の
  うち crate 責務分は slice 1/2 と #293 runner 配線で充足済み。
  唯一の in-repo 証拠ギャップは「trace context が相関分類に影響
  しない」「confidentiality 分類の忠実報告」の直接試験欠落であり、
  既存 Rule（R-7/R-8/R-1）の範囲内で test のみ追加した。
- **thread/async `tracing` Context 伝播・両 Consumer 受入は
  追加実装しない**: approved design/test-spec が明示的に host 責務・
  #293/#281 gate へ deferred しており、本スライスの scope 外。
  `with_default` scoped dispatch が ambient/global dispatch を
  汚染しないことは `tests/integration.rs` の
  `injected_dispatch_gets_events_scoped_over_ambient` /
  `global_default_never_captures_injected_emissions` で検証済み。

## Acceptance mapping（Issue #278 受入 8 行）

| # | 受入行 | 判定 | 根拠 |
| --- | --- | --- | --- |
| 1 | Collector 境界が `ObservationBatch` のみ・`tracing` 非依存をコンパイル検証 | Pass | `TracingExporter::emit(&ObservationBatch)` が唯一の境界。default build の `cargo check`/`cargo tree -e normal` で `tracing` 非含有を確認（collector/metrics/source/record/rt_bridge/diagnostics に `tracing` import なし） |
| 2 | Counter baseline/delta/window/schema/Severity/Source Generation の Golden test | Pass | `counter_onincrease_emits_fixed_schema_event`（baseline 抑制・delta・window event・全固定 key・WARN severity・`source_generation`）他、gauge/histogram/discontinuity/snapshot_only/emission_cap |
| 3 | nonRT Dispatch 注入・Thread/async Context 伝播・Context 未設定でも安全 | Pass（crate 責務分）+ deferred | Dispatch 注入は全 test が `Dispatch` を明示注入。scoped span context は `rt_record_inside_scoped_span_sees_context`、未設定は `Dispatch::none()`/span 外 emit で検証。ambient/global 分離は `tests/integration.rs` の dispatch 3 test。thread/async task 間の完全伝播は approved design/test-spec が host 責務（#293 `Instrument`/明示 dispatch・#281 gate）へ deferred → NotRun |
| 4 | W3C Trace Context 有効/不正/未信頼入力・権限と無関係な相関を試験 | Pass | `trace_context` unit test（valid/ff/zero-id/長さ/hex/uppercase/余剰 field）+ `rt_record_trace_context_adds_fields`（valid→field・invalid→なし）+ 本スライス追加 `rt_record_trace_context_is_metadata_not_correlation`（valid traceparent が unlinked 分類・span 関係 field 非存在を変えない） |
| 5 | RT Correlation token 寿命・Clock 不明/欠落/再生成/不正時の unlinked 処理 | Pass | `rt_record_correlation_statuses_are_distinct`（none/linked/unlinked×expired・superseded_epoch・unknown_token）・`rt_record_token_lifecycle_tracks_book`（issue/expire/remint）・`rt_record_clock_unstamped_and_estimated`（unstamped sentinel・estimated/derived 区別） |
| 6 | Logging OFF/Filter suppression/Exporter failure/queue full で Metric 正本への影響なし | Pass | `filter_suppression_does_not_mutate_exporter_or_collector`・`panicking_subscriber_does_not_affect_collector`（collector 継続動作）・`emission_cap_drops_and_reports_loss`/`rt_record_cap_counts_and_classifies_everything`（cap=queue 上限の別損失計数）・`rt_record_dropped_before_raises_to_warn`（bridge ring full→`dropped_before`）・`dispatch_none_is_safe_without_subscriber`/`rt_record_dispatch_none_and_filter_are_safe`（Logging OFF 相当）。Sampling/Writer error は design 上 `LoggingHealth`（host pipeline）責務 |
| 7 | 両 Consumer で同じ Key/Field/Severity・stdout 保護・Privacy を検証 | 一部 Pass + #281 NotRun | 固定 schema（`schema.rs` 単一正本）で consumer 非依存に同一 key/field/severity を保証。stdout 保護: module は global default/subscriber 構築/stdout 書込を持たず、`global_default_never_captures_injected_emissions` が非干渉を検証。Privacy: 本スライス追加 `rt_record_confidentiality_is_reported_not_enforced` で Internal/Confidential 分類の忠実報告を固定（値秘匿は host 認可 — Non-goal）。両 Consumer 実機受入は #281 cross-repo gate → NotRun |
| 8 | `tracing-export` Feature だけで必要な標準依存・強制 Subscriber・独立クレートなし | Pass | `Cargo.toml`: `tracing-export = ["dep:tracing"]` のみ。`cargo tree -e normal --features tracing-export` は `tracing`/`tracing-core`/`tracing-attributes` のみで `tracing-subscriber`/`tracing-appender` 非含有。`set_global_default`/`init` 呼出なし（grep 確認）。module 内実装で独立クレートなし |

## Related Experiments

### Adopted

（なし）

### Rejected

（なし）

## Validation

canonical（`mise run`、worktree HEAD = origin/main `1dc235b9` +
本スライス差分）:

- `mise run fmt-check` — exit 0。
- `mise run check`（`cargo check --workspace --all-targets`）—
  exit 0。
- `mise run lint`（`cargo clippy --workspace --all-targets --
  -D warnings`）— exit 0。
- `mise run test`（`cargo test --workspace`）— exit 0。
- `mise run test-rt`（`cargo test --workspace rt_`）— exit 0。
- `mise run test-observability-msrv`（`mise exec rust@1.89.0 --
  cargo test -p ruviform-observability --all-features`）— exit 0
  （lib 290 + integration 15 + logging 28 + doctest 15 全 pass、
  追加 2 test を含む）。

補助証拠（canonical = mise、直接 cargo・feature 組合せ）:

- `cargo check -p ruviform-observability`（default features）—
  exit 0。`cargo tree -e normal` に `tracing` 非含有（default
  build で Collector/Metrics/Source/Record/RT bridge/Diagnostics は
  `tracing` 非依存 — 受入行 1 のコンパイル検証）。
- `cargo check -p ruviform-observability --features
  tracing-export` / `--all-features` — ともに exit 0。
- `cargo tree -e normal --features tracing-export` — `tracing`/
  `tracing-core`/`tracing-attributes` のみ。`tracing-subscriber`/
  `tracing-appender` は normal dep に出ない（受入行 8:
  `dep:tracing` のみ・強制 Subscriber なし・独立クレートなし）。
- `cargo test -p ruviform-observability --features
  tracing-export` — lib 235 + doctest 11 全 pass（`tracing_export`
  34 tests、追加 2 件を含む）。
- `cargo test -p ruviform-observability --features "integration
  tracing-export" --test integration` — 12 tests 全 pass
  （`injected_dispatch_gets_events_scoped_over_ambient` /
  `global_default_never_captures_injected_emissions` /
  `dispatch_none_is_a_valid_noop_export` を含む — #293 runner の
  選択的接続契約を確認、二重実装なし）。
- `doc_status lint` — 本 IR 起因の finding なし（repo 全体の
  既存 finding は本スライス範囲外）。

## Risks

- 追加した 2 test は既存 Rule の直接証拠であり契約変更ではないが、
  test-spec の Examples には列挙されていない組合せケース。

## Follow-ups

- thread/async task 間の `tracing` Context 完全伝播 — host の
  `Instrument`/明示 dispatch 責務（#293 runner 契約・#281 gate、
  NotRun）。
- 両 Consumer（AudioLens/Synvelis）での同一 Key/Field/Severity・
  stdout 保護・Privacy の実機受入 — #281 cross-repo gate（NotRun）。
- `metric_observation`/`observation_window`/
  `diagnostics.health_transition` への trace field 付与 —
  `event_version` bump とセットの別 slice（slice 2 IR より継続
  Deferred）。
