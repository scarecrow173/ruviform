---
id: "DESIGN-034QkAdKcICxz0yVcZ8PPZ"
type: "design"
status: "approved"
title: "Issue #186 Parameter Control Events — overview"
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
  implemented-by: []
  depends-on:
    - "docs/designs/issue-189-rational-beat-variable-ppqn-tempomap.md"
    - "docs/designs/issue-190-transport-span-scheduler.md"
    - "docs/designs/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/parameter-control-events-contract-placement-and-deferral-of-generic-control-grap.md"
    - "docs/designs/issue-186-param-control-events-detailed.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-186-param-control-events.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/discovery/issue-186-param-control-events-inventory.md"
  defers: []
  deferred-by: []
metadata:
  gate: "M3"
  issue: 186
---

# Issue #186 Parameter Control Events — overview

## Context

Synvelis Automation/Modulation は logical parameter を target とし「Reference Value → 個別 Source 寄与 → Host-resolved Value」を評価した後、結果を plugin へ時刻付きで届ける必要がある。Ruviform 側の責務は host/plugin 境界の汎用契約のみ: パラメータイベントの精度・能力照会・時刻付き配送・RT 安全・有界失敗・観測性（boundaries ADR、discovery E9）。

棚卸しの結論（discovery doc）は、タイムド配送骨格が #190（`TransportPlan<P>`/`TransportScheduler`/generation/receipt/budget）と #196（`PluginNode`/`EventList` 構築/`PlanVault`/capability matrix）で実装済みで、**不足は周辺契約**: パラメータ単位 capability、ParamId↔TargetId 写像、scheduled/dispatched/reported の区別、coalescing 方針、defined rejection、規格差分の正直な宣言。

## Scope

### In Scope

- plugin-core に中立 write 契約: 更新手段・defined rejection・同一 offset 規約・opt-in coalescing ヘルパ。
- bridge に consumer 機構: `TargetId`↔`ParamId` binding（generation 付き）・dispatch ledger（Scheduled→Dispatched→Dropped/Reported/Unavailable）。
- `cap.rs` にパラメータ精度差分の正直な宣言（event_precision, per_note, reporting）。
- L0 in-process mock plugin + `PluginNode` の E2E 配送検証。時間変換/Loop/tempo/境界/非同期の再現テスト。`rt_` lane 整合。

### Out of Scope

- Control Graph クレート・汎用 Modulation Source・合成ポリシー（ADR 参照・製品所有）。
- plugin→host パラメータ報告の配線（観測区別の API 形のみ定義、配線は分割 Issue）。
- `NodeEvent` の param 拡張、L1 wire verb 追加、`PortFormat::Control` typed payload。
- Synvelis 固有 UI/Write/Undo/プリセット/Track セマンティクス。

## Components and Boundaries

- **`ruviform-plugin-core::param_write`**（新設・中立契約）: `ParamWriteMeans`（`TimedEvent`/`BlockBoundary`/`Immediate`/`PerNote`）、`ParamWriteRejection`（`ReadOnly`/`NotAutomatable`/`UnsupportedMeans`/`UnsupportedPrecision`/`UnknownParam`/`NonFiniteValue`）、`check_param_write(&ParamInfo, ParamWriteMeans)`、opt-in `coalesce_param_events_latest`。`ParamFlags` の意味: `AUTOMATABLE`=timed 更新許容、`READONLY`=外部 write 全拒否、`STEPPED`=量子化規則は param 提供側責務（中立基盤は規則未定義の量子化をしない）、`MODULATABLE`=per-note 能力の規格側宣言だが中立配送語彙なし（`PerNote` means は常に `UnsupportedMeans`）。
- **`ruviform-audio-plugin-bridge::target`**（新設・consumer 機構）: `TargetId(pub u64)`（アプリ内安定 ID、値の意味は consumer 所有）、`BindingGeneration(pub u64)`、`ParamBinding{target, node, param, generation}`、`TargetResolution::{Resolved, Unresolved, Defunct}`、`ParamTargetTable`（非RT。insert/rebind/remove/resolve）。未解決/旧 generation は defined rejection で silent fallback しない。
- **`ruviform-audio-plugin-bridge::dispatch`**（新設・consumer 機構）: `ParamDispatchLedger`。plan install 時に `Scheduled{value, position, revision}` を記録。`Dispatched{call_start, frame_offset}` は plan+`BlockContract`+`event_engine_frame` から**決定的に投影**（wire ack ではなく投影であることを型で表明）。`NodeReport` 系 fault で `Dropped{reason}`。plugin 報告は `Reported{value}` / `Unavailable`（現行全 backend 報告経路なし）。
- **`ruviform-audio-plugin-bridge::cap`**（拡張）: `AdapterCapability` に `param_event_precision: CapabilityLevel`（sample-accurate 時のみ Verified 相当）・`per_note_param: CapabilityLevel`（全 backend `Unsupported`）・`param_reporting: CapabilityLevel`（全 backend `Unavailable` 現状）を追加。
- **配送経路（既存・変更なし）**: `TransportPlanBuilder` → `install_plan` → `TransportScheduler::run_callback` → `build_call_event_list` → `PluginEndpoint::process`。

## Data and Control Flow

- 非RT: consumer が `ParamTargetTable::resolve(target_id)` で `ParamBinding` を得る → `check_param_write(info, TimedEvent)` で能力検証 → `TempoMap::beat_to_media` で位置決定 → `TransportPlanBuilder::event(pos, seq, EventKind::ParamValue(...))` → `build()` → `install_plan`（vault keep-alive）。
- RT: `PluginNode::process` → scheduler が generation 照合し span を `BlockContract` 分割 → per-call `EventList`（process-start-relative、非減少）→ `endpoint.process`。容量超過は `EventScratchOverflow`/`EventBudgetExceeded` report。`pop_report` で非RT 側が観測。
- 観測: `ParamDispatchLedger` が plan install・`run_callback` 投影・`NodeReport` fault・（将来）plugin 報告を束ねて `ParamValueObservation{stage, value, revision, at}` を返す。
- 世代切断: plan/transport/binding generation と `ClockEpochId` が一致しないイベントは `StaleGeneration`/`StaleDiscarded` fault（silent 不適用）。

## Key Decisions

- Timed 配送正本 = `TransportPlan<EventKind>` 経路（ADR Option A）。`NodeEvent` 不採用（タイミング・ParamId・f32 精度不足）。
- 同一 offset 同一 param: `EventList` は安定順を保持し、processor 適用は **last-wins** を中立規約として明文化（VST3 gain 実装と一致）。別 param 同士は source_sequence 順のまま。
- Coalescing: 既定は全配送（overflow=defined fault）。opt-in で同一 param 連続値のみ latest-wins 圧縮（不可逆イベント・別 param 混在不可）。
- 汎用 Control 評価基盤は不採用（ADR）。将来 crate が来ても消費する語彙（Target/Host-resolved/観測 stage）だけを本設計で確定。
- `Dispatched` は投影（deterministic projection）。plugin 側確認ではないことを型名と doc で明示（誠実な観測）。
