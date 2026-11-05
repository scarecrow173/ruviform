---
id: "DESIGN-fhvRxnPJKvnhRAlg7TbeKm"
type: "design"
status: "approved"
title: "Issue #280 OBS-05 RT Record Bridge・ClockContext・欠落envelope・CorrelationBook設計"
created: "2026-11-05"
updated: "2026-11-05"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/280"
    - "https://github.com/scarecrow173/ruviform/issues/275"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-280-rt-record-bridge-clock-correlation.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/foundation-observability-minimal-api.md"
    - "docs/designs/shared-foundations-and-modularization-policy.md"
    - "docs/designs/issue-277-observability-core-source-contract.md"
    - "docs/specs/issue-277-observability-core-source-contract.md"
    - "docs/adr/obs-05-observability-leaf-primitive-deps.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-280-rt-record-bridge-clock-correlation.md"
    - "docs/designs/foundation-observability-minimal-api.md"
    - "docs/adr/obs-05-observability-leaf-primitive-deps.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-280-rt-record-bridge-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/289"
    - "https://github.com/scarecrow173/ruviform/issues/278"
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/290"
    - "https://github.com/scarecrow173/ruviform/issues/185"
    - "https://github.com/scarecrow173/ruviform/issues/308"
    - "https://github.com/scarecrow173/ruviform/issues/309"
    - "https://github.com/scarecrow173/ruviform/issues/104"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/278"
    - "https://github.com/scarecrow173/ruviform/issues/289"
    - "https://github.com/scarecrow173/ruviform/issues/290"
  deferred-by: []
---

# Issue #280 OBS-05 RT Record Bridge・ClockContext・欠落envelope・CorrelationBook設計

## Context

`ruviform-observability` は現状 `core`（Counter/Gauge/Histogram）+ `source`
（登録・寿命契約）のみを持つ最小クレート。`foundation-observability-minimal-api`
は RT → 非 RT の固定長 Record Bridge を `rt_bridge` として本クレートに置く方針を
既に定めており、`shared-foundations-and-modularization-policy` も
「RT → 非 RT の有界観測レコード転送機構は `ruviform-observability` の
`rt_bridge` が所有する（構成候補）」と指名している。Issue #104 の PoC は
「固定長 `Copy` レコード + `rtrb` SPSC + drop-on-full + overflow counter」が
RT 安全であることを実測で示した。

本設計は次を確定する:

1. `rt_bridge` が既存 `ruviform_rt::channel`（SPSC 有界リング）を再利用する
   輸送契約と、その RT/非 RT 境界。
2. `record` モジュールが運ぶ Clock Context・Correlation link/book・
   固定長 `EventRecord` の語彙。
3. OBS-02 の「`ruviform-observability` は他 ruviform クレートに依存しない」
   という AC-6 を、本 Issue 限りで「**下向きの葉 primitive への一方向依存のみ
   許可**」へ明示的に改訂する（後述「依存方向の整合」）。

## Scope

### In Scope

- `crates/ruviform-observability/src/rt_bridge.rs`:
  `bridge`/`RtRecorder`/`RecordSink`/`ReceivedRecord`。
- `crates/ruviform-observability/src/record/`:
  `clock.rs`（`ClockContext`）、`correlation.rs`（`CorrelationToken`/
  `CorrelationLink`/`CorrelationBook`/`CorrelationStatus`/`UnlinkedReason`）、
  `event.rs`（`EventId`/`RecordHeader`/`EventRecord`）、`mod.rs`（再 export）。
- `crates/ruviform-observability/Cargo.toml` への
  `ruviform-rt`・`ruviform-media-core` path 依存追加（`dev-dependencies` には
  既存の `assert_no_alloc` を維持）。
- `crates/ruviform-observability/src/lib.rs` の crate doc 更新・新モジュール
  公開・依存方針節の追記。
- `crates/ruviform-observability/README.md`・`crates/README.md` の更新。

### Out of Scope

- 実 Collector/Registry/History・snapshot/trace 変換（OBS-07=#289 /
  OBS-03=#278）。本設計はそれらが `drain`/`drain_final`/`ReceivedRecord`/
  `CounterReader`/`RecordHeader` をそのまま消費できる契約のみ定義する。
- `tracing`/`tracing-subscriber`/外部 IO/Subscriber 依存、`rtrb` 等外部クレートの
  直接露出。
- `ruviform-rt` 側への変更（channel/retire/slot の API・実装とも不変）。
- #308（Probe Rebind の binding/queue/generation 寿命）・#309（MultiProbe
  非 RT 整列）の重複責務。
- Multi-producer bridge・可変長 payload・block/overwrite policy・欠落復元。
- 性能測定/on-hardware 証跡（OBS-08=#290）・実 Plugin/実 GUI/実機検証。

## Components and Boundaries

### モジュール構成

```text
ruviform-observability
├── core/            既存（counter/gauge/histogram。不変）
├── source/          既存（OBS-02 契約。不変）
├── record/          NEW
│   ├── clock.rs        ClockContext（InputStamp + ClockEpochId）
│   ├── correlation.rs  CorrelationToken/Link/Book/Status/UnlinkedReason
│   └── event.rs        EventId/RecordHeader/EventRecord
└── rt_bridge.rs     NEW（bridge/RtRecorder/RecordSink/ReceivedRecord）
```

依存方向: `rt_bridge` → `core`（Counter 再利用）・`record`（`R` の典型実装）、
`record` → `source`（`SourceIdentity`/`Confidentiality`/`ReportingImpact`
再利用）・`ruviform-media-core`（時刻語彙）、`rt_bridge` → `ruviform-rt`
（`channel` 再利用）。`source`/`core` は新規依存を持たない。

### 依存方向の整合（OBS-02 AC-6 の改訂）

OBS-02 spec の AC-6 は「observability は他 ruviform クレートへ依存しない」を
no-cycle 証明として採用した。これは OBS-02 の時点で「source/core 契約が
domain 依存を要しない」ことを固定したものであり、**全期間を通じて
`ruviform-observability` が一切の ruviform 依存を持てない**という規則では
ない。本設計では以下の 2 本の一方向依存を追加し、no-cycle は維持する:

- `ruviform-observability → ruviform-rt`:`channel<T>`（SPSC）は
  architecture baseline で「最下位・相互依存なし」の葉であり、上流からの
  一方向依存は `ruviform-media-core`/`ruviform-exec` が `ruviform-rt` に
  依存する既存パターンと同型。`ruviform-rt` 側の公開 API・RT 契約は不変で、
  `channel` を Foundation/observability へ「移動」はしない（#101 政策・
  「`ruviform-rt → ruviform-foundation` のような新規依存は既定路線ではない」
  に整合）。
- `ruviform-observability → ruviform-media-core`: Clock Domain/Epoch/
  `TimeBase`/`InputStamp`/`TimestampProvenance` は media-core が単一の
  語彙所有者（device stamp・`EngineFrame`・`ClockAnchor`/`map_stamp_to_frame`
  と同一 identity 空間）であり、observability が独自の clock 語彙を作ると
  同一概念の重複定義になる。media-core は「値型中心・Copy・alloc なし」で
  RT 適合が保証された pure な依存先。

いずれも **葉 primitive への一方向依存**であり、逆方向
（`ruviform-rt → ruviform-observability`、`ruviform-media-core →
ruviform-observability`）は作らない。外部 crates.io 依存・feature 増設はなし。

### rt_bridge: 輸送契約

```rust
pub fn bridge<R>(capacity: usize) -> (RtRecorder<R>, RecordSink<R>)
where
    R: Copy + Send + 'static;
```

- `capacity` はリングの slot 数。`capacity == 0` は `ruviform_rt::channel` と
  同じく panic（契約として test で pin）。構築時にのみ確保が起き、
  RT パスでは一切確保しない。
- プロデューサ基数は **SPSC 単一**。`RtRecorder`/`RecordSink` はそれぞれ
  1 本ずつ所有し、`Send` で RT/非 RT スレッドへ受け渡せるが `Clone` は
  提供しない（`ruviform_rt::channel` の既約束と同じ）。
- 内部表現: `ruviform_rt::channel::<Envelope<R>>(capacity)`、
  `Envelope<R> { record: R, dropped_before: u64 }`（private）。
- `RtRecorder` は `Producer<Envelope<R>>`・`recorded: Counter`・
  `dropped: Counter`・`pending_dropped: u64` を持つ。
  `RecordSink` は `Consumer<Envelope<R>>`・`recorded: CounterReader`・
  `dropped: CounterReader`・`drained_total: u64` を持つ。
- `Counter`/`CounterReader` は既存 `core::counter()` の共有 cell で、
  RT 側 write・非 RT 側 read の `Relaxed` 単調カウンタ（OBS-02 の RT 契約と同一）。

#### `RtRecorder`（RT 側）

```rust
impl<R: Copy + Send + 'static> RtRecorder<R> {
    /// RT-suitable: bounded・non-blocking・no alloc・no lock。
    /// 成功なら envelope に `pending_dropped` を焼き込み `recorded`++。
    /// 満杯なら record を drop し `pending_dropped`++・`dropped`++ で `false`。
    pub fn try_record(&mut self, record: R) -> bool;
    /// RT-suitable: 共有 `dropped` カウンタの現在値。
    pub fn dropped(&self) -> u64;
    /// RT-suitable: 共有 `recorded` カウンタの現在値。
    pub fn recorded(&self) -> u64;
}
```

- `try_record` は `Producer::try_push` を 1 回だけ呼ぶ。`TryPushError::Full`
  返却時は `record` をそのまま捨てる（`Copy` 値をコピーで保持するだけで
  追加 alloc は起きない）。成功時は `Envelope { record, dropped_before:
  self.pending_dropped }` を push し `pending_dropped = 0` に戻す。
- メモリ順序: リングの可視性は `ruviform_rt::channel` の内部 ordering
  （rtrb 系 SPSC の head/tail Release/Acquire）に委譲する。`Counter` の
  `recorded`/`dropped`/`pending_dropped` は統計用で `Relaxed` のみ。
  RT 側に `Release`/`Acquire` フェンスを自前で追加しない。
- `RtRecorder` の `Drop` は非 RT 契約（`Producer` drop はリングを解放しないが、
  RT スレッドで最終的な `Arc` 解放・dealloc が走る可能性を排除するため、
  RT セクション内での recorder 破棄は契約違反とする）。

#### `RecordSink`（非 RT 側）

```rust
impl<R: Copy + Send + 'static> RecordSink<R> {
    /// 利用可能な分だけ FIFO で `f` に `ReceivedRecord<R>` を渡す。
    /// 戻り値は実際に drain した件数（bounded by capacity）。
    pub fn drain(&mut self, f: impl FnMut(ReceivedRecord<R>)) -> usize;
    /// writer 静止後の最終 drain。契約上は drain と同じく bounded・非ブロッキング。
    pub fn drain_final(&mut self, f: impl FnMut(ReceivedRecord<R>)) -> usize;
    /// 共有 `recorded` カウンタ（受理総数）。
    pub fn recorded(&self) -> u64;
    /// 共有 `dropped` カウンタ（欠落総数）。
    pub fn dropped(&self) -> u64;
    /// これまで drain した総数。
    pub fn drained_total(&self) -> u64;
    /// consumer lag = `recorded() - drained_total()`。
    pub fn lag(&self) -> u64;
    /// `CounterReader` clone（OBS-02 登録用）。
    pub fn recorded_reader(&self) -> CounterReader;
    pub fn dropped_reader(&self) -> CounterReader;
}
```

- `drain` は `Consumer::try_pop` を空になるまで繰り返す（上限 `capacity`）。
  `f` には `ReceivedRecord { record, dropped_before }` を渡す。
- `drain_final` は「writer が静止した後の最終 flush」という契約名で、実装は
  `drain` と同一（quiescence は呼び出し側の責務）。名前を分けるのは
  `SourceLifecycle::FinalDrain` との対応を呼び出し側コードで明示するため。
- `RecordSink` の `Drop` でリングが解放される。RT スレッドでは絶対に drop
  しない。
- `lag()` は「書かれたがまだ読まれていない」件数。欠落は `dropped()` とは
  別系統で、lag が大きい=配達が遅れているだけで失われてはいない。

### record: Clock Context

```rust
/// RT 時点の clock-domain スタンプ。`stamp` は media-core::InputStamp
/// （ticks+base+domain+provenance）、`epoch` は domain の世代。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClockContext {
    stamp: InputStamp,
    epoch: ClockEpochId,
}
```

- `stamp.domain` = `ClockDomainId`（`UNKNOWN`/`SAMPLE_CLOCK`/`MONOTONIC`/
  backend 割当）、`stamp.ticks` = domain 時計での position（frame/sample
  position 相当）、`stamp.base` = `TimeBase`（sample rate / timebase）、
  `stamp.provenance` = `Carried`/`Derived`/`Estimated`。
- `epoch` = `ClockEpochId`（domain 時計の世代。engine epoch や backend の
  clock 世代の区別に使う）。`generation` は header 側の `generation: u64`
  （domain 宣言の世代・plan generation 等）と区別する。
- `Option<ClockContext>` を採用: `None` = 「この record は clock stamp を
  持たない（unstamped）」、 `Some(Estimated)` = 「stamp 自体はあるが推定値」。
  両者を区別する。
- 変換禁止の約束: `ClockContext` は wall/monotonic への変換 API を持たない。
  `recorded_at` 等の「収集時刻」を表すフィールドは持たない。
  `media-core` の `ClockAnchor`/`map_stamp_to_frame` のみが stamp↔frame の
  単一 authority であり、`MapRejection` は「変換不能」の区別を保持する。

### record: Correlation

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CorrelationToken(NonZeroU64);          // fixed-size・RT でも保持可

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CorrelationLink { token: CorrelationToken, epoch: u64 }

pub struct CorrelationBook { /* non-RT issuer/registry */ }

#[non_exhaustive]
pub enum CorrelationStatus {
    Linked,
    Unlinked(UnlinkedReason),
}

#[non_exhaustive]
pub enum UnlinkedReason {
    /// Book に未発行の token。
    UnknownToken,
    /// 発行後に remint されて epoch が進んだ（旧 link は無効）。
    SupersededEpoch,
    /// `expire` で tombstone 化された token。
    Expired,
}
```

- token は `NonZeroU64`（`0` は「link なし」と区別するため予約）。
  `CorrelationLink` は token+epoch の固定長で、record header に
  `Option<CorrelationLink>` として入る（`None` = 意図的に link しない）。
- `CorrelationBook` は非 RT 所有（`HashMap<u64,u64>` live + `HashSet<u64>`
  expired tombstone）。操作:
  - `issue() -> CorrelationLink`: 新 token（単調採番・0 skip）+ epoch 0。
  - `remint(token) -> Option<CorrelationLink>`: 既存 token の epoch++ で
    再発行。旧 epoch の link は以後 `SupersededEpoch`。
  - `expire(token)`: token を tombstone 化。以後 `Expired`。
  - `classify(link) -> CorrelationStatus`: live map で epoch 一致→`Linked`、
    epoch 不一致→`Unlinked(SupersededEpoch)`、live 外で tombstone 有→
    `Unlinked(Expired)`、いずれもなし→`Unlinked(UnknownToken)`。
- `CorrelationToken`/`Link` は `SourceId`/`SourceGeneration`（OBS-02 の
  登録識別）や Trace ID/Grant ID とは別軸であることを doc と試験で固定する。

### record: EventRecord

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EventId(u32);                    // new/get

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecordHeader {
    event: EventId,
    version: u16,
    source: SourceIdentity,                  // OBS-02 再利用
    generation: u64,                         // domain 宣言の世代（plan 等）
    correlation: Option<CorrelationLink>,
    clock: Option<ClockContext>,
    confidentiality: Confidentiality,        // OBS-02 再利用
    impact: ReportingImpact,                 // OBS-02 再利用
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EventRecord {
    header: RecordHeader,
    value: u64,
    reserved: [u64; 2],
}
```

- `RecordHeader::new(event, version, source)` + `with_generation`/
  `with_correlation`/`with_clock`/`with_confidentiality`/`with_impact` の
  const チェイン。既定: `generation=0`・`correlation=None`・`clock=None`・
  `confidentiality=Internal`・`impact=Normal`。
- `EventRecord::new(header, value)`、`header()`/`value()`/`reserved()`。
  `reserved` は `[0; 2]` を今版の正とし、将来の拡張予約を doc で明記する
  （`size_of::<EventRecord>() <= 128` を golden test で pin）。
- `EventRecord` は `Copy + Send + 'static` で `bridge<EventRecord>` にそのまま
  使える。raw PCM/signal のような可変長・生バッファは `EventRecord` に
  入らない（`Copy` + `value: u64` の固定形で構造的に排除）。`confidentiality`
  は「record が運ぶ情報の機密分類」であり、PCM/signal を無条件でログ化
  しないという issue の方針は「`EventRecord` には buffer を持たせない」
  構造側で担保する。

## Data and Control Flow

```text
RT thread (writer)                         non-RT thread (collector)
-----------------                          --------------------------
bridge(cap) ──> (RtRecorder, RecordSink)
  │
  │ try_record(R)
  │   Envelope { record, dropped_before = pending_dropped }
  │   Producer::try_push  ──────────────►  ring<Envelope<R>> (bounded)
  │   success: recorded++ (Relaxed)        Consumer::try_pop ×N
  │   full:    record drop               drain(f) → ReceivedRecord
  │              pending_dropped++         drained_total += n
  │              dropped++ (Relaxed)
  │                                        recorded()/dropped()
  │                                        lag() = recorded - drained_total
  │
  │ (recorder は RT セクション外に戻してから drop)
  │                                        drain_final(f) ← writer 静止後
  │                                        sink drop → ring 解放（非 RT）
```

- writer 静止（quiescence）は「recorder が RT スレッドから外れて drop されるか、
  RT パスから一切呼ばれなくなった状態」で保証する。`RecordSink` は recorder
  drop 後も残留レコードを `drain_final` できる（consumer が ring の
  `Arc` を保持するため）。
- `SourceLifecycle` との統合: `dropped_reader`/`recorded_reader` を
  `CounterSource` として OBS-02 登録 → `RegistrationLease::retire` →
  `begin_final_drain` → `drain_final` → `mark_removed`。Reader 所有権は
  Registrar/Collector 側に残る。

### RT セッション境界（Prepare / Publish / ON / OFF / Retire）

| セッション段階 | 操作 | 実行場所 |
| --- | --- | --- |
| Prepare | `bridge(capacity)` で endpoints 構築（唯一の確保点） | 非 RT |
| Publish | `RtRecorder` を RT プロデューサ側へ `move` / 引き渡し | 非 RT |
| ON（RT 実行中） | `try_record`・`dropped`/`recorded` 読みのみ | RT |
| OFF | `RtRecorder` を RT セクションから外へ `move` 戻す | 非 RT |
| Retire | `RtRecorder` drop + `RecordSink::drain_final`（writer 静止後） | 非 RT |
| FinalDestroy | `RecordSink` drop → ring 解放 | 非 RT |

「writer 静止」は OFF→Retire の遷移（recorder が RT パスに存在しないか
drop 済み）で保証する。RT パスが recorder を保持している間の `drain` は
契約内だが、`drain_final` は静止後に限る。

## Non-Functional Constraints

- RT 側で `alloc`/`dealloc`/OS I/O/lock/`tracing`/`format!`/subscriber を
  使わない（`assert_no_alloc` の RT レーンで pin）。
- `try_record`/`dropped`/`recorded` は bounded latency・non-blocking。
- `drain`/`drain_final` は `capacity` 上限の bounded 反復で non-blocking
  （`try_pop` を空になるまで回すだけ。sleep/condvar なし）。
- `Copy + Send + 'static` のみ。`Drop` を持つ payload は compile 時点で
  拒否される（`Copy` 制約で `Drop` 実装型は使えない）。
- 全公開 enum は `#[non_exhaustive]`。`unsafe` は使わない。
- `ruviform-rt`/`ruviform-media-core` 以外の依存・feature 追加なし。
- **ABI/DSP/コスト互換**: 既存クレートの公開 API・型レイアウト・DSP 値は
  一切変更しない（追加のみ）。RT パスのコストは `try_push` 1 回 +
  `Relaxed` increment 高々 2 回（成功時 `recorded`・失敗時 `dropped`）で、
  既存 `rt` の channel コストに単調カウンタ 1 個分を足したもの。
- **PoC 測定値との区別**: Issue #104 の PoC で得た alloc/レイテンシ観測は
  「固定長 Copy + drop-on-full が RT 安全に動く」という構造証拠であり、
  本実装の性能保証値ではない。OS/CPU 別の計測証跡は OBS-08（#290）に委ね、
  本 Issue では契約試験（no-alloc・bounded）のみを pin する。

## Risks and Trade-offs

- **`dropped_before` envelope のコスト**: 1 slot あたり `+8` byte。利点は
  「どの record の前に何件失われたか」を export が復元できることで、
  shared counter だけでは得られない位置情報。採用。
- **`recorded`/`dropped` の共有 counter**: writer 側の余計な atomic 1 回/回で
  済む。`RtRecorder` が `pending_dropped`（plain field）で envelope に焼き、
  `dropped`（`Relaxed`）で可視化する 2 系統は意図的（前者は per-record
  envelope、後者は O(1) 全体欠落）。
- **SPSC 単一**: multi-producer は scope 外（rt に MPMC が存在しない現状と
  一致）。将来必要になれば別 Issue で `channel` 側の拡張として検討。
- **`ClockContext` に media-core 依存**: 独自 clock 語彙を避けるための
  一方向依存。`InputStamp`/`ClockEpochId`/`TimeBase`/`TimestampProvenance`
  は Copy・`const`・RT 適合の pure 値型なので RT パスで安全。
- **`drain_final` の quiescence は呼び出し側責務**: sink 側で writer の
  静止を機械的に検知する機構は持たない（writer drop/RT セクション退出で
  保証）。doc + test で契約を固定する。

## Deferred Design Concerns

- Collector/Registry/snapshot の実装と集約ポリシー → OBS-07（#289）。
- `tracing` への export 変換・severity/grant mapping → OBS-03（#278）。
- 性能測定・on-hardware 証跡 → OBS-08（#290）。
- MPMC bridge・可変長 record・backpressure → 別 Issue。
- `RecordHeader.reserved` の将来 semantic（現版は `[0;2]` のまま）。

## Verification Notes

- `rt_` 契約試験（`assert_no_alloc` 窓）で `try_record` 成功/満杯 drop/
  `dropped`/`recorded` が no-alloc を pin。
- FIFO・drop-on-full・受理/欠落/lag/drain_final/quiescence を単体試験で pin。
- Clock/Correlation/EventRecord の golden 試験で「変換しない・区別する・
  混同しない」を固定。
- `SourceLifecycle` 統合試験で Retire→FinalDrain→Removed と最終 drain が
  矛盾しないことを pin。
- `mise run fmt`/`check`/`lint`/`test`/`test-rt` を全緑で通し、
  `git merge origin/main` 後に再検証する。
