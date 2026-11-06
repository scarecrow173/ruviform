---
id: "TSPEC-034T2NeVNZOhwkPaIzj0Q7"
type: "test-spec"
status: "approved"
title: "Issue #289 OBS-07 Collector/Metrics/Snapshot 試験仕様"
created: "2026-11-06"
updated: "2026-11-06"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/289"
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
    - "docs/designs/issue-289-observability-collector-metrics.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-277-observability-core-source-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-289-observability-collector-metrics.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-289-observability-collector-metrics.md"
    - "docs/designs/issue-289-observability-collector-metrics.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/277"
  defers: []
  deferred-by: []
---

# Issue #289 OBS-07 Collector/Metrics/Snapshot 試験仕様

## Purpose

Issue #289（OBS-07）で実装する `Collector`/`ObservationBatch`/`MetricsSnapshot`/
`CollectorHealth`/有界履歴の**契約**を、実装に依存しない入出力・状態遷移・
失敗モードの観点で固定する。対象は spec AC-1〜AC-9 と design の全 contract
（Baseline/Rate/不連続/drain/lifecycle/履歴/cardinality）。本試験は実装の
公開 API を通じた振る舞い検証であり、将来 API 面が変わらない限り維持する。

試験コードは crate 内 unit test（`src/collector/tests.rs` 等）に置く。
`#[cfg(test)]` のみで公開される test 補助（cell constructor 等）を使う
ケースは unit test である必要がある（`tests/` 統合試験からは到達不能）。

## Feature

Counter・Gauge・Histogram を登録した domain の source が、単一 poll owner
の `poll(now)` を通じて Baseline・実測 Delta/Rate・Validity・有界履歴・
Snapshot・Health を持つ型付き観測結果として提供されること。

## Rules

- R-1（登録契約・AC-1）: `Collector::registration_port()` は
  `ObservationRegistrar` を実装し、`register(MetricSource)` が
  `Result<(SourceIdentity, RegistrationLease, Arc<SourceState>),
  RegistrationError>` を返す。`register_counter` は
  `register` + `Counter` を束ねる補助。`RegistrarStopped`/`Draining`/
  `CapacityExceeded`/`KeyConflict`/`SourceReused`/`FinalDrainInProgress`/
  `InvalidDescriptor` は型付き Error で返る。
- R-2（単一 poll owner・AC-2/6）: `poll` は `&mut self` でしか呼べず、
  `Collector` は `Clone`/`Copy` ではない（`Shared`→`Collector` の構築は
  `Collector::new` のみ）。`CollectorHandle`/`CollectorRegistrar`/
  `PollRunner` から `poll` を呼ぶ API は存在しない。
- R-3（Baseline・AC-2）: 登録時に `Instant::now()` で `baseline_raw`・
  `registered_at` を確立し、登録前の累積は最初の `delta` に含めない。
- R-4（実測 Rate・AC-2）: `rate_per_sec` は `delta /
  checked_duration_since(anchor)`（初回 anchor=`registered_at`、以降は
  前回 `observed_at`）。elapsed が `None`（now < anchor）または 0 は
  rate/delta を `None` + `issue: ClockAnomaly` + `clock_anomalies_total` 計数。
  panic・`inf`/`NaN` は発生しない。
- R-5（不連続・AC-3）: `raw < prev_raw` → `Discontinuous` + `issue:
  CounterDecreased` + `prev_raw=raw` 再基準化。delta/rate/cumulative は
  `None`/据え置き。誤った巨大 delta を出さない。
- R-6（再登録・AC-3/4）: Removed 済み key の再登録は新 `SourceId` +
  `generation_seen+1` で受理（同 kind）。kind が変わる再登録は
  `InvalidDescriptor`。
- R-7（Gauge・AC-4）: `Gauge` の `0` は `Valid{value:0}`、Removed 済みは
  `Unavailable{value:None}` で型区別。`previous` は直前観測値。
- R-8（Histogram・AC-4）: 固定 bucket は `descriptor.buckets` と
  `reader.boundaries()` の**値完全一致**を登録時検査。読取は per-bucket
  非同時であり `total_approx` は近似として型付け。観測時不整合は
  `Partial` + `HistogramInconsistent`。
- R-9（Window・AC-4）: `window.started` は前回 `ended`（世代 1 は
  `created_at`）。`measured = ended-started`。`measured > expected` で
  `overdue_polls_total` 計数。`clock` は `ClockDomainId::MONOTONIC`。
- R-10（Stale・AC-4）: `stale_after: Some(d)` で `now-last_change_at > d`
  の Active source を `Stale`（値は保持）。`None` 既定は Stale 付与なし。
- R-11（drain・AC-3/7）: `begin_final_drain`→最終 read→`mark_removed` の
  順で最終値を commit。domain `retire()` 由来は `Valid`。collector 強制
  drain（`request_retire()` 実行）または `is_abandoned`（lease drop）由来は
  `Partial` + `WriterNotQuiesced`。
- R-12（mid-poll retire）: `lease.retire()` は lock-free CAS であり
  enumerate 後・commit 前に割り込み得る。commit 時に `state.lifecycle()`
  を再読し `Retiring` を観測した source はその poll で drain する。
- R-13（cardinality・AC-1/6）: key は bare `&'static str` の一意 claim
  （scope は含めない）。live claim への重複登録は `KeyConflict`。
  同一 cell の `shared_cell_ptr()` 一致は `SourceReused`（claim は live
  登録中のみ保持）。`max_sources`/`max_series`/`max_batch_size` を超える
  登録/poll 選択は `CapacityExceeded` または skip。
- R-14（有界履歴・AC-6）: `history_max_samples`/`history_budget_bytes` の
  FIFO ring。`history_*==0` は履歴無効（error ではない）。単一 entry が
  `history_budget_bytes` を超える場合は履歴に格納せず
  `history_oversized_dropped_total` を計数するが batch/snapshot には出る。
- R-15（世代共有・AC-2/6）: `poll` は `Result<ObservationBatch, PollError>`
  を返し、batch は `Arc` 共有。`latest_batch()`/`snapshot()` で複数
  購読者が同一世代を読める。終端 drain（shutdown/drop）も 1 世代を
  mint するため `Stopped` 後の `latest_batch` は terminal batch
  （drain measurement + `removed` + post-drain snapshot。drain 対象
  ゼロでも空 batch）を返し、`snapshot()` 世代と一致する。最終
  delta/Removed 事象の回収正本は terminal batch であり history
  容量（`history_*==0`）に依存しない。世代 gap は `generation`
  差で自己診断。
- R-16（shutdown・AC-7）: `shutdown()` は冪等。CAS 勝者のみ drain を
  実行し全 caller が `Stopped` まで待機して返る。`Draining` 中の登録は
  `FinalDrainInProgress`、poll は `CollectorStopped`。`Stopped` 後の登録は
  `RegistrarStopped`。drain 全件は 1 世代の terminal
  `ObservationBatch` として commit され `latest_batch`/`snapshot`
  に公開される（history 容量ゼロでも回収可能）。
- R-17（公平性・AC-6）: `max_batch_size` より多い Active source は
  round-robin で均等に poll される（cursor 移動で偏りなし）。
- R-18（RT 契約・AC-8）: Collector 実装は `Counter::increment` 等 RT
  writer の API・性能・ビルドを変更しない（既存試験は無改変で pass）。
- R-19（Logging 非依存・AC-5/9）: `logging` feature OFF・外部依存ゼロで
  `poll`/`snapshot`/`latest_batch`/`health`/`history`/`series` が全て
  動作する。Health は source に登録しない（再帰しない）。
- R-20（Error 型付き・AC-1/2/7）: `CollectorConfigError`・`PollError`・
  `MeasurementIssue`・`RegistrationError` は `#[non_exhaustive]` で列挙
  され、内部失敗を `None`/0/暗黙変換で表さない。

## Examples

### E-1 Baseline + Rate（R-3/R-4・AC-2）

Given: `Counter` を `c.increment()=5` して登録し `baseline_raw=5`、
`registered_at=T0` とする。さらに `c.increment()=7` で `raw=12`。
When: `now=T0+10ms` で `poll`。
Then: `Measurement.value = Counter{raw:12, cumulative_since_registration:7,
delta:Some(7), rate_per_sec:Some(700.0)}`（近似）。`validity=Valid`。
事前の 5 は delta に含まれない。

### E-2 Counter 減少（R-5・AC-3）

Given: `prev_raw=100` を登録済み counter。cfg(test) cell 補助で cell を
`store(40)` に減少させる。
When: `poll`。
Then: `validity=Discontinuous`・`issue=Some(CounterDecreased)`・
`delta=None`・`rate_per_sec=None`・`prev_raw=40`・
`cumulative_since_registration` は据え置き。

### E-3 clean retire（R-11・AC-3/7）

Given: `Counter` を登録して `raw=42` にする。
When: `lease.retire()` → `poll`。
Then: `removed` にその `SourceIdentity`。measurement の `lifecycle=
Removed`・`validity=Valid`・`value` は最終 raw=42・`rate_per_sec` は
実測幅で算出される。snapshot では `Unavailable`。

### E-4 lease drop / abandoned（R-11・AC-3/7）

Given: `Counter` 登録し lease を `retire()` せず `drop`。
When: `poll`。
Then: drain で `Partial` + `issue: WriterNotQuiesced`。`is_abandoned`
のため clean ではない。

### E-5 Gauge 0 vs Unavailable（R-7・AC-4）

Given: `Gauge` を登録し `set(0)`。
When: `poll`。
Then: `validity=Valid`・`value=Some(Gauge{value:0, previous:None})`。
`lease.retire()`→`poll` 後は `Unavailable`・`value=None`。

### E-6 Histogram 登録値一致（R-8・AC-4）

Given: `histogram(&[10,100])` を構築し `descriptor.buckets=Some(&[50,500])`
で登録。
When: `register`。
Then: `Err(InvalidDescriptor)`（本数は一致するが値が不一致）。

### E-7 window 遅延（R-9・AC-4）

Given: `poll_interval=100ms` で `poll(t=0)` 後、`poll(t=250ms)`。
Then: `window.measured≈250ms`・`window.expected=100ms`・
`overdue_polls_total` が 1 増加。

### E-8 elapsed=0 / 過去 now（R-4・AC-2）

Given: 同じ `Instant`（または anchor より過去の `Instant`）で連続 `poll`。
When: `poll(now)` で `checked_duration_since` が `None`/`0`。
Then: `rate_per_sec=None`・`issue=ClockAnomaly`・`clock_anomalies_total`
が増える。panic しない。

### E-9 同時 poll 型保証（R-2・AC-6）

When: `collector.handle()`/`collector.registration_port()`/
`collector.into_poll_runner().handle()` を見る。
Then: いずれにも `poll` 呼出 API が存在しない（型レベルで呼べない。

### E-10 batch 上限 round-robin（R-17・AC-6）

Given: `max_batch_size=2` で 5 source を登録。
When: 連続 5 poll。
Then: 各 source が約 1 回ずつ poll され `skipped` が均等に分布し、
いずれの source も飢餓しない。

### E-11 履歴 eviction（R-14・AC-6）

Given: `history_max_samples=2` で 3 poll 分の measurement を commit。
When: `handle.history()`。
Then: 直近 2 件のみ・`history_evicted_total≥1`・最古が pop されている。

### E-12 oversized entry（R-14・AC-6）

Given: `history_budget_bytes=16`（小）で Histogram を登録し observe。
When: `poll`。
Then: `history_oversized_dropped_total≥1`・`handle.history()` に当該
entry がない・batch/snapshot の measurement には存在する。

### E-13 世代 gap（R-15・AC-6）

Given: `poll` を 2 回呼び `latest_batch()` で batch を取る。
When: `batch1.generation=1`、`batch2.generation=2`。
Then: `latest_batch()` は新しい方を返し、`snapshot()` の世代と一致。
`Stopped` 後も最後の batch を返す。

### E-14 shutdown drain（R-16・AC-7）

Given: Active source を 2 件持つ collector。
When: `handle.shutdown()`。
Then: `health().state=Stopped`・`register` → `RegistrarStopped`・
`poll` → `Err(CollectorStopped)`・`latest_batch()` は新世代の
terminal batch を返し drain 対象の最終 measurement（強制 drain 分は
`Partial`+`WriterNotQuiesced`）と全 `removed` identity を保持、
`snapshot()` 世代と一致する。history 有効時は履歴にも残る。

### E-14b terminal batch — history 容量ゼロ（R-15/16・AC-7）

Given: `history_max_samples=0` かつ `history_budget_bytes=0` の
collector に Counter を登録し `poll` で baseline を取る。
When: 追加 increment → `lease.retire()`（または retire せず強制）→
`shutdown()`。
Then: `latest_batch()` は最終 poll 世代 +1 の terminal batch で、
`removed` に identity・`measurements` に `lifecycle: Removed` の最終
値（domain retire なら `Valid`・強制なら `Partial`+
`WriterNotQuiesced`）と post-poll delta を保持。`snapshot()` は同
世代の tombstone。`series()`/`history()` は空でも最終 delta は
batch から回収できる。drain 対象ゼロの shutdown でも空の terminal
batch（世代+1）が mint される。`PollRunner::stop` 経路でも同じ
terminal batch が公開される。

### E-15 key kind 付替拒否（R-13・AC-1）

Given: key `"x"` を `Counter` で登録→retire→Removed。
When: 同 key `"x"` を `Gauge` で再登録。
Then: `Err(InvalidDescriptor)`。同 kind なら新 `SourceId` +
`generation=1` で受理。

### E-16 Logging OFF（R-19・AC-5）

Given: `--no-default-features`（`logging` OFF）でビルド。
Then: `poll`/`snapshot`/`latest_batch`/`health`/`history`/`series`/
`registration_port` が全て compile+pass。`CollectorHealth` が `state`
を含む値を返す。

### E-17 poison 回復（R-16/R-20）

Given: 内部で `Mutex` が poison する経路（実装上の bug 注入試験が可能
なら）。
When: その後 `health()`/`poll` を呼ぶ。
Then: panic で伝播せず `poison_recoveries_total` が増加する。

### E-18 `buckets:Some` on 非 Histogram（R-8/AC-1）

Given: `Counter`/`Gauge` で `descriptor.buckets=Some(&[..])`。
When: `register`。
Then: `Err(InvalidDescriptor)`（意味を持たない指定は拒否）。

## Guarantees

- 全 measurement は `value: Option<MetricValue>` であり `None` は
  `Unavailable`/`Unsupported`/`Discontinuous` のみ（`0` 偽装しない）。
- `MeasurementIssue`・`PollError`・`CollectorConfigError`・
  `RegistrationError` は `#[non_exhaustive]` で列挙され、失敗理由は
  型で表現される。
- 1 poll の `ObservationBatch`/`MetricsSnapshot` は `Arc` 共有で複数
  購読者から読め、`generation` で欠落を自己診断できる。
- `poll`/`shutdown`/`register` の競合は `Mutex` で直列化され、`lease.
  retire()` の lock-free CAS は commit 時の `lifecycle()` 再読で捕捉される。
- Health は `AtomicU8` の collector 状態を含め Logging OFF で参照可能で、
  自己観測のため source として登録されない。
- 履歴・retained tombstone・cardinality の上限は全て設定駆動で、超過は
  evict/error/計数として型付きで報告される（黙殺しない）。

## Non-goals

- `ObservationBatch`→tracing/logging/Exporter 変換の試験（OBS-03/09/11 責務）。
- 実機/CI の macOS/Linux・AudioLens/Synvelis cross-repo gate の試験
  （#281・本 repo の検証責務外、worker 環境は Windows のみ）。
- `PollRunner` の周期精度・jitter の性能試験（起動/停止の境界のみ試験）。
- RT writer（`Counter::increment` 等）の性能回帰測定（RT 契約表・既存
  `test-rt` で担保、本試験は型・ビルドのみ）。
- `Instant` 系の時刻精度試験（`checked_duration_since` の契約のみ試験）。

## Risk

- `poll` が `&mut self` でない、または `Collector` が `Clone` になると
  「単一 poll owner」の型保証が崩れる → コンパイル時の型保証を試験する
  （`trybuild`/型のみでなく API 面の static な確認に留まる場合は
  `!Clone` 実装をレビューで担保）。
- `checked_duration_since` を外すと `Instant` panic・`inf` rate が
  発生する → R-4 の golden で固定。
- `shared_cell_ptr` の claim を Removed 後も残すと ABA 誤検知する →
  live-only 契約を R-13 で固定。
- `baseline_raw`/`registered_at` の確立を register 時にしないと、
  pre-registration 累積が delta に混入する → R-3/E-1 で固定。
- collector 強制 drain を `Valid` とすると「quiescence 未確認でも値を
  確定」に反する → R-11/E-14 で固定。
