---
id: "DESIGN-034T2dhkl03BuqPOGmNlmc"
type: "design"
status: "approved"
title: "Issue #289 OBS-07 Collector/Metrics/Snapshot: 単一Poll Owner・Baseline/Discontinuity・有界履歴・Health"
created: "2026-11-06"
updated: "2026-11-06"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/289"
    - "https://github.com/scarecrow173/ruviform/issues/275"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-289-observability-collector-metrics.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-277-observability-core-source-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/foundation-observability-minimal-api.md"
    - "docs/designs/shared-foundations-and-modularization-policy.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
    - "docs/adr/obs-05-observability-leaf-primitive-deps.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-289-observability-collector-metrics.md"
    - "docs/designs/issue-277-observability-core-source-contract.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-289-observability-collector-metrics-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/277"
    - "https://github.com/scarecrow173/ruviform/issues/278"
    - "https://github.com/scarecrow173/ruviform/issues/291"
    - "https://github.com/scarecrow173/ruviform/issues/293"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/278"
    - "https://github.com/scarecrow173/ruviform/issues/291"
  deferred-by: []
---

# Issue #289 OBS-07 Collector/Metrics/Snapshot: 単一Poll Owner・Baseline/Discontinuity・有界履歴・Health

## Context

`ruviform-observability` は OBS-02（#277）で登録/寿命契約、OBS-05（#280）で
`record`/`rt_bridge` を持つが、Registry・Poll・集約は未実装である。Issue #289
は OBS-02 の `ObservationRegistrar` の authoritative 実装である Collector を
求める: 単一の非 RT `poll` 経路で Counter/Gauge/Histogram を収集し、
Baseline/Delta/Rate/不連続処理・有界履歴・`MetricsSnapshot`・
`CollectorHealth` を Logging/Tracing 非依存で提供する。

設計上の中心問題は 3 つ: (1) `poll` の単一 owner を所有権で保証しつつ
登録ポートを複製・共有可能にする所有権構造、(2) 登録時 Baseline と
実測時間幅 Rate・不連続再基準化の意味論、(3) Registry lock の保持範囲を
`Source` の atomic 読取・内部状態 commit に限定し、callback/IO を構造的に
排除すること。

## Scope

### In Scope

- `src/metrics/` モジュール（共有値語彙・親設計の `metrics` モジュール候補）:
  `Measurement` / `MetricValue` / `MeasurementValidity` / `MeasurementIssue`。
- `src/collector/` モジュール群（poll エンジン）: `config` / `window` /
  `batch` / `snapshot` / `health` / `history` / `registry` / `collector`
  （`registry` と内部走査は非公開）。
- `Collector` + `CollectorRegistrar` + `CollectorHandle` + `PollRunner`。
- `ObservationBatch` / `ObservationWindow` / `MetricsSnapshot` /
  `CollectorHealth` / `CollectorState` + `Measurement` 系の上記値語彙。
- `CollectorConfigError`・`PollError`・`MeasurementIssue`（後述の各
  variant を列挙した型付き語彙）。
- reader cell 同一性検査用の `pub(crate)` アクセサ（core reader 3 種と
  `MetricReader` に追加）と counter 値減少の決定的試験用 `#[cfg(test)]`
  cell コンストラクタ（crate 内 unit test から参照。`Arc<AtomicU64>` の
  handle を test 側が保持して `store` する形にし、既存 shared cell の
  値を直接減少させられるようにする）。
- lib.rs crate doc の更新（モジュール一覧・RT 契約表への追記）。

### Out of Scope

- tracing/logging/Subscriber/Exporter 接続・発行（OBS-03/09/11）。
- 永続化・downsample・分位点・f64 Gauge・可変長 Label・callback 型
  Subscription。
- 新規外部依存・`unsafe`・core primitive/source 公開 API の変更。

## Components and Boundaries

### 所有権構造（単一 poll owner + 共有登録ポート）

```text
Arc<Shared> ────────────────────────────── 共有内部状態（Mutex<State>）
  ├── Collector        poll(&mut self) / snapshot() / health() / shutdown(&self)
  │                    registration_port() / handle() / into_poll_runner()
  ├── CollectorRegistrar   ObservationRegistrar 実装。Clone/Arc で
  │                        domain へ配布可能（Arc<dyn ObservationRegistrar>）
  └── CollectorHandle      snapshot()/latest_batch()/series()/history()/
                           health()/shutdown()/registration_port()。
                           Clone + Send + Sync。GUI/Diagnostics 側へ配布。
PollRunner ─ Collector を消費して専用 thread で定期 poll。
             handle() で読取面、registration_port() で登録面を返す。
```

- `Shared` は `Mutex<State>` + `AtomicU64` の `SourceId` mint +
  `AtomicU8` の collector 状態（Running/Draining/Stopped）を包む `Arc`。
  `State` は registry table・series 状態・履歴 ring・health 集計・最新
  snapshot `Arc`・最新 batch `Arc`・poll 世代・round-robin cursor・
  `created_at: Instant`（collector 生成時刻・世代 1 window の起点）を含む。
- `poll` が `&mut self` を要求するため、poll 権限は `Collector` の
  所有権にのみ存在する。`CollectorRegistrar`/`CollectorHandle` は poll を
  持たない → **同時 poll は型レベルで不可能**（「同時 poll 禁止」を所有権で
  保証）。不変条件: **`Collector` は `Clone`/`Copy` であってはならず、
  `Shared` → `Collector` の構築経路は `Collector::new` 1 つのみ**（私有）。
  この不変条件が「型レベル保証」の根拠であり、将来の変更で破っては
  ならない。
- `shutdown(&self)` は `&self` で冪等（Collector/Handle 双方から呼べる）。
  `AtomicU8` を `compare_exchange(Running→Draining)` した勝者だけが
  drain を実行する。実施手順: CAS 成功→`Mutex` 獲得→全 source を強制
  drain→`Stopped` へ store。lock 区間の先頭で `Stopped` かを見る
  ため、CAS 敗者が先に lock を獲得した場合はその caller 自身が
  drain を完遂する（勝者は `Stopped` を観測して即返る）。いずれに
  せよ drain は同一 lock 区間内で完結し **`Stopped` 到達済みで全
  caller が返る**。2 回目以降の呼出は即座に返る（冪等）。
- Collector 状態は `AtomicU8` に置くため `Draining` は lock 不要で観測
  可能（health の `state` もこの atomic を読む）。`register` は lock
  **前**に atomic を読み `Draining → FinalDrainInProgress`・
  `Stopped → RegistrarStopped` を早期返し、lock 後にも再検査する
  （atomic 先行読みは rejection window を広げる最適化、lock 下再検査が
  正しさの根拠）。
- `poll` は状態が `Running` でなければ `PollError::CollectorStopped`
  （`Draining` 中の poll も同じ error。drain へ加担する経路は設けない）。
- `Drop for Collector` = `shutdown()`（冪等）。registrar clone が残っていても
  `Stopped` で以後の登録は `RegistrarStopped`、retire 済み source は
  drain 済み。`PollRunner` の場合、thread 内で Collector が drop されて
  自動 shutdown される（後述）。
- **Mutex poison 方針**: 全 lock 獲得は
  `lock().unwrap_or_else(PoisonError::into_inner)` で回復する。lock 区間は
  atomic load + Vec 操作 + 状態 commit のみの短い in-place 更新で、
  panic は実装上の bug であって contract 上の状態を壊さない想定。
  観測機構（`health()` 含む）を 1 つの panic で永久不能にしないための
  方針であり、回復が発生したこと自体は `poison_recoveries_total` で
  health 計数する。

### Registry（内部・非公開）

- `HashMap<SourceId, SourceEntry>`: Active/Retiring の source を保持。
  `SourceEntry` は `identity`/`descriptor`/`scope`/`reader`/`state:
  Arc<SourceState>` と集約状態（`baseline_raw`/`cumulative`/`prev_raw`/
  `prev_buckets`/`last_change_at`/`registered_at`/`last_observed_at`/
  `lifecycle_seen`）。
- `HashMap<&'static str, KeyClaim>`: key → `{ owner: SourceId, kind,
  generation_seen }`。**collector 生存期間中は永続**（`'static` key 空間
  で有界・removed series evict でも消さない）。一意性の claim key は
  bare `key`（scope は一意性に含めない。OBS-02 に instance 識別子が
  ないため、同一 `'static` key を共有する複数 instance は共存不可 —
  異なる series が必要な domain は別の `'static` key を使う。cardinality
  契約の帰結として明記する）。
  - live（Active/Retiring/FinalDrain）source が所有する key への登録は
    `KeyConflict`（Retiring 中も同様。`FinalDrainInProgress` は collector
    全体の `Draining` 用で per-key には使わない）。
  - Removed 済み key の再登録: kind が一致 → 新 `SourceId` +
    `generation_seen+1` で受理。kind が過去と異なる →
    `InvalidDescriptor{reason:"metric kind changed for key"}`
    （「Metric Type を同名 ID で変更しない」を retained/evicted 期間を
    超えて永続的に保証）。
- `SourceId` mint: `AtomicU64` の単調増加（NonZero 保証）。
  `SourceGeneration`: 同一 key の再登録で +1（登録一意性の対を
  `SourceIdentity` として保持）。
- reader cell 同一性: `MetricReader::shared_cell_ptr()`（`pub(crate)`、
  `Arc::as_ptr`）で同一 cell の二重登録を `SourceReused` として拒否。
  claim は **登録が live な間のみ**保持する: registry 自身が reader の
  `Arc` を保持するため claim 期間中は cell アドレスが解放されず ABA
  誤検知は起きない。Removed 到達時に claim を解除するため、除去済みの
  cell（domain が clone を保持している場合）を再登録すると
  **新しい series + 新しい baseline として受理される**（=「live 登録の
  二重所有防止」という `SourceReused` の責務範囲。永続禁止とは読まない。
  この判断を Rustdoc に明記する）。
- descriptor 検査: 空 key → `InvalidDescriptor`；非 `Histogram` で
  `buckets: Some(_)` → `InvalidDescriptor`（意味を持たない指定の黙殺を
  防ぐ）；`Histogram` で `buckets == None`・空・非昇順・または reader の
  `boundaries()` と `descriptor.buckets` の**値完全一致**不一致（本数
  だけでなく値を比較。`HistogramReader::boundaries()` は公開済み）→
  `InvalidDescriptor`；`Counter` で `aggregation != Cumulative`、
  `Gauge` で `!= LatestValue`、`Histogram` で `!= BucketCounts` →
  `InvalidDescriptor`。
- `max_sources`/`max_series`: `max_series` は active + retained(removed)
  series 総数の上限。登録時に active 枠が満杯なら `CapacityExceeded`。
  Removed series は `retained_removed: VecDeque<SourceId>` で最古から
  evict して series 枠を回収（active 登録を古い removed で拒否しない）。
  retained evict は `removed_series_evicted_total` で health 計数する
  （snapshot から tombstone が消える事象を診断可能にする）。
- **設定検査**（`CollectorConfig::validate` → `CollectorConfigError`、
  `#[non_exhaustive]`）:
  - `poll_interval == 0` → `ZeroPollInterval`（PollRunner が busy-spin
    するため必須）
  - `max_sources == 0` → `ZeroMaxSources`
  - `max_series == 0` → `ZeroMaxSeries`
  - `max_batch_size == 0` → `ZeroMaxBatchSize`（Active が全件 skip され
    delta が永久不発行となるのを防ぐ）
  - `stale_after == Some(0)` → `ZeroStaleThreshold`
  - `max_series < max_sources` → `SeriesBelowSources{max_series, max_sources}`
  - `history_max_samples == 0` または `history_budget_bytes == 0` は
    **error ではなく履歴無効化**（明示的に 0 = disabled と定義）

### Poll の単一クリティカルセクション（lock 保持の限定）

`poll` は 1 回の `Mutex<State>` 獲得で完結する:

1. **enumerate**: Active/Retiring source を走査。Retiring は全件対象、
   Active は `max_batch_size` まで round-robin cursor で選択。
2. **read**: 各 reader を `Relaxed` 読取（Counter 単一 load、Gauge 単一
   load、Histogram bucket 別 load＝非同時観測）。
3. **commit**: Baseline 更新・不連続検出・`begin_final_drain`/
   `mark_removed`・履歴 push（eviction 込み）・最新 `MetricsSnapshot`
   再構築・health 集計・`ObservationBatch` 生成。

**重要な直列化の前提**: `Mutex` が直列化するのは registry 操作同士の
競合のみ。`lease.retire()`/`lease` drop は `SourceState` への
**lock-free CAS** であり registry mutex を取らない（`lifecycle.rs` 契約）。
したがって retire が enumerate と commit の間に割り込み得る。対策:
commit で **source ごとに `state.lifecycle()` を再読**し、`Retiring` を
観測した source はその poll で drain まで進める（enumerate 時点では
Active だった source が poll 中に retire されても最終値を取りこぼさない）。
逆に `lease.retire()` 呼出側は「次回 poll で drain される」ではなく
「その後のいずれかの poll で drain される」とだけ保証される。

**drain source の commit 順序**（Retiring/FinalDrain を観測した source）:
`begin_final_drain` → **最終 read** → `mark_removed` → measurement
commit。最終 read は `begin_final_drain` の**後**に行う（drain 開始宣言
より前に取った値を「最終値」と呼ばない。quiescence 宣言済み writer は
この後 increment しない契約なので通常は同じ値だが、abandoned/強制
drain では直前まで増えうるため順序を固定する）。

Issue の要件は「Registry Lock を保持したまま Subscriber/Exporter/
User Callback/ファイル I/O を実行しない」であり、本実装はそれらの操作を
**一切持たない**（callback 型 Subscription を提供しない Pull 型設計）。
lock 下で実行するのは atomic load・Vec 操作・状態遷移のみで、poll 全体が
O(登録 source 数 + 出力件数) の有界処理として直列化される。登録/解除/
shutdown との競合は同一 `Mutex` で完全に直列化され決定的となる
（`shutdown(&self)` が poll と競合しても、どちらかが先に完結する）。

3 相に分割して reader clone 後に lock を外す案は、mid-poll での source
消失（shutdown 競合）を扱う分岐が増えるだけで、callback/IO が存在しない
本設計では利益がないため採用しない。

### Measurement / Validity

```rust
pub struct Measurement {
    // identity + descriptor（clone・'static）+ scope + lifecycle（観測時）
    identity, descriptor, scope, lifecycle,
    observed_at: Instant,          // この観測が確定した時刻（commit 時の poll now）
    last_change_at: Instant,       // 値が最後に変化した観測時刻
    validity: MeasurementValidity, // Valid/Stale/Partial/Discontinuous/Unavailable/Unsupported
    issue: Option<MeasurementIssue>,
    loss: Option<u64>,             // 定量化できる欠落量（不明なら reason のみ）
    value: Option<MetricValue>,    // None = Unavailable/Unsupported（0 と区別）
}

pub enum MetricValue {
    Counter { raw: u64, cumulative_since_registration: u64,
              delta: Option<u64>, rate_per_sec: Option<f64> },
    Gauge { value: u64, previous: Option<u64> },
    Histogram { buckets: Box<[u64]>, total_approx: u64 }, // 非同時観測
}

pub enum MeasurementIssue /* #[non_exhaustive] */ {
    CounterDecreased,      // raw < prev_raw（wrap/reset 疑い・区別不能）
    ClockAnomaly,          // now < 前回観測時刻（elapsed 計算不能）
    WriterNotQuiesced,     // quiescence 未確認の drain（forced/abandoned）
    HistogramInconsistent, // 観測時に発覚した bucket 不整合（防御的）
}
```

- **Baseline**: `register` 時に raw を読み `baseline_raw` に固定
  （`Instant::now()` も `registered_at` として記録）。pre-registration
  累積は新規事象として出ない。`cumulative_since_registration` は有効
  delta のみを登録以降累積する（`saturating_add`。名前どおり
  「登録以降の観測累積」であり、不連続再基準化を跨いでもリセット
  しない点を Rustdoc で明記する）。
- **Delta/Rate**: `delta = raw - prev_raw`（減少なら不連続）。
  `rate_per_sec = delta / 実測経過秒`。実測経過は source ごとに
  `now.checked_duration_since(anchor)` で求める（anchor は前回観測時刻、
  初回は `registered_at`）。**elapsed が `None`（now が anchor より
  過去）または 0 の場合は rate/delta 算出を行わず
  `rate_per_sec = None` + `issue: ClockAnomaly` + `clock_anomalies_total`
  計数**（`Instant` 減算 panic と `inf`/`NaN` rate の両方を防ぐ）。
  `registered_at` は `Instant::now()`、caller の `poll(now)` は別時刻
  源を混在させてはならない旨を Rustdoc に明記し、`now < registered_at`
  も同じ ClockAnomaly 経路で扱う（caller は毎回新しい `Instant::now()`
  を渡すことが前提）。
- **不連続**: `raw < prev_raw` → `Discontinuous` + `issue:
  CounterDecreased`、delta/rate は `None`（巨大 delta 不発行）、
  `prev_raw = raw` で再基準化。`ClockAnomaly`（`now < 前回観測時刻`）は
  上記のとおり measurement にも issue を付す。
- **Gauge**: 直近値 + 前回値。`0` は `Valid{value:0}` であり、source 除去後の
  `Unavailable{value:None}` と型で区別。
- **Stale**: `stale_after: Some(d)` 設定時、`now - last_change_at > d` の
  Active source を `Stale`（値は保持・「更新が止まった可能性」を表現。
  既定 `None` = 自動 Stale なし）。
- **Partial**: writer の quiescence が確認できない drain 値に付ける。
  判定は「collector 側が `request_retire()` を実行して Active→Retiring
  遷移を行った（戻り値 true）」または「`is_abandoned`（lease drop で
  設定）」。いずれも `issue: WriterNotQuiesced`。domain が `retire()`
  で自発的に Retiring へ遷移させた source の drain 値は `Valid`。
  **shutdown の強制 drain は必ず前者なので `Partial` になる**
  （quiescence 未確認の値を `Valid` と偽装しない）。
- **Unavailable**: Removed series（retained）の snapshot entry。
  `value: None`。最終値は drain commit した `ObservationBatch`
  （poll または終端 drain 世代）に保持される。履歴が有効なら履歴
  ring にも残るが、回収経路の正本は batch であり history 容量に
  依存しない。
- **Unsupported**: 将来の kind 用。現行 `MetricReader` は 3 kind で
  exhaustive に処理されるため到達不能（contract 上の reserved variant
  であり発生条件はない。`#[non_exhaustive]` 付きで将来 kind 追加時の
  後方互換を確保）。

### Window / Batch / Snapshot

```rust
pub struct ObservationWindow {
    generation: u64,
    started: Instant,              // この観測区間の開始 = 前回 batch の ended
    ended: Instant,                // = poll(now) の now
    expected: Duration,            // 設定 poll_interval
    measured: Duration,            // ended - started（遅延時の実測区間幅）
    clock: ClockDomainId,          // 常に ClockDomainId::MONOTONIC
    planned: usize, observed: usize, skipped: usize, drained: usize,
    history_evicted: usize, clock_anomaly: bool,
    // 整合性: per-source 読取は非同時である旨は Rustdoc で確約
}
pub struct ObservationBatch /* Arc 共有・Clone */ {
    generation, window, measurements: Vec<Measurement>,
    removed: Vec<SourceIdentity>,  // この poll で Removed になった source
    // commit 後の全 series snapshot への参照（Issue の「Snapshot参照」）
    // → snapshot() accessor で MetricsSnapshot を返す
}
```

- **window.started**: 世代 N>1 は世代 N-1 の `ended`。世代 1 は
  `created_at`（collector 生成時の `Instant::now()`、「collector 開始
  から最初の観測まで」という意味で定義）。`measured = ended - started`
  が実測区間幅であり、`measured > expected` を「遅延 poll」として
  `overdue_polls_total` に計数する。`clock_anomaly` は
  `now < 前回 ended`（window 逆行）またはいずれかの source で
  `checked_duration_since` が失敗した場合に立てる。
- `poll(&mut self, now: Instant) -> Result<ObservationBatch, PollError>`。
  `PollError`（`#[non_exhaustive]`）: `CollectorStopped`（`Draining`/
  `Stopped` での呼出）。caller は毎回新しい `Instant::now()` を渡す前提
  （Rustdoc 明記）。
- `CollectorHandle::latest_batch() -> Option<ObservationBatch>`: `State`
  に保存した最新 batch `Arc` を返す。`None` は観測 commit（poll または
  終端 drain）が一度も走っていない場合のみ。終端 drain も 1 世代を
  mint するため `Stopped` 後は terminal batch（drain measurement +
  `removed` + post-drain snapshot。drain 対象ゼロなら空 batch）を返す
  — history 容量ゼロでも最終 delta/Removed はここから回収できる
  （登録・poll は停止しても観測結果は残す）。
  `ObservationBatch::snapshot() -> MetricsSnapshot` で
  batch が参照する post-commit snapshot を取り出せる。
- `snapshot() -> MetricsSnapshot`（Arc 共有）: 全 tracked series の最新
  `Measurement`（Active/Retiring/retained-removed）+ generation +
  taken_at。「同じ世代の結果を複数購読者が読める」=
  `ObservationBatch`/`MetricsSnapshot` を `Arc` で共有。購読者の
  取りこぼしは `generation` gap で自己診断
  （`batch.generation - 前回 generation - 1`）。

### History（容量有界・非永続）

- 全 series 共通の単一 `VecDeque<Measurement>` ring（commit 順 push_back、
  最古 pop_front evict）。`history_max_samples`（件数）と
  `history_budget_bytes`（概算 byte: entry 固定部 + Histogram bucket 動的部を
  加算）を両方満たすよう evict。`history_* == 0` は設定検査で「履歴
  無効」として受理される（ring を持たない）。
- **oversized entry**: 単一 entry の概算 byte が `history_budget_bytes`
  を超える場合、その entry は **履歴に格納しない**（evict しても budget
  を満たせないため）。batch/snapshot には通常どおり出る。拒否数は
  `history_oversized_dropped_total` で health 計数（黙って捨てない）。
- 単一 global FIFO のため「1 series が他の全 retained 履歴を追い出す」
  ことはあり得る。これは受け入れる trade-off（per-series 枠を増やさない
  単純さを優先）であり、evict 量は window・`history_evicted_total`・
  `series()` の有無で診断する。
- 読取: `CollectorHandle::series(identity) -> Vec<Measurement>`（系列ごと
  chronological clone）/ `CollectorHandle::history() -> Vec<Measurement>`
  （全保持分・chronological）。eviction 量は window（当該 poll）と
  `CollectorHealth::history_evicted_total`（累計）で診断可能。

### Health（Logging 非依存・非再帰）

```rust
pub struct CollectorHealth {
    state: CollectorState,                  // AtomicU8 から読む Running/Draining/Stopped
    polls_total, overdue_polls_total, last_poll_at, last_poll_duration,
    registered_total, registration_rejected_total,
    sources_active, sources_retiring, sources_removed_total,
    abandoned_sources_total, samples_total, samples_skipped_total,
    discontinuities_total, clock_anomalies_total, histogram_inconsistencies_total,
    lifecycle_errors_total,                 // commit 中の InvalidTransition 等
    history_len, history_bytes, history_evicted_total,
    history_oversized_dropped_total,        // budget 超過で格納拒否した entry
    removed_series_evicted_total,           // retained tombstone の evict
    poison_recoveries_total,                // Mutex poison からの回復
}
```

`overdue_polls_total` = 実測経過（`window.measured`）が `poll_interval`
を超えた poll の数（遅延の可視化）。`histogram_inconsistencies_total` =
構築後に発覚した descriptor/reader bucket 不整合（登録時の値一致検査で
防ぐが防御的計数。発生時は当該 source を `Partial` +
`issue: HistogramInconsistent` で記録）。`lifecycle_errors_total` =
commit 中に `begin_final_drain`/`mark_removed` が `InvalidTransition`
を返した回数（状態機械の防御的計数）。`poison_recoveries_total` =
Mutex poison から `into_inner` で回復した回数。Health 自身は source に
登録しない → 無限再帰しない。

### PollRunner（汎用非 RT 補助）

`Collector::into_poll_runner(self) -> PollRunner`: Collector を専用
`std::thread` に移し、`Instant::now()` で `poll` → 残り時間 `sleep` の
ループ。`stop` フラグ（`AtomicBool`）または collector `Stopped` で終了し
`JoinHandle` を join。`handle()` で `CollectorHandle`、
`registration_port()` で `CollectorRegistrar` を返す（`into_poll_runner`
後も source 登録は可能。`Collector` を消費するため、登録面はこの経路で
取得する）。thread 内で Collector が drop されると `Drop=shutdown` が
発火して全 source を drain するため、stop 後の残留 source は残らない。
`PollRunner::stop(self) -> thread::Result<()>`。tracing/logging は発行しない。

## Data and Control Flow

- Domain（非 RT）: `collector.registration_port()` を取得（複製可能）→
  `register(MetricSource)`。registry は `Instant::now()` で baseline を確立、
  `SourceIdentity`/`RegistrationLease`/`Arc<SourceState>` を返す。
- Poll owner: `collector.poll(Instant::now())`（または `PollRunner`）→
  1 回の `Mutex<State>` 獲得で enumerate → read → commit →
  `ObservationBatch`（世代 N）。`Stopped`/`Draining` なら
  `Err(PollError::CollectorStopped)`。
- Subscriber: `handle.snapshot()`/`latest_batch()` を `Arc` 経由で読み、
  世代 gap と `history` で欠落を診断。
- Retire: domain が `lease.retire()` → `Retiring`。次の drain 対象 poll
  で `begin_final_drain` → 最終 read → `mark_removed` → batch は最終値
  （domain retire: `Valid`、collector 強制 drain/abandoned: `Partial` +
  `WriterNotQuiesced`）+ `removed` リスト。reader の release は
  Removed 到達後に registry が行う（契約どおり）。
- Shutdown: `shutdown()` → `AtomicU8` CAS `Running→Draining`（登録は
  `FinalDrainInProgress`）→ `Mutex` 獲得→全 source を強制
  retire→drain→Removed→**終端 drain を 1 世代の `ObservationBatch`
  として commit（drain measurement + `removed` + post-drain
  snapshot を `latest_batch`/`latest_snapshot` に公開）**→`Stopped`
  へ store→unlock（登録 `RegistrarStopped`・poll
  `CollectorStopped`）。全 caller は drain 完了を待って返る（冪等）。
  強制 drain された source は `Partial` + `WriterNotQuiesced` で
  記録される。終端 drain は poll と同じ観測 commit 責務で世代を
  mint するため、history 容量ゼロでも最終 delta/Removed 事象は
  `latest_batch()`/`snapshot()` から必ず回収できる。終端 pass は
  poll ではないため `polls_total`/`overdue_polls_total`/
  `last_poll_at` は更新しない（window の `measured` は最後の
  window 終端からの実測経過を正直に記録する）。

## Non-Functional Constraints

- 依存: std + 既存 `ruviform-media-core`（`ClockDomainId` のみ）+
  既存 `ruviform-rt` 不使用。新規外部依存なし。default feature（`logging`
  OFF）で全 API が動作。
- RT 契約: collector は非 RT のみ。poll/登録/snapshot/health/履歴取得は全て
  alloc+lock を行う。RT writer 経路（`Counter::increment` 等）には一切
  触れず、既存 RT 契約表・性能を回帰させない。
- 上限: `max_sources`/`max_series`/`max_batch_size`/`history_max_samples`/
  `history_budget_bytes`。`poll` の計算量は O(登録 source 数 + 出力件数)、
  lock 保持は単一 `Mutex` 区間（enumerate/read/commit をまとめて）で、
  callback/IO は lock 下に存在しない。
- 全公開 enum `#[non_exhaustive]`・`unsafe` 不使用・`Send`/`Sync` は
  `Arc<Mutex<_>>`/`&'static str`/`Copy` 値から導出。

## Risks and Trade-offs

- `poll(&mut self)` と共有 `Arc<Shared>` の両立のため、poll 権限を
  `Collector` 所有権に閉じ込め、読取面を `CollectorHandle` に分離した。
  Trade-off: `Collector` 自体を `Arc` で共有しても poll できない点は
  ドキュメントで明確化する（`into_poll_runner` が正規の共有経路）。
- poll 全体を単一 `Mutex` 区間としたことで登録/解除は poll 完了を待つ。
  lock 下は atomic load + Vec 操作のみで callback/IO を持たないため、
  「lock 保持中の重い処理」を構造的に排除する目的に対し十分であり、
  3 相分割が生む mid-poll 消失分岐を避けられる。
  登録→最初の poll 間の delta は registered_at 起点の実測幅で正しい。
- Removed series の retained 期間を `max_series` の枠内最古 evict に一本化
  （独立した retention 設定を増やさない）→ 上限予算が単純。
- `Instant` を poll clock としたため「Clock 切替」は検出不能。代わりに
  `clock_anomaly`（逆行）検出 + window `clock: MONOTONIC` 記録で
  契約を満たす。sample-clock 相関は `ClockAnchor`（media-core）側の責務。
- `PollRunner` は optional 補助だが下流（GUI/Diagnostics/host runner）の
  確実な需要があるため最小形で同梱（tracing 発行なし・join まで実装）。
- `Measurement.value: Option` は Unavailable と 0 の区別のため必須。
  代償として参照側は `Option` を扱う（契約として意図的）。

## Deferred Design Concerns

<!-- Intentionally deferred future work. Link the deferred draft doc via relations.defers. -->

- `ObservationBatch`→`EventRecord`/tracing 変換、CorrelationLink の付与 |
  OBS-03/05 側の変換責務 | #278 実装時 | Collector が変換を内製すると
  Logging OFF 契約と rt_bridge 責務境界が崩れる。
- pull 型購読の世代通知（condvar/callback なし） | 現行は pull のみで十分 |
  下流が要した時点 | 仮の通知面は使われない API を増やす。
- downsample・多段履歴・永続化 | Issue 非対象 | 実需要 | 有界 ring の診断で
  現行要件は満たせる。
- `ClockDomainId` 以外の poll clock（host 供給 clock trait） | Instant で
  契約充足 | 実需要 | YAGNI。

## Verification Notes

- [ ] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` 全 pass。
- [ ] 既存 counter/source/record/rt_bridge 試験が無改変で pass（回帰なし）。
- [ ] golden: baseline/rate/不連続/再登録/drain/gauge 0-vs-Unavailable/
      histogram/ window/ stale/ partial/ removed retained/ key kind 付替拒否。
- [ ] 競合: poll 中の別スレッド register/retire/shutdown で決定的直列化。
- [ ] `PollRunner` の生成・周期 poll・stop/join の境界試験。
- [ ] crate doc/RT 契約表の更新と `logging` feature OFF ビルド。
