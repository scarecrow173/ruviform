---
id: "SPEC-hmdguU023dLBLtBKCM9kfH"
type: "spec"
status: "approved"
title: "Issue #196 M4 Typed Graph Plugin Adapter — multi-bus/stateful lifecycle/process offset precision"
created: "2026-11-02"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/196"
    - "https://github.com/scarecrow173/ruviform/issues/192"
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
    - "docs/designs/issue-195-m2-typed-graph-compiler-buffer-pdc-rt-runtime-ownership-activation.md"
    - "docs/specs/issue-190-transport-span-scheduler.md"
    - "docs/adr/transport-span-scheduler-applied-receipt.md"
    - "docs/adr/plugin-rt-ownership-split.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/specs/issue-189-rational-beat-variable-ppqn-m3.md"
    - "docs/designs/issue-184-typed-graph-ir-ports-buses-connections-validation-revisions-m1.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md"
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/188"
    - "https://github.com/scarecrow173/ruviform/issues/186"
    - "https://github.com/scarecrow173/ruviform/issues/113"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/188"
    - "https://github.com/scarecrow173/ruviform/issues/186"
  deferred-by: []
---

# Issue #196 M4 Typed Graph Plugin Adapter — multi-bus/stateful lifecycle/process offset precision

## What Should Be Built

typed multi-bus graph（#184/#195）へ plugin instance を安全に接着する
**`ruviform-audio-plugin-bridge`**（新規・利用側層）を実装する。
bus capability → typed port 写像、activated `ProcessConfig` に基づく
physical process call 分割、callback/span/process 3 座標系の offset 変換
正本、stateful lifecycle の owner 定義、fault/unsupported/degrade 契約を
検証する。`ruviform-plugin-core` には最小の bus 列挙契約（`BusRole` +
`buses()`）、`ruviform-plugin-host` には非 RT `buses(handle)` を追加する。
旧単一 bus `process_audio` 経路は残し、Rollback 可能にする。

## Why Now

- Synvelis v0.5 §5.4/§14 が「typed graph + multi-bus plugin + transport span
  配送」を要求しており、M1(#184)/M2(#195)/M3(#189,#190) が揃った今、
  plugin の adapter 側が最後の未接着層。
- 現行 `PluginEndpoint::process` の multi-bus erased view があるが、
  `process_audio` は単一 bus shell で `NodeIo` の port 群と
  `TransportScheduler` の span 配送を繋ぐ実装が存在しない。
- bus 列挙・`BusDescriptor` 消費・`min_frames`/`ProcessConfig` 事前検証・
  event offset 変換正本・capability matrix が未整備。
- Issue 指示: 未実装の #186/#188 を実装済みと見なさず、capability 宣言で
  正直に扱う。

## Users and Value

- **Synvelis（DAW consumer）:** typed graph 上で multi-bus plugin を
  stateful に駆動し、transport span/event/offset を正確に配送する。
- **AudioLens / generic consumer:** graph 上の plugin fault/state/capability
  を監査可能にする。
- **plugin adapter 実装者（clap/vst3/将来）:** capability 宣言の正本を
  共有し、規格別差分を正直に宣言する。

## Scope

### In Scope

- **新規 crate `ruviform-audio-plugin-bridge`**（利用側層、engine と同順位）:
  - `BusDecl`/`bus_decl_to_port_def`/`validate_bus_decls` — bus→port 写像と
    検証（missing port・role・layout・direction）。
  - `PluginNode`（`GraphProcessor`）— `PluginEndpoint` を wrap し、
    node 内蔵 `TransportScheduler<EventKind>` + control channel で
    `run_callback` → span → `calls_for_span` → 物理 `process` 呼出し。
    `NodeIo` の input/output を `[offset, offset+frames)` に windowed する
    stack view を構築（no alloc/no unsafe）。unconnected enabled input は
    silence scratch、disabled bus は empty view。
  - `PluginNodeControl`（非 RT handle）— `request`/`install_plan`/
    `set_clock`/`device_restart`/`drain_reports`/`status`。
  - `PluginNodeFactory`（`NodeFactory`）— NodeId 別に attach 済み node を
    `PrepareOutcome::{New,Reuse}` で供給。構成変更は New、同一構成は Reuse。
  - `AdapterCapability`/`capability_for(Backend)` — capability matrix の
    コード正本。
  - `NodeStatus`（`Arc<AtomicU64>` 系）— bridge fault 観測。
  - `PlanVault` — `Arc<TransportPlan<EventKind>>` の RT-drop 回避
    （keep-alive + retire channel + leak-on-full + VaultExceeded fault）。
- **`ruviform-plugin-core` 最小拡張:**
  - `BusRole`（`#[non_exhaustive]` Main/Aux/Sidechain/Send/Return）。
  - `BusDescriptor.main: bool` → `role: BusRole` 置換 + `is_main()`。
  - `Plugin::buses()`/`ErasedPlugin::buses()`（default `&[]` = 未宣言）。
- **`ruviform-plugin-host` 最小拡張:**
  - `PluginHost::buses`/`InProcessHost::buses` →
    `Result<BusReport, HostError>`（`Reported(Vec)`/`Unavailable`）。
    L0 は Shared core lock（非 RT）で読む。L1 は `Unavailable`。
- **media-core 最小拡張:**
  - `TransportScheduler::pop_receipt`/`pop_fault`（capacity bounded・RT-safe
    な `Vec::remove(0)`）— node が receipt/fault を report channel へ
    橋渡しするために必要。
- **events/offset:** span window → physical call への process-relative
  `EventList` 変換（`build_call_event_list`）、same-frame causal order、
  generation 不一致 event の遮断、overflow → fault + event drop。
- **fault 契約:** graph swap（plan generation）・state save/restore・
  crashed/missing plugin・retire overflow・control/report channel
  overflow・undeliverable span → silence + fault の defined 挙動。
- **L0/L1 差:** L0 は `min_frames` 強制を実測。L1 は child 委譲のため
  capability で区別。CLAP `CLAP_EVENT_TRANSPORT` は証拠なし →
  `Declared` 止まりで採用しない。VST3 は個別 capability 宣言。
- **検証:** multi-bus E2E・Loop=100/min=128 違反 0 件・座標一致・fault・
  回帰・rt_ 無割当。

### Out of Scope

- Plugin State 公開/差戻しの Instance 二重駆動（RT 上での heap 破棄は
  Vault + Retirer で防止、public state 面は従来の verb のみ）。
- Synvelis Automation Write/Plugin UI/Track/Project State 意味論、各 plugin
  製品の Bit-perfect 保証。
- `CLAP_EVENT_TRANSPORT` の adapter 実装・実 plugin 検証（`Verified` 化は
  将来 Issue）。
- Timed MIDI(#188)、Parameter Event(#186) の規格別完全対応。
- L1 wire bus 列挙 verb（wire v4 拡張・別 Issue）。
- 非 RT `preflight` の pending+wrap 全状態 simulation（control 静的検査 +
  receipt で代替、完全版は deferred）。
- 旧 `process_audio` 単一 bus shell の削除（Rollback 経路として維持）。

## Requirements

- R1: `BusDecl` を `PortDef` へ写像する。`main`/`Aux`/`Sidechain`/`Send`/
  `Return` を `PortRole` へ、`enabled` を `Activation` へ、`layout` を
  `AudioFormat` へ対応させる。missing port・不一致は defined error。
- R2: `Plugin`/`ErasedPlugin`/`PluginHost` で bus 列挙を提供。L1 は
  `Unavailable` を返し、未検証を `Verified` と扱わない。
- R3: node は `TransportScheduler<EventKind>` を内蔵し、control channel
  の `request`/`install_plan`/`set_clock`/`device_restart` を callback 先頭で
  有界に適用。receipt/fault は report channel で非 RT へ返す。
- R4: `EventList.frame_offset` は process call 開始相対。
  callback→span→process の offset 変換は `build_call_event_list` を正本とし、
  same-frame causal order は `source_sequence` 安定順を保持する。
- R5: `calls_for_span`/`BlockContract::partition` で legal な physical call
  のみを生成。legal でない span は `deliverable==false` で silence + fault。
  `frames < min_frames` または `> max_frames` の `process` を 0 件にする。
- R6: node の `process` は `NodeIo` の各 input/output port を windowed
  view として plugin の bus 順に変換する（no alloc/no unsafe、stack
  `core::array::from_fn` + `iter_mut`）。unconnected enabled input は
  silence scratch、disabled bus は empty view。
- R7: graph swap は `GraphRuntime::publish` の generation 一致 install/
  remove で plugin instance を重複実行させない。state save/restore は
  nonRT verb のみ。Retire は `Retirer` + `PlanVault` で RT drop を回避。
- R8: crash/missing/Failed plugin は `silence + Failed` に converge し、
  RT で block/hang/alloc しない。`EndpointUnavailable`/`Lifecycle` は
  defined rejection。
- R9: capability matrix は backend（L0/L1/CLAP/VST3）× {bus enumeration,
  multi-bus, aux/sidechain, events, in-block transport, min_frames, state,
  crash isolation} を Verified/Declared/Unknown/Unavailable/Unsupported で
  宣言する。`Unknown`/`Declared` を採用済みと見なさない。
- R10: 旧 `process_audio` 単一 bus 経路を Rollback 経路として保持。

## Acceptance Criteria

- [ ] AC-1 (M4-01): `capability_for(Backend)` が L0/L1 + 採用規格ごとの
  capability を Verified/Declared/Unknown/Unavailable/Unsupported で返し、
  code/doc matrix が一致する。
- [ ] AC-2 (M4-02): Main+Aux/Sidechain と Mono/Stereo の `PortDef` 写像・
  typed graph validation・構成変更時の port/node identity・state 保存・
  missing port error を検証。
- [ ] AC-3 (M4-03): 256F callback + loop boundary=100 + `min_frames=128`
  で違反 `process` 呼出し 0 件、`Exact`/`Degraded`/`undeliverable` 判定
  に誤報なし。
- [ ] AC-4 (M4-04): callback/span/process 座標で event timestamp/offset が
  一致。境界 off-by-one・0-frame・same-frame 順・wrap occurrence を再現。
- [ ] AC-5 (M4-05): CLAP in-block transport は証拠なし → `Declared` で
  採用せず、VST3 等へ無検証一般化しない。
- [ ] AC-6 (M4-06): graph swap / state restore / crashed / missing /
  retire overflow で RT no-alloc/no-block・last-good/bypass/明示 fault を
  維持。
- [ ] AC-7 (M4-07): TT-025〜030 と TT-001〜024 の該当ケースを test/spec/
  実行証拠へ紐付け。未対応は名前と理由を残す。
- [ ] AC-8 (M4-08): `cargo test -p ruviform-plugin-host --locked` と
  `cargo test --workspace --locked`、L0/L1 別検証記録、fmt/check/lint/
  test/test-rt 緑。

## Deferred Design Concerns

- `CLAP_EVENT_TRANSPORT` の adapter 実装・実 plugin 証拠 → `Verified` 化は
  別 Issue。
- L1 wire bus 列挙 verb（wire v4）→ 別 Issue。
- 非 RT `preflight` の全状態 simulation → control 静的検査 + receipt で代替、
  完全版は別 Issue。
- `InProcessHost::buses`/`PluginHost::buses` の L1 wire 対応 → 別 Issue。
- `EndpointBounds` に `min_frames` を追加するかは compat 確認後の別 Issue。

## Sources and Relations

- 要件正本: [ruviform#196](https://github.com/scarecrow173/ruviform/issues/196)。
- 前提 spec/design/adr: `docs/specs/issue-190-transport-span-scheduler.md`、
  `docs/designs/issue-195-m2-typed-graph-compiler-buffer-pdc-rt-runtime-ownership-activation.md`、
  `docs/designs/issue-184-typed-graph-ir-ports-buses-connections-validation-revisions-m1.md`、
  `docs/adr/plugin-rt-ownership-split.md`、
  `docs/adr/transport-span-scheduler-applied-receipt.md`。
- 上流: `docs/discovery/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md`。
- 下流: ADR/design/plan/test-spec/tasks が本 spec を上流として derive する。
