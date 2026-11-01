---
id: "ADR-20261102-transport-span-scheduler-applied-receipt"
type: "adr"
status: "accepted"
title: "Generic Transport Span Scheduler・Applied Receipt・Clock/Plan Generation 契約と Plugin min/max Block 方針"
created: "2026-11-02"
updated: "2026-11-02"
owners: ["scarecrow173"]
metadata:
  adr:
    supersedes: []
    decision-makers: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/190"
    - "https://github.com/scarecrow173/synvelis/issues/20"
    - "https://github.com/scarecrow173/synvelis/blob/main/docs/designs/time-transport-midi-automation-plugin-integration-architecture.md"
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
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/plugin-rt-ownership-split.md"
    - "docs/adr/configurable-ppqn-resolution-independent-musical-time.md"
    - "docs/adr/synvelis-driven-generic-execution-boundaries.md"
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-190-transport-span-scheduler.md"
    - "docs/discovery/issue-190-generic-transport-span-scheduler-applied-receipt-clock-plan-generation.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-190-transport-span-scheduler.md"
    - "docs/discovery/issue-190-generic-transport-span-scheduler-applied-receipt-clock-plan-generation.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/195"
    - "https://github.com/scarecrow173/ruviform/issues/196"
    - "https://github.com/free-audio/clap/blob/main/include/clap/events.h"
  defers: []
  deferred-by: []
---

# Generic Transport Span Scheduler・Applied Receipt・Clock/Plan Generation 契約

## Status

**accepted — architectural direction（2026-11-02）。** 本 ADR は Generic
Transport Command→Applied Receipt、Half-open Logical Span、Event time
coordinate、Clock/Plan Generation の契約と、activated `min_frames/max_frames`
を破らない配送方針を確定する。既存 ADR（RT 契約・Plugin RT Ownership・
Variable PPQN・Generic Execution Boundaries・Snapshot Swap）を置換しない。
具象 API は PoC で確定し、本 ADR は既存合意を黙って変更しない。

## Context

[Synvelis v0.5](https://github.com/scarecrow173/synvelis/blob/main/docs/designs/time-transport-midi-automation-plugin-integration-architecture.md)
§3/§4/§5/§14（spec 正本）と [Ruviform #190](https://github.com/scarecrow173/ruviform/issues/190)
が要求する契約: 論理 Transport Span ≠ 物理 Plugin Process Call、Canonical
Engine Frame 正本、Command→Applied Receipt、Epoch/Generation 区別。

現行コードの確定事実（discovery 実測）: `EventList.frame_offset` は process
呼出し開始相対で `offset < block_frames`・非減少・zero-frame flush を検証。
L0 endpoint gate は `min_frames ≤ frames ≤ max_frames` を Reject、L1 は
max のみで min を child contract へ委譲。`TransportStatus`/`TransportInfo`
はあるが Epoch/Generation/Span/Receipt がない。層規約上 engines → 拡張層
（plugin-core）依存は禁止で、canonical 契約を generic consumer が使うには
foundation 層に置く必要がある。

## Decision

1. **Canonical identity（`ruviform-media-core` 所有）:** `EngineFrame`（epoch
   内単調の canonical 位置正本）、`ClockEpochId`、`TransportGeneration`、
   `PlanGeneration`、`TempoMapRevision`、`LoopOccurrence` を区別する
   `Copy` newtype を media-core に置く。異なる `ClockEpochId` の frame を
   比較・混入しない。Device Restart は `ClockEpochId` 更新、Seek/Play/
   Pause/Stop は `TransportGeneration` 更新、Loop Wrap は `LoopOccurrence`
   増分（Clock 断絶でも新 generation でもない）、TempoMap/Plan revision
   切替は `PlanGeneration`/`TempoMapRevision` で区別する。
2. **Command→Applied Receipt:** Play/Pause/Stop/Seek/SetLoop/ClearLoop を
   `Requested → 非RT Validate/Prepare → Pending → RT 有界 Apply` で処理し、
   `AppliedReceipt`（status・requested frame・actual applied `EngineFrame`・
   `TransportGeneration`・`PlanGeneration`・`ClockEpochId`・degradation・
   reject 理由）を返す。Accepted ≠ Applied。即時 Apply を無条件保証しない。
3. **Half-open Logical Span:** callback `[F,F+N)` を Transport Context 不連続
   （seek/status 変更/loop wrap/plan revision 適用点）で `[start,end)` に
   分割する。`end` 境界の event は次 span で 1 回だけ配送。1 process call =
   1 Transport Context snapshot。
4. **Frame coordinate 単一権威:** `EngineFrame` 正本 → callback-relative・
   span-relative・process-relative offset の変換を canonical 契約側の 1
   箇所に定義する。Wire `EventList.frame_offset` は従来どおり **process
   開始相対**で、`offset < block_frames`・非減少・zero-frame flush の既存
   契約を維持する。Span/Callback offset を無検証で転送しない。
5. **Plugin min/max 方針 — Exact-first + 明示選択の有界 Degraded:**
   - capability `{min_frames, max_frames, in_block_transport}` を非RT
     Preflight で評価する。各物理 call は `min ≤ frames ≤ max` を満たすか、
     `len > max` の場合は `len` を `ceil(len/max)` 分割して各 chunk が
     `min` 以上になる場合のみ合法とする。
   - 適法分割を構成できない不連続（例: 256 callback・wrap@100・min=128）は
     **Exact policy（既定・Offline 含む）では Preflight Reject**
     （`UnsupportedExactTransport`）とし、Last-good 状態を維持する。
   - **Live のみ明示選択で `Degraded`** を許し、不連続の適用を次の適法
     境界（`max_deferral` 有界内）へ延期する。Receipt は requested frame と
     actual applied frame・degradation を記録する。Offline Exact Render は
     無断量子化しない。
   - CLAP `CLAP_EVENT_TRANSPORT` 等の block 内 Transport 更新は
     `in_block_transport: Verified` と確認できた Plugin/Adapter に限り将来
     利用可能とし、`Unknown`/`Declared` を対応済みと扱わない。本 PoC は
     `Unknown` 経路のみ。
   - L0/L1 差: L0 は min を lock 内 gate、L1 は child 契約委譲であることを
     capability 判定で区別し、未確認の child 契約を対応済みと推定しない。
6. **Crate 配置:** canonical 契約・scheduler は `ruviform-media-core`（全上位
   層が依存可・transport/time の正本）。plugin wire 変換（span window →
   frames+`EventList`+`TransportInfo`）は wire 型の所有元
   `ruviform-plugin-core` の薄い bridge に置く。新規 crate は今回作らない
   （再検討条件は spec の Deferred 節）。
7. **Event 配送:** `ScheduledEvent` は position 正本・`source_sequence`
   （同 frame 因果順）・`LoopOccurrence` 区別・`plan_generation`/
   `transport_generation` tag を持つ。generation 不一致の event は配送せず
   Defined Fault。同一 occurrence で同一 source event を二重配送しない。
8. **Budget/Fault:** max spans/callback・events/span・pending applies・
   deferral 上界を非RT plan validation で強制する。RT で無制限
   split/alloc/I/O/blocking を行わず、予期しない超過は Defined Fault とし
   て観測する（静かな成功禁止）。
9. **Compatibility:** `TransportInfo`/`TransportStatus`/`EventList`/
   `ProcessConfig`/`MusicalTimeInfo`/`TransportSnapshot` の既存 ABI/wire は
   変更せず、追加型のみ。Synvelis 固有の Project/Clip/Track/Undo/Take/
   Touch 型は公開 API に持ち込まない。

## Considered Options

### Option A: canonical 契約を `ruviform-plugin-core` に集約

- 利点: `EventList`/`TransportInfo`/`ProcessConfig` の語彙が揃う。
- 不採用理由: 層規約で engines → 拡張層の直接依存が禁止され、generic
  consumer（engine/composition/#187/#188/#195）が利用できない。

### Option B: canonical を `ruviform-media-core`、wire bridge を `ruviform-plugin-core`

- 利点: transport/time 正本への自然な集約、全上位層が利用可能、wire 型の
  所有元で変換が完結、新規 crate 不要。
- **採用。**

### Option C: 新規 `ruviform-transport` crate

- 利点: 専用 crate で責務明確。
- 不採用理由: media-core のみ依存なら wire 型を参照できず bridge が別途必要、
  plugin-core 依存なら拡張層順位で engine 利用不可。29→30 crate の保守も
  現段階では利益が小さい。将来再検討（spec Deferred）。

### Option D: `ruviform-rt` に scheduler を置く

- 不採用理由: `ruviform-rt` は無依存で `TempoMap`/`MediaTime` を参照不可。
  media-core 依存追加は Primitives 層規約に反する。

### Option E: min/max 不整合を CLAP in-block Transport Event で解消

- 利点: 分割せず正確な block 内更新。
- 不採用理由: 実 Plugin 対応・adapter 実装・event 順序が未検証。
  `Unknown` capability を対応済みと扱うと TT-027 に反する。将来の拡張枠と
  して `Verified` 経路を型に残す。

### Option F: min/max 不整合を常に Preflight Reject

- 利点: 単純・正確。
- 不採用理由: Live で継続動作が必要な場合に機能しない。明示選択の有界
  Degraded を併設した Option B の方針を採用。

## Consequences and compatibility boundaries

- `media-core` は値型だけでなく immutable prepared plan・RT 有界 scheduler
  状態機械を持つことになる（character の拡張。`exec` ↔ `media-core`
  相互依存禁止は維持）。
- canonical 契約が foundation 層にあるため #186/#187/#188/#195/#196 は同じ
  型を消費できる。Graph RT Generation（#195）と Plugin Adapter（#196）は
  本契約の consumer であり本 Issue の所有範囲外。
- L1 の child 契約未確認は min/max を厳密に評価できないため、capability
  `Unknown` として扱い Exact では Reject 方向に倒す。
- RT fault/dropped event は Receipt・fault 記録で観測可能にし、静かな
  成功・無断 quantum 化・無断 context 偽装を禁止する。
- `Degraded` の延期上界は `ScheduleBudget.max_deferral` で有界。延期不能は
  Reject/Fault。
- 新規実装・試験は Issue #190 の PoC で検証。本 ADR は設計合意であり、
  実装済みを意味しない。

## Acceptance and validation

- TT-006/007/008/009/010/012 相当 + TT-025〜030 を Mock Driver 上で検証。
- `min_frames` 違反 process call を生成しないこと、actual applied frame が
  Receipt で観測できること、stale generation event が誤配送されないこと、
  `rt_` 無割当であることを受入とする。
- 方式の確定は本 ADR で行い、既存合意を黙って変更しない。後続の
  format 別検証（CLAP/VST3）は #196、graph generation は #195 で行う。

**実装完了ではなく、設計上の決定を記録した ADR である。**

## Sources

- 上流: [spec](../specs/issue-190-transport-span-scheduler.md)、
  [discovery](../discovery/issue-190-generic-transport-span-scheduler-applied-receipt-clock-plan-generation.md)。
- 下流: [design](../designs/issue-190-transport-span-scheduler.md)。
