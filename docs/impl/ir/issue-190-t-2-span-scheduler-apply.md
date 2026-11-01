---
id: "IMPL-034QgS4TFyEN937G8j8DKR"
type: "impl"
status: "completed"
title: "Issue #190 T-2: span scheduler apply"
created: "2026-11-01"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/190"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-media-core/src/transport_schedule.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-190-t-2-span-scheduler-apply.md"
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

# Issue #190 T-2: span scheduler apply

## Summary

`TransportScheduler` を実装した: 非 RT `request`/`preflight`/`install_plan`/
`device_restart`、RT `run_callback`（pending apply 消化・境界で半開 span
分割・`SpanSink` 配信・budget 監視・defined fault 記録）。

## Context

task T-2。design doc §Scheduler apply pipeline に従う。

## Implementation

### Changes

- `request`（非 RT）: validate → `Requested` receipt 記録 → epoch/検証失敗・
  pending 溢れは即 `Rejected` → 受理分は `Pending` receipt + queue。
- `preflight`（非 RT）: callback 窓内の apply/loop wrap を `SimState` で
  全シミュレートし、Exact で物理分割不可能な境界を生じる command を
  `UnsupportedExactTransport` で事前拒否。
- `run_callback`（RT・no-alloc/no-blocking）:
  - callback 内の due pending apply と loop wrap を境界として半開 span に
    分割し `SpanSink::span` へ配信。
  - Exact: 不正境界の apply は `Rejected` receipt + fault、wrap は
    `UnrepresentableLoopWrap` fault + `deliverable=false` span。
  - Degraded: apply を次の合法境界へ遅延し receipt に `DeferredApply`
    （requested vs applied frame）を記録。
  - budget: pending/spans-per-callback/events-per-span/callback work 上限を
    `ScheduleBudget` で監視、超過は `ScheduleFault`（`PendingBudgetOverflow`
    /`SpanBudgetExceeded`/…）で defined fault — 無制限 split/alloc は行わ
    ない。
  - stale generation event は配信せず `StaleEventGeneration` fault。
  - `device_restart` は `ClockEpochId` 更新（transport generation とは独立）。
- `SpanSink` trait: driver/plugin 側が span + event 窓を受け取る唯一の配信
  口。`CallPartition` は `BlockContract::partition` で算出（均等割・全 chunk
  が min/max 内であることのみ保証）。

### Decisions During Implementation

- `preflight` は command 適用点だけでなく、その apply 後の transport 状態
  で生じうる loop wrap まで含めてシミュレートする（loop そのものが
  unrepresentable な場合の早期検出）。
- 同一 frame に複数 due apply がある場合の drain は defensive に行う
  （正常系では境界ごとに 1 件ずつだが、巻き戻し seek 等で同一 frame が
  生じうるため）。

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

- 15 unit tests（`crates/ruviform-media-core/src/transport_schedule.rs`
  tests モジュール）: lifecycle / loop wrap 分割 / 多周 occurrence /
  stable order / stale generation / Exact reject・Degraded defer /
  preflight 拒否 / restart vs seek / pending・span budget /
  `rt_run_callback_no_alloc` — 全 pass。
- `mise run lint` / `mise run test` / `mise run test-rt` — pass。

## Risks

- `preflight` のシミュレーションは非 RT でのみ安全（探索自体は有界だが
  caller が十分な budget を与える前提）。

## Follow-ups

- Issue #195: `install_plan`/generation 刻印の RT 所有権移譲。
- 実ドライバ結合時の `ScheduleBudget` チューニング値は別 Issue で決定。
