---
id: "PLAN-AAoMPoJ2iWazZqcu0pX2XQ"
type: "plan"
status: "in-progress"
title: "Issue #196 M4 Typed Graph Plugin Adapter 実装計画"
created: "2026-11-02"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/196"
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
    - "docs/designs/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md"
    - "docs/specs/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md"
    - "docs/adr/issue-196-plugin-adapter-bridge-placement-scheduler-ownership-bus-role.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md"
    - "docs/specs/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md"
    - "docs/adr/issue-196-plugin-adapter-bridge-placement-scheduler-ownership-bus-role.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #196 M4 Typed Graph Plugin Adapter 実装計画

## Goal

`ruviform-audio-plugin-bridge`（新規利用側層 crate）で typed graph へ
plugin instance を接着し、plugin-core/host へ最小の bus 列挙拡張を加え、
M4-01〜08 の受入を検証する。

## Task Breakdown

| Task | Scope | Deliverable |
| --- | --- | --- |
| T-1 | plugin-core bus 契約 | `BusRole`、`BusDescriptor.role`、`Plugin::buses`/`ErasedPlugin::buses`（default `&[]`） |
| T-2 | media-core + plugin-host 列挙 | `pop_receipt`/`pop_fault`、`BusReport`、`PluginHost::buses`/`InProcessHost::buses`（L0 lock・L1 Unavailable） |
| T-3 | bridge crate 実装 | `ruviform-audio-plugin-bridge`: capability・bus 写像・`PluginNode`・`PluginNodeControl`・`PluginNodeFactory`・`PlanVault`・`NodeStatus` |
| T-4 | 検証 + 証拠 | multi-bus E2E・loop/min 違反 0・座標一致・fault・capability matrix・rt_ 無割当・impl records |

## Sequence

1. T-1 → T-2（並行可だが T-1 の enum を T-2 が使うため直列推奨）
2. T-3（T-1/T-2 完了後）
3. T-4（T-3 完了後・検証と記録）

## Verification

- `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` 緑
- `cargo test -p ruviform-audio-plugin-bridge`・`cargo test -p
  ruviform-plugin-host --locked`・`cargo test --workspace --locked`
- `mise run test-l1-real-plugin`/`lint-l1-real-plugin`（L1 lane 回帰）
- AC-1〜8 の実測証拠（impl record + test 結果）

## Risks

- `BusDescriptor.role` 置換は公開 API 破壊（0.1.0・実利用 doc のみ）—
  consumer 影響を PR 説明に分類。
- node 内蔵 scheduler の `stage` RT 実行は実測 capacity guard に依存
  — `pop_receipt`/`pop_fault` 追加で bounded 取り出しを担保。
- `Arc<TransportPlan>` RT drop → `PlanVault` + retire channel で防御。
- L1 bus 列挙・非 RT 全状態 preflight・CLAP in-block transport は
  deferred として capability 宣言。

## Sources

- `docs/specs/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md`
- `docs/designs/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md`
- `docs/adr/issue-196-plugin-adapter-bridge-placement-scheduler-ownership-bus-role.md`
