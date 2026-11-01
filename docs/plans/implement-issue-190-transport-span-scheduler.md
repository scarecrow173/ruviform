---
id: "PLAN-034Qh3xYzAbCdEfGhIjK"
type: "plan"
status: "completed"
title: "Issue #190 Transport Span Scheduler 実装計画"
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
    - "docs/designs/issue-190-transport-span-scheduler.md"
    - "docs/specs/issue-190-transport-span-scheduler.md"
    - "docs/adr/transport-span-scheduler-applied-receipt.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-190-transport-span-scheduler.md"
    - "docs/specs/issue-190-transport-span-scheduler.md"
    - "docs/adr/transport-span-scheduler-applied-receipt.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #190 Transport Span Scheduler 実装計画

## Goal

ADR・design 確定の契約（canonical identity / Command→Applied Receipt /
半開 span scheduler / plugin min-max contract / Mock Driver PoC）を
media-core + plugin-core に実装し、受入基準 AC-1〜AC-12 を green にする。

## Task Breakdown

| Task | Doc | 内容 |
| --- | --- | --- |
| T-1 | `docs/tasks/issue-190-t-1-canonical-contract-types.md` | media-core `transport_schedule` モジュール: identity newtypes・Command/Receipt/Fault/Policy/Budget/Contract/ScheduledEvent/TransportPlan(+Builder)・`legal_len`/chunk 計算・単体テスト |
| T-2 | `docs/tasks/issue-190-t-2-span-scheduler-apply.md` | `TransportController`（非RT request/preflight/install_plan/device_restart/receipt drain）+ `TransportScheduler`（RT run_callback・境界走査・span emit・apply・receipt/fault ledger）+ rt_ 無割当テスト |
| T-3 | `docs/tasks/issue-190-t-3-plugin-bridge-mock-poc.md` | plugin-core `transport_bridge`（`BlockContract::from_process_config`・`CallPlan`・`SpanDelivery`）+ Mock Driver 統合テスト（TT-025〜030/AC）+ impl record |

## Sequence

1. T-1: 型と純粋関数（`legal_len`, chunk, frame offset 変換）を先に固定。
2. T-2: 状態機械と span 分割。T-1 の型にのみ依存。rt_ テスト同梱。
3. T-3: bridge + Mock Driver + 統合受入テスト + impl record。

T-1→T-2→T-3 は直列（T-2 は T-1 型に依存、T-3 は T-2 scheduler に依存）。

## Verification

- `mise run fmt-check` / `check` / `lint` / `test` / `test-rt`。
- 最終: `git fetch origin main` → `git merge origin/main` → `Cargo.lock`
  再生成 → `verify-ci`。
- `rt_` テストは `run_callback` 無割当契約を `rt_transport_*` 命名で追加。

## Risks

- `run_callback` の境界走査でアロケーション混入 → rt_ テストで早期検出。
- loop wrap と pending apply の同 frame 競合 → 境界走査の優先規則を
  design に明記（apply 先・wrap 後）＋テスト化。
- `max_deferral` 内の legal boundary 探索の複雑さ → 有界線形走査
  （O(max_deferral)、上界つき）で簡潔に実装。

## Sources

- 上流: [spec](../specs/issue-190-transport-span-scheduler.md)、
  [design](../designs/issue-190-transport-span-scheduler.md)、
  [ADR](../adr/transport-span-scheduler-applied-receipt.md)。
- 検証 matrix: [test-spec](../test-specs/issue-190-transport-span-scheduler-tests.md)。
