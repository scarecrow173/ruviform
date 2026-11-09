---
id: "SPEC-f0JtG8wUPC8ziWo3EERrZ2"
type: "spec"
status: "approved"
title: "Issue #290 OBS-08 RT Timing — 事前準備・選択的記録の実行時間観測 slice"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/290"
    - "https://github.com/scarecrow173/ruviform/issues/275"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "https://github.com/scarecrow173/ruviform/issues/290"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-277-observability-core-source-contract.md"
    - "docs/specs/issue-280-rt-record-bridge-clock-correlation.md"
    - "docs/specs/issue-289-observability-collector-metrics.md"
    - "docs/specs/issue-290-observability-allocation-probe.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/foundation-observability-minimal-api.md"
    - "docs/designs/issue-290-observability-allocation-probe.md"
    - "docs/designs/rt-verification-lane-design.md"
    - "docs/adr/obs-05-observability-leaf-primitive-deps.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-277-observability-core-source-contract.md"
    - "docs/specs/issue-280-rt-record-bridge-clock-correlation.md"
    - "docs/specs/issue-289-observability-collector-metrics.md"
    - "docs/specs/issue-290-observability-allocation-probe.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-290-observability-rt-timing-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/275"
    - "https://github.com/scarecrow173/ruviform/issues/280"
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/289"
    - "https://github.com/scarecrow173/ruviform/issues/104"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/293"
  deferred-by: []
---

# Issue #290 OBS-08 RT Timing — 事前準備・選択的記録の実行時間観測 slice

## What Should Be Built

既存 `ruviform-observability` クレート内に `src/rt_timing/` モジュールを
新設し、Issue #290（OBS-08）の残件である **RT Timing slice** を提供する。
新規クレート・新規外部依存・`unsafe`・feature gate は追加しない
（常時コンパイルの語彙 + 既存 `rt_bridge`/`record`/`core::histogram`/
`source`/`metrics`/`collector` の再利用のみ）。

Issue 本文の RT Timing 要件:

- 既存 OBS-05（#280）の Clock Domain 語彙と RT-safe record/Counter を
  使い、処理 Block/Callback/Plugin/Graph の実行時間を**事前準備・
  選択的記録**で観測する。各 Sample に Span を作らない。
- 非 RT の OBS-07（#289）Collector が Histogram/Window/Deadline
  Utilization へ変換する。
- `duration / callback_budget` は**測定区間の Budget 使用率**であり、
  Deadline Overrun・XRUN 発生・Device Scheduling 遅延と同義ではない。
  Clock 基準・単位・Sample Rate/Buffer Duration/Generation・欠落を
  明示する。
- 正確な全件統計と固定 Bucket Histogram の推定 P50/P95/P99 を区別し、
  記録が失われた Window を通常分布と偽装しない。

具体的には:

- `TimingClock`: 記録準備時に固定する clock 同一性
  （`ClockDomainId` + `TimeBase` + `TimestampProvenance` +
  `ClockEpochId`）。tick の読み取り自体はドメイン側の責務
  （`MasterClock::tick`・sample 位置・backend `StreamInstant` 由来の
  tick 等、本モジュールは tick を発明も変換もしない）。
- `TimingSample`: 呼出側が構築する 1 計測（`point: EventId`・
  `generation: u64`・`budget_ticks: u64`・`start_tick`・`end_tick`）。
  `budget_ticks == 0` は「budget 未主張」を意味する。
- `TimingRecord`: `rt_bridge` を渡る固定長 `Copy` レコード。
  `RecordHeader`（OBS-05 語彙: event=point・version・source・
  generation・`ClockContext`（position = start_tick）・
  confidentiality/impact）+ `end_tick` + `budget_ticks`。
- `timing_bridge(capacity, clock, source, duration_boundaries)`:
  非 RT のみの構築点。`(TimingRecorder, TimingSink)` を返す。
- `TimingRecorder`: RT 側 endpoint。`record(sample) -> bool` は
  header 構築（const 純粋演算）+ `RtRecorder::try_record` のみで、
  alloc/lock/panic/subscriber を追加しない。drop-on-full の損失は
  `dropped` カウンタと `dropped_before` envelope で可視化される。
- `TimingSink`: 非 RT 側 endpoint。`poll() -> TimingWindow` で
  queue を排出し window 会計を返し、累積 `snapshot()`・
  `histogram_reader()`・`recorded_reader()`/`dropped_reader()` を
  提供する。duration 統計（count/min/max/mean）・over-budget 会計・
  clock anomaly・generation 会計・損失計数を保持し、
  duration_ticks を `core::histogram` の固定 bucket へ観測する。
  `HistogramReader` は OBS-02/OBS-07 の `MetricSource` として登録
  可能で、`MetricValue::Histogram` として Collector が読む。

## Why Now

Issue #290 の audit（本 PR の docs/IR 記録）で、allocation-probe
slice（PR #506）以外の crate 内完結項目のうち RT Timing のみが
未実装と判明した。CPU sampling 手順・Tracy 制約は設計書・rustdoc
で文書化済み、`alloc_counter` 共存判断・RT test lane 保持も済んで
いる。残る crate 内受入行は「RT Timing の Clock/overrun/Callback
budget 意味論と Histogram の統合試験」であり、#281 証拠・実 OS 実機
測定・両 Consumer 回帰は cross-repo gate として対象外（NotRun）。

## Users and Value

- Ruviform の audio engine / plugin host / graph runtime が、RT
  callback 内の処理時間を事前準備済みの recorder で選択的に記録し、
  非 RT 側で分布・budget 使用率・欠落を観測できる。
- AudioLens/Synvelis 等の Consumer は、得られた `Histogram`
  measurement と window 会計を解析・診断へ利用する（Consumer 側の
  組込みは #293/#281 の責務）。

## Scope

### In Scope

- `crates/ruviform-observability/src/rt_timing/` 新設（record・
  bridge endpoint・sink・window/snapshot 報告・契約試験）。
- `lib.rs` への module 登録と crate doc 追記。
- 本 slice の docs chain（spec/design/test-spec/plan/task/IR）と
  Issue #290/#291/#292 audit 証跡の記録。

### Out of Scope

- `#[global_allocator]` install・`profiling`（allocation-probe）・
  `diagnostics`・`debug`・`collector`・`rt_bridge`・`record` の既存
  実装変更（全て再利用のみ）。
- tick を読む clock adapter（`MasterClock`・`StreamInstant`・sample
  clock 等の domain 側実装）、Consumer への組込み、#293 Host 統合。
- CPU sampling・Tracy・samply 実機手順の検証、AudioLens/Synvelis
  回帰評価、#281 への証拠提示、Windows/Linux/macOS 実機測定
  （全て NotRun / cross-repo gate）。
- 正確な全件分布の保存（bounded memory 契約上、exact scalar 統計 +
  bucket 推定で代替し、推定であることを型で区別する）。

## Requirements

- RT 側 `record` は alloc/lock/panic/subprocess I/O/subscriber 呼出を
  追加しない（`assert_no_alloc` レーンで検証）。
- 記録は drop-on-full を隠さない: `dropped`/`dropped_before`/window
  `dropped` で損失を計数し、損失のある window を完全分布と偽装しない。
- `end_tick < start_tick`（clock anomaly）は通常 sample として
  分布に混ぜず計数する。
- clock 同一性（domain/base/provenance/epoch）は bridge ごとに
  `TimingClock` で固定され、全 record の `ClockContext` に刻まれる。
  `TimingRecord` は `TimingRecorder` のみが構築するため foreign
  clock の record は型レベルで到達不能とし、sink 側に防御的検査は
  置かない。clock の変更（epoch 更新・domain 切替）は新しい bridge
  構築で行う。
- `budget_ticks == 0` は budget 未主張。`duration > budget` は
  measured-interval の budget 超過であり、deadline overrun・XRUN・
  device scheduling 遅延とは別概念として文書化する。
- duration の単位は record が運ぶ `TimeBase` tick であり、本
  モジュールは tick→秒/サンプル変換を行わない（`ClockAnchor` の責務）。
- 公開 enum には `#[non_exhaustive]`（本 slice は新規公開 enum を
  最小限に留め、原則 struct/const で構成する）。

## Acceptance Criteria

- [ ] `TimingRecord` が OBS-05 `RecordHeader`/`ClockContext` 語彙を
  再利用し、point/generation/clock/budget を型で明示する
- [ ] RT `record` が no-alloc/no-lock/no-panic で drop-on-full を
  計数する（`assert_no_alloc` + bridge 損失試験）
- [ ] `end < start` の clock anomaly が分布を汚染せず計数される
- [ ] record の `ClockContext` が `TimingClock` の
  domain/base/provenance/epoch と `start_tick` を保持する
- [ ] `duration > budget` の over-budget 会計と budget 未主張
  （`budget_ticks == 0`）の区別が試験される
- [ ] `HistogramReader` を OBS-07 `Collector` の `MetricSource` と
  して登録し `MetricValue::Histogram` が得られる統合試験
- [ ] 損失のある window が `dropped` で識別でき、完全分布と
  偽装されない
- [ ] bucket 推定 quantile が区間推定として返り、exact 統計と
  区別される
- [ ] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` が PASS

## Deferred Design Concerns

- tick reader（domain clock の RT-safe な読み取り口）の concrete
  adapter — `ruviform-audio`/`ruviform-rt` 側の責務（#293 統合時）。
- Consumer（AudioLens/Synvelis）での計測点選定・budget 値決定・
  回帰評価 — cross-repo（#281/#293）。
- 実機での overhead 計測・OS/feature matrix — #281 gate。
