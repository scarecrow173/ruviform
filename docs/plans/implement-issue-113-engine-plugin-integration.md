---
id: "PLAN-034MrrISpCH4LObubGof0Q"
type: "plan"
status: "in-progress"
title: "Issue #113 実施計画 — エンジン/利用側へのプラグイン統合（RT endpoint 所有分離 + グラフ接着）"
created: "2026-10-26"
updated: "2026-10-26"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/113"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/engine-plugin-host-integration.md"
  implemented-by:
    - "docs/tasks/plugin-core-endpoint-primitives.md"
    - "docs/tasks/l0-shared-slot-and-endpoint-inner.md"
    - "docs/tasks/l1-endpoint-arm-and-leg-return.md"
    - "docs/tasks/plugin-host-endpoint-surface.md"
    - "docs/tasks/audio-fnmut-node-and-plan-retirement.md"
    - "docs/tasks/plugin-graph-e2e-and-rt-lane.md"
    - "docs/tasks/engine-plugin-final-verification.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/implement-issue-74-plugin-host-l0.md"
    - "docs/plans/implement-issue-76-plugin-l1-out-of-process.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/engine-plugin-host-integration.md"
    - "docs/designs/engine-plugin-host-integration.md"
    - "docs/adr/plugin-rt-ownership-split.md"
    - "docs/designs/audio-engine.md"
    - "docs/adr/audio-engine-snapshot-swap.md"
    - "docs/designs/plugin-l1-out-of-process.md"
    - "docs/designs/plugin-host-l0.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  derived-by:
    - "docs/tasks/plugin-core-endpoint-primitives.md"
    - "docs/tasks/l0-shared-slot-and-endpoint-inner.md"
    - "docs/tasks/l1-endpoint-arm-and-leg-return.md"
    - "docs/tasks/plugin-host-endpoint-surface.md"
    - "docs/tasks/audio-fnmut-node-and-plan-retirement.md"
    - "docs/tasks/plugin-graph-e2e-and-rt-lane.md"
    - "docs/tasks/engine-plugin-final-verification.md"
  verifies: []
  verified-by:
    - "docs/test-specs/plugin-endpoint-contract-tests.md"
    - "docs/test-specs/plugin-graph-e2e-tests.md"
    - "docs/test-specs/plugin-rt-retirement-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #113 実施計画 — エンジン/利用側へのプラグイン統合（RT endpoint 所有分離 + グラフ接着） Implementation Plan

> status: in-progress — Refs #113。approved design `docs/designs/engine-plugin-host-integration.md`（DESIGN-034Mclz5y96MrTg7R2BNCx・決定サマリ a–k）を正本とし、spec `docs/specs/engine-plugin-host-integration.md`（proposed・11 AC）と ADR `docs/adr/plugin-rt-ownership-split.md`（accepted・transferable RT endpoint + non-RT 制御所有分離）の確定分岐を実装する。Task 1 (plugin-core-endpoint-primitives) 完了（actions 1–3・`HostError::EndpointUnavailable` local-only variant・`encode`/`encode_host_error` の `Result<(), WireError>` 化 + `encode_body` 抽出の function-boundary rollback（all-or-nothing append）・`create_sendable`/`erase_sendable` opt-in `Send` capability + host re-export parity・`AudioBufferView`/`AudioBufferViewMut` の `BufferView`/`BufferViewMut` impl・call site 4 件 + doctest 3 件の `Result` 対応・`fmt-check`/`check`/`lint`/`test` 55 binaries・`test-plugin-core` 103・`lint-plugin-core` 全 exit 0・独立 review 2 件 — A=APPROVE・B=APPROVE-WITH-FIXES・確定修正 4 件適用）。Task 2–7 未着手 — 全体完了を意味しない。**7 buildable task** と **3 test-spec** を起票済み（task は Task 1 `done`・残り `todo`・test-spec は全て `proposed`）。実装・commit は lead が管理し、GitHub merge は lead の authoritative `gh` のみ — 本計画は PR merged / issue closed の主張を含まない。

## Goal

`PluginHost`（`!Send`/`!Sync`・non-RT 制御側専有）と `Send` な per-instance `PluginEndpoint`（RT 処理側・グラフノードが所有）へ所有を分離し、L0/L1 双方のプラグインインスタンスを `ruviform-audio` の pull グラフ（`Graph`/`GraphPlan`/`Session` コールバック）から per-block 駆動できるようにする。接着は consumer 配線の `Send` closure のみ — engine は plugin 型を名指さず、host は engine 型を名指さない（baseline A.2）。

中核機構（design 決定サマリの pinned 形、全て後続 task が引き受ける）:

- **L0**: opt-in `Send` capability（`PluginFactory::create_sendable`/`erase_sendable`）+ `Instance` スロット enum 化（`Plain`/`Shared(SharedSlot)` — `SharedSlot { core: Arc<Mutex<SendCore>>, mirror: Arc<AtomicU8>, attached: Arc<AtomicU64>, bound, next_epoch }`・`SendCore { plugin, context, activated }`）+ 全 verb 3 段 lock 形態（entry pre-check → `match core.lock()` → lock 内 mirror 再読）+ commit 順序（transition commit → `set_state` mirror store → plugin hook を同一 CS 内）+ endpoint `try_lock`・lock 内再読・`catch_unwind` + `mirror.fetch_max(Failed)` 収束・epoch CAS attach/detach
- **L1**: `L1Instance.rt: Option<RtLeg>` の `take` で leg 移出 + `Shared::rt_state` の `Arc<AtomicU8>` 化 + per-attach arm channel（capacity 2・seed arm は `seed.generation > leg 現装着 generation` のときのみ apply・同 generation は正規化 no-op — detached-Activate の自己修復）+ `pending_arm` latest-wins 退避と **全 `&mut` non-RT entry point** 再送（`&self` query は defined non-drain）+ generation 単調 apply + drain した全 arm の `Retirer`（capacity 4）行き + `RtLeg::arm_apply` 簿記分割 + `last_output` 事前 `reserve` + leg 返却 channel（capacity 1）+ drain-before-early-return 順序
- **公開面**: `PluginHost::prepare_endpoint(handle, EndpointBounds) -> Result<PluginEndpoint, HostError>` + `PluginHost::latency_frames(handle) -> Option<u32>`（L0=`Some(0)`・L1=`Some(lookahead_blocks × activated max_frames)`・未 Activate/無効 handle=`None`）+ `PluginEndpoint::{process, process_audio, latency_frames, bounds}`（`process_audio` = bound clip → per-block `AudioBufferViewMut::try_new` view → channel-major 複写 → `inner.process` — in-bus-first gate・`Err(Prepare(UnsupportedConfig))` 系・`Err` は `out` 不変更）+ `HostError::EndpointUnavailable`（local-only variant — `encode`/`encode_host_error` の `Result<(), WireError>` 化 + `out.truncate(start)` all-or-nothing append）
- **engine シーム**: `AudioNode` の `FnMut` blanket impl + `plan.rs` 退役シーム（`RtPlan.retirer: Retirer<Arc<GraphPlan>>`・`active()` の `ptr_eq` retire・`Drop for RtPlan`・`PlanSwap` collector + 全 non-RT entry point drain + `drain_retired`/`retired_overflows`）

## Non-Goals

- 実規格アダプタ（CLAP/VST3/OFX）経由の実プラグイン動作 — Issue #114（real-adapter work・別 Issue 級）
- video plugin の RT/アウトプロセス経路・L1 video wire（`VideoPluginInstance` 全操作 NONRT — trigger: NONRT video port の分離要求・AC10 は境界明文化まで）
- OFX NONRT ノードの engine/video 側接続形（第 2 ターゲット video engine 統合）
- 自動再起動・supervisor ポリシー（#31 確定 — 再起動判断は呼出し側/プラグイン責務）
- `ErasedPlugin`/`Plugin` への全域 `Send` bound — opt-in capability のみ（ADR rejected）
- `ruviform-audio` → plugin-host 依存・plugin-host → engine 依存（baseline 禁止・相互に型非露出）
- `PluginHost`/`InProcessHost`/`L1Instance` の host 側 API 互換破壊（追加は `#[non_exhaustive]` 方針で）
- プラグイン GUI 配線・plugin chain の routing/自動接続・複数 bus（v1 グラフは順序リストのまま）
- standalone `InProcessHost` への endpoint API 配線（endpoint 面は v1 では `PluginHost` コンポジットのみ — design 確定）
- `PlanSwap`/`RtPlan` の snapshot `slot` 全面移行（退役 hole の最小修正のみ — 将来の選択肢として design deferred）
- endpoint `process` の `RtProcessor` 準拠（`Io` 関連型の借用形適合 — host 語彙一貫性を優先）
- `process_audio` 一律入力複写の最適化・`FnMut` ノード role メタデータ（YAGNI — source-position `fill(0.0)` 慣行で足りる）
- イベント・transport のグラフ配線（v1 は `EventList::empty`/`None` transport — music-time 統合の論点）

## Scope and Split Boundary

単一クリティカルパスの 1 計画（plugin-core 共有面 → L0/L1 endpoint 機構 → composite 公開面 → engine シーム → E2E/RT 検証の直列依存 — 独立並行は主張しない）。**7 buildable task**（task 文書は起票済み・各 task が ordered substeps A1…An を持つ）。commit は論理ビルド可能単位で lead が管理 — 1 substep/1 commit 強制・red 専用 commit はしない。

- Task 1（plugin-core 前提面: `EndpointUnavailable`・fallible encode・`create_sendable`・view impl）は全後続の共有前提 — L0/L1 双方が新契約を使う
- Task 2（L0 Shared スロット + endpoint inner）と Task 3（L1 leg 移出 + arm/返却経路）はともに Task 1 に依存し、さらに **T3→T2 の `depends-on` edge が task frontmatter に宣言済み** — file-sharing の直列化（両者が `src/endpoint.rs` を順次拡張する: Task 2 が enum 骨格 + `L0` variant、Task 3 が `L1` variant を充填するため T3 は T2 の骨格を前提とする）
- Task 4（`PluginHost` 公開面）は Task 2 + Task 3 に依存（統一 rejection は両層の機構が揃ってから）
- Task 5（audio `FnMut` impl + `plan.rs` 退役シーム）は plugin 側に依存せず論理上独立 — 単一レーンでは Task 4 後に実行（`ruviform-audio` へ plugin 依存は入らない — AC3）
- Task 6（endpoint `process`/`process_audio` shell + グラフ E2E + `rt_` レーン + 退役観測）は Task 4 + Task 5 に依存
- Task 7（final verification + closeout）は Task 2–6 の全完了後に 1 回のみ — full gate の繰返し実行はしない
- テストは test-spec 3 件の契約を、各クレートの inline `#[cfg(test)] mod tests`（単体）と `crates/ruviform-plugin-host/tests/` 統合テスト（dev-dep `ruviform-audio`・mock device・`CARGO_BIN_EXE_l1-test-worker`）で実装する

## Files / Ownership

| Path | Responsibility | Notes |
| --- | --- | --- |
| `crates/ruviform-plugin-core/src/boundary.rs` | `HostError::EndpointUnavailable` variant + `Display` arm（**local-only 宣言** — wire 経路に乗らない） | Task 1。`#[non_exhaustive]` で non-breaking |
| `crates/ruviform-plugin-core/src/wire.rs` | `encode_host_error` → `Result<(), WireError>`（wildcard → `Err(UnsupportedVariant)` — `encode_transport` 先例 L1260-1269 と同形）・`pub fn encode` → `Result<(), WireError>` + `out.truncate(start)` all-or-nothing append（`encode_reply` は `HostError` 不到達のため infallible 維持）・`Display`/roundtrip 用手動 variant リスト（L1611-1627・L1958-1971）・roundtrip ヘルパ更新 + negative test（`Reply{Err(EndpointUnavailable)}` → `Err(UnsupportedVariant)` + `out` 不変） | Task 1。`encode` は公開 API — signature 変更を plan で開示（pre-1.0 許容） |
| `crates/ruviform-plugin-core/src/plugin.rs` | `PluginFactory::create_sendable(&self) -> Option<Box<dyn ErasedPlugin + Send>>`（default `None`）+ `erase_sendable<P: Plugin + Send + 'static>` | Task 1。`Erased<P>` は `P: Send` なら `Send` — 全域 bound なし |
| `crates/ruviform-plugin-core/src/processor.rs` | `impl BufferView for AudioBufferView<'_, f32>` + `impl BufferView/BufferViewMut for AudioBufferViewMut<'_, f32>`（channel-major・`channel(i) = &data[i*frames..]` の const 実装 — `PlanarBufferView*` impl と同じ seam） | Task 1。`ruviform-audio-core` は既存依存 |
| `crates/ruviform-plugin-core/src/lib.rs` | `erase_sendable` re-export + crate doc 整合 | Task 1 |
| `crates/ruviform-plugin-host/src/l1/mod.rs`（L421）・`src/bin/l1-test-worker.rs`（L160）・`crates/ruviform-plugin-export/src/lib.rs`（L794）・`crates/ruviform-plugin-export/tests/l1_runner_e2e.rs`（L125） | `wire::encode` call site を `Result` 対応へ更新（`io::Error` 写像 — `decode` と同じ `InvalidData` 形）。statement-position caller は上記 4 箇所 — 加えて `wire.rs` の compile-tested doctest（L239・L641・L717 付近）と inline test ヘルパ/直接 call も `Result` に追従（`#[test]` 内の ignored `Result` も `lint` の `-D warnings` で fail するため全て処理必須） | Task 1 の波及修正（signature 変更の追随 — 全 caller・doctest・test helper を同 task 内で更新） |
| `crates/ruviform-plugin-host/src/l1/data.rs`（`mirror_encode`/`mirror_decode` — **`src/mirror.rs` は新設しない**） | `src/l1/` 木は `crate::`/`ruviform_plugin_host::` パス禁止（`l1/mod.rs` L19-26 — `tests/l1_proxy_e2e.rs` が `#[path]` include する制約のため crate 直下 `src/mirror.rs` を l1 から import できない）。**`mirror_encode`/`mirror_decode` は `l1::data` に維持し、L0 側は `crate::l1::data::` 経路で利用する**（`l1` 木の外側にある `endpoint.rs`/`host.rs` は `crate::` パス使用可 — 制約は `src/l1/` 木の内側のみ）。u8 順 `Created0/Initialized1/Activated2/Processing3/Failed4/Destroyed5` — `Failed`/`Destroyed` ≥ `Activated` なので `≥` 比較を不変条件に使わず列挙で書く | Task 2。`tests/l1_proxy_e2e.rs` は制約証人（crate-free discipline の維持確認 — 変更不要） |
| `crates/ruviform-plugin-host/src/host.rs` | `Instance` スロット enum（`Plain { plugin, context }`/`Shared(SharedSlot)`）・`Instantiate` で `create_sendable()` 分岐・全 verb 3 段 lock 形態 + commit 順序 + `destroy` `into_inner` 例外・`plugin_mut`→`Shared` で `None`・host `process`→`try_lock` busy は `Err(EndpointUnavailable)`・`lifecycle_state`→`Shared` は mirror Acquire 読みのみ（`core.lock()` 不許可・`&self` は `Failed` 観測 union のみ）・`pub(crate)` attach 内側（bounded 検証 + epoch CAS） | Task 2。`InProcessHost` の公開 endpoint API は作らない（v1 は `PluginHost` のみ） |
| `crates/ruviform-plugin-host/src/endpoint.rs`（新規） | `pub struct PluginEndpoint`（`inner: EndpointInner`・`bound: EndpointBounds`・`in_scratch: AudioBuffer<f32>`・`latency_frames: u32`）・`pub struct EndpointBounds { pub max_frames: u32, pub channels: u32 }`（公開 `u32` field — `(usize, usize)` タプルではない）・`enum EndpointInner { L0(L0Endpoint), L1(L1Endpoint) }`・`L0Endpoint`（core/mirror/attached Arc + epoch + `Drop` で CAS(epoch→0) 返却 + `catch_unwind` + `fetch_max(Failed)`）・`L1Endpoint`（`leg: Option<RtLeg>`・mirror・`arm_rx`・`arm_retirer`・`return_tx` + `Drop` で leg 返却 — **`endpoint.rs` 側で `crate::l1::data::EndpointLegParts` から組立てる**・`L1Instance::prepare_endpoint` は parts を返す — l1 木から `L1Endpoint`/`EndpointBounds` を名指さない crate-free 分離）・`EndpointInner::process`（variant gate — L0 = lock 内 `activated` `[min,max]`、L1 = `armed_frame_capacity` pre-gate で `frames ≤ armed staging.frame_capacity` を `Err(Prepare(UnsupportedConfig))` — `Err(Ipc)` が `process`/`process_audio` 公開面に出ない） | Task 2 が骨格+L0・Task 3 が L1 variant・Task 6 が `process`/`process_audio` 公開 shell。`Send` 証拠は `unsafe impl` なしの構造型のみ |
| `crates/ruviform-plugin-host/src/l1/data.rs` | `RtLeg::arm_apply(&mut self, arm: Arc<Staging>, retirer: &mut Retirer<Arc<Staging>>)` 新設（ring 束縛のみ reset — `staging`/`frame_capacity`/`last_output` 中身/`last_frames`/`last_channels`/`have_output`; seq 空間簿記 `last_pushed`/`cleared`/`held`/`submit_dims`/`next_block`/`depth`/`lookahead` は持続）+ **`EndpointLegParts`（`pub(crate)` struct — `leg`/`mirror`/`arm_rx`/`arm_retirer`/`return_tx` のプリミティブ束・crate-free — `endpoint.rs` がここから `L1Endpoint` を組立てる）** + `RtLeg::armed_frame_capacity(&self) -> usize` accessor（`EndpointInner::L1::process` の per-block dims pre-gate 用 — private `staging` field を読む） | Task 3。現行 `RtLeg::arm`（L491-506）は不変 — host 側 `Activate` は従来経路のまま |
| `crates/ruviform-plugin-host/src/l1/mod.rs` | `rt: Option<RtLeg>` 化（現行 `rt: data::RtLeg` L888）・`Shared::rt_state: Arc<AtomicU8>` 化（`set_mirror` 更新点不変）・`Shared::result_tx: Mutex<Producer<u64>>`（共有 result producer — driver が drop した seq の `mark_failed` completion を synthesize して strict-alternation slot を wedged にしない）・arm/return/collector field（`arm_tx`/`pending_arm`/`return_rx`/`arm_collector`・`endpoint_bound: Option<(u32,u32)>` — 公開 `EndpointBounds` を名指さないプリミティブ対・crate-free discipline）・`pub(crate) prepare_endpoint(&mut self, bound_max_frames: u32, bound_channels: u32) -> Result<data::EndpointLegParts, HostError>`（**順序 pin: (1) state check `Activated`/`Processing` → (2) bounds validation（take 前 — bad bounds は leg 不動）→ (3) `rt.take()` → (4+) channel mint/seed/reserve/簿記 → (8) `EndpointLegParts` 返却。(3) 以降の全 `Err` は `self.rt = Some(leg)` で leg 復元 — Err 後も attachable/processable**。`channel::<Arc<Staging>>(2)`/`retire_channel(4)`/`channel::<RtLeg>(1)` mint・`Inner::staging` clone の **seed arm** `try_push`・`last_output` を `bound.max_frames × WIRE_MAX_CHANNELS` へ事前 `reserve`）・`Activate` 成功時 `try_push`/`pending_arm`・再 Activate bound gate（**`self.rt.is_none()` が authoritative 判定** — `endpoint_bound` は簿記値のみ）・`drain_endpoint_channels`（全 `&mut` entry: `execute`/`prepare_endpoint`/`destroy`/teardown + `process` は wait-free 部分（resend + restore **のみ** — destructor/clear なし）・**`rt.is_none()` early-return より前に drain**・restore は mirror ≠ `Destroyed` gate・**non-RT drain が復元の仕上げ** — 復元 leg が `Inner::staging` より古い generation なら `rt.arm` で re-arm + per-attach field 全 clear — `arm_tx`/`pending_arm`/`return_rx`/`arm_collector`/`endpoint_bound` = None）・detached `process` → `Err(EndpointUnavailable)`・`arm_retire_overflows()`（`DropCollector::overflows()` 観測点）・`latency_frames` 内側 | Task 3 |
| `crates/ruviform-plugin-host/src/composite.rs` | `PluginHost::prepare_endpoint(handle, bounds)`/`latency_frames(handle)` 公開 API・**bounds-validation prelude 所有**（拒否順序 (3) bounds 位置での uniform 検査 — state 系・slot-shape 判定の後・内側 attach 呼出し前 — 非 0・`≤ WIRE_MAX_*`・`≥ activated`）・統一 defined-rejection 表（**拒否順序 pin: (1) state 系 → (2) slot-shape/presence → (3) bounds** — `u32::MAX` も `Err(Prepare(UnsupportedConfig))`）・`try_recv_notification(&mut)` で `drain_endpoint_channels` 呼出し・旧遅延式 doc を `lookahead × activated max_frames` へ同期 | Task 3（drain 配線のみ）+ Task 4（公開面）。`latency_frames` は `&self` — L0 const-cheap・L1 は `shared.lock()`（既存 `lifecycle_state` と同じ non-RT 形） |
| `crates/ruviform-plugin-host/src/lib.rs` | `PluginEndpoint`/`EndpointBounds` re-export + `erase_sendable` re-export（`plugin.rs` 経由の parity — `erase` と同じ連鎖）+ crate doc（endpoint 面・遅延契約・video NONRT 境界・RT 契約）+ **L44-49 付近の stale 遅延式 doc 修正**（`lookahead_blocks × block_frames / sample_rate` → frames 整数式） | Task 1（`erase_sendable` re-export parity）+ Task 2（endpoint re-export 骨格）+ Task 4（doc 完成 + stale 式修正） |
| `crates/ruviform-audio/src/graph.rs` | `impl<F: FnMut(&mut AudioBufferViewMut<'_, f32>, &RtContext) + Send> AudioNode for F` blanket impl 1 本 | Task 5。named newtype は不採（design 確定） |
| `crates/ruviform-audio/src/plan.rs` | `RtPlan.retirer: Retirer<Arc<GraphPlan>>` + `active()`（`current` slot の `try_read` 成功で **displaced `cached`** を `Arc::ptr_eq` で判定し実差替のみ retire — 退役対象は `self.cached` であり `current` slot（`PlanSwap` の publish 先 `Arc<RwLock<Arc<GraphPlan>>>`）は touch しない）+ `impl Drop for RtPlan`（`self.cached.take()`）・`PlanSwap` に `Arc<Mutex<DropCollector<Arc<GraphPlan>>>>`（`Clone`/公開形維持）+ `swap(&self)` 冒頭 drain + `drain_retired(&self) -> usize` + `retired_overflows(&self) -> usize` + `with_retire_capacity`（全て `&self` — 現行 `PlanSwap::swap(&self)`/`Session::swap(&self)` シグネチャに整合） | Task 5。**plugin 型・依存の露出ゼロ**（AC3 — `ruviform-rt` は既存依存の再利用） |
| `crates/ruviform-audio/src/session.rs` | teardown 順序の確認（field 順 `stream`→`swap` — retirer より collector が後に死ぬ安全順を doc で pin）+ **`Session::retired_overflows(&self)`/`Session::drain_retired(&self)` passthrough 追加**（`self.swap.*` を呼ぶ — `Session` 経由の退役観測点を公開し graph e2e が到達可能に） | Task 5。既存順序が適合するか確認のみ + `Session` API 追加 |
| `crates/ruviform-plugin-host/tests/plugin_graph_e2e.rs`（新規） | L0/L1 グラフ E2E — mock `Device`/`OutputStream` double（callback 手動駆動）+ `Session::open` + per-block gain 正当性 + `kill_child`→`Failed`+通知 once+無音 + latency 観測 + 退役 DropFlag + leg 再 attach + source-position `fill(0.0)` 慣行 + `rt_endpoint_*` | Task 6。`CARGO_BIN_EXE_l1-test-worker` は crate 内に閉じる（`l1_proxy_e2e` と同じ方式） |
| `crates/ruviform-plugin-host/Cargo.toml` | `ruviform-audio-core` を dev-dep → **正式 dep** へ昇格（`process_audio` が `AudioBufferViewMut` を名指す — Domain foundations で A.2 許可範囲）+ dev-dep `ruviform-audio`（graph E2E 専用・prod dep 不変で循環なし）+ dev-dep `static_assertions`（`assert_not_impl_any!` 負側 Send 証拠 — 通常の `expect_not_send` 型関数は `!Send` を compile 時に表明できないため採用） | dep 昇格は Task 2、`ruviform-audio`/`static_assertions` dev-dep は Task 6 |
| `crates/ruviform-audio/Cargo.toml` | dev-dep `static_assertions`（`assert_impl_all!(GraphPlan: Send)`/`assert_impl_all!(RtPlan: Send)` 等 — `[dependencies]` には追加しない・AC3 の `ruviform-audio` 非依存は維持） | Task 5 |

### 依存の確定（design「依存追加の判断」→ 本計画の配線）

- `ruviform-plugin-host` += `ruviform-audio-core`（正式 dep 昇格 — `process_audio` の `AudioBufferViewMut` 型名指しに必要・plugin-core 経由の推移依存を直接化するのみで依存木不変）
- `ruviform-plugin-host` dev-dep += `ruviform-audio`（graph E2E のみ — prod 依存は非循環・`ruviform-audio` の Cargo.toml/公開 API に plugin 型・依存の露出ゼロ — AC3 厳格解釈）
- `ruviform-rt` の `channel`/`retire_channel` を再利用 — 新規機構・新規外部依存なし。`ruviform-rt::slot`（snapshot）は不採（初回 `read` の per-thread debt-node alloc が RT 初回適用ブロックに発生し warm-up 例外が要る — SPSC+Retirer は全操作 wait-free で例外ゼロ）
- `Cargo.lock` は追跡継続 — dep 昇格・dev-dep 追加の差分を記録

## Implementation Sequence（21 action — 将来 task 名のみ確定）

| # | Action | Scope |
| --- | --- | --- |
| 1 | `HostError::EndpointUnavailable`（local-only variant・`Display` arm）+ `encode_host_error`/`encode` の `Result<(), WireError>` 化（wildcard → `Err(UnsupportedVariant)`・`out.truncate(start)` all-or-nothing）+ `encode_reply` infallible 維持 + 3 call site 更新（`l1/mod.rs` write_message・`l1-test-worker.rs`・export `send` — `io::Error` 写像）+ roundtrip ヘルパ/variant リスト更新 + negative test | plugin-core + call sites |
| 2 | `PluginFactory::create_sendable`（default `None`）+ `erase_sendable<P: Plugin + Send>` + `BufferView`/`BufferViewMut` impls on `AudioBufferView`/`AudioBufferViewMut` + 単体テスト | plugin-core |
| 3 | narrow gate — `mise run check`/`lint`/`test` + `test-plugin-core`/`lint-plugin-core` exit 0（既存テスト無退行・wire roundtrip・新 variant の encode 拒否を固定） | foundations |
| 4 | `Instance` スロット enum 化（`Plain`/`Shared(SharedSlot)` — `SharedSlot { core: Arc<Mutex<SendCore>>, mirror: Arc<AtomicU8>, attached: Arc<AtomicU64>, bound, next_epoch }`）+ `Instantiate` で `create_sendable()` → `Shared` / なければ `create()` → `Plain` + **`mirror_encode`/`mirror_decode` は `l1::data` に維持し L0 側は `crate::l1::data::` 経路で利用**（`src/mirror.rs` は新設しない — `src/l1/` 木は `crate::`/`ruviform_plugin_host::` パス禁止・`tests/l1_proxy_e2e.rs` `#[path]` include の制約のため l1 から crate 直下へ到達不能。`endpoint.rs`/`host.rs` は l1 木の外側で `crate::` 使用可） | host L0 slot |
| 5 | 全 verb 3 段 lock 形態（entry pre-check mirror → `match core.lock()` → lock 内 mirror 再読 — `Failed` なら hook/transition/set_state 不実行で `LifecycleEvent::Failed` + `Err(HostError::Failed)` 収束）+ commit 順序（transition commit → `set_state` mirror store → plugin hook を同一 CS）+ `destroy` のみ `into_inner`（panic 済み plugin の destroy hook は呼ばず teardown のみ）+ `&self` 照会は mirror 観測 union のみ（`lifecycle_state` は mirror Acquire 読み・`core.lock()` 不許可）+ `plugin_mut`→`Shared` で `None` + host `process`→`try_lock` busy `Err(EndpointUnavailable)` | host L0 verb 経路 |
| 6 | `src/endpoint.rs` 新設 — `PluginEndpoint`/`EndpointBounds`/`EndpointInner`/`L0Endpoint` + epoch attach（`next_epoch` 採番・`attached.compare_exchange(0, epoch, AcqRel, Acquire)`）+ `Drop` で `compare_exchange(epoch, 0)` + L0 inner process（mirror Acquire fast-skip → `try_lock` → lock 内 mirror 再読 → `activated` gate `[min_frames,max_frames]`+channels → `catch_unwind(plugin.process_host)` → panic/poisoned/`Failed` で `mirror.fetch_max(Failed)` + `Ok(CONTINUE)`+無音 — `≥` 比較禁止・`{Activated, Processing}` 列挙）+ `InProcessHost` `pub(crate)` attach 内側（state `Activated`/`Processing` 要求・bounded 検証 非0/≥activated `max_frames`/≤`WIRE_MAX_*`・`AudioBuffer::try_new` の `Err` → `Err(Prepare(UnsupportedConfig))` 写像・`in_scratch` 事前確保） | host endpoint |
| 7 | narrow gate — `check`/`lint`/`test`（L0 単体: Shared dispatch・全 verb 到達・epoch 二重 attach 拒否・`Failed` 収束・busy bypass・`plugin_mut` `None`） | gate |
| 8 | `L1Instance.rt: Option<RtLeg>`（現行 `rt: data::RtLeg` L888）+ `Shared::rt_state: Arc<AtomicU8>` 化 + `arm_tx`/`pending_arm`/`return_rx`/`arm_collector`/`endpoint_bound: Option<(u32,u32)>` field（crate-free — 公開 `EndpointBounds` 不名指し）+ `L1Endpoint`（`leg`/`mirror`/`arm_rx`/`arm_retirer`/`return_tx` — **`endpoint.rs` が `crate::l1::data::EndpointLegParts` から組立て**・l1 は parts を返すのみ）+ `pub(crate) prepare_endpoint(&mut self, bound_max_frames: u32, bound_channels: u32) -> Result<data::EndpointLegParts, HostError>`（**順序 pin: (1) state check `Activated`/`Processing` → (2) bounds validation（take 前 — bad bounds は leg 不動の `Err(Prepare(UnsupportedConfig))`）→ (3) `rt.take()` → (4+) `channel::<Arc<Staging>>(2)`/`retire_channel(4)`/`channel::<RtLeg>(1)` mint + `Inner::staging` clone seed arm `try_push` + `last_output` reserve + 簿記 → (8) `EndpointLegParts` 返却。(3) 以降の全 `Err` は `self.rt = Some(leg)` で leg 復元 — Err 後も attachable/processable**。seed arm は `seed.generation > leg 現装着 generation` のときのみ apply・同 generation のみ正規化 no-op（detached-Activate で `Inner::staging` が新世代でも、還流 leg は host 側 non-RT drain が `rt.arm` で re-arm するため再 attach の seed は常に同 generation — endpoint 側 apply は生存 endpoint への arm 配送のみが担う）・`last_output` を `bound.max_frames × WIRE_MAX_CHANNELS` へ事前 `reserve`） | host L1 endpoint |
| 9 | `RtLeg::arm_apply` 簿記分割（ring 束縛のみ reset・seq 空間簿記持続）+ `Activate` 成功時の arm `try_push`（`Full` → `pending_arm` latest-wins 退避）+ 再 Activate gate（**attach 生存の authoritative 判定は `self.rt.is_none()`（leg 移出中）** — `endpoint_bound` は簿記値のみで liveness 判定に使わない。`self.rt.is_none()` かつ `new_config.max_frames > endpoint_bound.0` → `Err(Prepare(UnsupportedConfig))`） | host L1 arm |
| 10 | endpoint 側 arm drain（`process` 冒頭・bounded pop ≤ arm_capacity+1・`generation > 現装着` のみ `arm_apply`・superseded/stale を含む全 drained arm → `arm_retirer.retire`・`frame_capacity > bound` の arm は apply せず retire + pass-through 収束（`leg.pass_through(outputs)` + `Ok(CONTINUE)`）— **`Err(Ipc)` は `process`/`process_audio` 公開面に出さない**・公開面の dims 拒否は `armed_frame_capacity` + `bound.channels` pre-gate の `Err(Prepare(UnsupportedConfig))`）+ `drain_endpoint_channels`（`pending_arm` resend + `return_rx` `try_pop` → `self.rt` 復元 + `arm_collector.drain()` + 復元の仕上げ: stale-generation leg の `rt.arm` re-arm + per-attach field 全 clear — **全 `&mut` non-RT entry point** 冒頭: `execute`/`prepare_endpoint`/`destroy`/teardown + composite `try_recv_notification(&mut)`; host `process` は wait-free 部分のみ（resend + restore — `arm_collector.drain()` も `clear_endpoint_fields` も除く）・**`rt.is_none()` early-return より前に drain**・restore は mirror ≠ `Destroyed` gate（Destroyed へは復元せず queued leg は teardown で drop）・**clear で `arm_tx`/`pending_arm`/`return_rx`/`arm_collector`/`endpoint_bound` = None** — rtrb free・`Arc<Staging>` destructor は non-RT のみ）+ `&self` query（`lifecycle_state`/`latency_frames`/`L1Instance::try_recv_notification`）は defined non-drain | host L1 drain |
| 11 | endpoint `Drop` → `leg.take()` を `return_tx.try_push`（`Full` → `into_inner()` 回収して drop スレッド上で drop — `TryPushError::Full` のみ・unit test で pin）+ `return_rx` drain → `self.rt` 復元 → 再 attach 可能・leg 喪失（host 側 endpoint 既 drop）は defined 終端 + narrow gate `check`/`lint`/`test` | host L1 返却 |
| 12 | `PluginHost::prepare_endpoint(handle, bounds) -> Result<PluginEndpoint, HostError>` + `latency_frames(handle) -> Option<u32>`（L0=`Some(0)`・L1=`Some(lookahead × activated max_frames)`・未 Activate/無効=`None`・`&self` — L0 は blocking `core` mutex lock 不許可・try-lock/cheap 観測のみ）+ **bounds-validation prelude 所有**（拒否順序 (3) bounds 位置で uniform 検査 — state 系・slot-shape 判定の後・内側 attach 呼出し前 — 非 0・`≤ WIRE_MAX_*`・`≥ activated max_frames`・`u32::MAX` も `Err(Prepare(UnsupportedConfig))` 統一）+ 統一 defined-rejection 表（design「エラー・拒否面」表の逐語写し + **拒否順序 pin: (1) state 系 → (2) slot-shape/presence → (3) bounds**）+ crate doc（endpoint 面・遅延式・video NONRT 境界 — AC10）+ `composite.rs` 旧遅延式 + **`lib.rs` 旧遅延式 doc 修正**（`lookahead_blocks × block_frames / sample_rate` → frames 整数式。`Slot` enum は `InProcess`/`OutOfProcess` のみで `Slot::Video` は存在しない — video は unreachable dead-row 概念であり `Err(Unsupported)` 分岐は追加しない） | composite |
| 13 | narrow gate — `check`/`lint`/`test`（composite 級: 統一 rejection・latency 値・`plugin_mut` L0-Shared `None` parity・`EndpointUnavailable` 系） | gate |
| 14 | `AudioNode` `FnMut` blanket impl（`graph.rs` — `FnMut(&mut AudioBufferViewMut<'_, f32>, &RtContext) + Send` → `pull` 転送）+ 単体テスト（closure が `Box<dyn AudioNode>` として **`graph.push(Box::new(closure))`** で push できる — `push_node` という API は存在せず `Graph::push` を使う・`Send` でない capture は compile-fail — **`static_assertions` dev-dep で `assert_impl_all!`/`assert_not_impl_any!` を用いる**（`expect_send` 型関数のみでは `!Send` の負側 compile-time 表明ができないため）） | audio seam |
| 15 | `plan.rs` 退役シーム — `RtPlan.retirer: Retirer<Arc<GraphPlan>>` + `active()` は **`self.cached`（現行 `cached: Option<Arc<GraphPlan>>` field）を displace したときのみ retire** — `Arc::ptr_eq` で実差替を判定し same-Arc reread・`try_read` 失敗時は retire しない・退役対象は `self.cached` であり `PlanSwap` 所有の shared `current` slot は touch しない + `Drop for RtPlan` で `self.cached.take()` retire・`PlanSwap` collector（`Arc<Mutex<DropCollector<Arc<GraphPlan>>>>`・`Clone` 維持）+ `swap(&self)` drain + `drain_retired(&self)`/`retired_overflows(&self)`/`with_retire_capacity`（現行 `PlanSwap::swap(&self)`/`Session::swap(&self)` シグネチャに整合）+ `Session` field 順序の doc pin + **`Session::retired_overflows(&self)`/`Session::drain_retired(&self)` passthrough 追加**（現行 `Session` は `swap` のみ公開 — graph e2e から退役観測点へ到達可能にする） | audio plan |
| 16 | `rt_plan_active_no_alloc`・`rt_plan_swap_retire_no_alloc`（旧 `Arc` が retire され RT drop しない — DropFlag が drain まで不変）・`rt_fn_node_pull_no_alloc` + narrow gate `check`/`lint`/`test`/`test-rt`（`rt_` 実一致件数記録） | audio gate |
| 17 | `PluginEndpoint::process`（`PluginHost::process` と同語彙 — `&[HostInput]`/`&mut [HostOutput]`/`events: EventList<'_>`/`transport: Option<TransportInfo>` は **by-value・両者 `Copy`**・ctx なし・返り値 `Result<ProcessResult, HostError>` — `ProcessStatus` ではなく `ProcessResult` で `tail_frames` を保持）+ `process_audio`（`out`→`in_scratch` channel-major 複写 → per-block `AudioBufferViewMut::try_new(&mut slab[..ch*fr], ch, fr)` → `inner.process(bound, …)` — **処理順序固定**: (1) bound clip `out.channels()==bound.channels` かつ `out.frames()≤bound.max_frames` 違反は `Err(Prepare(UnsupportedConfig))`、(2) view 構築+複写、(3) `inner.process` — in-bus-first gate 情報源・`Err` は `out` 不変更・返り値同じく `Result<ProcessResult, HostError>`）+ source-position `fill(0.0)` 慣行の doc（`Err`/bypass で stale でなく無音）+ **Send 証拠は `static_assertions`（`assert_impl_all!(PluginEndpoint: Send)`・`assert_not_impl_any!` 負側）+ concrete 型の `expect_send::<PluginEndpoint>()` 併用** — `unsafe impl Send` なし・具象 `L0Endpoint`/`L1Endpoint` 型（enum variant 名は assert 引数にしない）・ノード closure 型・`GraphPlan`/`RtPlan`/callback capture を対象 | endpoint public shell |
| 18 | `tests/plugin_graph_e2e.rs` — mock `Device`/`OutputStream` double（callback 手動駆動・device-independent）+ Send-capable gain plugin: **L0 graph E2E**（`host.register_factory(Box<dyn PluginFactory>)` → `host.execute(HostCommand::Instantiate { plugin_id })` → lifecycle → `prepare_endpoint` → `graph.push(Box::new(closure))` → `Session::open` → callback 駆動の per-block `process` + gain 正当性・`latency_frames==0`・構成面 rejection — 実在する公開登録 API のみ使用・`PluginHost::instantiate(InProcess)` のような経路は存在しない）+ **L1 graph E2E**（`host.register_out_of_process(OutOfProcessSpec { .. })` → `HostCommand::Instantiate` → `CARGO_BIN_EXE_l1-test-worker` spawn → lifecycle → `prepare_endpoint` → `graph.push` → process → destroy・`kill_child`→`Failed`+`InstanceFailed` once+以後無音/pass-through・`latency_frames==lookahead×activated max_frames`（例: `2×512`）・遅延未着 pass-through）+ 退役 DropFlag（node removal/plan-swap で RT 経路に立たず `Session::drain_retired`/teardown で立つ・`Session::retired_overflows()==0` — `Session` passthrough 経由で観測）+ leg 返却→再 attach + source-position Err→無音 | E2E |
| 19 | `rt_endpoint_l0_process_no_alloc`（shared core `try_lock` 経路 0 alloc）・`rt_endpoint_l1_process_no_alloc`・`rt_endpoint_arm_apply_no_alloc`・`rt_endpoint_busy_bypass`（busy/poisoned/非 Processing の定義済み出力）+ narrow gate `test`/`lint`/`test-rt`（`rt_` 実一致件数記録 — 0 件スキップを成功と偽らない） | RT レーン |
| 20 | **Full Gate（1 回のみ・本計画最後段）** — `mise run fmt-check`・`check`・`lint`・`test`・`test-rt`・`bench-smoke`・`mise exec -- prek run --all-files`・doc audit・`cargo tree -p ruviform-audio` DAG 監査（plugin 依存ゼロ — AC3）・**`mise exec -- cargo doc -p ruviform-plugin-host -p ruviform-audio --no-deps`（rustdoc warning 0 — endpoint/退役 API の doc 参照整合を検証）** — 全 log `docs/impl/exp/data/issue-113-final-gate-*-*.log` 保存・`test-rt` は実一致件数で証明 | gate |
| 21 | canonical closeout — 実績 IR（`docs/impl/ir/issue-113-*.md`）・coverage 記述・task/plan status 同期・3 test-spec を実結果で `approved` 化・design/spec の実装相違は正本へ同期反映。GitHub merge は lead の authoritative `gh` のみ | closeout |

- test-spec（起票済み・proposed）: [plugin-endpoint-contract-tests](../test-specs/plugin-endpoint-contract-tests.md)・[plugin-graph-e2e-tests](../test-specs/plugin-graph-e2e-tests.md)・[plugin-rt-retirement-tests](../test-specs/plugin-rt-retirement-tests.md) — `verified-by` は実ファイルに更新済み
- **7 task 文書**（起票済み・todo・actions 1-3/4-7/8-11/12-13/14-16/17-19/20-21）: [plugin-core-endpoint-primitives](../tasks/plugin-core-endpoint-primitives.md)・[l0-shared-slot-and-endpoint-inner](../tasks/l0-shared-slot-and-endpoint-inner.md)・[l1-endpoint-arm-and-leg-return](../tasks/l1-endpoint-arm-and-leg-return.md)・[plugin-host-endpoint-surface](../tasks/plugin-host-endpoint-surface.md)・[audio-fnmut-node-and-plan-retirement](../tasks/audio-fnmut-node-and-plan-retirement.md)・[plugin-graph-e2e-and-rt-lane](../tasks/plugin-graph-e2e-and-rt-lane.md)・[engine-plugin-final-verification](../tasks/engine-plugin-final-verification.md)
- 各 task 間は狭い検証のみ（full suite 繰返しはしない — Full Gate は action 20 の一回）

## Test Oracles（design/test-spec の正本どおり・新規代替機構禁止）

- **endpoint 公開形**: `PluginHost::prepare_endpoint(handle, bounds) -> Result<PluginEndpoint, HostError>` — `Activated`/`Processing` 以外は `Err(Lifecycle)`（`Failed`→`Err(Failed)`・`Destroyed`/stale→`Err(InvalidHandle)`）。単一 endpoint 規則 — L0 は `attached` epoch CAS（非 0 → `Err(EndpointUnavailable)`）、L1 は `rt.is_none()` → `Err(EndpointUnavailable)`（leg は `return_rx` drain でのみ復元）。`EndpointBounds { max_frames, channels }` — 非 0・`max_frames ≥ activated max_frames`・両層共通で `≤ WIRE_MAX_FRAMES`(8192)/`≤ WIRE_MAX_CHANNELS`(16) — 違反は全て `Err(Prepare(UnsupportedConfig))`
- **L0 Shared セル**: 全 verb（lifecycle 以外も `SaveState`/`RestoreState`/`OpenGui`/`CloseGui` 含む）が `core.lock()` 内で transition commit + `set_state` mirror store + plugin hook を atomic に実行 — `try_lock` は常に完全 commit 済みのみ観測し `mirror ∈ {Activated, Processing} ⟹ activated.is_some()` が endpoint 不変条件（mirror u8 順は `Failed`=4/`Destroyed`=5 ≥ `Activated`=2 — `≥` 比較禁止）。verb 3 段: entry pre-check → `match lock()`（poisoned → 収束）→ lock 内再読。`destroy` のみ `into_inner` で teardown（hook 不呼出）
- **L0 endpoint `process`**: mirror Acquire fast-skip → `try_lock`（busy → `Err(EndpointUnavailable)` bypass）→ lock 内 mirror 再読 → `activated` gate → `catch_unwind(process_host)` — panic/poisoned/`Failed` 観測で `fetch_max(Failed)` して `Ok(CONTINUE)`+無音（終端 — 以後 mirror 読みで short-circuit）
- **L1 endpoint**: leg 移出で host `process` は `Err(EndpointUnavailable)`（wait-free drain は early-return より前 — restore のみ・destructor/clear なし・mirror ≠ `Destroyed` gate）。arm channel capacity 2・**seed arm は `seed.generation > leg 現装着 generation` のときのみ apply — 同 generation のみ正規化 no-op で unapplied のまま retire（detached-Activate で `Inner::staging` が新世代でも還流 leg は host 側 drain が re-arm するため再 attach の seed は常に同 generation）**・endpoint drain は bounded pop ≤3・`generation > 現装着` のみ apply・全 drained arm → `Retirer`(4)・`frame_capacity > bound` arm → retire + pass-through 収束（`Ok(CONTINUE)` — **`Err(Ipc)` は公開面に出さない**）。`pending_arm` latest-wins・全 `&mut` entry で resend（`&self` は defined non-drain）・non-RT drain が per-attach field（`arm_tx`/`pending_arm`/`return_rx`/`arm_collector`/`endpoint_bound`）を全 clear + stale-generation leg を re-arm・driver が drop した seq には `mark_failed` completion を synthesize（`Shared::result_tx` — strict-alternation slot の wedge 防止）
- **`arm_apply` 簿記分割**: reset は ring 束縛のみ（`staging`/`frame_capacity`/`last_output` 中身/`last_frames`/`last_channels`/`have_output` + 旧 ring → retirer）— `last_pushed`/`cleared`/`held`/`submit_dims`/`next_block`/`depth`/`lookahead` は arm を跨いで持続（strict-alternation 不変条件の維持）。`last_output` は `prepare_endpoint` で `bound.max_frames × WIRE_MAX_CHANNELS` 事前 `reserve` 済み — `arm_apply` の `resize` は常に capacity 内（realloc なし）
- **leg 返却**: endpoint `Drop` → `leg.take()` → `return_tx.try_push`（cap 1）— `Full` のみが失敗形で `into_inner()` leg は drop スレッドで drop。host 側 `&mut` entry point が `return_rx.try_pop` で `self.rt` 復元 — plan swap が走らない session でも `&mut` 呼出し毎に leg が還り再 attach が塞がれない
- **`process_audio`**: 順序固定 — (1) bound clip、(2) per-block `AudioBufferViewMut::try_new` view + channel-major 複写、(3) `inner.process`。gate の `frames`/`channels` 情報源は in-bus-first 規則（`frames` = in-bus → なければ out-bus、`channels` = 両側なら `min`）— `bound` は上限定数のみ。bound/activated 違反は `Err(Prepare(UnsupportedConfig))`（`Err(Ipc)` は L1 内部の wire 表現不能に留め公開面に出さない）。`Err` は `out` 不変更（transform 位置は上流 bypass・source 位置は closure が呼出し前に `out.data_mut().fill(0.0)` — adapter 契約の必須慣行）
- **pass-through/終端（L1 継承）**: 結果未着/slot busy → 直前受信 else 無音・`Failed` → 無音・常に `Ok(CONTINUE)` — エラーでなくフォールバック。自動再起動なし・`Failed` 以後コマンド（`Destroy` 除く）→ `Err(Failed)`
- **latency**: `PluginHost::latency_frames` — L0=`Some(0)`・L1=`Some(lookahead_blocks × activated max_frames)`・未 `Activate`/無効 handle=`None`（`&self`）。`PluginEndpoint::latency_frames` は prepare 時点値を保持（re-activation で変わり得ること doc 明記）。秒換算 `frames / sample_rate`（2×512 @48kHz ≈ 21.3ms）
- **退役**: `PlanSwap`/`Session` の全 non-RT entry point で collector drain（`swap(&self)`・`drain_retired(&self)`・teardown — `Session::drain_retired(&self)`/`Session::retired_overflows(&self)` passthrough 経由で観測）。退役対象は `RtPlan.cached`（`Arc::ptr_eq` で実差替のみ・same-Arc reread/`try_read` 失敗は retire しない・`Drop for RtPlan` で `cached.take()`）— `PlanSwap` 所有の shared `current` slot は touch しない。退役 `GraphPlan` の node/endpoint/`Mutex<SendCore>`/`RtLeg` scratch/`Arc<Staging>`/SPSC endpoint の dealloc は全て collector スレッドか collector drop（常に non-RT）。`retired_overflows()` は leak-on-full 飽和の観測点 — drain 後も 0 を AC で検査
- **`EndpointUnavailable` wire 不載**: `Reply{Err(EndpointUnavailable)}` encode → `Err(UnsupportedVariant)` かつ `out` は `truncate(start)` で不変（all-or-nothing append — partial frame 不残存）

## Required mise tasks

- 既存タスクのみで完結（新規 mise タスクなし）: `fmt-check`/`check`/`lint`/`test`/`test-rt`/`bench-smoke`・`test-plugin-core`/`lint-plugin-core`（core 集中検証の既存レーン）・`mise exec -- prek run --all-files`・`mise exec -- node .agents/skills/doc-status/scripts/doc_status.js audit --json`・`mise exec -- cargo doc -p ruviform-plugin-host -p ruviform-audio --no-deps`（Full Gate の rustdoc warning 0 検証 — action 20）
- `rt_` 命名テストは `mise run test-rt`（`rt_` 名フィルタ）で選択 — 実一致件数を記録し 0 件一致スキップを成功と偽らない
- CI: `.github/workflows/ci.yml` の `verify` ジョブ（3 OS matrix・`workflow_dispatch`）が `verify-ci` 経由で `cargo test --workspace`（E2E 含む）を実行 — ローカル実証は Windows のみ・macOS/Linux は CI 実行時のみ記録（計画書は未実行の主張をしない）

## Risks / Readiness

| # | Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- |
| 1 | `encode` の signature 変更の波及（公開 API・`Result<(), WireError>` 化 — pre-1.0 許容だが call site/test の追随漏れ） | med | med | Task 1 で call site 4 箇所（`l1/mod.rs` L421・`l1-test-worker.rs` L160・`plugin-export/lib.rs` L794・`l1_runner_e2e.rs` L125）+ wire.rs doctest 3 箇所（L239/L641/L717）+ inline test helper/直接 call を同 task 内で更新 — `lint` の `-D warnings` が ignored `Result` を捕捉・narrow gate `test`/`test-plugin-core` で検出 |
| 2 | L0 verb 3 段 lock 形態の実装誤り（`Failed` regression — verb の非 `Failed` store が endpoint の `Failed` を上書き） | med | high | design の順序規則を test-spec に固定（pre-check/`match`/lock 内再読・CS 内 mirror store・`destroy` 例外）+ poison 経路の unit test（故意 panic fixture） |
| 3 | `catch_unwind` 境界の `AssertUnwindSafe` 範囲誤り・panic 後 `SendCore` 不整合 | med | high | catch 範囲は `plugin.process_host` 呼出しのみ・guard 保持のまま `fetch_max(Failed)` — panic fixture test で収束を固定 |
| 4 | arm channel の generation/容量規則誤り（seed arm の誤適用・superseded arm の残存・retire 漏れ → shm ring の RT drop） | med | high | test-spec で「全 drained arm → Retirer」「generation 単調」「bounded pop ≤3」を固定・DropFlag 内蔵 `Staging` probe で観測 |
| 5 | leg 返却・`pending_arm` の滞留（`&mut` entry が来ない間の塞がり） | low | med | design 確定の defined 挙動（次 `&mut` entry で必ず処理）+ `prepare_endpoint`/`try_recv_notification` が drain 点 — unit test で swap なし運用の leg 還流を固定 |
| 6 | plan 退役の leak-on-full（`Full` で `mem::forget` — swap 頻度超過で退役 `GraphPlan` がリーク） | low | med | `retired_overflows()` 観測点を AC で検査 + `with_retire_capacity` で調整可能 — 既定容量は swap 頻度比例の小 |
| 7 | `process_audio` の borrow 分割・per-block view の寸法誤り（`ch*fr` slice・`try_new` の `data.len()==channels*frames` 前提違反） | med | med | 順序固定（bound clip → view → inner）+ view 不成立パスの unit test — `try_new` は `Result`（panic なし） |
| 8 | L0 `try_lock` busy の過度な bypass（重い verb 実行中の実質ミュート） | med | low | design 許容の defined 挙動 — doc に「重い verb は音声クリティカル区間外へ呼ぶ」運用規律を明記 |
| 9 | `bounds.channels` の「activated bus 形状」情報源が現行 plugin 契約に存在しない（`ProcessConfig`/`PluginDescriptor` に channel 数がない） | low | low | **解決済み**（design amendment item 3 + 本 Open Questions 解決記録）— v1 は prepare 時の非0/`WIRE_MAX_CHANNELS` 検査 + bound-shaped scratch + per-block `out.channels()==bound.channels` 等値が実効 enforcement。真の channel 照会/reconciliation は channel-query API 不在のため Issue #114-era work へ明示延期 |
| 10 | Windows/POSIX 差・CI のみ 3 OS（endpoint E2E は mock ベースで機構上 OS 非依存だが L1 spawn 経路を経る） | low | med | ローカルは Windows 実証・3 OS matrix は CI `verify` ジョブに委譲（issue-76 と同じ platform notes） |

## Rollback

- 新規ファイル（`endpoint.rs`・`plugin_graph_e2e.rs`）と additive 変更が主体 — 履歴保持の corrective/revert commit で task 単位に戻す（reset/force なし）。`Cargo.lock` は `git checkout -- Cargo.lock` で戻し再生成
- `HostError::EndpointUnavailable` は `#[non_exhaustive]` enum への variant 追加 — revert しても downstream match を破壊しない。`encode` の `Result` 化は call site と同時 revert で完結
- `Instance` スロット enum 化は内部構造 — revert は enum 戻しで完結（`plugin_mut`→`None` の公開挙動差分は L1 と同じ defined 挙動）
- `ruviform-audio` の変更は `FnMut` impl 1 本 + `plan.rs` 退役配線のみ — 公開 API に plugin 型なし・revert は局所完結
- DB/persistent data migration なし

## Verification

- per-task narrow checks のみ — 各 task の `## Verification` が run 可能コマンドを持つ（`mise run check`/`lint`/`test`/`test-rt`・core 集中は `test-plugin-core`/`lint-plugin-core`）
- Full Gate は action 20 の一回のみ — 全 log 保存・`test-rt` 実一致件数で証明
- spec AC1–AC11 ↔ actions/test-specs の写像: AC1（実グラフ消費）→ actions 17–18（tspec graph-e2e）・AC2（所有分離/`Send` 証拠）→ actions 6・8・17（tspec endpoint/graph-e2e）・AC3（構成面・`ruviform-audio` 非露出）→ actions 12・14–16・20（`cargo tree` DAG 監査・tspec graph-e2e）・AC4（明示的 prepare + bounded）→ actions 6・8・12・17（tspec endpoint）・AC5（error/fallback）→ actions 5・10–11・18（tspec endpoint/graph-e2e）・AC6（latency `lookahead × activated max_frames`）→ actions 12・18（tspec graph-e2e）・AC7（RT alloc `rt_`）→ actions 16・19（tspec rt-retirement）・AC8（退役の非 RT drop）→ actions 15・18（tspec rt-retirement）・AC9（L0/L1 graph E2E mock device）→ action 18（tspec graph-e2e）・AC10（video NONRT 境界明文化）→ action 12・21（crate doc/design deferred 整合）・AC11（quality gate + doc audit）→ actions 20–21
- doc 監査チェーン起因 findings 0（`mise exec -- node .agents/skills/doc-status/scripts/doc_status.js audit --json`）
- test-spec link は `verified-by` に更新済み — **実検証は未実施**（PLANNED・未実行）

## Open Questions / Gaps

- **`bounds.channels` の activated bus 情報源** — **解決済み**（design「spec 修正記録」の amendment item 3 反映・lead 確認不要の記録完了項目）: 当初 design は「`bounds.channels` は plugin の activated メイン bus チャンネル数と一致必須」と pin したが、現行の標準中立契約（`Plugin`/`ErasedPlugin`/`ProcessConfig`/`PluginDescriptor`）に per-instance channel 照会面がないことが確認された。**v1 決定**: prepare 時は非 0/`WIRE_MAX_CHANNELS` 上限検査 + bound-shaped scratch のみ要求し、`bounds.channels == activated bus channels` の等値は要求しない。実効 enforcement は per-block `out.channels()==bound.channels` 等値（不一致 → `Err(Prepare(UnsupportedConfig))`・`out` 不変更）。真の plugin bus 照会/reconciliation（channel-query API + adapter 論点）は **Issue #114-era work へ明示延期** — 本計画は契約拡張しない
- それ以外の Open Questions なし — design が discovery の全論点（endpoint 公開形・L0 `Send` 形・arm 経路・退役シーム・crate 配置・`Io` 写像・verb 到達・遅延 API・E2E 配置・video 境界）を確定済み

## Self-Review

- [x] Every spec requirement maps to a task（11 AC → 7 task + 3 test-spec の写像を Verification に記録）
- [x] No placeholders remain（ファイル名・型名・channel capacity・mise コマンドは確定値のみ — 未確定点は Open Questions で明示）
- [x] File paths, relations, status values, and terminology are consistent（`PluginEndpoint`/`EndpointBounds`/`EndpointInner`/`SharedSlot`/`SendCore`/`RtLeg::arm_apply`/`drain_endpoint_channels`/`HostError::EndpointUnavailable`/`WireError::UnsupportedVariant`/`Retirer`/`DropCollector`/`latency_frames`/`rt_endpoint_*`/`rt_plan_*` — design の正本どおり）
- [x] pinned mechanism の owning task 重複なし — `process` 経路は T2（L0 inner）・T3（L1 inner + arm drain）・T6（公開 shell + bound clip/per-block view）で層分割される唯一の跨 task 機構（変種 gate は各 variant inner が所有・shell は順序と写像のみ）

## Implementation Handoff

- Recommended mode: delegated 実行 + lead の task 完了ごとの実レビュー（ユーザ承認済みを前提 — 環境依存の skill ID は本計画に埋め込まず、実環境で発見される capability に routing する）
- Independent work streams: Task 5（audio シーム）は plugin 側に依存せず論理上独立だが、単一クリティカルパスで直列実行（Task 2/3 が `src/endpoint.rs` を順次拡張するため全 task 同一レーン）
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- レビュー焦点: `SharedSlot`/`SendCore` の形と全 verb の 3 段 lock 形態（pre-check → `match lock` → lock 内再読）・commit 順序（transition → `set_state` → hook が同一 CS）・`destroy` の `into_inner` 例外が design どおりか — `Failed→Activated` regression が構造的に不可能か
- endpoint L0 inner の `catch_unwind` + `fetch_max(Failed)`（第 2 書き手）と「mirror ∈ {Activated,Processing} ⟹ activated.is_some()」不変条件の保持（`≥` 比較禁止の根拠と共に）
- L1: `rt: Option<RtLeg>` take（post-take `Err` は `self.rt = Some(leg)` で復元）・seed arm は**同 generation のみ正規化 no-op（detached-Activate 後の還流 leg は host 側 drain が re-arm — 再 attach seed は常に同 generation）**・generation 単調 apply・全 drained arm の `Retirer` 行き・`arm_apply` 簿記分割（seq 空間持続）・`return_rx` drain が全 `&mut` entry + `process`（wait-free 部分 — restore のみ・destructor/clear なし）の `rt.is_none()` early-return より前にあるか・restore が mirror ≠ `Destroyed` gate されているか・non-RT drain の leg 復元仕上げ（stale-generation re-arm + per-attach field `arm_tx`/`pending_arm`/`return_rx`/`arm_collector`/`endpoint_bound` 全 clear）・driver drop seq の `mark_failed` synthesize が slot gate を解放するか（`Shared::result_tx`）・`EndpointLegParts` が crate-free プリミティブ束（`endpoint.rs` が `L1Endpoint` を組立て）か
- `encode`/`encode_host_error` の `Result` 化が `truncate(start)` all-or-nothing を守り `encode_reply` が infallible のままか・`EndpointUnavailable` が wire 表現可能 variant リストに入っていないか・call site 4 箇所 + doctest 3 箇所の全 `Result` 対応が揃ったか
- `plan.rs` 退役シーム: 退役対象が `RtPlan.cached` のみ（`PlanSwap` 所有の shared `current` slot を touch しない）・`ptr_eq` で実差替のみ retire（same-Arc reread/`try_read` 失敗は retire しない）・`Drop for RtPlan` で `cached.take()`・drain が全 non-RT entry（`swap(&self)`/`drain_retired(&self)`/teardown）・`retired_overflows` 観測点・`Session::drain_retired`/`Session::retired_overflows` passthrough 追加・`ruviform-audio` に plugin 依存/型が露出していないか（AC3）
- `process_audio` の順序固定（bound clip → per-block `AudioBufferViewMut::try_new` → `inner.process`）・in-bus-first gate 情報源・`Err`=`out` 不変更・source-position `fill(0.0)` 慣行の doc 化
- `PluginEndpoint: Send` が型構造のみで保持されること（`unsafe impl Send` なし）・`PluginHost`/`L1Instance`/`ErasedPlugin`（無印）の参照が RT 側にないこと
