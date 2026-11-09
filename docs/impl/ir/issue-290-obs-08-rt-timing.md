---
id: "IMPL-034UzejWEL4tv3LAZGmQKA"
type: "impl"
status: "completed"
title: "Issue #290 OBS-08 RT Timing"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/290"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-observability/src/rt_timing/mod.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/rt_timing/record.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/rt_timing/bridge.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/rt_timing/sink.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/rt_timing/tests.rs"
      - type: "file"
        path: "docs/specs/issue-290-observability-rt-timing.md"
      - type: "file"
        path: "docs/designs/issue-290-observability-rt-timing.md"
      - type: "file"
        path: "docs/test-specs/issue-290-observability-rt-timing-tests.md"
      - type: "file"
        path: "docs/plans/implement-issue-290-observability-rt-timing.md"
      - type: "file"
        path: "docs/tasks/implement-issue-290-observability-rt-timing.md"
    modified:
      - type: "file"
        path: "crates/ruviform-observability/src/lib.rs"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/implement-issue-290-observability-rt-timing.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-290-observability-rt-timing.md"
    - "docs/tasks/implement-issue-290-observability-rt-timing.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-290-observability-rt-timing.md"
    - "docs/designs/issue-290-observability-rt-timing.md"
    - "docs/plans/implement-issue-290-observability-rt-timing.md"
    - "docs/test-specs/issue-290-observability-rt-timing-tests.md"
    - "docs/impl/ir/issue-290-obs-08-allocation-probe.md"
    - "docs/impl/ir/issue-291-obs-09-diagnostics.md"
    - "docs/impl/ir/issue-292-obs-10-debug-inspection.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-290-observability-rt-timing.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/277"
    - "https://github.com/scarecrow173/ruviform/issues/280"
    - "https://github.com/scarecrow173/ruviform/issues/289"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #290 OBS-08 RT Timing

## Summary

`ruviform-observability` に Issue #290 OBS-08 の残件である RT Timing
slice を `rt_timing` モジュールとして実装した。事前準備で固定する
`TimingClock`、RT 側の `TimingRecorder`（OBS-05 `RecordHeader`/
`ClockContext`/`EventRecord` 語彙と OBS-04 `RtRecorder`/`RtBridge`
の再利用）、非 RT の `TimingSink`（min/max/mean・予算超過・clock
anomaly・generation・損失の集計と既存 `core::histogram` への分布記録）
を追加し、`TimingSnapshot` 経由で OBS-02/OBS-07 `MetricSource` として
`Collector` へ `MetricValue::Histogram` として露出する。外部依存・
`tracing` 依存の追加なし。tick→秒変換は行わず、duration は clock の
native tick のまま保持する。

## Context

- 実施タスク:
  docs/tasks/implement-issue-290-observability-rt-timing.md
- 上流: spec（要求・AC）・design（型構成・所有権・clock 固定・
  budget/anomaly/loss 意味論）・test-spec（R1–R10 証拠行列）・
  plan。
- 動機: Issue #290 の受入条件「RT Timing の Clock/overrun/Callback
  budget 意味論と Histogram の統合試験」が main 上で未実装だった。
  OBS-08 audit で唯一の未充足行であり、既存 #506（allocation
  probe）・#504・#508 の上に最小差分で補完する。

## Implementation

### Changes

- `src/rt_timing/record.rs` — `TIMING_SCHEMA_VERSION`、`TimingClock`
  （domain/timebase/provenance/epoch の固定 identity）、
  `TimingSample`（event/point/generation/budget_ticks/start/end）、
  `TimingRecord`（`RecordHeader` stamp + `TimingSample`、`Copy +
  Send + 'static`、128B 以下）。`duration_ticks()` は
  `end < start` を clock anomaly として `None` で返す。
- `src/rt_timing/bridge.rs` — `timing_bridge(capacity, clock, source,
  duration_boundaries)` が `RtBridge<TimingRecord>` と `TimingSink`
  を対で構築（構築は非 RT・alloc 可）。戻り値は
  `Result<_, HistogramError>`（不正 bucket 境界）、`capacity == 0`
  は `crate::rt_bridge::bridge` と同じ契約で panic。`TimingRecorder`
  は `record(sample) -> bool`・`recorded()`・`dropped()` のみ公開し、
  RT 側は const-header 構築 + bounded `try_push` で alloc/lock/
  panic なし。
- `src/rt_timing/sink.rs` — `TimingSink::poll() -> TimingWindow`
  （per-poll: records/samples/clock_anomalies/dropped/over_budget/
  min・max・mean_ticks/max_utilization_permille/generation 範囲・
  generation_transitions）。`TimingSnapshot` は累積の厳密 scalar
  （min/max/mean・sample 数・over_budget・max_utilization_permille・
  generation 範囲/遷移数）と `dropped_total`（bridge 権威の累積
  損失）、および `HistogramReader` を保持。`estimated_quantile` は
  `QuantileEstimate{lower_ticks, upper_ticks}` の bucket 区間推定を
  返す。`histogram_reader()`/`recorded_reader()`/`dropped_reader()`
  が OBS-02/OBS-07 `MetricSource` 登録用の reader を供給する。
- `src/rt_timing/mod.rs` — module doc（RT/非 RT 境界・意味論）。
- `src/rt_timing/tests.rs` — R1–R10 対応の 21 試験（`#[cfg(test)]`
  の `inner_sink` 経由で bridged record の stamp/envelope も検査）。
- `src/lib.rs` — `pub mod rt_timing` 登録と crate doc への RT Timing
  段落追加。
- docs chain: spec/design/test-spec/plan/task/IR。

### Decisions During Implementation

- **clock identity は型で固定**: `TimingRecord` は bridge 内の
  `TimingRecorder` からのみ構築可能なため、foreign-clock record は
  公開 API 上到達不能。sink 側に防御的 clock 比較を置かず、bridge
  再構築で clock を変える設計とした。
- **duration は native tick のまま**: tick→秒/sample 変換・`ClockAnchor`
  換算は domain の責務として本 module に持ち込まない。budget も
  `budget_ticks` で同一単位比較のみ行う。
- **budget_ticks == 0 = 未 assert**: utilization/overrun 集計を
  skip し、`max_utilization_permille` は `None`（0% ではなく
  「budget なし」）。overrun は `duration > budget` のみ計上し、
  deadline overrun/XRUN/device scheduling とは区別する。
- **clock anomaly は分布に混ぜない**: `end < start` は
  `clock_anomalies` に計上し histogram/scalar duration から除外。
  ただし generation accounting は anomaly record も含めて行い、
  generation mixing を明示する。
- **損失の帰属**: drop-on-full の `dropped` は bridge counter と
  `dropped_before` positional marker で可視化し、carrier record が
  drain された poll window の `TimingWindow.dropped` に帰属させる。
  `dropped == 0` は完全性を証明しない（in-flight carrier が後で
  損失を帰属させ得る）ため、権威累積は `TimingSnapshot.dropped_total`
  とし、`dropped > 0` の分布を完全分布として提示しない契約を
  `TimingWindow` doc に明記した。
- **quantile は区間推定**: `core::histogram` の固定 bucket のみを根拠に
  `estimated_quantile`（`QuantileEstimate` 区間）を提供し、厳密
  percentile を装わない。
- **公開 enum 追加なし**: 境界値エラーは既存 `HistogramError` を
  そのまま返し、新規 `#[non_exhaustive]` enum を導入しない。

## Related Experiments

### Adopted

<!-- 該当なし: 実装方針は既存設計（rt_bridge/histogram/collector）の
再利用で確定しており、探索的 exp は作成しなかった。 -->

### Rejected

<!-- 該当なし。 -->

## Validation

canonical（mise、base `0d07471d` = 最新 origin/main merge 済み）:

- `mise run fmt-check` — PASS（exit 0、`cargo fmt --all -- --check`）。
- `mise run check` — PASS（exit 0、`cargo check --workspace
  --all-targets`）。
- `mise run lint` — PASS（exit 0、clippy `-D warnings` ゲート）。
- `mise run test` — PASS（exit 0、`cargo test --workspace`。
  初回は `ruviform-ui-test` lib test の rustc が
  `STATUS_STACK_BUFFER_OVERRUN`（OOM 系 flake、当該 crate は本変更と
  無関係）で失敗。`CARGO_BUILD_JOBS=4` で再実行して全緑）。
- `mise run test-rt` — PASS（exit 0、`cargo test --workspace rt_`。
  `rt_timing` 21 tests を含む）。
- `mise run test-msrv` — **不存在**: `mise.toml` に task なし
  （`test-ofx-msrv`/`test-ui-test-msrv` は他 crate 限定）。
  MSRV 実測は #223 lane の責務として NotRun 記録。

補助証拠（直接 cargo、canonical ではない）:

- `cargo check -p ruviform-observability` 成功、
  `cargo clippy -p ruviform-observability --all-targets` 警告 0、
  `cargo fmt -p ruviform-observability -- --check` クリーン、
  `cargo test -p ruviform-observability rt_timing` 21 passed /
  0 failed。

review-protocol: cooperative + adversarial の分離パスで実装 diff を
レビュー。候補は全て「構築上到達不能」または「文書化済み意味論」
（clock None fallback・`estimated_quantile(0)`・capacity==0 panic
契約・generation 初回非遷移・`dropped==0` 非完全性・mean 整数除算）
であり、CONFIRMED P0/P1 なし。blocker なし = 承認扱い。

## Risks

- `assert_no_alloc` は開発環境の counting allocator に依存し、RT 実機
  での jitter 保証を意味しない。実機 latency 証拠は #281 cross-repo
  gate 側の責務であり本 slice では NotRun。
- histogram quantile は bucket 分解能に依存する区間推定であり、厳密な
  percentile ではない。

## Follow-ups

- cross-repo gate（#281）・実 OS 行は本 PR の対象外（NotRun 記録）。
- #290 IR（issue-290-obs-08-allocation-probe）の残件欄にある
  CPU sampling procedure・Tracy constraints・consumer regression は
  本 slice の範囲外として引き続き open。
