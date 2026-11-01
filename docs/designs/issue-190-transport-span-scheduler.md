---
id: "DESIGN-034Qh2kLmNoPqRsTuVwX"
type: "design"
status: "approved"
title: "Issue #190 Transport Span Scheduler — 実装設計"
created: "2026-11-02"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/190"
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
    - "docs/specs/issue-190-transport-span-scheduler.md"
    - "docs/adr/transport-span-scheduler-applied-receipt.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-189-rational-beat-variable-ppqn-tempomap.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-190-transport-span-scheduler.md"
    - "docs/adr/transport-span-scheduler-applied-receipt.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/195"
    - "https://github.com/scarecrow173/ruviform/issues/196"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/195"
    - "https://github.com/scarecrow173/ruviform/issues/196"
  deferred-by: []
---

# Issue #190 Transport Span Scheduler — 実装設計

## Overview

ADR 決定に従い、canonical 契約・span scheduler・command→apply pipeline は
`ruviform-media-core` の新モジュール `transport`（現行は `music.rs` 内の
`TransportSnapshot`/`TransportStatus` — 新 module `transport.rs` に拡張）、
plugin wire 変換 bridge は `ruviform-plugin-core` の新モジュール
`transport_bridge.rs` に置く。Mock Driver PoC は
`ruviform-plugin-core/tests/` の統合テストで検証する。

新規 crate は作らない。media-core の既存 `TransportSnapshot`/
`TransportStatus`/`LoopRegion`/`MediaTime`/`TempoMap` はそのまま消費する。

## Architecture

```
ruviform-media-core
  src/transport_schedule.rs        [new]
    EngineFrame, ClockEpochId, TransportGeneration, PlanGeneration,
    TempoMapRevision, LoopOccurrence, CommandId          — identity newtypes (u64/u32)
    TransportCommand { Play, Pause, Stop, Seek(MediaTime),
                       SetLoop(MediaRange), ClearLoop }
    DeliveryPolicy { Exact, Degraded }
    ApplyStatus { Requested, Pending, Applied, Rejected }
    RejectReason { InvalidCommand, UnsupportedExactTransport,
                   StaleGeneration, BudgetExceeded, DeferralExceeded }
    ApplyDegradation { DeferredApply { requested, applied } }
    AppliedReceipt { id, status, requested_frame, applied_frame,
                     transport_generation, plan_generation, clock_epoch,
                     degradation, reason }
    ScheduleFault { kind, at: EngineFrame, detail }       — bounded RT ledger
    ScheduleBudget { max_spans_per_callback, max_events_per_span,
                     max_pending_applies, max_deferral }
    BlockContract { min_frames, max_frames, in_block: InBlockTransportSupport }
    InBlockTransportSupport { Unknown, Declared, Verified }
    ScheduledEvent<P> { position: MediaTime, source_sequence,
                        plan_generation, transport_generation: Option<..>,
                        payload: P }
    TransportPlan { generation, tempo_revision, tempo_map, events,
                    budget-checks }
    TransportPlanBuilder   — nonRT: validate order/budget, assign plan gen
    TransportController    — nonRT: request(), preflight(), receipts drain
    TransportScheduler     — RT: run_callback() → spans+per-call windows

ruviform-plugin-core
  src/transport_bridge.rs          [new]
    BlockContract::from_process_config(&ProcessConfig, in_block)
    SpanWindow→CallPlan      — 1 span → N physical calls (≤max, ≥min)
    SpanDelivery::for_call() — call → (frames, EventList, TransportInfo)
```

## Data Flow

1. **非RT (Prepare):** caller → `TransportController::request(cmd, at)` →
   semantic validate（範囲・status 遷移）→ `preflight()` で cadence +
   `BlockContract` に対する境界 legality 評価 → `Pending` stage or
   `Rejected` receipt。`TransportPlanBuilder::build()` は event 整列・budget・
   generation tag を検査して immutable `Arc<TransportPlan>` を作る。
   `install_plan(plan)` は budget 再検査→`PlanGeneration` bump。
2. **RT (Apply):** `TransportScheduler::run_callback(start, frames, sink)`
   が 1 callback を駆動。境界点集合 = {pending apply due in (F,F+N)} ∪
   {loop wrap frames}。境界で半開 span に分割し、`SpanSink::span(span, calls)`
   で caller へ受け渡す。applies は due frame で有界適用（atomic 状態差替）
   → receipt を bounded ledger に記録。loop wrap は `LoopOccurrence++` で
   position を `loop.start` へ折り返す（epoch/gen は不変）。
3. **Delivery:** sink 実装（plugin-core bridge 経由の Mock Driver）が各
   span を `CallPlan` で物理 call に分割し、call 毎に
   `SpanDelivery::for_call()` で `EventList` + `TransportInfo` を構築して
   `PluginProcessor::process` を呼ぶ。

## Detailed Design

### Engine frame → offsets（単一権威）

`transport_schedule.rs` に `FrameWindow { start: EngineFrame, frames }` と
`EngineFrame::offset_within(window)` を定義。event の `engine_frame` は
plan delivery 時に `position` から派生（PoC は engine frame ↔ position を
1:1、`MediaTime` は既存 `Time` ベース tick — 設計ノート: 1 frame = 1 tick
として固定し varispeed は deferred）。call offset = `event_frame − call_start`。
`[F,F+N)`・`[start,end)` 半開のみ。負 offset は `debug_assert` + 拒否。

### Transport 状態機械（RT 側）

```rust
struct RtTransportState {
  epoch: ClockEpochId, transport_generation: TransportGeneration,
  plan_generation: PlanGeneration,
  status: TransportStatus,           // Stopped|Playing|Paused
  position: MediaTime,               // anchor position at anchor_frame
  anchor_frame: EngineFrame,         // position が確定している frame
  loop_region: Option<MediaRange>,
  occurrence: LoopOccurrence,
}
```

- `position_at(frame)`: `status==Playing` なら `position + (frame − anchor)`、
  else `position`。loop wrap 越境は run_callback が境界として検出し
  `position = loop.start`、`occurrence++`。
- Pending apply の due frame で atomic 差替（`tg++`、Seek は position 変更、
  SetLoop は loop 変更のみで `tg` は据置 — position lineage が不変なため。
  Play/Pause/Stop/Seek は `tg++`）。plan install は `pg`/`tempo_revision` 更新。
- receipts: `Vec<AppliedReceipt>`（capacity = budget.max_pending + α、
  request 時に非RT確保）。RT は push のみ（capacity 内、alloc なし）。
  faults も bounded `Vec<ScheduleFault>`（cap 超過は `fault_overflow`
  counter で表現）。

### Span 分割

`run_callback(F, N, sink)`:

1. pending applies で `due ∈ (F, F+N)` となるものを抽出（順序保持）。
2. loop wrap frames を走査（`position` + loop から wrap 位置を逐次算出 —
   `max_spans_per_callback` まで）。
3. 境界列を昇順安定で走査し `[prev, b)` span を emit。各境界で:
   - pending apply → 適用・receipt 記録
   - loop wrap → `occurrence++`、position 折返し
4. span の legality: `legal_len(len, contract)` —
   `min ≤ len ≤ max` または `len > max && len ≥ min*ceil(len/max)`。
   - Exact かつ illegal → `ScheduleFault::UnrepresentableBoundary` を記録し、
     その不連続の適用は `UnrepresentableApply` fault + receipt Rejected
     （Last-good 状態維持 = 状態は差し替えない）。
   - Degraded → `next_legal_boundary(B, prev, cb_end)` を
     `max_deferral` 有界で走査。見つかれば boundary を B' に移して適用し
     receipt に `DeferredApply{requested:B, applied:B'}` を記録。
     見つからなければ fault + Rejected。
5. `max_spans_per_callback` 超過 → `ScheduleFault::SpanBudgetExceeded` で
   callback を打ち切り（静かな成功しない）。
6. 各 span について `event_window(span)`: plan events で
   `position ∈ [span.start_pos, span.end_pos)` かつ
   `plan_generation == span.pg`、かつ `transport_generation` が `None`
   （absolute）または span の tg と一致するものを安定順に取得。
   occurrence は `position` が loop 内の event を各 wrap で再評価することで
   occurrence 別に 1 回ずつ配送（event の absolute position + occurrence
   内の相対位置で engine_frame を算出 → 二重配送なし）。
   `events > max_events_per_span` → `EventBudgetExceeded` fault、
   span の event 配送を打ち切り。

### CallPlan（plugin-core bridge）

`CallPlan::for_span(span_len, contract) -> impl Iterator<Item=(u32 offset, u32 frames)>`

- `min ≤ len ≤ max` → 1 call。
- `len > max` → `k = ceil(len/max)`、各 chunk = `len/k`（端数を先頭側に配分 —
  各 chunk ≥ `len/k ≥ min` は legality 判定済み前提）。
- `len < min`（Degraded で発生しない／発生時）→ `None`（deliverable でない）。

`SpanDelivery::for_call(ctx, call_start_frame, call_frames, events_window,
 musical_ppqn) -> (TransportInfo, EventList-scratch writes)`:

- `TransportInfo.status = ctx.status`、`position = ctx.position_at(call_start)`、
  `loop_range = ctx.loop_region`、`musical = ctx.tempo_map.time_info(beat, ppqn)`
  （BeatPosition 経由、PPQN は delivery config）。
- events_window 内の `event.engine_frame ∈ [call_start, call_start+frames)`
  → `frame_offset = frame − call_start`（u32）、`EventList::new(&scratch,
  frames)` で既存 wire 検証に通す。0-frame span → `frames=0` flush call のみ。

### Mock Driver（tests/integration）

`plugin-core/tests/transport_scheduler.rs`（feature なし）:

```rust
struct MockDriver {
  scheduler: TransportScheduler,
  contract: BlockContract,
  cadence: Vec<u32>,              // declared callback sizes
  recorded: Vec<RecordedCall>,    // frames, TransportInfo, Vec<(offset,kind)>
}
struct RecordingProcessor { /* PluginProcessor impl: records args */ }
```

- `drive(n_callbacks)`: cadence に従い `run_callback` を走らせ、各 span →
  CallPlan → `SpanDelivery` → processor。
- `request`/`preflight`/`install_plan`/`device_restart` を非RT側で駆動し
  receipts を drain して assertion。

### rt_ テスト

`assert_no_alloc` 系既存ユーティリティ（`media-core` に `rt_` テストあり）を
確認し、`run_callback` が alloc しないこと（plan は `Arc` 不変参照、
pending/faults/receipts は事前確保 Vec、境界走査は stack iterator）を
`rt_transport_scheduler` テストで検証。

## Error Handling

- 非RT request: `RejectReason` 付き receipt を即時発行（InvalidCommand /
  UnsupportedExactTransport / BudgetExceeded）。
- RT: `ScheduleFault` ledger（bounded）+ 該当 command の receipt を
  Rejected に。callback は継続（fail-open でなく defined fault + continue;
  静かな成功ではない）。
- `device_restart()`: `epoch++`、全 pending を Rejected（`StaleGeneration`）、
  plan events の tg bound ものは stale。

## Compatibility

- 追加のみ: 既存 `TransportSnapshot`/`TransportInfo`/`EventList`/
  `ProcessConfig` API 不変。
- consumer 影響: synvelis = impacted（新 API 利用予定・#190 本筋）、
  audio-lens = no impact（既存 wire 不変・追加のみ）。
- `MediaTime`/`MediaRange`/`TempoMap`/`BeatPosition`/`Ppqn` 再利用 — #189
  契約消費、固定 960 再導入なし。

## Open Design Notes

- `transport_generation` tag: plan-compiled event は `None`（absolute）を
  既定とし、`Some(tg)` は position-epoch bound（live inject / one-shot —
  #188 領域だが PoC の stale 検証用に型だけ用意）。
- multi-plugin: contract ごとに CallPlan/Delivery を別評価。span は global、
  delivery legality は per-plugin（#196 で実 adapter 化）。
- `run_callback` の sink trait は media-core で generic (`impl FnMut` でも
  良いが trait object 不要、generic で zero-cost)。

## Sources

- [spec](../specs/issue-190-transport-span-scheduler.md)、
  [ADR](../adr/transport-span-scheduler-applied-receipt.md)、
  [discovery](../discovery/issue-190-generic-transport-span-scheduler-applied-receipt-clock-plan-generation.md)
  に derive。下流は [plan](../plans/implement-issue-190-transport-span-scheduler.md)。
