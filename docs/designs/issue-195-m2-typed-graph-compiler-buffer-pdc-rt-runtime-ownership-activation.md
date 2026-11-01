---
id: "DESIGN-034QfjYdt3o2a4lijg4lwt"
type: "design"
status: "approved"
title: "Issue #195 M2 Typed Graph Compiler・Bounded Buffers/PDC・RT Runtime・Stateful Ownership・Activation"
created: "2026-11-01"
updated: "2026-11-01"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/195"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-195-typed-graph-audio-compiler-buffer-pdc-stateful-ownership-activation-m2.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-184-typed-graph-ir-ports-buses-connections-validation-revisions-m1.md"
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
    - "docs/designs/audio-engine.md"
    - "docs/discovery/issue-195-m2-typed-graph-compiler-runtime-audit.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/audio-engine-snapshot-swap.md"
    - "docs/adr/plugin-rt-ownership-split.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/specs/audio-engine.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-195-typed-graph-audio-compiler-buffer-pdc-stateful-ownership-activation-m2.md"
    - "docs/adr/m2-typed-graph-audio-runtime-placement-stateful-rt-ownership-and-activation-reti.md"
    - "docs/discovery/issue-195-m2-typed-graph-compiler-runtime-audit.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-195-m2-typed-graph-compiler-runtime-tests.md"
  references: []
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/196"
    - "https://github.com/scarecrow173/ruviform/issues/188"
    - "https://github.com/scarecrow173/ruviform/issues/190"
    - "https://github.com/scarecrow173/ruviform/issues/185"
  deferred-by: []
---

# Issue #195 M2 Design: Typed Graph Compiler・Buffers/PDC・RT Runtime・Ownership・Activation

## Context

`ruviform-graph`（M1）が提供する `ValidatedGraph`+`StableHandleMap` を、`ruviform-audio` 内の非 RT Compiler が immutable `PreparedPlan`（deterministic schedule + bounded buffer slot layout + boundary ports + processor install 計画）へ変換し、RT 側単一 owner `RtRuntime` が `ruviform_rt::snapshot`/`spsc`/`retire` primitive を再利用して駆動する。spec の AC-1〜AC-9・ADR の Option B（Stable Runtime Slot、PoC 実測前提）を実装形状へ落とす。

## Scope

### In Scope

- `ruviform-audio` 新 module `typed`: `buffer.rs`（AlignedSamples/Slot/SlotArena）、`io.rs`（NodeIo/view）、`compile.rs`（Compiler/CompiledPlan/CompileError/PDC/lifetime 検証）、`runtime.rs`（GraphProcessor/RuntimeCommand/RuntimeEvent/GraphRuntime/RtRuntime/RuntimeFault/SharedStatus）。
- `ruviform-audio` に `ruviform-graph` path 依存追加。integration test `tests/typed_m2.rs`・`tests/golden_legacy.rs`・`tests/ownership_poc.rs`。
- 旧 API 不変（`graph.rs`/`plan.rs`/`session.rs`/`mixer.rs`/`denormal.rs` には手を入れない）。

### Out of Scope

- Plugin adapter 接続（M4/#196）、TimedEvent/Control payload（M3）、Probe（#185）、Offline driver（#191）、旧 API 削除、Session/device 配線の置換（typed path は additive・既存 Session と並存）。

## Components and Boundaries

### crate 配置と依存（ADR 決定）

- `ruviform-audio`（engine 層）が `ruviform-graph`（構造層）を下向き参照。`ruviform-rt`/`ruviform-audio-core` は既存依存のまま。`ruviform-graph` 側には一切追加しない（TA-003 維持）。plugin-host → audio は dev-dep のまま不変（TA-005）。

### `typed::buffer` — bounded slot arena

```rust
pub struct AlignedSamples { buf: Vec<f32>, offset: usize, channels: usize, stride: usize, frames: usize }
// channels×frames の planar。alignment >= 64B: Vec を (align/4-1)+len で over-allocate し
// `as_ptr() as usize` を alignment に丸めた offset から [channels*stride] を使用（unsafe なし・
// ポインタ演算と slice index のみ）。stride = frames を align 境界にパッド。
pub struct Slot { data: AlignedSamples, pub channels: u16, pub frames_valid: u32, stamp: u64 }
// stamp: この slot が何番目の callback で書かれたか。consumer は stamp==現在 seq を要求。
pub struct SlotArena { slots: Vec<RefCell<Slot>> }  // prepare 時に全確保・RT で resize しない
```

- 全 slot は prepare 時に `max_frames`・alignment で確保（最大 slot 数も budget で宣言）。RT 中に arena の shape は不変 → `Vec` 再割当なし・`RefCell` borrow flag のみ（lock ではない）。
- 境界 slot（caller が埋める graph input）は `input_writer` が `stamp = seq+1`（次回 process の seq）を事前設定。

### `typed::io` — processor への borrowed IO

```rust
pub struct NodeIo<'a> {
    arena: &'a [RefCell<Slot>],
    delay_lines: &'a [RefCell<DelayLine>],
    binding: &'a NodeBinding,   // plan 内 immutable metadata
    pub frames: usize,
    seq: u64,
}
impl NodeIo<'_> {
    pub fn inputs(&self) -> usize;                    // binding.inputs.len()
    pub fn input(&self, i: usize) -> Option<InputView<'_>>;   // Ref<Slot> 解決（direct or scratch）
    pub fn output(&self, i: usize) -> Option<OutputView<'_>>; // RefMut<Slot>
}
// InputView/OutputView: channels()/frames()/channel(c)->&[f32]/copy_from 等の read/write API。
```

- 事前構築 `Vec<InputView>` を避け per-access materialize → **RT alloc 0**。
- alias 拒否: 各 node の input slot 集合と output slot 集合は compile で disjoint 検証（`CompileError::BufferAlias`）。`RefCell` が残りの二重 borrow を panic で防ぐ（test で audit）。
- uninit-read 拒否: merge materialize・processor read 時に producer slot `stamp != seq` → `RuntimeFault::UninitializedRead` を shared status に記録し scratch は 0-fill（deterministic degrade・UB なし）。unit test が壊れた schedule で fault を誘発（crate 内 test フック）。

### `typed::compile` — 非 RT Compiler

```rust
pub struct CompilerConfig { pub max_frames: usize, pub alignment: usize, pub max_slots: usize,
                            pub max_delay_frames: u32, pub max_processors: usize }
pub trait NodeFactory {                        // 非 RT: processor の用意と latency 報告
    fn prepare(&mut self, node: &NodeSpec, existing: Option<&InstalledInfo>)
        -> Result<NodeBinding, FactoryError>;
}
pub struct NodeBinding { pub processor: Option<Box<dyn GraphProcessor>>, pub latency: u32 }
// processor=None → boundary node（caller-fed Source out / tap-only Sink in）
pub struct CompiledPlan {
    pub generation: u64,
    pub schedule: Vec<ScheduledNode>,        // topo 順（StableHandleMap の runtime index 順）
    pub slots: Vec<SlotSpec>,                // {channels, frames, align} — arena 形状
    pub bindings: Vec<NodeBinding>,          // node ごとの input/output wiring + edge map/delay
    pub delay_lines: Vec<DelaySpec>,         // per-edge PDC ring の確保仕様
    pub inputs: Vec<(PortId, usize)>,        // graph input: PortId -> slot index
    pub outputs: Vec<(PortId, usize)>,       // graph output: PortId -> slot index
    pub installs: Vec<Install>,              // {slot_idx, node: NodeId, processor} 移送物
    pub total_latency: u32,
}
#[non_exhaustive] pub enum CompileError {
    UnsupportedEdgeDelay { connection: ConnectionId },
    MissingProcessor { node: NodeId },   // Processor kind で factory が None を返した（定義済み失敗）
    BufferAlias { node: NodeId },
    SlotOverflow { needed: usize, max: usize },
    DelayOverflow { needed: u32, max: u32 },
    BoundaryConflict { port: PortId, reason: &'static str },
    Factory { node: NodeId, source: FactoryError },
    Undetermined { reason: &'static str },
}
```

compile アルゴリズム:

1. `StableHandleMap` の runtime index 順に node を走査（deterministic・BTreeMap tie-break 継承）。
2. 各 node の output port ごとに 1 slot（channel = PortFormat::Audio layout）。**in-place 再利用なし** → input/output は構造的に disjoint。
3. input port ごとに `InputBinding { edges: Vec<EdgeBinding>, scratch: Option<usize> }`。`EdgeBinding { src_slot, channel_map, delay_frames, delay_idx }`。scratch slot は multi-edge・map・delay>0 のいずれかで割当。
4. edge `delay` が `EdgeDelay::Declared` → `UnsupportedEdgeDelay`（M1 で cycle 側は既拒否・DAG 上の宣言は量を持たず実行不能のため）。
5. latency: `accum[n] = latency[n] + max_{inputs}(accum[prod])`（input 無しは自身のみ）。merge では `edge_delay = max_p(accum[p']) - accum[p]`。`total = max accum`。`max_delay_frames` 超過 → `DelayOverflow`。graph PDC は node latency 整列のみで、ADC/DAC・録音位置補正とは別系統（二重計上しない）。
6. lifetime 監査（compile-time assert）: 全 producer slot が consumer より schedule 先行・input∩output=∅・scratch の write 先行。
7. `existing` registry（NodeId→InstalledInfo{slot_idx,latency}）を factory へ渡し、同一 NodeId は新規作成しない（Option B: state 継続）。slot index は registry で再利用・未使用は掃き出し remove 対象へ。

### `typed::runtime` — RT 実行と ownership

```rust
pub trait GraphProcessor: Send {
    fn latency(&self) -> u32 { 0 }
    fn activate(&mut self, _ctx: &RtContext) {}     // install 後 1 回（RT・短 bounded 契約）
    fn event(&mut self, _ev: &NodeEvent, _ctx: &RtContext) {}  // generation 照合済みのみ配送
    fn process(&mut self, io: &mut NodeIo<'_>, ctx: &RtContext);
    fn deactivate(&mut self, _ctx: &RtContext) {}   // remove 前 1 回
}

pub struct GraphRuntime {   // 非 RT 側 owner
    writer: SnapshotWriter<PreparedPlan>, producer: Producer<RuntimeCommand>,
    registry: BTreeMap<NodeId, InstalledInfo>, free_slots: Vec<usize>,
    collector: DropCollector<Box<dyn GraphProcessor>>,   // RT が retire した旧 processor の drain
    status: Arc<SharedStatus>, generation: u64,
}
pub enum RuntimeCommand {  // spsc payload（全て Send・generation tag 付き）
    Install { generation: u64, slot: usize, node: NodeId, processor: Box<dyn GraphProcessor> },
    Remove  { generation: u64, slot: usize },
}
pub struct NodeEvent { pub generation: u64, pub node: NodeId, pub opcode: u32, pub value: f32 }

pub struct RtRuntime {      // RT 側単一 owner（Send・!Sync・Clone 不可）
    reader: SnapshotReader<PreparedPlan>, consumer: Consumer<RuntimeCommand>,
    events: Consumer<NodeEvent>,
    processors: Vec<Option<Box<dyn GraphProcessor>>>,   // stable slot registry
    arena: Vec<RefCell<Slot>>, delay_lines: Vec<RefCell<DelayLine>>,
    retirer: Retirer<Box<dyn GraphProcessor>>,           // 旧 processor の非 RT 送り
    active_generation: u64, seq: u64, status: Arc<SharedStatus>,
}
```

- `GraphRuntime::publish(compiled)`（非 RT）: installs を `producer.try_push`（失敗→`ActivationError::CommandChannelFull`・publish しない=rollback）→ `writer.publish(Arc::new(plan))` → generation++。
- `RtRuntime::process(ctx, frames)`（RT）:
  1. `frames > max_frames` → `ProcessError::FramesExceedMax`。`frames == 0` → Ok（no-op・processor 非呼出）。
  2. `seq += 1`。`reader.read()` guard で plan 取得。`plan.generation != active_generation` → command drain（generation 一致のみ apply・不一致は stale 計数）→ install（processor を slot へ・`activate`）→ remove（`deactivate`→`retirer.retire`）→ `active_generation = plan.generation`。
  3. event drain: `ev.generation != active_generation` → stale 計数・配送しない。一致 → 対象 processor の `event()`。
  4. schedule 順に各 node: input materialize（edge ごと `Ref` 読み出し+map+DelayLine 適用 → scratch `RefMut` へ累積）→ `processors[slot].process(&mut io, ctx)` → output slot `stamp = seq`・`frames_valid = frames`。
  5. fault は shared atomic status（bit フラグ + カウンタ: stale_events・uninit_reads・retire_overflows・install_conflicts）。
- `Drop for RtRuntime`: processor Box を `retirer.retire`（try_push・full は mem::forget+計数）→ RT 側で destructor を走らせない。
- `GraphRuntime::collect()`/`retire_overflows()`/`faults()` で非 RT 観測。`NodeIo` 借用の `RefCell` は `&self` 共有 borrow で足りる（runtime が `&mut self` を持つため panic も設計上発生しない）。

### Ownership PoC（`tests/ownership_poc.rs`・同一 MockDsp）

- **Stable-slot harness**: production `GraphRuntime`/`RtRuntime` をそのまま使用。
- **Plan-owned harness**: test 内で `Mutex<PlanOwnedInner{Vec<Box<dyn GraphProcessor>>}>` 付き plan を `snapshot::slot` で publish する等価経路（`try_lock`→process・retire channel）。production API には露出しない。
- 測定項目 = ADR PoC 判定基準 6 項目 + swap あたりの install 数・box 移送数。結果を impl record に記録し ADR を accepted 更新。

## Data and Control Flow

```
GraphBuilder → GraphProject::prepare → ValidatedGraph(+StableHandleMap)
  → Compiler::compile(&validated, &mut factory, existing_registry, config)
       → CompiledPlan{schedule, slots, bindings, delays, inputs, outputs, installs}
  → GraphRuntime::publish(compiled)
       ├─ producer.try_push(Install/Remove{gen}) × n   (full → Err, publish 中止)
       ├─ writer.publish(Arc<PreparedPlan>)            (ArcSwap)
       └─ generation += 1
  → RtRuntime::process(ctx, frames)                   (RT thread, &mut self)
       ├─ read() → guard                              (lock-free + Arc clone)
       ├─ gen 変化 → command drain → activate/deactivate
       ├─ event drain（旧 gen 遮断）
       └─ schedule: materialize → processor.process → stamp
  → retire: Retirer → (非 RT) GraphRuntime::collect() → destructor
```

## Non-Functional Constraints

- **RT 契約**: `process` は `&RtContext` を要求。区間内の alloc/clone-heavy/blocking lock/block/OS-I/O を `alloc_counter` 監査 + `rt_` test で 0 検証。`RefCell`・atomic・spsc `try_push`/`try_pop`・ArcSwap load のみ使用。
- **決定性**: schedule = `StableHandleMap` の runtime index 順、install/remove・event は FIFO、ID/slot 採番は BTreeMap。同入力 2 回 compile で同一 plan。
- **unsafe 禁止**・`#[non_exhaustive]` public enum・crate-local error・`BTreeMap`・`Send`/`!Sync` の thread 契約を型で表現（RtRuntime: `Send`+`!Sync`、GraphRuntime: `Send`）。
- **互換**: 旧 module 不変。新 API は `pub mod typed` で additive 公開。
- **失敗**: compile/publish は全て定義済み失敗（Result）。RT 側は fault 記録+deterministic degrade（panic しない・test 内の RefCell audit のみ例外）。

## Risks and Trade-offs

- RefCell borrow flag 書き込みは atomic でないため `RtRuntime` は `!Sync`（single RT owner を型で強制）— 並列駆動は M2 範囲外。
- Option B の processor registry は NodeId→slot の sticky 写像を必要とし registry 拡張コストがあるが、state 継続（EX-008/010）を満たせる唯一の形。Plan-owned は単純だが swap ごとに state 消失 → spec AC-4 不適合（実測で確認予定）。
- stamp 検査は callback ごとの O(slot) メタデータのみで計測影響は微小。未初期化読取は「拒否」= fault+zero-fill degrade（RT で panic しない）。
- DelayLine ring は `delay_frames + max_frames` の確保で wrap を処理。PDC は audio domain 接続のみ適用（Event/Control は M3）。

## Deferred Design Concerns

- Plugin `HostInput`/`HostOutput` adapter・規格 bus 宣言写像 | M4 #196
- TimedEvent/Control port の runtime 意味・clock | M3/#188/#190
- Probe tap・offline driver・多 channel map 重み付け | #185・#191・consumer 要求時
- `EdgeDelay::Declared` の数量化（cycle 含む delayed feedback 実行） | IR の variant 拡張が必要な別 Issue
- Legacy→typed の consumer 移行・旧 API removal | 承認待ち別 Issue

## Verification Notes

- [ ] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` exit 0（exit code 記録）。
- [ ] `cargo test -p ruviform-audio --locked`・`cargo test --workspace --locked` exit 0。
- [ ] `cargo tree --locked -p ruviform-audio --edges normal` で `ruviform-graph`+`ruviform-rt`+`ruviform-audio-core`+`ruviform-media-core` のみ・循環 0。
- [ ] `rg "unsafe" crates/ruviform-audio/src/typed` → 0 match。
- [ ] design gate: `route_graph.js --current design` が design→planning edge を返す。
