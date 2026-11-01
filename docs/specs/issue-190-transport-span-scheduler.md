---
id: "SPEC-034Qg9hT0w1z3B9vJkLmNo"
type: "spec"
status: "approved"
title: "Issue #190 Generic Transport Span Scheduler / Applied Receipt / Clock-Plan Generation contract"
created: "2026-11-01"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/190"
    - "https://github.com/scarecrow173/ruviform/issues/192"
    - "https://github.com/scarecrow173/synvelis/issues/20"
    - "https://github.com/scarecrow173/synvelis/issues/22"
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
    - "docs/adr/configurable-ppqn-resolution-independent-musical-time.md"
    - "docs/adr/synvelis-driven-generic-execution-boundaries.md"
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-190-generic-transport-span-scheduler-applied-receipt-clock-plan-generation.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/plugin-rt-ownership-split.md"
    - "docs/specs/issue-189-rational-beat-variable-ppqn-m3.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-190-generic-transport-span-scheduler-applied-receipt-clock-plan-generation.md"
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
    - "docs/adr/synvelis-driven-generic-execution-boundaries.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/187"
    - "https://github.com/scarecrow173/ruviform/issues/188"
    - "https://github.com/scarecrow173/ruviform/issues/186"
    - "https://github.com/scarecrow173/ruviform/issues/195"
    - "https://github.com/scarecrow173/ruviform/issues/196"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/195"
    - "https://github.com/scarecrow173/ruviform/issues/196"
  deferred-by: []
---

# Issue #190 Generic Transport Span Scheduler / Applied Receipt / Clock-Plan Generation contract

## What Should Be Built

`ruviform-media-core` に Clock Epoch / Engine Frame / Transport Generation /
Plan Generation（+TempoMap Revision）を区別する汎用契約型と、
Play/Pause/Stop/Seek/SetLoop/ClearLoop の Generic Transport Command を
非 RT Validate/Prepare → RT 有界 Apply → Applied Receipt で処理する
Transport 実行契約、ならびに Transport Context 不連続で Audio Block を
半開 `[start,end)` Span に分割する Bounded Span Scheduler を実装する。
`ruviform-plugin-core` には span window → 物理 process call
（frames + `EventList` + `TransportInfo`）の薄い delivery bridge を追加し、
activated `min_frames/max_frames` を破る call を生成しない。
Mock Driver 上の PoC で受入条件を検証する。

## Why Now

- Synvelis v0.5 §14（spec 正本）が「論理 Transport Span ≠ 物理 Plugin
  Process Call」「Canonical Engine Frame 正本・各 Origin 相対 offset」
  「Command→Applied Receipt」「min/max を破る分割をしない」を要求して
  おり、Migration #192 M3 の #189（Rational Beat/PPQN）に続く実装段。
- 横断監査（Issue #190 本文）で、256 Frame Callback / Loop Wrap at 100
  （100+156 split）と activated `min_frames > 100` の不整合が確定して
  おり、方式の最終決定を本 Issue の PoC/ADR で行う必要がある。
- 現行 `TransportStatus`/`TransportInfo`/`EventList` には epoch・generation・
  span・receipt がなく、Timed MIDI(#188)・Parameter Event(#186)・Clock
  Device(#187)・Graph RT Generation(#195)・Plugin Adapter(#196) が共有する
  canonical 契約が未整備。

## Users and Value

- **Synvelis（DAW Consumer）:** Project とは独立した汎用 Transport
  Command→Receipt、Loop/Seek の正確な Plugin Context 配送、Loop
  occurrence 区別。
- **AudioLens・Generic Consumer（非 DAW）:** DAW 型なしで
  Epoch/Generation/Span/Fault を観測できる基盤。
- **#186/#187/#188/#195/#196 の実装者:** canonical EngineFrame・
  generation tag・EventList wire 契約・capability 判定の正本仕様。

## Scope

### In Scope

- canonical 型: `EngineFrame` / `ClockEpochId` / `TransportGeneration` /
  `PlanGeneration` / `TempoMapRevision` / `LoopOccurrence` /
  `ScheduledEvent`（canonical position + 因果 source order + generation tag）。
- Generic Transport Command: Play/Pause/Stop/Seek/SetLoop/ClearLoop の
  非 RT Validate/Prepare → Pending → RT 有界 Apply → `AppliedReceipt`
  （`Requested/Pending/Applied/Rejected`・actual applied frame・generation・
  status・degradation）。Accepted ≠ Applied。
- Half-open Span Scheduler: callback `[F,F+N)` を Transport Context 不連続
  （seek/status/loop wrap/plan revision 適用点）で分割。Loop wrap は
  `LoopOccurrence` 増分であって Clock Epoch 断絶でも新
  TransportGeneration でもない。
- Frame coordinate 単一権威: EngineFrame 正本 → callback-relative・
  span-relative・process-relative offset の変換を 1 箇所で定義。
  Wire `EventList.frame_offset` は process 開始相対を維持。
- Capability/Budget: `min_frames`/`max_frames`・in-block transport update
  対応（`Unknown`/`Declared`/`Verified`）・max spans/events/pending を非RT
  Preflight で評価。RT 超過は Defined Fault。`Unknown` capability を
  対応済みと扱わない。
- Plugin delivery policy（ADR で最終決定）: Exact-first — 適法分割を構成
  できない command/plan は非RT Reject（`UnsupportedExactTransport`）。
  Live のみ明示選択で「次の適法境界への延期」を Degraded とし Receipt に
  actual frame・degradation を記録。Offline Exact は無断量子化しない。
- L0/L1 差を含む capability 判定、zero-frame flush、同 frame 因果順、
  loop 多周の occurrence 区別、stale generation event の非配送。
- Mock Driver PoC + 受入テスト + `rt_` 無割当契約テスト + impl record。

### Out of Scope

- Plugin Multi-Bus/Stateful Process Adapter 実装・format 別互換（#196 の
  所有）、Graph RT Generation・Plan 所有権方式（#195 の所有）、Timed MIDI
  Note Ledger（#188）、Parameter coalescing（#186）、Device Clock backend
  実装（#187）。
- Synvelis 固有の Project/Clip/Track/Undo/Take/Touch/Automation Write 型 —
  Ruviform 公開 API へ持ち込まない。
- CLAP `CLAP_EVENT_TRANSPORT` の adapter 実装・実 Plugin 対応検証 —
  capability を `Verified` とみなすための調査対象として残し、本 PoC では
  `Unknown`/`Declared` を対応済みと扱わない。
- 実 audio device / 実 plugin での動作保証（Mock Driver のみ）。
- `TransportInfo`/`TransportStatus`/`EventList`/`ProcessConfig`/`MusicalTimeInfo`
  の既存 ABI・wire 形式の破壊的変更。
- varispeed・非 1:1 の frame↔position 変換レート（PoC は engine frame と
  transport timebase を 1:1 とし、拡張条件は設計に記録）。

## Requirements

- R1: `EngineFrame` は epoch 内で単調な canonical 位置正本とし、
  `ClockEpochId` が変わる frame 系列は比較・混入しない。
  Device Restart は `ClockEpochId` 更新、Transport Seek は
  `TransportGeneration` 更新として別に観測可能にする。
- R2: Command は `Requested → Pending → Applied | Rejected` の状態を持ち、
  Receipt は actual applied `EngineFrame`・`TransportGeneration`・
  `PlanGeneration`・status・degradation を返す。
- R3: callback を半開 span に分割し、span は開始位置・status・loop range・
  occurrence・tempo revision・generation を持つ。`event_frame == span_end`
  の event は次 span で 1 回だけ配送する（重複・消失なし）。
- R4: loop 多周で同一 source event は occurrence 別に 1 回ずつ配送し、
  同一 occurrence で二重配送しない。同 frame event は `source_sequence` の
  安定順序を保持する。
- R5: 1 process call = 1 Transport Context。activated
  `min_frames ≤ frames ≤ max_frames` を満たさない物理 call を生成しない。
  適法分割を構成できない場合は Exact policy では非RT Reject、明示選択の
  Live Degraded では次の適法境界へ延期して actual frame を記録する。
- R6: event の generation tag（plan/transport）が配送時点の span と一致
  しないものは配送せず Defined Fault とする。
- R7: span/event/pending budget は非RT plan validation で強制し、RT で
  無制限 split・alloc・I/O・blocking を行わない。RT での予期しない超過は
  Defined Fault として観測する（静かな成功を禁止）。
- R8: `EventList` の frame offset 昇順・同 offset 安定順序・zero-frame
  flush・`offset < block_frames` の wire 契約を維持し、frame_offset は
  process 開始相対のままとする。
- R9: #189 の `BeatPosition`/`Ppqn`/`TempoMap` 契約を消費し、固定 960 ・
  960/3840 リテラルを再導入しない。`musical` 位置は `TempoMap::time_info`
  の exact rational 経路で生成する。
- R10: Synvelis 固有の Project/Clip/Track/Undo/Take/Touch/Automation Write
  型を公開 API に持ち込まない。

## Acceptance Criteria

- [ ] AC-1: Mock Driver で Play/Pause/Stop/Seek/SetLoop の
  `Requested/Pending/Applied/Rejected` と actual applied `EngineFrame` が
  Receipt で検証できる（TT-008 相当）。
- [ ] AC-2: 256 Frame Callback・Loop Wrap at engine frame 100・
  `min_frames=1` で `[0,100)`+`[100,256)` の 2 span と、各 call の正しい
  `TransportInfo` 位置（TT-006/TT-025）。
- [ ] AC-3: `min_frames=128` で 100-Frame call を**絶対に生成しない**。
  Exact policy では Preflight Reject、明示 Degraded では次の適法境界へ
  延期して actual frame を記録（TT-026）。
- [ ] AC-4: event at engine frame 100/120/256 が、process-relative offset
  0/20/次 callback 0 で重複・消失なく配送され、同 frame の因果順序と
  loop occurrence が保持される（TT-028/TT-007/TT-010）。
- [ ] AC-5: loop 多周で同一 source event が各 occurrence でちょうど 1 回
  配送され、意図しない重複・消失がない（TT-007/TT-029）。
- [ ] AC-6: Tempo Revision/Plan 変更の適用中に、古い generation tag を
  持つ event が誤配送されず Defined Fault として観測される（TA-008 相当）。
- [ ] AC-7: Device Restart は `ClockEpochId` 更新、Seek は
  `TransportGeneration` 更新として別に観測できる（TT-009）。
- [ ] AC-8: RT の span/event/pending budget 超過が Defined Fault/Reject と
  して観測され、無制限 split/alloc/I/O/blocking がない（TT-012 相当）。
- [ ] AC-9: `EventList` の frame offset 昇順・安定順序・zero-frame flush・
  `offset < block_frames` 契約が維持される。
- [ ] AC-10: L0 で `min_frames` gate が実施されること、L1 で child 契約へ
  委譲されることが capability 判定で区別され、`Unknown` capability を
  対応済みと扱わない（TT-027/TT-029）。
- [ ] AC-11: Offline Exact は無断量子化せず Reject、Live Degraded は
  ユーザーが選択した精度低下のみ Receipt に記録される（TT-030）。
- [ ] AC-12: `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` 全緑、
  `rt_` 無割当契約テストを含む。

## Deferred Design Concerns

- `ruviform-transport` 独立 crate 抽出 | 理由: canonical 契約は media-core
  に集約し wire 変換を plugin-core に残す方が現行層と整合するため | 再検討
  条件: generic consumer が複数の下位 foundation に跨る実害が出た場合。
- CLAP `CLAP_EVENT_TRANSPORT` の adapter 実証・実 plugin 対応 — #196 で
  capability=`Verified` 化を判断。
- varispeed / 非 1:1 frame↔position 変換、非整数 rate — 別設計。
- stable slot 型 RT 所有権・plan swap 実方式 — #195。
- Recording session frame・Note safety — #187/#188。

## Sources and Relations

- 要件正本: [ruviform#190](https://github.com/scarecrow173/ruviform/issues/190)。
  spec 正本: [Synvelis v0.5](https://github.com/scarecrow173/synvelis/blob/main/docs/designs/time-transport-midi-automation-plugin-integration-architecture.md)
  §3（Command/Receipt）§4（Half-open Span）§5（Timed Event）§14（min/max
  互換契約 + TT-025〜030）。
- 上位: `docs/designs/synvelis-driven-target-architecture-and-migration.md`
  §5.4、`docs/adr/synvelis-driven-generic-execution-boundaries.md`。
- 前提: `docs/specs/issue-189-rational-beat-variable-ppqn-m3.md`、
  `docs/discovery/issue-194-m0-baseline-audit.md`、
  `docs/discovery/issue-190-generic-transport-span-scheduler-applied-receipt-clock-plan-generation.md`。
- 出力契約先: ruviform #186/#187/#188/#195/#196。
- 下流: [ADR](../adr/transport-span-scheduler-applied-receipt.md)、
  [design](../designs/issue-190-transport-span-scheduler.md)、
  [test-spec](../test-specs/issue-190-transport-span-scheduler-tests.md)
  が本 spec を上流として derive する。
