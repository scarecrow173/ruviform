---
id: "DISC-U91vIxxv8vbdgKjs4t9moz"
type: "discovery"
status: "resolved"
title: "Issue #196 M4 Typed Graph Plugin Adapter — multi-bus/stateful lifecycle/process offset precision"
created: "2026-11-02"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/196"
    - "https://github.com/scarecrow173/ruviform/issues/192"
    - "https://github.com/scarecrow173/ruviform/issues/193"
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
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/designs/issue-184-typed-graph-ir-ports-buses-connections-validation-revisions-m1.md"
    - "docs/designs/issue-195-m2-typed-graph-compiler-buffer-pdc-rt-runtime-ownership-activation.md"
    - "docs/designs/issue-190-transport-span-scheduler.md"
    - "docs/specs/issue-190-transport-span-scheduler.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/plugin-rt-ownership-split.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/transport-span-scheduler-applied-receipt.md"
    - "docs/discovery/issue-194-m0-baseline-audit.md"
    - "docs/discovery/issue-195-m2-typed-graph-compiler-runtime-audit.md"
    - "docs/test-specs/issue-190-transport-span-scheduler-tests.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/184"
    - "https://github.com/scarecrow173/ruviform/issues/195"
    - "https://github.com/scarecrow173/ruviform/issues/190"
    - "https://github.com/scarecrow173/ruviform/issues/189"
    - "https://github.com/scarecrow173/ruviform/issues/188"
    - "https://github.com/scarecrow173/ruviform/issues/186"
    - "https://github.com/scarecrow173/ruviform/issues/113"
    - "https://github.com/scarecrow173/ruviform/issues/114"
  defers: []
  deferred-by: []
metadata:
  gate: "M4"
  issue: 196
---

# Issue #196 M4 Typed Graph Plugin Adapter — multi-bus/stateful lifecycle/process offset precision

## Summary

Issue #196 は typed multi-bus graph（#184/M1・#195/M2）へ plugin instance を
安全に接着する M4 adapter。現状の調査で「`PluginEndpoint::process` の
複数 bus erased view は存在するが `process_audio` は単一 bus shell」
「`TransportScheduler`/`transport_bridge` は実装済みだが plugin node
への接着配線が存在しない」「`Plugin`/`ErasedPlugin`/`PluginHost` に
bus 列挙 API がなく、`BusDescriptor` は誰にも消費されていない」
「scheduler の `install_plan`/`request` は非 RT 宣言で、RT 側に scheduler
を置く設計は未解決」が確定した。

## Exploration Goal

- 作業境界 1〜7 の各項目について、既存契約で足りる部分・新規に必要な
  API/型・実際に存在するギャップを特定する。
- Bridge crate を新設すべきか（境界 7）を依存 DAG の実測で判断する。
- Loop boundary=100 + min_frames=128 の 256 frame callback で違反
  `process` 呼出し 0 件を、どの層の機構で保証するか決める。

## Key Issues and Assumptions

### 既存資産（変更なしで使えるもの）

- `ruviform-graph`: `PortDef`（Direction/PortRole{Main,Aux,Sidechain,Send,
  Return}/PortFormat/Multiplicity/capabilities/Availability/Activation）、
  `Validator`（暗黙変換禁止・format/domain 一致・required connections）、
  `ValidatedGraph`（常に DAG）。
- `ruviform-audio::typed`: `Compiler`+`NodeFactory`（`PrepareOutcome::
  New/Reuse/Boundary`）→ `PreparedPlan` → `GraphRuntime`（非 RT owner）→
  `RtRuntime`（single RT owner）。`GraphProcessor::{activate,event,process,
  deactivate}`、`NodeIo::{input,output}`（seq-stamp uninit-read 監査付き）、
  generation 一致の Install/Remove、`SharedStatus`/`RuntimeFault`、
  `Retirer`（leak-on-full）。
- `ruviform-media-core::transport_schedule`: `TransportScheduler<P>`
  （pending/receipts/faults が capacity-bounded で RT 安全）、
  `run_callback`（半開 span 分割・Exact/Degraded 方針・undeliverable span）、
  `BlockContract::partition`、`event_engine_frame`、`span_event_window`、
  `AppliedReceipt`/`ScheduleFault`。
- `ruviform-plugin-core`: `PluginProcessor::process(&[I],&mut [O],EventList,
  Option<TransportInfo>)`（**すでに複数 bus**）、`ErasedPlugin::process_host`
  （dyn-safe mirror）、`EventList`（process 開始相対 `frame_offset`・昇順・
  `offset < block_frames` 検証）、`TransportInfo`、`ProcessConfig`
  （min/max_frames）、`transport_bridge::{contract_from_process_config,
  calls_for_span, call_transport_info, build_call_event_list}`、
  `Lifecycle` 状態機械、`sort_events_by_frame_offset`。
- `ruviform-plugin-host`: nonRT `PluginHost`/`InProcessHost`（12 verb）と
  `PluginEndpoint`（Send RT leg、`process(&[HostInput],&mut [HostOutput])`
  の複数 bus erased view）。L0 leg は lock 内で activated
  `min_frames ≤ frames ≤ max_frames` を強制（endpoint.rs step 4）。
- 規格 adapter（clap/vst3）は単一 stereo main のみ。CLAP bridge は
  audio-ports 走査を一般化しているが、`host` feature の `ErasedPlugin` は
  `!Send` で L1 専用。

### 実測ギャップ（新規作業が必要なもの）

- **bus 列挙の欠如**: `Plugin`/`ErasedPlugin`/`PluginHost` に bus 列挙 verb
  がない。endpoint 経路の plugin は Shared slot or L1 のため
  `plugin_mut()` は `None` を返し、instance から bus 宣言を読む経路が
  host 側に存在しない。`BusDescriptor` は doc 例以外で未使用。
- **`BusDescriptor` の role 欠落**: `main: bool` のみで Aux/Sidechain/
  Send/Return を表現できない（M4-02 の構成検証に必要）。`PortRole` は
  graph 層に存在するが plugin-core は ruviform-graph に依存できない。
- **scheduler の owner 未解決**: `install_plan`/`request`/`preflight`/
  `drain_*` は非 RT 宣言、`run_callback` は RT。node（RT 側）に
  scheduler を埋めると非 RT API に届かず、非 RT に置くと callback
  同期が取れない。`pending`/`receipts`/`faults` は capacity で guard
  済みのため `stage()` 相当の有界適用は RT でも非 alloc。ただし
  `Arc<TransportPlan>` の差替・pending reject は RT 上で Arc の
  destructor を起動し得る（`device_restart` の pending drain、
  InstallPlan apply の旧 plan 差替）—「RT で heap resource を破棄しない」
  違反。
- **EngineFrame の到達経路なし**: `RtRuntime` は内部 `frame_cursor: u64`
  を持つが `GraphProcessor::process` には届かず、`NodeEvent` は
  `{opcode:u32, value:f32}` で transport plan/command を運べない。
  node 自身が engine frame cursor を持つ必要がある。
- **`EndpointBounds` に min_frames なし**: `{max_frames, channels}` のみ。
  契約は attach 時の consumer 供給 `ProcessConfig` で補完できる。
- **event payload 型**: plan の `P` は consumer 固定が必要。wire
  `EventKind` に射影できる `P = EventKind` が最薄い接着。
- **CLAP in-block transport**: adapter は `clap_process.transport` field のみ
  消費し、event list 内の transport event は破棄する（実測
  `translate_param_event`）。host bridge は transport=None を渡す。
  `InBlockTransportSupport::Verified` を宣言できる証拠なし → `Declared`
  止まり、in-block 経路は不採用。

## Alternatives and Comparison

### A. Bridge の置き場所（境界 7）

| 案 | 判定 | 根拠 |
| --- | --- | --- |
| `ruviform-plugin-host` 内 module | 却下 | M2 設計で `plugin-host → audio は dev-dep のまま不変`（TA-005）と確定済み。prod 依存にすると層違反 + 記録済み決定との矛盾 |
| `ruviform-audio` 内 module | 却下 | engine が plugin 依存を持てない（Issue #113 の dev-dep AC3、engine は plugin に依存しない規則） |
| `ruviform-plugin-core` 内 | 却下 | foundations 層が graph/audio 層に依存する逆向き |
| **新規 crate（`ruviform-audio-plugin-bridge`）** | **採用** | 結合 API（bus→port 写像・endpoint→GraphProcessor・scheduler owner）を置ける既存 crate がなく、設計 baseline の「条件付き」候補と合致。循環なし（誰も bridge に依存しない） |

### B. TransportScheduler の owner

| 案 | 判定 | 根拠 |
| --- | --- | --- |
| 非 RT control 側に scheduler、RT へ span 配送 | 却下 | `run_callback` は callback の EngineFrame を要し、制御側は callback タイミングを持たない。lookahead は別レイテンシを導入し exact apply を壊す |
| **node 内蔵 + SPSC control channel** | **採用** | `stage()` 相当は capacity 内で非 alloc・有界。`install_plan`/`request`/`device_restart` を control message として node 側で適用。receipt/fault は `pop_receipt`/`pop_fault`（media-core に RT-safe な pop 追加）→ report channel → 非 RT drain。`Arc` 差替は PlanVault（RT 側で destructor が走らないよう clone 保持 + retire channel + leak-on-full）で防御 |

### C. event/transport payload

| 案 | 判定 | 根拠 |
| --- | --- | --- |
| `NodeEvent`（u32+f32）を流用 | 却下 | 表現力不足（generation 済みでも payload が plan/command を運べない） |
| **`P = EventKind` の `TransportPlan<EventKind>`** | **採用** | `build_call_event_list` の `kind` 写像が `*p` で済む最小の接着。MIDI(#188)/Param(#186) 未実装のため `EventKind::ParamValue` 系のみが現行到達可能。規格別完全対応は capability で正直に宣言 |

### D. `min_frames` 違反の防止層

| 案 | 判定 | 根拠 |
| --- | --- | --- |
| adapter 側のみ（endpoint.process gate） | 不十分 | L0 gate は既存だが L1 は child 契約委譲。「100 frame process を呼ぶ実装は不合格」の要求は「そもそも違反 call を生成しない」 |
| **`calls_for_span`/`BlockContract::partition` で legal partition のみ生成、undeliverable は silence + fault** | **採用** | #190 で確定した contract を typed path でもそのまま適用。決定的検証可能 |

## Tentative Conclusions and Hypotheses

1. **新規 crate `ruviform-audio-plugin-bridge`**（利用側層）。依存:
   plugin-host / plugin-core / ruviform-audio / ruviform-graph /
   media-core / rt / audio-core。
2. `plugin-core` に `BusRole`（`#[non_exhaustive]` Main/Aux/Sidechain/
   Send/Return）を追加し `BusDescriptor.main: bool` を `role: BusRole`
   に置換（`is_main()` 互換 helper 付き）。`Plugin::buses()`/
   `ErasedPlugin::buses()` を default `&[]` で追加。
3. `plugin-host` に `buses(handle) -> Result<BusReport, HostError>`
   （`BusReport::{Reported(Vec<BusDescriptor>), Unavailable}`）を追加。
   L0 は core lock で読む（非 RT）、L1 は wire verb 未実装のため
   `Unavailable` を正直に返す。
4. node 内蔵 `TransportScheduler<EventKind>` + `PluginNodeControl`
   （非 RT handle、SPSC）。media-core に `pop_receipt`/`pop_fault`
   （容量 bounded の RT-safe 取り出し）を追加。
5. bus→port 写像は `BusDecl { descriptor, role }` を consumer が宣言し、
   host 側 report（あれば）と検証。`enabled` bus → `Activation::Enabled`、
   disabled → `Disabled`。port 順 = bus 順（direction 別 ordinal が
   `NodeIo::input(i)/output(i)` index と一致）。
6. attach では `ProcessConfig`（activated）+ `EndpointBounds` を受け、
   `config.max_frames ≤ bounds.max_frames` 等を非 RT で検証。
7. `Capability Matrix` は `capability_for(Backend)` のコード定数 +
   docs 表で L0/L1/CLAP/VST3 × {bus enum/multi-bus/events/transport/
   in-block/process-config/state/crash isolation} を Verified/Declared/
   Unknown/Unavailable/Unsupported に分別。
8. 旧単一 bus `process_audio` 経路はそのまま残す（Rollback 経路）。

## Open Questions

- `Arc<TransportPlan>` の RT drop 回避策として PlanVault（bounded
  keep-alive + retire channel + VaultExceeded fault）で十分か、
  media-core 側に retire hook を入れるべきか — **設計で決定。Vault
  方式を採用する方針（media-core 変更不要・層を跨がない）**。
- L1 の bus 列挙は wire v4 拡張が必要 — **本 Issue では Unavailable と
  宣言し、wire 拡張は別 Issue へ defer**。
- 非 RT `preflight`（pending+loop wrap 含む全状態 simulation）は node
  内蔵 scheduler に届かない — **control handle の静的 contract 検査 +
  receipt 応答を以て代替し、完全 preflight は deferred として宣言**。

## Promotion Candidates

- spec: Issue #196 の 8 boundary + M4-01〜08 Gate を spec 化
- adr: bridge crate 新設 + scheduler owner + plan retire 方式 + BusRole
  追加 + L1 unavailable 宣言
- design: 上記を型/API レベルに落とす

## Sources and Relations

- 要件正本: ruviform#196（上記 source）。
- 前提マージ済み: #184（typed graph IR）/#195（typed runtime）/
  #190（transport span + bridge）/#189（rational beat）/#185（probe）。
- 未実装前提: #188 Timed MIDI・#186 Parameter Event（capability 宣言で
  正直に扱う）。

## Evidence

- `crates/ruviform-plugin-host/src/endpoint.rs`（L0 activated gate /
  multi-bus `process` / `process_audio` 単一 bus）
- `crates/ruviform-plugin-host/src/host.rs`, `composite.rs`（slot・Shared
  vs Plain・`plugin_mut() -> None` for Shared/L1）
- `crates/ruviform-plugin-core/src/processor.rs`/`event.rs`/`transport_bridge.rs`
- `crates/ruviform-media-core/src/transport_schedule.rs`
  （`TransportScheduler` fields/`stage`/`run_callback`/`device_restart`/
  `push_receipt`/`push_fault` の capacity guard）
- `crates/ruviform-audio/src/typed/{runtime,compile,io}.rs`
- `crates/ruviform-graph/src/port.rs`
- `crates/ruviform-plugin-clap/src/{lib,host}.rs`・`crates/ruviform-plugin-vst3/`
  （single stereo main・CLAP transport field・host bridge `!Send`）
