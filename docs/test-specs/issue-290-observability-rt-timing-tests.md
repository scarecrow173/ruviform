---
id: "TSPEC-1nETjBT2y9V4C04J9pwXer"
type: "test-spec"
status: "approved"
title: "Issue #290 OBS-08 RT Timing 試験仕様"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/290"
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
    - "docs/designs/issue-290-observability-rt-timing.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-290-observability-allocation-probe-tests.md"
    - "docs/test-specs/issue-289-observability-collector-metrics-tests.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-290-observability-rt-timing.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-290-observability-rt-timing.md"
    - "docs/designs/issue-290-observability-rt-timing.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/275"
    - "https://github.com/scarecrow173/ruviform/issues/280"
    - "https://github.com/scarecrow173/ruviform/issues/289"
    - "https://github.com/scarecrow173/ruviform/issues/281"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/293"
  deferred-by: []
---

# Issue #290 OBS-08 RT Timing 試験仕様

## Purpose

`src/rt_timing/` が Issue #290 の「RT Timing の Clock/overrun/
Callback budget 意味論と Histogram の統合試験」受入行を満たすことを
検証する。既存 `rt_bridge`・`record`・`core::histogram`・`collector`・
`metrics`・`source` の契約は変更せず再利用する。

## Feature

- `TimingClock`: 準備時固定の clock 同一性
  （domain/base/provenance/epoch）。
- `TimingSample`: 呼出側が構築する計測入力
  （point/generation/budget_ticks/start_tick/end_tick）。
- `TimingRecord`: bridge を渡る固定長 `Copy` レコード
  （`RecordHeader` + end_tick + budget_ticks、≤128 byte）。
- `timing_bridge` / `TimingRecorder` / `TimingSink`: 非 RT 構築・
  RT 記録・非 RT 集計の endpoint。
- `TimingWindow` / `TimingSnapshot` / `QuantileEstimate`: 報告。

## Rules

### R1. RT 側記録の無割当・無ロック・無 panic

- `TimingRecorder::record` は header 構築（const 純粋演算）+
  `RtRecorder::try_record` のみを行い、alloc/lock/panic/subscriber/
  I/O を追加しない。
- **検証**: `assert_no_alloc` ブロック内で `record` を連続呼出し、
  割当が発生しないことを確認する（既存 dev-dependency の
  `AllocDisabler` グローバルアロケータを使用）。

### R2. drop-on-full の損失可視化

- bridge 満杯時 `record` は `false` を返し、`dropped()` が増加する。
- 後続 record の `dropped_before` と sink の `dropped` 会計により、
  どの window がどれだけの record を失ったかが識別できる。
- **検証**: 小 capacity（例: 4）で capacity+N 件記録し、
  `recorded + dropped == 送信総数`、window/snapshot の `dropped`
  が失われた件数と一致することを確認する。

### R3. clock anomaly の分離

- `end_tick < start_tick` の record は `clock_anomalies` に計数され、
  duration 統計（samples/min/max/mean/histogram）に混入しない。
- **検証**: anomaly record を混ぜて poll し、`samples` が正常件数
  のみ、`clock_anomalies` が anomaly 件数と一致、histogram の総数が
  正常件数と一致することを確認する。

### R4. clock 同一性の固定と明示

- `TimingClock`（domain/base/provenance/epoch）は bridge 構築時に
  固定され、`TimingRecorder` が全 record の `ClockContext` に刻む。
- `TimingRecord` は `TimingRecorder` のみが構築するため foreign
  clock の record は到達不能 — sink に防御的検査を置かないことを
  設計として確認する（コードレビュー観点、試験では record の
  clock accessor が `TimingClock` と一致することを検証する）。
- clock 変更（epoch/domain 切替）は新しい bridge 構築で行う。
- **検証**: sink が poll した record の `header().clock()` が
  `TimingClock` の domain/base/provenance/epoch と一致し、
  `position() == start_tick` であることを確認する
  （`TimingRecord` の accessor 経由で record 内容を直接検査する
  テストを `rt_timing` module 内に置く）。

### R5. budget 意味論

- `budget_ticks == 0` → budget 未主張。over_budget/utilization 会計
  を行わず、`max_utilization_permille` は `None` のまま。
- `budget_ticks > 0` かつ `duration <= budget` → over_budget に
  カウントされず、utilization = duration*1000/budget（permille）。
- `budget_ticks > 0` かつ `duration > budget` → over_budget に
  カウントされ、max_utilization_permille が更新される。
- `duration / budget` は測定区間の budget 使用率であり、deadline
  overrun・XRUN・device scheduling 遅延とは別概念（文書で明示、
  コード上は utilization のみを提供）。
- **検証**: 3 ケース（未主張/内側/超過）を個別に試験する。

### R6. histogram 統合

- `TimingSink::histogram_reader()` が返す `HistogramReader` を
  OBS-02/OBS-07 の `MetricSource`（`HistogramSource`）として
  `Collector` に登録し、poll で `MetricValue::Histogram` が得られる。
- duration_ticks の分布が固定 bucket に観測される。
- **検証**: 既知の duration 列を記録→poll→histogram snapshot で
  bucket 加算が期待通りか確認。Collector 登録→`MetricValue::
  Histogram` 取得の統合試験を行う。

### R7. window の損失識別

- `TimingWindow.dropped > 0` は「この poll で消費した record の
  `dropped_before` 合計 > 0」= 損失の位置的証拠を意味し、分布
  field が「生存 sample のみ」を記述することが文書・型で明示される
  （完全分布と偽装しない）。
- `TimingWindow.dropped == 0` は「この window で損失が確認されな
  かった」だけであり、未着の担ぎ手（in-flight/pending drop）を
  否定しない — 権威ある累積損失は `TimingSnapshot.dropped_total`
  （bridge カウンタ）と `dropped_reader()` が担う。
- **検証**: drop を発生させた poll で `dropped` が `dropped_before`
  合計と一致し、`records == 実際に消費した件数`、
  `snapshot().dropped_total == bridge dropped()` であることを
  確認する。

### R8. quantile 推定と exact 統計の区別

- `TimingSnapshot::estimated_quantile(q)` は bucket 分解能の
  区間推定 `{lower_ticks, upper_ticks}` を返し、exact percentile
  ではないことが型・doc で明示される。
- exact 統計（count/min/max/mean）は全 sample から正確に計算される。
- **検証**: 既知分布で p50/p95/p99 の区間推定が bucket 境界に
  沿った妥当な区間を返すこと、sample 0 件で `None` を返すこと、
  `q_permille > 1000` で `None` を返すことを確認する。

### R9. generation 会計

- `TimingSample.generation` が snapshot の min/max/transitions に
  反映され、世代遷移が計数される。
- **検証**: 異なる generation の record を順に記録し、
  min/max/transitions が期待値と一致することを確認する。

### R10. レコード語彙の整合

- `TimingRecord` が `RecordHeader`（event=point, version=1,
  source, generation, ClockContext(position=start_tick),
  confidentiality, impact）を保持し、`size_of <= 128`。
- `ClockContext.position` が `start_tick` を保持することを
  accessor 経由で確認する。
- **検証**: record 構築後に sink が受け取る record の header
  field を直接検査（sink は private なので、`poll` 前に
  `RecordSink::try_recv` 的な直接経路があればそれを使う。なければ
  sink の内部テストとして `mod tests` 内で検査する）。

## Examples

```rust
// 準備（非 RT — 起動時/graph 構築時）
let clock = TimingClock::new(domain, base, provenance, epoch);
let source = SourceIdentity::new(SourceId::new(1).unwrap(),
                                 SourceGeneration::new(0));
let (mut rec, mut sink) = timing_bridge(
    256, clock, source, &[100, 500, 1000, 5000],
).unwrap();

// RT 側（callback 内 — alloc/lock/panic なし）
let start = read_tick();           // domain clock の読み取り
run_callback();
let end = read_tick();
rec.record(&TimingSample {
    point: EventId::new(7),
    generation: plan_generation,
    budget_ticks: callback_budget_ticks, // 0 = 未主張
    start_tick: start,
    end_tick: end,
});

// 非 RT 側
let window = sink.poll();
let snap = sink.snapshot();
let p99 = snap.estimated_quantile(990); // 区間推定
```

## Guarantees

- `TimingRecord: Copy + Send + 'static`、`size_of <= 128`。
- `TimingRecorder: Send`（`RtRecorder` 継承）、`!Sync`。
- `TimingSink` は非 RT のみ（alloc/lock 許容）。
- `timing_bridge` は `HistogramError` を返し得る（非 RT）。
- 全ての公開型は `Debug`。`TimingWindow`/`TimingSnapshot` は
  `Clone`（`PartialEq` は test 用に derive）。
- `unsafe` なし・新規外部依存なし・feature gate なし。

## Non-goals

- tick reader の concrete adapter（domain 側責務）。
- Consumer 組込み・budget 値決定・回帰評価（#281/#293）。
- 実機 overhead 計測・CPU sampling/Tracy 検証（#281）。
- 全件分布の保持（bounded memory 契約）。
- `tracing`/logging 連携（OBS-09 側の責務）。
- 既存 `rt_bridge`/`record`/`histogram`/`collector` の変更。

## Risk

- `assert_no_alloc` は test binary 全体の global allocator を
  計測するため、`record` 呼出しのみに絞ったブロックで検証する。
  `timing_bridge` 構築は alloc するため assert ブロック外で行う。
- `TimingSink`/`RecordSink` は bridge 経由でのみ record を受け取る
  ため、foreign clock record の再現は不要（型レベルで到達不能）。
  clock 同一性の検証は poll 済み record の accessor 検査で行う。
- `Histogram::observe` は「RT-safe 宣言はまだない」が alloc/lock
  なしで atomic counter 更新のみ。sink は非 RT 側なので問題ない。
