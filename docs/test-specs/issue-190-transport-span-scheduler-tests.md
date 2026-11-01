---
id: "TSPEC-034Qh4wErTyUiOpAsD"
type: "test-spec"
status: "approved"
title: "Issue #190 Transport Span Scheduler テスト仕様"
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
    - "docs/designs/issue-190-transport-span-scheduler.md"
    - "docs/plans/implement-issue-190-transport-span-scheduler.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-189-m3-rational-beat-ppqn-tests.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-190-transport-span-scheduler.md"
    - "docs/designs/issue-190-transport-span-scheduler.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-190-transport-span-scheduler.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #190 Transport Span Scheduler テスト仕様

## Scope

`ruviform-media-core` の `transport_schedule` 単体テストと、
`ruviform-plugin-core` の Mock Driver 統合テストで spec の AC-1〜AC-12 と
Synvelis §14 TT-006〜030 相当をカバーする。

## Test Matrix

| ID | 内容 | AC | 層 |
| --- | --- | --- | --- |
| U-1 | `legal_len(len,min,max)` 正負・`len>max` partition 可否 | R5 | media-core unit |
| U-2 | CallPlan 分割（300@max128 → 各 chunk ∈[min,max]・和=len） | R5 | plugin-core unit |
| U-3 | frame→offset 変換（`offset_within`・半開境界・負拒否） | R3/R8 | media-core unit |
| T-008 | Play/Pause/Stop/Seek/SetLoop で Requested→Pending→Applied/Rejected・actual applied frame 観測 | AC-1 | mock |
| T-006/025 | 256F callback・wrap@100・min=1 → [0,100)+[100,256)・各 call の TransportInfo 位置 | AC-2 | mock |
| T-026a | 同上・min=128・Exact → 100F call 不生成・Preflight/RT Reject・Last-good 維持 | AC-3 | mock |
| T-026b | 同上・min=128・Degraded → wrap 適用を ≤max_deferral の legal boundary へ延期・receipt に actual frame+degradation | AC-3/AC-11 | mock |
| T-027 | in_block=`Unknown`/`Declared` のみ → in-block 利用せず Reject/Degraded へ倒れる（`Verified` 扱いしない） | AC-10 | mock |
| T-028 | event @100/120/256 → process-relative offset 0/20/次callback 0・重複消失なし | AC-4 | mock |
| T-007/029a | loop 多周で同一 source event が各 occurrence ちょうど1回・同一 occurrence 二重なし | AC-5 | mock |
| T-029b | zero-frame flush call が block size 0 で送られる・`offset<block_frames` 維持 | AC-9 | mock |
| T-029c | L0 gate（min 違反 reject）と L1（min=0 委譲）で capability 判定が区別される | AC-10 | mock |
| T-010 | 同 frame event の `source_sequence` 安定順が EventList 順に反映 | AC-4/AC-9 | mock |
| TA-008 | stale `plan_generation`/`transport_generation` event → 非配送 + Defined Fault | AC-6 | mock |
| T-009 | device_restart → `ClockEpochId` 変更 / seek → `TransportGeneration` 変更として別観測・pending は StaleGeneration Reject | AC-7 | mock |
| T-012 | spans/events/pending budget 超過 → Defined Fault（無制限 split/alloc なし） | AC-8 | mock |
| T-030a | Offline(=Exact) では無断量子化せず Reject | AC-11 | mock |
| T-030b | Degraded は明示選択時のみ・receipt に degradation 記録 | AC-11 | mock |
| rt_sched | `run_callback` が alloc しない（rt_ 命名・assert_no_alloc） | AC-12 | media-core rt |

## Notes

- TT-025〜030 の番号は Synvelis §14 の matrix に対応。
- `RecordedCall` = (frames, TransportInfo snapshot, Vec<(frame_offset, EventKind)>)
  を mock が記録し assertion。
- `EventList::new` の既存検証（非減少・offset<frames・zero-frame）に通ることを
  各 mock テストで暗黙に検証し、T-029b で明示アサート。

## Sources

- 検証対象: [spec](../specs/issue-190-transport-span-scheduler.md)、
  [design](../designs/issue-190-transport-span-scheduler.md)、
  [ADR](../adr/transport-span-scheduler-applied-receipt.md)。
- 実行計画: [plan](../plans/implement-issue-190-transport-span-scheduler.md)。
