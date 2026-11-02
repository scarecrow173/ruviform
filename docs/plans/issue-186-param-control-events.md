---
id: "PLAN-034QkFry8lZR9PeeXNpx09"
type: "plan"
status: "completed"
title: "Issue #186 Parameter Control Events — implementation plan"
created: "2026-11-02"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-core/src/param_write.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/target.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/dispatch.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/tests/param_delivery.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-core/src/event.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/cap.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/lib.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/tasks/issue-186-01-plugin-core-param-write.md"
    - "docs/tasks/issue-186-02-bridge-target-dispatch.md"
    - "docs/tasks/issue-186-03-delivery-observability-tests.md"
    - "docs/tasks/issue-186-04-rt-verification.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-186-param-control-events-detailed.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-186-param-control-events.md"
  references:
    - "docs/discovery/issue-186-param-control-events-inventory.md"
  defers: []
  deferred-by: []
metadata:
  gate: "M3"
  issue: 186
---

# Issue #186 Parameter Control Events — implementation plan

## Goal

ADR Option A の設計を実装する: plugin-core に中立 write 契約（手段・defined rejection・同一 offset 規約・opt-in coalescing）、bridge に TargetId binding・dispatch ledger・cap 行列拡張を追加し、L0 mock plugin E2E と rt lane で検証する。

## Scope

単一実装ストリーム。境界: 中立契約型は plugin-core、consumer 機構は bridge、規格固有差分は adapter 側に触れない（観測のみ cap.rs に宣言）。配送経路（`TransportPlan`→`PluginNode`→`EventList`）は既存不変。

## Upstream Documents

- `docs/specs/issue-186-param-control-events.md`
- `docs/adr/parameter-control-events-contract-placement-and-deferral-of-generic-control-grap.md`

## Design Inputs

- `docs/designs/issue-186-param-control-events-detailed.md`（API 形・行列・stage 規則）
- `docs/designs/issue-186-param-control-events.md`（overview）

## File Map

- `crates/ruviform-plugin-core/src/param_write.rs` — `ParamWriteMeans`/`ParamWriteRejection`/`check_param_write`/`coalesce_param_events_latest`
- `crates/ruviform-plugin-core/src/event.rs` — 同一 offset last-wins 中立規約の doc 明文化
- `crates/ruviform-plugin-core/src/lib.rs` — `pub mod param_write` + re-export
- `crates/ruviform-audio-plugin-bridge/src/target.rs` — `TargetId`/`BindingGeneration`/`ParamBinding`/`TargetResolution`/`ParamTargetTable`
- `crates/ruviform-audio-plugin-bridge/src/dispatch.rs` — `DispatchStage`/`DropReason`/`ParamValueObservation`/`ParamDispatchLedger`
- `crates/ruviform-audio-plugin-bridge/src/cap.rs` — `param_event_precision`/`per_note_param`/`param_reporting` 追加
- `crates/ruviform-audio-plugin-bridge/src/lib.rs` — `pub mod target; pub mod dispatch;`
- `crates/ruviform-audio-plugin-bridge/tests/param_delivery.rs` — L0 mock plugin E2E + rejection + time/loop/tempo/境界/非同期 + ledger
- `crates/ruviform-audio-plugin-bridge/tests/`（rt_ テスト内包または既存 rt ファイルへ追加）— bounded dispatch no-alloc、vault keep/retire

## Implementation Sequence

- [ ] task-01: plugin-core `param_write` モジュール + event.rs 規約 doc + lib 公開（doctest 付き）
- [ ] task-02: bridge `target`+`dispatch` モジュール + `cap.rs` 行列拡張 + lib 公開
- [ ] task-03: `param_delivery.rs` テスト — E2E 配送（順序/offset/値）、defined rejection、時間変換/Loop/tempo/境界/非同期、ledger stage
- [ ] task-04: `rt_` テスト — dispatch 経路 `assert_no_alloc`、PlanVault keep/retire、有界 fault；全検証緑化

## Dependencies and Risks

- 依存: `ruviform-media-core`（`TransportPlan`/`ScheduledEvent`/`TempoMap`）、`ruviform-plugin-host`（`PluginEndpoint`/`PluginHost`）、`ruviform-rt`（`assert_no_alloc`）。
- リスク: `PluginNode` の `install_plan`/`run_callback` 内部 API が bridge 外から直接触れない場合、テストは bridge 公開面（NodeControl/PluginNode::process）で構成する。mock plugin は `Plugin` trait + `InProcessHost` で実装。
- リスク: `Dispatched` 投影が `event_engine_frame`/`partition` を内部で参照できるか — media-core の公開 API で構成可能（`TransportPlan::events`、`BlockContract::partition`、`event_engine_frame` 公開）。

## Task Breakdown

- `docs/tasks/issue-186-01-plugin-core-param-write.md`
- `docs/tasks/issue-186-02-bridge-target-dispatch.md`
- `docs/tasks/issue-186-03-delivery-observability-tests.md`（depends-on: 01, 02）
- `docs/tasks/issue-186-04-rt-verification.md`（depends-on: 02, 03）

## Verification

- [ ] `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` / `verify-ci`
- [ ] AC-3/4/5/6/7 の各テストが存在し緑
- [ ] doctest が `check`/`test` に含まれる

## Open Questions / Gaps

- plugin→host 報告配線は本計画外（deferred）。`Reported` stage は API 形のみ・`Unavailable` が現行実値。
- `Immediate`/`PerNote` means は常に `UnsupportedMeans`（語彙不在の正直な宣言）。

## Self-Review

- [x] Every spec requirement maps to a task（R1-R5/R8: task-01+02、R3/R4/R7/R9/R10/R11: task-02+03+04、R12: 全 task の lint/doctest/rt 規約）。
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（本 worker が直接実装）
- Independent work streams: task-01 と task-02 は並行可能（crate 別）。task-03/04 は依存。
- User approval required before dispatch: yes（worker は coordinator 委任済みとして進行）
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 焦点: `check_param_write` の手段×flags 行列（design §1）との一致、`Dispatched` が projection である明示、`Reported` が現行 Unavailable のみである誠実さ、`NodeEvent` 不使用。
