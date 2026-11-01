---
id: "IMPL-034QgS61F2OZZNh6JVUeV3"
type: "impl"
status: "completed"
title: "Issue #190 T-3: plugin bridge + mock PoC"
created: "2026-11-01"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/190"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-core/src/transport_bridge.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/tests/transport_scheduler.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-core/src/lib.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-190-t-3-plugin-bridge-mock-poc.md"
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

# Issue #190 T-3: plugin bridge + mock PoC

## Summary

`crates/ruviform-plugin-core/src/transport_bridge.rs` に plugin 向け配信
bridge を、`crates/ruviform-plugin-core/tests/transport_scheduler.rs` に
Mock Driver PoC（recording `PluginProcessor`）を実装し、Issue #190 の
受入条件（Synvelis §14 TT-006..030 範囲）をテストで証明した。

## Context

task T-3。bridge は media-core の canonical 契約を plugin wire 契約
（`TransportInfo`・`EventList`・`ProcessConfig`）へ変換する薄い層 — 座標
変換の authority は `transport_schedule.rs` にあり、bridge は再定義しない。

## Implementation

### Changes

- `contract_from_process_config`: activated `ProcessConfig` の
  `min_frames`/`max_frames` → `BlockContract`。`in_block` capability は
  adapter-attested 入力（Unknown/Declared は supported 扱いしない）。L0 は
  gate 内で `frames < min_frames` を拒否、L1 は子 contract へ委譲 — 呼出側は
  実効 activated minimum を渡す。
- `calls_for_span`: span の合法 physical partition（`BlockContract::partition`
  委譲）。undeliverable span は `None` — plugin へ届かない。
- `call_transport_info`: 1 physical call の `TransportInfo` — status、call
  offset の transport position、loop range、tempo map + ppqn 指定時は
  `TempoMap::media_to_beat` + `time_info` による musical snapshot（Issue
  #189 の rational beat 経路・固定 PPQN なし）。
- `build_call_event_list`: span event 窓のうち `[call_start,
  call_start+frames)` に入る event を process-start 相対 `frame_offset` へ
  変換して scratch 配列へ書込・`EventList` で検証。engine frame の半開窓
  判定は `EngineFrame::checked_offset`（epoch-safe・`PartialOrd` 不使用）。
- mock PoC test: `MockDriver`（`SpanSink` impl が span → `calls_for_span` →
  `call_transport_info` + `build_call_event_list` → `RecordingProcessor::
  process`）で 14 acceptance tests。

### Decisions During Implementation

- process 呼出の frame 数は `BufferView::frames()` から plugin が読むため、
  mock driver は call 長に合わせた slice を渡す — 「min_frames 違反 call を
  呼ばない」は「呼出前に partition を要求する」bridge 構造で保証。
- zero-frame flush は callback ではなく wire 契約として直接検証
  （`EventList::new(&[ev@0], 0)` は受理・offset>0 は拒否）。

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

- bridge unit tests 5 件（config 写像・offset 位置・loop range・process
  相対 offset・窓外 skip）— pass。
- integration tests 14 件（`tests/transport_scheduler.rs`）:
  TT-008 lifecycle / TT-025 loop wrap `[0,100)+[100,256)` /
  TT-026a Exact: min 違反 call 非発行 + `UnrepresentableLoopWrap` +
  rejected receipt / TT-026b Degraded `DeferredApply` / TT-027 Unknown・
  Declared 非 supported / TT-028 process 相対 offset（0/20/next-cb-0）/
  TT-029 多周 occurrence・zero-frame flush・L0/L1 / stale generation 非配信 /
  TT-009 restart vs seek / TT-012 span budget fault / TT-010 stable order /
  TT-030 Exact 非 quantize vs Degraded 明示 — 全 pass。
- `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` — pass。
  `mise run verify-ci` — pass（fmt-check/check-ci/lint-ci/test-ci）。

## Risks

- `InBlockTransportSupport::Verified` 経路（in-block transport event）は PoC
  未実装 — #196 adapter 側の責務。

## Follow-ups

- Issue #196: adapter capability attestation + multi-bus/stateful process。
- Issue #187: 実 device clock backend への `device_restart` 結線。
