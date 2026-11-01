---
id: "IMPL-034QgS3HSwmt1pYqzqoBhg"
type: "impl"
status: "completed"
title: "Issue #190 T-1: canonical transport contract types"
created: "2026-11-01"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/190"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-media-core/src/transport_schedule.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-media-core/src/lib.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-190-t-1-canonical-contract-types.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-190-transport-span-scheduler.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-190-transport-span-scheduler.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-190-transport-span-scheduler.md"
    - "docs/adr/transport-span-scheduler-applied-receipt.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-190-transport-span-scheduler-tests.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #190 T-1: canonical transport contract types

## Summary

`crates/ruviform-media-core/src/transport_schedule.rs` に Issue #190 の正準
generic 契約型を実装した。基盤層に置くことで engine/plugin 層ルール
（foundation → 拡張層依存禁止）を守り、plugin-core 側は薄い bridge のみと
なった。

## Context

task T-1。spec §Contract model と ADR の決定（Exact-first / epoch・generation
分離 / 半開 span / source_sequence stable order）に従う。

## Implementation

### Changes

- 識別子・リビジョン型: `ClockEpochId`, `TransportGeneration`,
  `PlanGeneration`, `TempoMapRevision`, `CommandId`, `LoopOccurrence`。
- `EngineFrame`: epoch スコープ付き。`checked_offset` は同一 epoch 間のみ
  `Some` を返す設計で、epoch 混在を型レベルで防ぐ（`PartialOrd` 非実装は
  意図的 — 順序比較ではなく offset 計算のみ許可）。
- コマンド系: `TransportCommand`（Play/Pause/Stop/Seek/SetLoop/ClearLoop）、
  `AppliedReceipt`（requested→applied frame・status・degradation・各
  generation）、`ApplyStatus`（Requested/Pending/Applied/Rejected）、
  `RejectReason`, `ApplyDegradation`, `FaultKind`, `ScheduleFault`。
- 能力・ポリシー: `BlockContract`（min/max/in_block）、
  `InBlockTransportSupport`（Unknown/Declared/Verified — Unknown・Declared
  は決して supported 扱いしない）、`DeliveryPolicy`（Exact/Degraded）、
  `ScheduleBudget`（spans/events/pending/callback 上限）。
- plan 系: `TransportPlan`（Arc 共有・RT で clone-free 参照）、
  `TransportPlanBuilder`（位置→tick 変換・plan/transport generation 刻印・
  position+source_sequence ソート）、`ScheduledEvent`。
- span 系: `TransportSpan`（半開 `[start,end)`・`deliverable` フラグ）、
  `SpanContext`（status/position/loop/occurrence/epoch/generation/revision）。
- 変換 helper: `event_engine_frame`（stale generation・非 playing・範囲外を
  拒否）、`span_event_window`（半開 position 窓・stable order）、
  `media_time_ticks`。
- 全公開 enum は `#[non_exhaustive]`。

### Decisions During Implementation

- `EngineFrame` に `PartialOrd` を実装しないことで epoch 混在比較を不可能に
  した。窓判定は `checked_offset` の Some/None で行う（plugin bridge 側も
  同手法）。
- plan event の engine frame への写像は PoC では 1:1（tick=frame・varispeed
  なし）。`media_time_ticks` が変換の唯一の場所。

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

- `cargo check -p ruviform-media-core` — pass。
- `mise run lint` — pass（clippy -D warnings、全 workspace）。
- `mise run test` — pass。
- `mise run test-rt` — `rt_run_callback_no_alloc` 含む全 rt_ テスト pass。

## Risks

- Graph RT generation/plan 所有は Issue #195 扱い — 本契約の generation
  刻印は plan builder 入力であり、RT 側の所有権移譲は未配線。

## Follow-ups

- Issue #195: plan/generation の RT 所有権。
- Issue #196: multi-bus/stateful adapter が本契約を消費する際の capability
  attestation（`InBlockTransportSupport::Verified` の証明経路）。
