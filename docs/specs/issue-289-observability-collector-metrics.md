---
id: "SPEC-034T2dgEk6l5QpkecJOJYo"
type: "spec"
status: "approved"
title: "Issue #289 OBS-07 Collector・Metrics・履歴・Snapshot（単一クレート）"
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
    - "https://github.com/scarecrow173/ruviform/issues/289"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-277-observability-core-source-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/foundation-observability-minimal-api.md"
    - "docs/designs/shared-foundations-and-modularization-policy.md"
    - "docs/designs/issue-277-observability-core-source-contract.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
    - "docs/adr/obs-05-observability-leaf-primitive-deps.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-277-observability-core-source-contract.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-289-observability-collector-metrics-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/277"
    - "https://github.com/scarecrow173/ruviform/issues/278"
    - "https://github.com/scarecrow173/ruviform/issues/279"
    - "https://github.com/scarecrow173/ruviform/issues/280"
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/290"
    - "https://github.com/scarecrow173/ruviform/issues/291"
    - "https://github.com/scarecrow173/ruviform/issues/292"
    - "https://github.com/scarecrow173/ruviform/issues/293"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/278"
    - "https://github.com/scarecrow173/ruviform/issues/291"
  deferred-by: []
---

# Issue #289 OBS-07 Collector・Metrics・履歴・Snapshot（単一クレート）

## What Should Be Built

既存 `ruviform-observability` クレート内に、OBS-02（Issue #277）の登録ポートを
実装する **Collector** を実装する。Counter/Gauge/Histogram を**単一の非 RT
観測経路**（`poll`）で取得・集約し、Logging/Tracing OFF（feature 無効・依存ゼロ）
でも参照できる `MetricsSnapshot`・`ObservationBatch`・容量有界時系列
（history）・`CollectorHealth` を提供する。複数 Exporter/GUI/Diagnostics が
各自で Counter 差分を算出する重複をなくすため、Baseline・Delta・Rate・
不連続検出・Validity の責務を Collector に集約する。新規クレートは作らない。

## Why Now

- OBS-02（#277）で `ObservationRegistrar`/`RegistrationLease`/
  `Active→Retiring→FinalDrain→Removed` 寿命契約がコンパイル済み契約として
  固定された。次の一手はそのポートの authoritative 実装である Collector。
- OBS-03 Tracing 変換（#278）・OBS-04 Plugin 登録（#279）・OBS-08 Profiling
  （#290）・OBS-09 Diagnostics（#291）・OBS-11 Integration（#293）が
  Collector の `ObservationBatch`/`Snapshot`/`Health` を消費する前提であり、
  下流 API 契約を先に固定する必要がある。
- Collector 不在のままだと各 Consumer が Baseline・Rate・Wrap 疑い・
  Source 再生成を独自に再実装し、「0 偽装」「巨大 Delta」等の観測誤りが
  分散する。

## Users and Value

- **OBS-03/08/09/11 実装者**: `registration_port()`・`poll()`・`snapshot()`・
  `health()` の確定 API と、Validity/Window/欠落の型付き結果を得る。
- **Domain 側（plugin host 等）**: OBS-02 の通りに `MetricSource` を登録する
  だけで、Baseline/Rate/Drain まで正しく処理される。
- **Consumer（AudioLens/Synvelis）**: Logging OFF でも同一の型付き観測
  結果を Pull で読め、差分計算を自前で持たない。

## Scope

### In Scope

- `src/metrics/` + `src/collector/` モジュール群（責務に応じた内部再編）:
  - `CollectorConfig`（`poll_interval`/`max_sources`/`max_series`/
    `max_batch_size`/`history_budget_bytes`/`history_max_samples`/
    `stale_after`）と上限・ゼロ/無効設定の検査 Error。
  - `Collector::new`・`registration_port()`（`ObservationRegistrar` 実装の
    返却・複製・共有可能）・`poll(now) -> Result<ObservationBatch,
    PollError>`・`snapshot()`・`latest_batch()`・`health()`・`shutdown()`。
  - `ObservationBatch`（poll 世代・Window・型付き Metric・欠落）・
    `ObservationWindow`（開始/終了/実測経過・有効サンプル数・欠落/間引き・
    Clock Domain・整合性）・`Measurement`/`MeasurementValidity`/
    `MetricValue`/`MeasurementIssue`。
  - Counter の Baseline/実測時間幅 Rate/不連続（値減少・Wrap 疑い・Source
    再生成）検出と再基準化・Gauge の最新/増減/0 vs Unavailable/Stale 閾値・
    固定 Bucket Histogram（非同時観測の明記）・Derived Rate/Window。
  - Source 寿命連携: Retiring 検出→最終 Drain→Removed、abandoned source は
    `Partial` 明示、Lease Drop で最後の Delta を失わない。
  - 容量有界履歴（ring・byte/count 両上限・eviction 診断）と Health
    （error・過負荷・不整合・更新停止の Logging 非依存可視化）。
  - 単一 poll owner を `&mut self` で型保証、batch/snapshot は `Arc` 共有で
    複数購読者が同じ世代を読める Pull 型設計。永続用全件キュー・無制限
    fan-out は置かない。
  - 公平性付き Batch 上限（round-robin 走査）と汎用非 RT の定期実行補助
    （`PollRunner`。Tracing 発行はしない）。
- Collector 実装に必要な最小の crate 内連携（reader cell の同一性検査用
  `pub(crate)` アクセサ等）。
- 契約試験: Baseline/Rate/不連続/再登録/最終 Drain、Gauge/Histogram/Window/
  Stale/Partial/Unavailable golden、ログ OFF での Snapshot/Health、同時
  poll 禁止・登録/解除競合・Slow subscriber・容量上限・履歴破棄。

### Out of Scope

- `tracing::Dispatch`/Subscriber/Logging への依存・発行（OBS-03 #278 の責務）。
- OTLP/Prometheus 等 Exporter の必須導入・標準 tracing の Log Event。
- 永久保存・監査永続化・Signal Capture・Record Replay。
- Raw PCM/Signal/Trace/Profiler stack の Series 履歴格納（形状上も格納しない）。
- 新規クレート・外部依存追加・`Counter`/`Gauge`/`Histogram` RT 計装 API の変更
  （性能・型・ビルドの回帰禁止）。
- 任意の `T` を無条件 Atomic Gauge 化する API。
- macOS/Linux 実機検証・AudioLens/Synvelis Cross-Repo gate（#281）は
  証跡として報告するが、本 Issue のコード責務ではない。

## Requirements

- `poll` は `&mut self` で単一 owner を型保証する。`ObservationBatch` は
  `Arc` 共有の世代付き結果で、複数購読者が同一世代を安全に読める。
- 登録/解除/遷移/履歴 commit 以外に Registry lock を保持したまま
  Subscriber/Exporter/User Callback/ファイル I/O を実行しない設計とする
  （本実装は callback を持たず poll 内は atomic read + Vec 操作のみ）。
- Counter は登録時 Baseline を確立し、登録前の累積を新規事象として出さない。
  Delta/Rate は実測経過時間で算出する。値減少/Wrap 疑い/再登録は
  `Discontinuous` として再基準化し、不確定な区別は不確定のまま型で表現する。
- Gauge は 0 と Unavailable を区別し、観測時刻と stale 閾値を持つ。
  Collector 空振り/Source 終了で値を 0 に偽装しない。
- Window は開始/終了/有効サンプル数/欠落・間引き/Clock Domain/整合性を持ち、
  poll 遅延時は予定周期ではなく実測時間幅を記録する。
- `MeasurementValidity` は `Valid/Stale/Partial/Discontinuous/Unavailable/
  Unsupported` を型で区別し、loss 量と reason は別フィールドで保持する。
- `Active→Retiring→最終Drain→Removed` を OBS-02 契約どおり駆動し、RT writer
  quiescence 宣言（`retire`）なしに最終値を確定しない。`abandon` 経由の
  drain は `Partial` を明示する。終端 drain（shutdown/drop）は poll と
  同じ観測 commit として新世代の `ObservationBatch` を公開し、最終
  delta/Removed 事象は履歴容量に依存せず `latest_batch()`/`snapshot()`
  から回収できる。
- Series/key/scope の cardinality は `max_sources`/`max_series` と OBS-02 の
  `&'static str` key・enum scope で制御する。ユーザー由来文字列を Series key
  にせず、内部 Source ID を高 cardinality Label に自動公開しない。
- 履歴は容量有界 ring で既定プロセス内非永続。evict/購読者世代 gap/Health で
  取りこぼしを診断できる。
- 収集 Error・過負荷・Histogram 不整合・更新停止を `CollectorHealth` で可視化
  し、自己観測は無限再帰しない（Health は source として登録しない）。
- 全公開 enum は `#[non_exhaustive]`。`unsafe` 不使用。新規外部依存なし
  （`ruviform-media-core` は既存依存を再利用）。
- RT producer（`Counter::increment` 等）は Collector の lock/alloc/IO を
  一切使わず、既存 API・性能・ビルドを回帰させない。

## Acceptance Criteria

- [ ] AC-1: `Collector` が `ObservationRegistrar` を実装し、`register`/
  `register_counter` で OBS-02 の `RegistrationLease`・`SourceIdentity`・
  型付き `RegistrationError`（CapacityExceeded/KeyConflict/SourceReused/
  FinalDrainInProgress/InvalidDescriptor/RegistrarStopped）を返す。
- [ ] AC-2: `poll(now)` が Window・実測経過・型付き Metric を持つ
  `ObservationBatch` を返し、Counter の Baseline/Delta/Rate が実測時間幅で
  算出される（golden 試験）。
- [ ] AC-3: Counter 値減少・再登録・最終 Drain で誤った巨大 Delta が出ず、
  `Discontinuous`/`Unavailable`/`Partial` が正しく付く（決定的試験）。
- [ ] AC-4: Gauge の 0 vs Unavailable、固定 Bucket Histogram の非同時観測、
  Window の Stale/Partial/欠落が golden 試験で検証される。
- [ ] AC-5: default feature（`logging` 無効・外部依存ゼロ）で
  Snapshot/Health/Batch/履歴が利用可能であることがビルド・試験で示される。
- [ ] AC-6: `&mut self` 以外から poll 不能・登録/解除競合・Batch 上限
  round-robin・履歴 eviction・購読者世代 gap が試験される。
- [ ] AC-7: `shutdown` が残存 source を最終 Drain→Removed まで駆動し、
  以後の登録/poll が型付き失敗になる。quiescence 未確認の drain
  （`abandon`・shutdown 強制）は `Partial` + reason を明示する。
- [ ] AC-8: `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` が pass し、
  既存 counter/source/rt_bridge 試験が回帰しない。RT producer の公開
  API・RT 契約表に変更がない。
- [ ] AC-9: 精度/定義/Clock/provenance・将来 Exporter 接続点が Rustdoc に
  記述され、Health が自己 source 登録を行わないことが試験される。

## Deferred Design Concerns

<!-- Intentionally deferred future work. Link the deferred draft doc via relations.defers. -->

- ObservationBatch→tracing Event/Span 変換・Subscriber 接続 | 責務は OBS-03 | Issue #278 | Collector が tracing 依存になると Logging OFF 契約が壊れる。
- Diagnostics/Debug への Health/履歴接続・Plugin 側の registered 観測公開 | 責務は OBS-09/10/11 | Issue #291/#292/#293 | 下流契約を Collector が内製すると責務が漏れる。
- f64 Gauge・可変長 Label・厳密分位点・downsample/再集約 | OBS-02 と同じ保留判断 | 実需要の出現 | 未定義の整合性期待と cardinality 破壊を防ぐ。
- Exporter 接続 API（pull 契約の安定化・バッチ識別子の確定） | OBS-03/09/11 の消費実装を見て確定 | 下流 Issue 着手 | 未使用の接続面を仮固定すると再作業になる。
- Cross-Repo gate(#281)・macOS/Linux 実機検証 | 本 repo の責務外の CI/実機 | #281/統合時 | worker 環境は Windows のみ。

## Sources and Relations

- Issue #289（本 spec の正本・完了条件）、親設計 Issue #275（単一クレート構成・
  責務分離の合意版）、先行 Issue #277（OBS-02 登録/寿命契約・コンパイル済み）。
- `docs/specs|designs/issue-277-observability-core-source-contract.md`
  （`ObservationRegistrar`/`RegistrationLease`/`SourceState`/reader 所有権の
  authoritative 契約）、`docs/designs/shared-foundations-and-modularization-policy.md`
  （observability 内 logging/metrics/rt_bridge 等のモジュール分割方針）、
  `docs/adr/obs-05-observability-leaf-primitive-deps.md`（leaf 依存方向の制約）。
