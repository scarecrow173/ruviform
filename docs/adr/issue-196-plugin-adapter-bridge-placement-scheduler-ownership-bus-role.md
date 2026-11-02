---
id: "ADR-Tm4TW9CWQgvmIkMzROw0ei"
type: "adr"
status: "accepted"
title: "M4 plugin adapter: bridge crate placement, node-internal scheduler ownership, bus role enumeration, plan retire model"
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
    - "docs/specs/issue-190-transport-span-scheduler.md"
    - "docs/adr/plugin-rt-ownership-split.md"
    - "docs/adr/transport-span-scheduler-applied-receipt.md"
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md"
    - "docs/designs/issue-195-m2-typed-graph-compiler-buffer-pdc-rt-runtime-ownership-activation.md"
    - "docs/designs/issue-184-typed-graph-ir-ports-buses-connections-validation-revisions-m1.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md"
    - "docs/specs/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# M4 plugin adapter: bridge crate placement・scheduler ownership・bus role・plan retire

## Status

accepted

## Context

Issue #196 は typed multi-bus graph へ plugin instance を接着する。
調査で確定した制約:

- M2 設計が `plugin-host → ruviform-audio は dev-dep のまま不変`（TA-005）
  を確定済みで、plugin-host 内に typed 接着を置けない。
- engine は plugin に依存しない規則（Issue #113 AC3）があり、
  `ruviform-audio` 内に置けない。
- `plugin-core` は foundations 層で graph/audio へ依存できない。
- `TransportScheduler` の非 RT API（`install_plan`/`request`/`preflight`/
  `drain_*`）と RT API（`run_callback`）が分離しており、owner を決める
  必要がある。`pending`/`receipts`/`faults` は capacity bounded で
  `stage()` 相当は RT でも非 alloc。
- `Arc<TransportPlan>` の差替・pending reject は RT 上で Arc destructor
  を起動し得る（`device_restart` の pending drain、InstallPlan apply の
  旧 plan 差替）。「RT で heap resource を破棄しない」に抵触する。
- `BusDescriptor` は `main: bool` のみで Aux/Sidechain/Send/Return を
  表現できず、bus 列挙 API が `Plugin`/`ErasedPlugin`/`PluginHost` に
  ない。endpoint 経路の plugin は `plugin_mut()` が `None`。
- L1 wire に bus 列挙 verb がない。
- `EventList.frame_offset` は process call 開始相対（#190 で確定）。

## Decision

1. **Bridge を新規 crate `ruviform-audio-plugin-bridge`（利用側層）に置く。**
   `plugin-host`/`plugin-core`/`ruviform-audio`/`ruviform-graph`/
   `media-core`/`rt`/`audio-core` に依存し、誰も bridge に依存しない
   （循環なし）。`plugin-host → audio` は dev-dep のまま不変。

2. **`TransportScheduler<EventKind>` を `PluginNode`（RT 側）に内蔵させ、
   `PluginNodeControl`（非 RT）が SPSC control channel で command/plan/
   clock/restart を stage する。** `stage()` 相当の有界適用は RT で実行し、
   `pending`/`receipts`/`faults` の capacity guard を信頼する。
   receipt/fault は media-core に追加する `pop_receipt`/`pop_fault`
   （`Vec::remove(0)`・bounded）で node が取り出し report channel へ送る。
   非 RT `preflight` は control の静的 contract 検査 + receipt 応答で代替し、
   pending+wrap 全状態 simulation は deferred とする。

3. **`Arc<TransportPlan>` の RT drop は `PlanVault`（node 内 keep-alive +
   retire channel + leak-on-full）で防ぐ。** scheduler が Arc を差替/
   reject しても vault が少なくとも 1 ref を保持するため RT では refcount
   decrement のみ。vault は bounded（`plan_vault_capacity`）で、超過は
   `VaultExceeded` fault + install reject（定義済み・静かな成功禁止）。
   退避は retire channel 経由で非 RT collector へ。channel full は
   `mem::forget` + counter（`Retirer` の leak-on-full と同じ方針）。

4. **`BusRole`（`#[non_exhaustive]` Main/Aux/Sidechain/Send/Return）を
   `ruviform-plugin-core` に追加し、`BusDescriptor.main: bool` を
   `role: BusRole` に置換する。** `is_main()` helper を残す。
   `Plugin::buses()`/`ErasedPlugin::buses()`（default `&[]`）を追加し、
   `PluginHost::buses`/`InProcessHost::buses` → `Result<BusReport,
   HostError>`（`Reported(Vec)`/`Unavailable`）を追加。L1 は `Unavailable`
   を返す（wire v4 拡張は別 Issue）。未検証を `Verified` と扱わない。

5. **Event/offset の正本は `build_call_event_list`（plugin-core
   transport_bridge）に集約する。** `EventList.frame_offset` は process
   call 開始相対。callback→span→process の 3 座標変換は
   `event_engine_frame`/`span_event_window`/`calls_for_span` のみで行い、
   adapter に独自変換を持たせない。

## Considered Options

### Bridge 置き場所

- **`ruviform-plugin-host` 内 module** — 却下。TA-005（plugin-host →
  audio は dev-dep のまま）に反し、prod dep は層違反。
- **`ruviform-audio` 内 module** — 却下。engine が plugin に依存する
  逆向き。
- **`ruviform-plugin-core` 内** — 却下。foundations が graph/audio に
  依存する逆向き。
- **新規 crate `ruviform-audio-plugin-bridge`** — **採用。** 結合 API の
  正当な置き場所がなく、循環・層違反なし。設計 baseline の条件付き
  候補と合致。

### Scheduler owner

- **非 RT control 側に scheduler、RT へ span 配送** — 却下。callback の
  EngineFrame を RT が握るため制御側は先読み配送が必要になり、exact
  apply とレイテンシを壊す。
- **`PluginNode` 内蔵 + SPSC** — **採用。** `run_callback` の RT
  requirement を満たし、`stage` の bounded 性を実測根拠に使う。

### Plan 退避

- **media-core に retire hook を追加** — 見送り。層を跨ぐ API 変更で、
  PoC としては bridge 内 Vault の方が局所的。
- **`PlanVault`（keep-alive + retire channel）** — **採用。**
  `Retirer`/`leak-on-full` 既存パターンと一致。

### Bus role

- **`main: bool` のまま + bridge 側 BusDecl.role** — 却下。VST3/CLAP の
  bus 意味論を plugin が知っている方が正しく、bridge への推測移譲は
  検証不能を増やす。
- **`role: BusRole` に置換** — **採用。** 公開 API 変更だが 0.1.0・
  構築例は doc のみで blast radius が小さい。

## Consequences and compatibility boundaries

- 新規 crate が依存 DAG の最上位 consumer として追加される。
  `plugin-host → audio` は dev-dep のまま不変（TA-005）。
- `BusDescriptor` の `main: bool` → `role: BusRole` は公開 API の
  破壊的変更。0.1.0 かつ実利用が doc 例のみのため受容。`is_main()`
  で旧意味は保持。
- `Plugin::buses`/`ErasedPlugin::buses`/`PluginHost::buses` は additive。
  default `&[]` は「未宣言」を意味し、検証は consumer 宣言との
  一致がある場合のみ行う。
- `pop_receipt`/`pop_fault` は additive・RT-safe・capacity bounded。
- `EndpointBounds` への `min_frames` 追加は行わない（compat 観点で
  別 Issue）。activated `ProcessConfig` は attach で consumer 供給。
- L1 bus 列挙・非 RT 全状態 preflight・CLAP in-block transport は
  deferred。`Unknown`/`Declared`/`Unavailable` を採用済みと扱わない。

## Acceptance and validation

- M4-03: `calls_for_span`/`partition` のみで physical call を生成し、
  `min_frames` 違反を 0 件にする。
- M4-04: `build_call_event_list` が process 開始相対 offset を正本化。
- M4-06: vault + Retirer + generation 一致で graph swap / crashed /
  missing / retire overflow の fault 契約を維持。
- 実測: `cargo test -p ruviform-audio-plugin-bridge`・
  `cargo test -p ruviform-plugin-host --locked`・workspace 回帰。

## Sources

- [ruviform#196](https://github.com/scarecrow173/ruviform/issues/196)
- `docs/specs/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md`
- `docs/discovery/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md`
- `docs/adr/plugin-rt-ownership-split.md`
- `docs/adr/transport-span-scheduler-applied-receipt.md`
