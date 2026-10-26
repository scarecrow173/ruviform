---
id: "ADR-034MbMOuLa1T5TWfzINoHl"
type: "adr"
status: "accepted"
title: "プラグイン処理の non-RT 制御/RT 処理所有分離（transferable RT endpoint）"
created: "2026-10-25"
updated: "2026-10-26"
owners: ["scarecrow173"]
metadata:
  adr:
    supersedes: []
    decision-makers: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/113"
    - "https://github.com/scarecrow173/ruviform/issues/76"
    - "https://github.com/scarecrow173/ruviform/issues/74"
    - "https://github.com/scarecrow173/ruviform/issues/114"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/engine-plugin-host-integration.md"
    - "./audio-engine-snapshot-swap.md"
    - "./plugin-isolation-strategy.md"
    - "./realtime-contract-and-verification-policy.md"
    - "./rt-foundation-primitives-implementation.md"
    - "./ofx-host-video-contract-and-c-abi.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/engine-plugin-host-integration.md"
    - "../designs/plugin-l1-out-of-process.md"
    - "../designs/ruviform-architecture-design-baseline.md"
    - "./audio-engine-snapshot-swap.md"
    - "./plugin-isolation-strategy.md"
  derived-by:
    - "docs/designs/engine-plugin-host-integration.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# プラグイン処理の non-RT 制御/RT 処理所有分離（transferable RT endpoint）

## Status

accepted（design-phase review で accept — Issue #113 briefing 起点の実アーキテクチャ分岐。review-protocol cycles 1-2、verdict ACCEPT-WITH-NOTES — [design](../designs/engine-plugin-host-integration.md) 側の承認と同時確定）

## Context and Problem Statement

Issue #113 は `PluginHost`/`PluginHandle` を audio グラフへ接続することを要求する。しかし型レベルの矛盾がある:

- `AudioNode: Send`（`crates/ruviform-audio/src/graph.rs:16`）+ `OutputDataCallback = Box<dyn FnMut + Send + 'static>`（`crates/ruviform-audio-core/src/device.rs:512`）→ `GraphPlan`/`RtPlan` 経由でコールバックに入る全ノードは `Send` 必須
- `PluginHost`/`L1Instance`/`Box<dyn ErasedPlugin>` は `!Send`/`!Sync` — L0 は auto-trait なしの erased object、L1 は `Shmem` raw pointer（`composite.rs:205-209`・`l1/shm.rs:256-257`）
- `PluginHost::process(&mut self)` は RT-suitable だが `&mut` ホスト状態を要求 — host 本体は RT 側に置けない

同時に baseline は「engines → 拡張層の直接依存は禁止（エンジンは `Processor` 等の下位ポート経由でプラグインを駆動する）」を確定済み — engine が plugin 型を名指すこともできない。

決定事項: **plugin 処理を audio graph へ届ける所有構造をどう分けるか**。

## Decision Drivers

- `Send` 契約の型強制 — `!Send` な制御/インスタンス状態はコールバック経路に入れない（`Mutex<T>: Sync` も `T: Send` を要求、`Arc<T>: Send` は `T: Send + Sync` を要求）
- RT 契約 — コールバック経路で alloc/block/OS 呼出し不可（`realtime-contract-and-verification-policy`）
- baseline 依存方向 — engine は拡張層非依存、`Processor` port 経由駆動
- `Processor→RtProcessor` の型レベル分離 — `prepare` が RT 処理を持つ別オブジェクトを産む（`ruviform-rt/processor.rs:10-15`）は既存の正解パターン
- L0 の直接呼出し価値 — in-process = IPC なし・低遅延。可能なら保全する
- non-RT retirement — plan-swap 退役の plugin リソースを RT で drop しない（`audio-engine-snapshot-swap` ADR 期待・`RtPlan.cached` の最終 `Arc` が RT drop する現行観測）
- `unsafe` 原則禁止・依存最小

## Considered Options

- **host-in-node wrapper** — `PluginHost`/`Arc<Mutex<PluginHost>>` を `AudioNode` 内に保持し `process` を直接呼ぶ
- **transferable RT endpoint + non-RT control** — `PluginHost` は non-RT 専有、インスタンス毎の `Send` endpoint を non-RT `prepare` で産みノードが所有（`Processor→RtProcessor` の写像）
- **full async service** — host を専用 non-RT スレッドに閉じ込め、全ブロックを SPSC で marshal
- **global `Send` bound** — `ErasedPlugin`/`Plugin` に `Send` supertrait を全域付与し host/インスタンスを `Send` 化

## Decision Outcome

Chosen option: **transferable RT endpoint + non-RT control**。

- `PluginHost` は non-RT 制御側にのみ存在する — `register_factory`/`register_out_of_process`/`execute`/`lifecycle_state`/`try_recv_notification`/`kill_child`/`Destroy` はすべて non-RT で、ホスト本体は `GraphPlan`・`RtPlan`・コールバックに入らない
- RT 側はインスタンス毎の **`Send` 処理 endpoint** をノードが所有する — `Processor::prepare → Rt` の型レベル分離と同形で、non-RT 側の `prepare` 系ステップが endpoint を産み、1 度だけ `Send` でノードへ移す
- **L1**: endpoint = `data::RtLeg` + `Shared::rt_state`（lock-free lifecycle mirror）+ `arm` 経路。`RtLeg` は既に `Send` 形状（`Producer`/`Consumer`/`Arc<Staging>`/所有 Vec）で shm を触らない — `Shmem` は `!Send` だが I/O スレッドが各々 `View` を開く設計のため data plane は分離済み（`l1/mod.rs:1249-1257`・`l1/data.rs:413-448`）
- **L0**: endpoint は `Send` な erased 処理オブジェクトを要 — **opt-in `Send` capability** とし、非対応 plugin の graph 投入は defined rejection（全域 `Send` bound は不採 — OFX/foreign-ABI adapter が満たせない）
- **non-RT verb の扱い**: graph-attached handle への `SaveState`/`RestoreState`/`stop_processing`/GUI 等の到達経路と defined 挙動は design が確定（共有 `Arc<Mutex>` try-access vs endpoint 抽出 + verb relay）
- **退役**: node removal/plan-swap で退役した endpoint/plugin リソースは RT コールバック上で drop しない — `ruviform-rt::retire_channel` offload または non-RT 最終参照保持（機構は design）
- **engine 境界**: `ruviform-audio` は plugin 型を露出しない — endpoint → `AudioNode` の接着は engine 所有の port-level adapter または consumer 配線（crate 配置は design）

### Consequences

- Good: `Send` が型の証拠になる — 制御状態の RT 漏れが構造的に不可能。L1 は既存 `RtLeg`/`rt_state`/`arm` の分割で実現。`unsafe` 不要
- Good: L0 の直接呼出しを保全（Send-capable plugin に限り）— async 化しない
- Bad: L0 graph 投入に plugin 側 `Send` 前提が必要 — `ErasedPlugin`/`PluginFactory` 契約の opt-in 拡張が要
- Bad: endpoint の arm/退役配線が新規（control→RT の `Arc<Staging>` 経路・retire offload）
- Bad: graph-attached handle の non-RT verb 到達経路は未確定 — design の論点

### Confirmation

- spec `docs/specs/engine-plugin-host-integration.md` の AC（実グラフ消費・Send 証拠・構成面・退役・E2E）が plan/test で検証される
- `cargo doc`/DAG 監査で `ruviform-audio` に plugin 型/依存が露出しないこと
- `rt_` テストで endpoint `process` が 0 alloc・no-block・try のみであること

## Implementation Plan

- Affected paths:
  - `crates/ruviform-plugin-host` — endpoint 公開面（`RtLeg` の公開/`prepare` 系 verb・`Arc<Staging>` arm 経路・L0 `Send` erased 経路）・`l1/data.rs` 可視性
  - `crates/ruviform-plugin-core` — `Send` erased 契約（opt-in capability — 全域 bound ではない）
  - `crates/ruviform-audio` — endpoint → `AudioNode` の port-level adapter（engine は plugin 型を名指さない）
  - `crates/ruviform-audio/src/plan.rs` — 退役 offload シーム（`RtPlan.cached` 差替で最終 `Arc` が RT drop しない）
  - `crates/ruviform-rt` — `retire_channel`/`spsc` の再利用（新規機構なし）
- Patterns to follow: `RtLeg`/`rt_state` mirror の分離、`GraphPlan::drive` の `try_lock` + defined 出力、`retire_channel` の deferred-drop、`Processor→RtProcessor` の型分離、`assert_no_alloc` の `rt_` レーン
- Patterns to avoid: host-in-node（`Send` 違反）、`unsafe impl Send`（契約外の `Send` 偽装）、全域 `Send` bound（OFX 不適合）、RT 側での blocking lock（`Mutex::lock`）、RT 上での plugin/resource drop
- Dependencies: 新規外部依存なし。`ruviform-rt` SPSC/retire を再利用

## Verification

- [ ] `Send` 証拠 — ノード/`GraphPlan`/コールバック capture が `Send` でコンパイル、`PluginHost`/`L1Instance`/`ErasedPlugin` が RT 側に存在しない（型検査）
- [ ] mock device E2E — L0/L1 ノードが callback 経路で per-block 駆動・データ正当
- [ ] `rt_` テスト — endpoint `process` が `assert_no_alloc` 窓内 0 alloc・`mise run test-rt` グリーン
- [ ] 退役 — plan-swap/node removal で plugin リソースの drop が RT 上で発生しない
- [ ] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` パス

## Pros and Cons of the Options

| Option | Pros | Cons |
| --- | --- | --- |
| host-in-node | host API そのまま | `Send` 契約で構造的に不可能（`Mutex`/`Arc` も `T: Send` 要求） |
| transferable endpoint | `Send` が型証拠・`Processor→RtProcessor` と同形・L1 既存分割・unsafe 不要 | L0 `Send` 契約拡張が要・arm/退役配線が新規 |
| full async service | Send 契約変更不要 | L0 直接呼出しを喪失（in-process 化の意味がない）・hop スレッド/遅延増 |
| global `Send` bound | 単純 | OFX/foreign-ABI adapter が満たせない・過剰契約 |

## More Information

- spec: `docs/specs/engine-plugin-host-integration.md`
- discovery: `docs/discovery/engine-plugin-host-integration.md`
- 関連 ADR: `audio-engine-snapshot-swap.md`（non-RT retirement 期待）・`plugin-isolation-strategy.md`（L0/L1 階層）・`realtime-contract-and-verification-policy.md`・`rt-foundation-primitives-implementation.md`（`retire_channel`/SPSC）・`ofx-host-video-contract-and-c-abi.md`（video NONRT 先例）
- 一次証拠: `composite.rs`（`PluginHost` !Send）・`l1/data.rs`（`RtLeg` Send 形状）・`l1/mod.rs`（`rt_state` mirror・`process` 委譲）・`l1/shm.rs`（`Shmem` !Send・per-thread `View`）・`plugin-core/plugin.rs`（`ErasedPlugin` Send なし）・`audio/graph.rs`・`plan.rs`・`session.rs`・`audio-core/device.rs`（callback `Send`）
