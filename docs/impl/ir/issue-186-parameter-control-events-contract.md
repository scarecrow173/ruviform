---
id: "IMPL-034QlaADDFHTb4n47aDNfW"
type: "impl"
status: "completed"
title: "Issue 186 parameter control events contract"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/186"
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
      - type: "file"
        path: "docs/impl/ir/issue-186-parameter-control-events-contract.md"
      - type: "file"
        path: "docs/discovery/issue-186-param-control-events-inventory.md"
      - type: "file"
        path: "docs/specs/issue-186-param-control-events.md"
      - type: "file"
        path: "docs/adr/parameter-control-events-contract-placement-and-deferral-of-generic-control-grap.md"
      - type: "file"
        path: "docs/designs/issue-186-param-control-events.md"
      - type: "file"
        path: "docs/designs/issue-186-param-control-events-detailed.md"
      - type: "file"
        path: "docs/plans/issue-186-param-control-events.md"
      - type: "file"
        path: "docs/test-specs/issue-186-param-control-events.md"
      - type: "file"
        path: "docs/tasks/issue-186-01-plugin-core-param-write.md"
      - type: "file"
        path: "docs/tasks/issue-186-02-bridge-target-dispatch.md"
      - type: "file"
        path: "docs/tasks/issue-186-03-delivery-observability-tests.md"
      - type: "file"
        path: "docs/tasks/issue-186-04-rt-verification.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-core/src/event.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/cap.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "docs/adr/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/discovery/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-186-01-plugin-core-param-write.md"
    - "docs/tasks/issue-186-02-bridge-target-dispatch.md"
    - "docs/tasks/issue-186-03-delivery-observability-tests.md"
    - "docs/tasks/issue-186-04-rt-verification.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-186-param-control-events.md"
    - "docs/specs/issue-186-param-control-events.md"
    - "docs/adr/parameter-control-events-contract-placement-and-deferral-of-generic-control-grap.md"
    - "docs/designs/issue-186-param-control-events-detailed.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-186-param-control-events.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue 186 parameter control events contract

## Summary

Issue #186 の設計/PoC/棚卸しを完了し、plugin-core に中立な param-write 契約（`ParamWriteMeans` / `ParamWriteRejection` / `check_param_write`・opt-in coalescing helper）、bridge に `ParamTargetTable`（TargetId↔ParamId・binding generation）と `ParamDispatchLedger`（Scheduled/Dispatched/Dropped/Reported/Unavailable）、`AdapterCapability` の param 拡張を実装した。配送は既存 `TransportPlan<EventKind>` → `PluginNode` → `EventList` を正本のまま再利用し、新規 Control Graph クレートは ADR の判断どおり未導入。

## Context

- 実施タスク: [task-01](../../tasks/issue-186-01-plugin-core-param-write.md) / [task-02](../../tasks/issue-186-02-bridge-target-dispatch.md) / [task-03](../../tasks/issue-186-03-delivery-observability-tests.md) / [task-04](../../tasks/issue-186-04-rt-verification.md)
- 上流: [plan](../../plans/issue-186-param-control-events.md)、[spec](../../specs/issue-186-param-control-events.md) R1–R12、[ADR](../../adr/parameter-control-events-contract-placement-and-deferral-of-generic-control-grap.md)、[detailed design](../../designs/issue-186-param-control-events-detailed.md)、[discovery](../../discovery/issue-186-param-control-events-inventory.md)
- 契約保証の正本: [test-spec](../../test-specs/issue-186-param-control-events.md)

## Implementation

### Changes

- `param_write.rs`: 中立な更新手段 enum・typed rejection・能力/flag 検証・連続値のみ対象の opt-in coalescing。
- `event.rs`: 同一 frame offset の stable source order / last-wins 適用意味論を契約文書化。
- `target.rs`: `TargetId`/`ParamBinding`/`BindingGeneration`/`TargetResolution`/`ParamTargetTable`（rebind で generation 加算、defunct/invalidate）。
- `dispatch.rs`: `DispatchStage`/`DropReason`/`ParamValueObservation`/`ParamDispatchLedger`。Dispatched は plan/block partition からの決定論的投影。`Reported` は plugin→host report 不在のため現行 `Unavailable`。
- `cap.rs`: `param_event_precision`/`per_note_param`/`param_reporting` を宣言面に追加（per-note/reporting は未証明のため unsupported/Unavailable を正直に宣言）。
- `param_delivery.rs`: AC-3〜AC-7 の E2E/単体証跡 9 test（複数時刻×複数対象の順序/offset/value、defined rejection、beat→frame、loop refire、async install、ledger stage、rt no-alloc）。

### Decisions During Implementation

- `Immediate`/`PerNote` means は中立語彙に存在しないため現行 `UnsupportedMeans` で reject（無言の降格なし）。
- `STEPPED` param への timed write は受理し、量子化は consumer の明示ルールに委譲（R6: silent quantization しない）。
- `Dispatched` 投影は plugin acknowledgment ではなく partition からの決定論的写像として実装（R10 の honest observability）。
- RT lane は `PluginEndpoint::process` を `assert_no_alloc` 測定窓で検証（CountPlugin は atomic のみ使用）。

## Validation

- `mise run fmt-check` — PASS
- `mise run check` — PASS（workspace --all-targets）
- `mise run lint` — PASS（clippy -D warnings）
- `mise run test` — PASS（bridge 19 test 含む全クレート）
- `mise run test-rt` — PASS（`rt_param_dispatch_no_alloc` 含む）
- `mise run verify-ci` — PASS（fmt-check → check-ci → lint-ci → test-ci）

## Risks

- plugin→host report 配線は本 Issue 範囲外のため `Reported` stage は API 形状のみ（`Unavailable` が現行値）。配線時に ledger の `record_reported` 接続が必要。
- per-note modulation は adapter 未対応を宣言済み。将来対応時は capability と means の双方を拡張。

## Follow-ups

- 汎用 Control Graph クレートは ADR で deferred。PoC 実需が立証された時点で実装分割を再 ADR 提案。
- Plugin reporting 配線は別 Issue（API 形状は本 IR で確定済み）。
