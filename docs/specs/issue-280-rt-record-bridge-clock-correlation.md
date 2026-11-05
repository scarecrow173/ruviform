---
id: "SPEC-ML6zMi4szNWY33LanXM0JL"
type: "spec"
status: "approved"
title: "Issue #280 OBS-05 RT固定長Record Bridge・Clock・欠落・Correlation統合"
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
    - "https://github.com/scarecrow173/ruviform/issues/280"
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
    - "docs/specs/issue-277-observability-core-source-contract.md"
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

# Issue #280 OBS-05 RT固定長Record Bridge・Clock・欠落・Correlation統合

## What Should Be Built

既存 `ruviform-observability` クレート内に、RT スレッドから非 RT 集約側へ
固定長レコードを届ける **RT Record Bridge**（`rt_bridge` モジュール）と、
レコードが運ぶ **Clock Context・欠落（loss）情報・Correlation 連携**を表す
固定長語彙（`record` モジュール）を定義・実装する。

- `bridge<R>(capacity)` が `(RtRecorder<R>, RecordSink<R>)` を返す。
  RT 側は `RtRecorder::try_record(record)` のみを呼び、非 RT 側は
  `RecordSink::drain`/`drain_final` で `ReceivedRecord<R>` を受け取る。
- 輸送は既存 `ruviform_rt::channel` の有界 SPSC リングを再利用する
  （第 2 のリング実装は作らない）。満杯時は **drop-on-full** で、受理件数・
  欠落件数・直前欠落数を明示的に計測し、欠落の発生を Collector/export へ
  保持して渡す。
- `record` モジュールは Clock Context（Clock Domain + Epoch + position +
  timebase + provenance の有効性）、固定長 Correlation token/link/book、
  汎用の `EventRecord`（`RecordHeader` + 固定 payload + reserved）を提供する。
- 欠落計数・受理計数は既存 `Counter`/`CounterReader`（OBS-02）として露出し、
  `SourceId`/`SourceGeneration`/`RegistrationLease`/`SourceLifecycle` の
  既存契約にそのまま登録できる。新しい Source 識別機構は増やさない。

## Why Now

OBS-01/02/04 で「軽量 Counter + Source 登録/寿命」は揃ったが、RT スレッドから
固定長レコードを安全に出す経路がまだ存在しない。Issue #275（統合設計正本）と
`shared-foundations-and-modularization-policy` は `rt_bridge` を
`ruviform-observability` の構成候補責務として指名しており、#104 の PoC は
「固定長 `Copy` レコード + drop-on-full + overflow counter」が RT 安全に
動くことを示している。本 Issue は PoC の知見を**プロダクション契約**として
固定し、後続の OBS-03 export（#278）・OBS-07 Collector（#289）が
再解釈不要な語彙と輸送境界を提供する。

## Users and Value

- **Domain 側（audio engine/probe 等）**: RT コールバック内で
  `try_record` 一発で固定長イベントを出せる。alloc・lock・I/O・subscriber への
  依存を持ち込まない。
- **Consumer/Collector 側（OBS-07・OBS-03・製品側）**: 受理/欠落件数と
  位置付きの欠落ギャップ（`dropped_before`）を受け取り、「何件・どこで・どの
  epoch/domain のレコードが失われたか」をエクスポートできる。Correlation link
  の有効性を `CorrelationBook` で検証し、期限切れ/再 mint は unlinked として
  扱う。

## Scope

### In Scope

- `crates/ruviform-observability` に `rt_bridge` モジュールを追加:
  `bridge`/`RtRecorder`/`RecordSink`/`ReceivedRecord`。
- `crates/ruviform-observability` に `record` モジュールを追加:
  `ClockContext`・`CorrelationToken`/`CorrelationLink`/`CorrelationBook`/
  `CorrelationStatus`/`UnlinkedReason`・`EventId`/`RecordHeader`/`EventRecord`。
- `Cargo.toml` への `ruviform-rt`・`ruviform-media-core` への **path 依存追加**
  （下向き・一方向・葉への依存。逆方向 `ruviform-rt → ruviform-observability`
  は作らない）。
- 試験: `rt_` 契約試験（`assert_no_alloc` 窓）・FIFO/drop-on-full/受理・欠落
  計測・consumer lag・source quiescence・clock domain/epoch/provenance・
  correlation linked/unlinked・EventRecord 形状・OBS-02 寿命契約との統合試験。
- `crates/ruviform-observability/README.md`・`crates/README.md`・crate-level
  doc の更新（RT 契約表・依存方針の差分を明記）。

### Out of Scope

- 新規独立 tracing/observability クレートの新設、`tracing`/`tracing-subscriber`
  や外部 IO/Subscriber への依存追加。
- `ruviform-rt` 側への新規実装（channel/retire/slot の変更を含む）、
  `ruviform-rt → ruviform-observability` 方向の依存。
- 実 Collector/Registry/History・snapshot 変換・`tracing` への export 変換
  （OBS-07=#289・OBS-03=#278 の責務。本 Issue はそれらが消費できる契約のみ）。
- 性能測定レーン/on-hardware 証跡（OBS-08=#290）、実 Plugin/実 GUI/実機
  デバイスでの検証。
- #308（Probe Rebind ライフサイクル）・#309（MultiProbe 非 RT 整列比較）の
  責務の重複実装。
- Multi-producer (MPMC) bridge、容量可変 ring、欠落レコードの復元機構。

## Requirements

- 固定長 Record payload は `Copy + Send + 'static`（静的にサイズが確定し、
  RT パスで確保しない）。`EventRecord` は `RecordHeader`（`EventId`・
  `version`・`SourceIdentity`・`generation`・`correlation`・`clock`・
  `confidentiality`・`impact`）+ `value: u64` + `reserved: [u64; 2]` の
  固定形とする。
- RT 側公開操作は `try_record` とカウンタ読みのみとし、`no alloc・no lock・
  no OS I/O・no subscriber・bounded latency` を試験で pin する。
- 満杯時は drop-on-full。戻り値 `false` と共有 `dropped` counter の両方で
  欠落を見せる。欠落レコードは後で回復できるとみなさない。
- `ReceivedRecord` は `record: R` と `dropped_before: u64`（直前に欠落した
  件数の envelope 情報）を運び、export 側が「この record の前に何件失われた
  か」を再構築できる形にする。
- Clock Context は `media-core` の `ClockDomainId`/`ClockEpochId`/`InputStamp`
  （`ticks`+`base`+`domain`+`provenance`）を構成して使い、**時刻を事後に
  別セマンティクスへ変換・捏造しない**。device/StreamInstant 由来の時刻は
  backend 証拠が揃う場合のみ Carried として扱う。
- Correlation token は固定長（`NonZeroU64`）で、`CorrelationBook`（非 RT）が
  `issue`/`remint`/`expire`/`classify` を提供する。token epoch が合わない・
  expire 済み・未発行の link は `Unlinked` として扱い、Trace ID/Grant ID/
  `SourceId`/`SourceGeneration` と混同しない。
- Bridge の生成・再配置・clone・最終破棄は非 RT 操作。`RecordSink` drop で
  ring が解放されるため、RT スレッドで最終 Drop を走らせない。
- `SourceId`/`SourceGeneration`/`SourceIdentity`/`RegistrationLease`/
  `SourceLifecycle` は OBS-02 のまま再利用する。Reader 所有権は
  Registrar/Collector 側に残る。
- 全公開 enum は `#[non_exhaustive]`。MSRV 1.89・`workspace.lints`・
  `-D warnings` ゲートを通す。`unsafe` は使わない。
- 依存方針: `ruviform-observability` は `ruviform-rt`（SPSC/時刻 primitive）
  と `ruviform-media-core`（時刻・domain/epoch 語彙）への一方向依存のみ追加し、
  他の ruviform クレート・外部依存・feature 増設はしない。

## Acceptance Criteria

- [ ] AC-1: `bridge(capacity)` が `(RtRecorder, RecordSink)` を返し、RT 側が
  `try_record` だけで固定長レコードを bounded・non-blocking に送出できる
  （`rt_` 契約試験で no-alloc を pin）。
- [ ] AC-2: 満杯時 drop-on-full で `dropped` が増え、`ReceivedRecord.dropped_before`
  が直前欠落数を保持する。受理件数 `recorded`・欠落件数 `dropped`・
  `drained_total`・consumer lag が観測できる。
- [ ] AC-3: `RecordSink::drain`/`drain_final` が FIFO で `ReceivedRecord` を
  届け、writer 静止後の最終 drain までを一貫して処理する。recorder drop 後も
  sink 側は残留レコードを drain できる（quiescence 契約を試験で固定）。
- [ ] AC-4: Clock Context が `ClockDomainId`/`ClockEpochId`/position/timebase/
  provenance を保持し、`Option<ClockContext>` の `None` と `Estimated` を
  区別する。事後に wall/monotonic 等の別時刻へ変換する API を提供しない
  （golden 試験で固定）。
- [ ] AC-5: Correlation token/link/book が linked/unlinked（`SupersededEpoch`・
  `Expired`・`UnknownToken`）を型で区別し、header の `correlation: None` と
  混同しない。`SourceId`/`SourceGeneration` と同一視しないことを試験で固定。
- [ ] AC-6: 欠落・受理カウンタが `Counter`/`CounterReader` として露出し、
  `SourceLifecycle`（Active→Retiring→FinalDrain→Removed）と組み合わせた
  登録・退役・最終 drain が矛盾なく動く（統合試験で固定）。
- [ ] AC-7: 依存 DAG が下向き一方向（observability → rt/media-core）に限定され、
  `ruviform-rt → ruviform-observability` のような逆依存・外部依存追加がない。
- [ ] AC-8: `mise run fmt`/`check`/`lint`/`test`/`test-rt` が全緑で、Issue の
  非対象（MPMC・変動長・timestamp 捏造・subscriber 連携等）を実装しないことが
  設計と試験で固定される。

## Deferred Design Concerns

- Collector/Registry/snapshot 変換と集約戦略 → OBS-07（#289）。
- `tracing` span/event への変換・export フォーマット → OBS-03（#278）。
- 性能測定・OS/CPU 別証跡 → OBS-08（#290）。
- Multi-producer bridge・リング拡張・backpressure 方針 → 別 Issue。

## Sources and Relations

- Issue #280（本仕様の正本）・#275（Observability 統合設計正本）。
- Issue #277（OBS-02: source/登録/寿命契約）・#279（OBS-04: plugin-host 登録）。
- `docs/designs/foundation-observability-minimal-api.md`（rt_bridge 方針の上位設計）。
- `docs/designs/shared-foundations-and-modularization-policy.md`
  （`rt_bridge` を observability の構成候補責務として指名）。
- `crates/ruviform-rt`（channel/retire/slot の既存実装）・
  `crates/ruviform-media-core`（時刻・ClockDomainId/Epoch/InputStamp 語彙）。
- `crates/ruviform-audio/src/typed/probe.rs`（既存 RecordMeta・drop-newest・
  欠落 counter/status の先行例）。
