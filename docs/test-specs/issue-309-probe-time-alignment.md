---
id: "TSPEC-034Uja9CLADV2KHCRHEtFS"
type: "test-spec"
status: "approved"
title: "Issue #309 — Probe時間整列・欠落比較の検証契約"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/309"
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
  related:
    - "docs/designs/issue-309-probe-time-context-and-alignment.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-309-probe-time-context-and-alignment.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-309-probe-time-frame-clock-epoch-alignment.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #309 — Probe時間整列・欠落比較の検証契約

## Purpose

Issue #309 の PROBE-TIME 契約 — `ProbeClockMark`/`ProbeTimeContext`・`align_observation_points` の `Aligned`/`Conditional`/`Incomparable` 分類・coverage/loss mask 再構成 — が将来の実装で実際に保証されるべき性質を固定する。特に「異なる時間系列を同一座標と誤認しない」「測定無音と未観測を混同しない」「RT 契約を壊さない」という #185/#308 の既存保証を本契約が退行させないことを検証対象とする。実装が設計と異なる形で確定した場合は本 spec を改訂する。

## Feature

複数 Probe 観測の汎用 Frame/Clock/Epoch/Latency 整列可能性と欠落比較（PROBE-TIME）

## Rules

### Rule: clock identity が無い観測は決して Aligned にならない

- `RecordMeta` の clock mark が `domain = ClockDomainId::UNKNOWN`（未配線・`SetClockMark` 未到達）の record を含む `ObservationSpan` は、policy に関わらず `Incomparable{reason: UnknownClock}` を返す。
- 2 つの span が共に `UNKNOWN` domain を持っても「両方 unknown 同士」で `Aligned`/`Conditional` に昇格しない（`same_clock_domain` 規則: equal かつ非 `UNKNOWN` のみ同一時計）。
- `domain` が異なる、または同一 domain で `epoch` が異なる span 対は `Incomparable{DomainMismatch}`/`Incomparable{EpochMismatch}`（anchor による写像根拠がある場合のみ `EngineFrame` basis の `Conditional`/`Aligned` を許す）。

### Rule: 整列は basis と誤差上限を証拠として返し、無条件昇格しない

- 同一 `(domain, epoch)` の `SharedCursor` basis で、latency が `Known`・format（channels/sample rate/timebase）一致・coverage が比較区間で連続有効なら `Aligned{basis: SharedCursor, offsets, error_budget}` を返し、offset は basis 座標の frame 差として検証可能である。
- latency `Unknown` は `AlignPolicy.require_known_latency` で `Incomparable{LatencyUnknown}`、bounded 許容 policy でのみ `Conditional{LatencyBounded{bound}}`。既知 0 として扱わない。
- sample rate/timebase 不一致で変換根拠（`TimeBase` 有理数+明示丸め）が無ければ `Incomparable{RateMismatch}`。overflow/underrun・区間の不連続（epoch 境界・generation 境界を跨ぐ）は `Incomparable{DiscontinuityInWindow}` または policy 次第で `Conditional`。
- `error_budget` は 0 でなくてもよいが必ず根拠（丸め残差・provenance）を伴い、`Estimated` provenance の anchor/stamp は `Conditional{ProvenanceEstimated}` へ降格する。

### Rule: generation 境界は座標不連続ではなく観測対象の不連続として扱う

- 同一 `(domain, epoch)` で `frame_cursor` が連続していても、`generation_range` が異なる plan generation を跨ぐ span 対は、`require_same_generation` policy で `Incomparable{GenerationSpan}` または `Conditional{CrossGeneration}` を返す。#308 の `RebindReceipt`/`CaptureStopped{GenerationBoundary}`/`RebindDropped` status から境界が再構成できる。
- publish（generation 変更）だけでは epoch は変わらず、epoch bump 宣言だけでは generation は変わらない — 両者を互いの代用にしない。

### Rule: coverage/loss mask が無音と未観測を区別する

- `MetricRecord.levels` の Peak=0/RMS=0 は `Valid` 区間内の「測定されたデジタル無音」であり、`Unobserved`/`LostBlocks`/`LostRecords`/`Off`/`Unsupported`/`Unavailable`/`Unknown` 区間には無音値を割り当てない。
- `ObservationCoverage` は status record の `sample_position`・chunk/metric の `[sample_position,+frames)`・`lost_records`/`lost_blocks`/`unobserved`/`unbound` counter から再構成され、telemetry 欠落（`lost_records>0`）の window は `Unknown` に降格する — 有効と偽らない。
- loss mask のある span を「完全連続 PCM 列」として返す API・既定値は存在しない（`ObservationSpan` は常に coverage を参照する）。

### Rule: RT 契約と非 RT bounded 性を維持する

- `SetClockMark` 相当の clock 配線は `Copy` 値の置換のみで、`assert_no_alloc` 窓内の alloc 0・非ブロッキング・OS I/O なしを維持する（#185/#308 の既存 RT 検証を壊さない）。
- `align_observation_points`/coverage 再構成は非 RT で bounded（入力 record 数に線形・明示的な上限または打ち切り）であり、RT thread から到達不能である。
- probe ON/OFF/Capture のいずれでも sink 出力 PCM が bit-identical（#308 test-spec の既存保証を維持）。

## Examples

- Given 2 probe が同一 runtime instance・同一 `(domain, epoch)`・同一 format で、latency が既知かつ異なる値を持つ observation を drain したとき、When `align_observation_points(a, b, policy{require_same_generation:true})` を同一 generation 窓で呼ぶと、Then `Aligned{basis: SharedCursor}` と frame 単位 offset・error_budget が返り、expected offset（latency 差を含む）が一致する。
- Given probe A の epoch が 0、probe B（または同一 probe の後続区間）が epoch 1 の record を持つとき、When align を呼ぶと、Then `Incomparable{EpochMismatch}`（anchor 根拠なし）を返し、両者の `sample_position` が数値的に近くても整列しない。
- Given `SetClockMark` 未送信の deck が生成した record（`domain=UNKNOWN`）と、正常 mark の record を比較するとき、Then `Incomparable{UnknownClock}` を返し、`UNKNOWN==UNKNOWN` の 2 record 対でも `Aligned` にならない。
- Given chunk 列に `[pos, pos+frames)` の飛び（`LostBlocks`）と `Unobserved{Stale}` status を含む span と、連続 `Valid` の span を比較するとき、Then `PartialCoverage` の `Conditional` または policy により `Incomparable` を返し、coverage が飛び区間を `LostBlocks`/`Unobserved` として報告する。飛び区間の「無音」扱いはされない。
- Given telemetry channel 溢れで `lost_records>0` の window を含む span と連続 span を比較するとき、Then 該当 window の coverage は `Unknown` に降格し、有効区間として数えない。

## Guarantees

- `Alignment`/`AlignmentBasis`/`IncomparableReason`/`ObservationCoverage` の公開 enum は `#[non_exhaustive]` を維持する。
- 整列結果は必ず basis・offsets・error_budget・reason のいずれかの証拠を伴い、「理由なしの Aligned」を返す経路が存在しない。
- `ProbeTimeContext` の registry は非 RT・bounded で、record 解釈が registry version に依存しない（identity は record 内の mark が保持）。
- 既存 #185（ex01–ex16）・#308 の acceptance（atomic rebind・PCM bit-identical・loss counter 分離）を退行させない。

## Non-goals

- device session への `OutputCallbackInfo`/`InputCallbackInfo` 配線・`StreamLatency` の context 反映（別 Issue — 本 spec は `anchors`/`LatencyKnowledge` の受け皿のみ）。
- `EngineFrame`/media time anchor の生成実装（render driver/OBS-05 bridge 側の責務）。
- PCM 波形差分の可視化・resample・PCM 相関の自動実行・sound causality 推論（Issue が明示的に禁止/対象外）。
- Synvelis 側の DAW graph reasoning・原因候補・UI（SF-04/SF-06 の責務）。
- AudioLens f64 native 解析との bit-exact 比較（#294 の別検証）。
- cross-session・永続 probe ID。

## Risk

- clock mark の RT 配線が実装で `TapCtx`/`RtProbe` の形と整合しない場合、`RecordMeta` の additive 変更が全 record 生成箇所に波及する — 実装時に tap 点の一貫性を確認する。
- domain 採番 namespace が media-core well-known 値や device domain と衝突すると `same_clock_domain` が誤判定する — registry の採番規則を実装時に検証する。
- coverage 再構成が保守的すぎて有効区間を過小報告する場合、consumer が「観測なし」と誤認しうる — `Unknown` と `Valid` の区別を UI/consumer 契約で明示する。
- `error_budget` の算出根拠が曖昧だと `Aligned` の信頼性が形骸化する — provenance・丸め残差の内訳を結果に含めることを実装で確認する。
