---
id: "DESIGN-C1EIkNXtQvzacfPU0a2bhv"
type: "design"
status: "approved"
title: "Issue #196 M4 Typed Graph Plugin Adapter — 実装設計"
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
    - "docs/specs/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md"
    - "docs/adr/issue-196-plugin-adapter-bridge-placement-scheduler-ownership-bus-role.md"
    - "docs/designs/issue-195-m2-typed-graph-compiler-buffer-pdc-rt-runtime-ownership-activation.md"
    - "docs/designs/issue-190-transport-span-scheduler.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md"
    - "docs/specs/issue-190-transport-span-scheduler.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md"
    - "docs/adr/issue-196-plugin-adapter-bridge-placement-scheduler-ownership-bus-role.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #196 M4 Typed Graph Plugin Adapter — 実装設計

## Overview

新規 crate `ruviform-audio-plugin-bridge`（利用側層）が、
`PluginHost`/`PluginEndpoint`（plugin-host）と typed `GraphProcessor`/
`NodeIo`（ruviform-audio）/`PortDef`/`NodeDef`（ruviform-graph）/
`TransportScheduler`（media-core）/`PluginProcessor`/`EventList`/
`transport_bridge`（plugin-core）を接着する。node は RT 側、control は
非 RT 側で SPSC で繋ぐ。plugin-core/host には最小の bus 列挙拡張のみ。

## Architecture

### Crate 位置（依存 DAG）

```
Primitives:      rt / media-core / audio-core
Domain:          (audio-core は foundations)
拡張層:          plugin-core ──→ audio-core/media-core/video-core
Host 層:         plugin-host ──→ plugin-core/audio-core/media-core/observability/rt/video-core
構造層:          ruviform-graph ──→ audio-core
Engines:         ruviform-audio ──→ graph/audio-core/media-core/rt/observability
利用側層(new):   ruviform-audio-plugin-bridge ──→ plugin-host + plugin-core + ruviform-audio + ruviform-graph + media-core + rt + audio-core
```

誰も bridge に依存しない。`plugin-host → audio` は dev-dep のまま不変。

### Owner 分離

| 面 | Owner | 内容 |
| --- | --- | --- |
| nonRT control | `PluginHost`/`PluginNodeControl` | Instantiate/Initialize/Activate/StartProcessing/StopProcessing/Deactivate/Destroy/SaveState/RestoreState + transport request/install_plan/set_clock/device_restart + receipt/fault drain |
| RT process | `PluginNode`（`PluginEndpoint` wrap） | scheduler.run_callback → span → partition → 物理 `process`（windowed views + per-call EventList + TransportInfo） |
| nonRT retire | `GraphRuntime::collect` + `Retirer` + `PlanVault` retire channel | plugin/processor/plan destructor は非 RT のみ |
| nonRT attach | `attach()` / `PluginNodeFactory` | bus decl 検証 → PortDef 生成 → endpoint attach → node 供給 |

### Data flow（1 callback）

```
NodeIo（callback frames=N, engine frame=cursor）
  └─ PluginNode::process
       1. control channel drain（bounded ≤ CTRL_DRAIN_MAX=32）
            request→stage / install_plan→stage(Arc→vault clone) /
            set_clock / device_restart
       2. scheduler.run_callback(cursor, N, sink)
            └─ per TransportSpan [s,e):
                 deliverable==false → silence outputs [s,e) + report fault
                 deliverable → partition(e-s) → per chunk (off,len):
                     build_call_event_list(span, events, call_start=s+off, len, scratch, |p| *p)
                     call_transport_info(&span.ctx, off, tempo_map, ppqn)
                     endpoint.process(&hins, &mut houts, event_list, Some(info))
       3. scheduler pop_receipt/pop_fault → report channel（≤ REPORT_MAX）
       4. cursor = cursor.advance(N)
```

- `cursor` は node 内蔵の `EngineFrame`（epoch+frame）。`set_clock` で
  rebase、device restart で epoch 増分。
- `hins`/`houts` は stack `array::from_fn`/`iter_mut` 構築の
  `HostInput`/`HostOutput` 配列（各 `WindowedIn/WindowedOut` が
  `&plane[off..off+len]` / `&mut plane[off..off+len]` を持つ）。
- event scratch は attach 時 `Vec<Event>::with_capacity(
  budget.max_events_per_span)`。bus views は `MAX_BUSES=16`、
  channel views は `MAX_CHANNELS=16`（WIRE bound と一致）。
- report channel: `SPSC<NodeReport>`（Receipt/Fault/RetirePlan/Undeliverable）。
  full → node の `NodeStatus` counter を立てて drop（観測可能・静かでない）。

## Detailed Design

### plugin-core 追加

```rust
#[non_exhaustive] pub enum BusRole { Main, Aux, Sidechain, Send, Return }
pub struct BusDescriptor { direction, layout, name, role: BusRole, enabled }
BusDescriptor::is_main() -> bool   // role == Main
Plugin::buses(&self) -> &[BusDescriptor]  // default &[]（未宣言）
ErasedPlugin::buses(&self) -> &[BusDescriptor]
```

### plugin-host 追加

```rust
pub enum BusReport { Reported(Vec<BusDescriptor>), Unavailable }
PluginHost::buses(&mut self, handle) -> Result<BusReport, HostError>
InProcessHost::buses(&mut self, handle) -> Result<BusReport, HostError>
// L0: Shared→core.lock()、Plain→直接読み。L1/video→Unavailable。Failed→Err。
```

### media-core 追加

```rust
TransportScheduler::pop_receipt(&mut self) -> Option<AppliedReceipt>
TransportScheduler::pop_fault(&mut self) -> Option<ScheduleFault>
// Vec::remove(0)・capacity bounded・RT-safe
```

### bridge crate modules

- `cap.rs`: `CapabilityLevel::{Verified,Declared,Unknown,Unavailable,Unsupported}`、
  `AdapterCapability{bus_enumeration,multi_bus,aux_sidechain,events,in_block_transport,min_frames_enforced,state_save_restore,crash_isolation,notes}`、
  `capability_for(Backend::{L0,L1,Clap,Vst3})`。
- `bus.rs`: `BusDecl{descriptor:BusDescriptor, role:PortRole}`、
  `bus_decl_to_port_def(&BusDecl, AudioFormat)->PortDef`、
  `validate_bus_decls(&[BusDecl], Option<&[BusDescriptor]>)->Result<(), BusError>`、
  `BusError::{MissingPort,RoleMismatch,LayoutMismatch,DirectionMismatch,EmptyDecl,TooManyBuses}`。
- `control.rs`: `PluginNodeControl{tx,rx,status}`、
  `NodeControl::{Request{cmd,due},InstallPlan{plan,due},SetClock{epoch,frame},DeviceRestart{base_frame}}`、
  `NodeReport::{Receipt,Fault,UndeliverableSpan{start,end},RetireOverflow}`。
- `node.rs`: `PluginNode{endpoint,scheduler,ctrl_rx,report_tx,status,
  cursor,event_scratch,silent_scratch,plan_vault,in_scratch_views,out_scratch_views}`、
  `impl GraphProcessor`。
- `attach.rs`: `AttachConfig{buses:Vec<BusDecl>,process_config:ProcessConfig,
  bounds:EndpointBounds,frame_base:TimeBase,policy:DeliveryPolicy,
  budget:ScheduleBudget,ppqn:Option<Ppqn>}`、
  `attach(host,handle,cfg)->Result<Attached,AttachError>`（buses 検証 →
  Activate/StartProcessing は consumer が先に実行 → prepare_endpoint →
  node 構築 → control 返却）。
- `factory.rs`: `PluginNodeFactory{pending:HashMap<NodeId,PendingAttach>}`
  （`prepare`: installed かつ同一 sig → Reuse、新規 → New(node)、
  Boundary kind → Boundary、不一致 → FactoryError）。
- `vault.rs`: `PlanVault{store:Vec<Arc<TransportPlan<EventKind>>>,cap}`、
  `install(Arc)->bool`、`purge()->drain to retire`、overflow→forget+counter。
- `status.rs`: `NodeStatus{undeliverable_spans,endpoint_errors,
  event_scratch_overflow,control_overflow,report_overflow,vault_overflow,
  physical_calls,min_violations_would_have_been}`。

### Windowed view の構築（no alloc/no unsafe）

```rust
const MAX_BUSES: usize = 16;
const MAX_CHANNELS: usize = 16;
struct WindowedIn<'a>{channels:[&'a [f32];MAX_CHANNELS],count:usize,frames:usize}
struct WindowedOut<'a>{channels:[&'a mut [f32];MAX_CHANNELS],count:usize,frames:usize}
// impl BufferView/BufferViewMut
// inputs:  let in_views=[from_fn(|i| io.input(i))]; WindowedIn{channels:[&[];16],..}
//          per bus fill from in_views[i].channel(c)[off..off+len]
// outputs: let out_views=[from_fn(|i| io.output(i))]; WindowedOut{channels:[&mut [];16],..}
//          per bus fill via out_views[i].channel_mut(c) → &mut plane[off..off+len]
// erased:  let mut it=in_views.iter(); hins=array::from_fn(|_|HostInput(it.next().unwrap() as &dyn BufferView))
//          let mut it=out_views.iter_mut(); houts=array::from_fn(|_|HostOutput(it.next().unwrap() as &mut dyn BufferViewMut))
//          endpoint.process(&hins[..in_count], &mut houts[..out_count], events, transport)
```

unconnected enabled input bus → `silent_scratch[ch][..frames]` から読む
windowed view（attach 時に `channels×max_frames` 確保）。
disabled bus → `WindowedIn{count:0,frames:0}` empty view（bus 順位保持）。

### Fault contract

| Fault | 挙動 |
| --- | --- |
| undeliverable span | silence + `NodeReport::UndeliverableSpan` + status counter |
| endpoint `Failed`（panic/crash） | `converge_failed` → silence + `Ok(CONTINUE)`（既存） |
| endpoint `EndpointUnavailable`/`Lifecycle`/`Prepare` | windowed 出力へ silence + counter + report |
| control/report channel full | counter のみ（bounded drop） |
| vault overflow | install reject + `VaultExceeded` fault + counter |
| event scratch overflow | `build_call_event_list` None → events 欠落 + counter |
| graph swap | generation 一致 install/remove で instance 重複なし |
| retire overflow | leak-on-full + counter（既存パターン） |

## Error Handling

`AttachError`: `HostError` 伝播 + `BusError` + `CapabilityUnsupported`。
`Attach` は Activate/StartProcessing の前に consumer が実行した
invariant を前提とし、`prepare_endpoint` 失敗は Err で伝播。
Node 側の実行時エラーは `silence + counter/report` に converge
（定義済み出力表）。

## Compatibility

- `BusDescriptor.role` は `main` を置換（0.1.0 破壊的変更・doc 例更新）。
- `Plugin::buses`/`ErasedPlugin::buses`/`PluginHost::buses`/`pop_*` は additive。
- `PluginNodeFactory`/`attach`/`PluginNodeControl` は新規。
- `process_audio` 単一 bus 経路は維持（Rollback）。

## Open Design Notes

- `EndpointBounds.min_frames` は追加しない。activated `ProcessConfig` を
  attach 時 consumer 供給とし、`config.max_frames ≤ bounds.max_frames` を
  検証。
- `NodeEvent`（u32+f32）は transport plan/command を運べないため使わない。
- `Arc<TransportPlan>` の RT drop 回避は PlanVault に限定（media-core
  retire hook は設けない）。
- `report` channel capacity は `REPORT_CAP=256`（receipt+fault+undeliverable
  を束ねる）。drain は非 RT のみ。

## Sources

- `docs/specs/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md`
- `docs/adr/issue-196-plugin-adapter-bridge-placement-scheduler-ownership-bus-role.md`
- `docs/designs/issue-190-transport-span-scheduler.md`
- `docs/designs/issue-195-m2-typed-graph-compiler-buffer-pdc-rt-runtime-ownership-activation.md`
