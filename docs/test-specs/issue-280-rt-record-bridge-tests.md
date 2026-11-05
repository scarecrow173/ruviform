---
id: "TSPEC-iF1sER54cUJ1FKmvvSy6W2"
type: "test-spec"
status: "approved"
title: "Issue #280 OBS-05 RT record bridge・record vocabulary tests"
created: "2026-11-05"
updated: "2026-11-05"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified: []
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
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/issue-280-rt-record-bridge-clock-correlation.md"
    - "docs/designs/issue-280-rt-record-bridge-clock-correlation.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #280 OBS-05 RT record bridge tests

## Purpose

Issue #280（OBS-05）の `rt_bridge` と `record` 語彙が spec AC-1〜AC-8 を
満たすことを固定する試験仕様。Collector/export 側が「受理件数・欠落件数・
位置付き欠落・clock/correlation の区別」を再解釈せず消費できることを保証する。

## Feature

`ruviform-observability` の RT → 非 RT 固定長 Record Bridge
（`ruviform_rt::channel` SPSC 再利用・drop-on-full・envelope 欠落情報）と、
レコードが運ぶ Clock Context・Correlation link/book・`EventRecord`。

## Rules

- Rule: `bridge(capacity)` は `(RtRecorder<R>, RecordSink<R>)` を返し、
  `capacity == 0` は panic。構築・clone・再配置・最終 drop は非 RT 操作。
- Rule: `try_record` は bounded・non-blocking。成功時 `recorded`++・
  `pending_dropped` を envelope に焼いてリセット。満杯時は record を drop し
  `pending_dropped`++・`dropped`++ で `false` を返す。
- Rule: `RecordSink::drain`/`drain_final` は `ReceivedRecord { record,
  dropped_before }` を FIFO で渡し、戻り値は実 drain 件数（capacity 上限）。
  `drained_total`・`lag() = recorded - drained_total` を保持する。
- Rule: `RtRecorder`/`RecordSink` は `Send`（RT/非 RT スレッドへ移せる）で、
  `Clone` は提供しない。recorder drop 後も sink は残留を `drain_final` できる。
- Rule: `recorded_reader`/`dropped_reader` は `CounterReader` clone を返し、
  OBS-02 の `CounterSource` 登録にそのまま使える。Reader 所有権は Collector 側。
- Rule: `ClockContext` は `InputStamp`（domain+`ticks`+`TimeBase`+provenance）
  + `ClockEpochId` を保持し、`Option<ClockContext>::None`（unstamped）と
  `Some(Estimated)`（推定値）を区別する。wall/monotonic への変換 API を持たない。
- Rule: `CorrelationBook`（非 RT）が `issue`/`remint`/`expire`/`classify` を
  提供し、link は `Linked`/`Unlinked(UnknownToken|SupersededEpoch|Expired)`
  に分類される。header の `correlation: None` は link とは別概念。
- Rule: `EventRecord` は `RecordHeader`（`EventId`/`version`/`SourceIdentity`/
  `generation`/`correlation`/`clock`/`confidentiality`/`impact`）+ `value`+
  `reserved: [u64; 2]` の固定形で `Copy + Send + 'static`。
  `size_of::<EventRecord>() <= 128` を pin。
- Rule: 全公開 enum は `#[non_exhaustive]`、`unsafe` なし、外部依存なし
  （`ruviform-rt`・`ruviform-media-core` のみの一方向依存）。

## Examples

- `rt_bridge_try_record_allocates_nothing`: `assert_no_alloc` 窓内で
  `try_record` 成功/満杯 drop/`dropped`/`recorded` を実行。
- `rt_bridge_fifo`: capacity 分を push し `drain` が同順に `dropped_before=0`
  で返す。
- `rt_bridge_drop_on_full`: capacity 満杯→`try_record` が `false`・`dropped=1`。
  drain で先頭分を取り除き、次 push が `dropped_before=1` を運ぶ。
- `rt_bridge_consumer_lag`: drain なしで `lag()` が増え、`recorded`/`dropped`/
  `drained_total` が一致する。
- `rt_bridge_quiescence`: recorder drop → `drain_final` で残留を全件受領 →
  `lag()==0`。
- `rt_bridge_zero_capacity_panics`: `#[should_panic]`。
- `clock_context_fields`: domain/epoch/`ticks`/`TimeBase`/provenance が
  round-trip し、domain/epoch が違えば `PartialEq` で区別される。
- `clock_context_unstamped_vs_estimated`: `None` と `Some(Estimated)` を区別。
- `correlation_book_linked`: `issue` → `classify` が `Linked`。
- `correlation_book_remint`: `remint` 後の旧 link が `Unlinked(SupersededEpoch)`、
  新 link が `Linked`。
- `correlation_book_expire`: `expire` 後が `Unlinked(Expired)`。
- `correlation_book_unknown`: 未発行 token が `Unlinked(UnknownToken)`。
- `event_record_shape`: `header`/`value`/`reserved` の round-trip・`reserved` が
  `[0;2]`・`size_of` 上限を pin。
- `obs02_lifecycle_integration`: `dropped_reader`/`recorded_reader` を
  `CounterSource` 登録 → `retire` → `begin_final_drain` → `drain_final` →
  `mark_removed` まで一貫して動く。

## Guarantees

- `try_record`・`dropped`・`recorded` は RT-suitable（`rt_` 試験で pin）。
- `drain`/`drain_final`/`drained_total`/`lag`/`recorded`/`dropped`/`*_reader`
  は非 RT 側操作で、bounded・non-blocking。
- `bridge`/`drain`/`drain_final`/`clone`/`drop` は非 RT でのみ呼ぶ契約。
- `received` 件数と `dropped` 件数は別系統で、両者は矛盾しない。

## Non-goals

- 実 Collector/Registry/History・`tracing` export・snapshot 変換の試験
  （OBS-07/OBS-03）。
- MPMC・可変長 payload・block/overwrite・欠落復元の試験。
- 性能測定・on-hardware 証跡・実 Plugin/実 GUI/実機検証（OBS-08）。
- wall 時刻・monotonic 時刻への変換 API の試験（提供しない）。
- `ruviform-rt`/`ruviform-media-core` 側の試験（本 Issue の外）。

## Risk

- `Envelope` は private 実装のため、外部公開 API（`ReceivedRecord`）で
  `dropped_before` を pin する必要がある。
- `lag()` は `recorded - drained_total` の派生値。drain 中に writer が
  増やした場合は差分として表れ、欠落とは混同しない試験が要る。
