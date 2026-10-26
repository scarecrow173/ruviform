---
id: "DISC-034MbMMsIDqdQqkkTzFSTf"
type: "discovery"
status: "resolved"
title: "エンジン/利用側へのプラグイン統合（plugin-host ノード化 + L1 選択配線）の探索（Issue #113）"
created: "2026-10-25"
updated: "2026-10-26"
owners: ["scarecrow173"]
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
    - "../adr/realtime-contract-and-verification-policy.md"
    - "../adr/rt-foundation-primitives-implementation.md"
    - "../adr/ofx-host-video-contract-and-c-abi.md"
    - "../designs/rt-foundation-api-design.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/plugin-l1-out-of-process.md"
    - "../specs/plugin-l1-out-of-process.md"
    - "../designs/plugin-l1-out-of-process.md"
    - "../designs/ruviform-architecture-design-baseline.md"
    - "../specs/plugin-host-l0.md"
    - "../designs/plugin-host-l0.md"
    - "../specs/audio-engine.md"
    - "../designs/audio-engine.md"
    - "../adr/plugin-isolation-strategy.md"
    - "../designs/plugin-isolation.md"
    - "../adr/audio-engine-snapshot-swap.md"
  derived-by:
    - "../specs/engine-plugin-host-integration.md"
    - "../adr/plugin-rt-ownership-split.md"
    - "../designs/engine-plugin-host-integration.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# エンジン/利用側へのプラグイン統合（plugin-host ノード化 + L1 選択配線）の探索（Issue #113）

## Exploration Goal

Issue [#113](https://github.com/scarecrow173/ruviform/issues/113) は、#76 で完成した L1 アウトプロセス経路（`PluginHost` コンポジット）が host API としては完成したが、利用側（engine/audio graph）から到達不能であることを指摘し、`PluginHost`/`PluginHandle` を利用側へ接続することを要求する。

Issue スコープ:

- `Processor` 契約ノードとして `PluginHost`/`PluginHandle` を audio グラフへ接続（プラグインチェーン接続は crate 責務内・未実装）
- `register_out_of_process`/`OutOfProcessSpec` の opt-in 選択面をエンジン構成 API へ露出（`lookahead_blocks` 遅延契約を含む）
- video 側（OFX NONRT ノード）の境界を本 Issue 内で確定 or 後続 Issue へ明示

トリガ: `docs/designs/plugin-l1-out-of-process.md` L268 deferred 行（「エンジン/利用側への L1 配線 | plugin-host の engine 消費自体が未実装 | engine プラグイン統合 Issue | L1 が consumer から到達不能」）— 本 Issue がその trigger。

本探索は **issue framing・コード観測事実（Send/Sync 制約・graph/session/plan 構造・engine 境界ルール・L1 面・video NONRT 状態）・選択肢比較・確認済みスコープの記録**を行う。採否・方式の確定権限は design フェーズにあり、本書は最終判断を含まない。所有分離のアーキテクチャ判断は対の ADR [`plugin-rt-ownership-split.md`](../adr/plugin-rt-ownership-split.md)（proposed）に記録した。

## Key Issues and Assumptions

### 上流で確定済み（confirmed — design 論点ではない）

Issue スコープ（briefing で再確認済み — ユーザー承認の確定意図）:

- 実 audio-graph 消費（example-only の host 呼出しではなく、グラフ内部からの per-block 駆動）
- `PluginHost` が `!Send`/`!Sync` であることに起因する **non-RT 制御側 / RT 処理側の所有分離**
- non-RT 構成面が default L0 と opt-in `OutOfProcessSpec` を選択 — engine は拡張層非依存のまま（baseline DAG: engines → 拡張層依存禁止）
- 明示的 prepare・bounded scratch/capacity 検証・error/fallback・L1 遅延報告（`lookahead_blocks × block_frames / sample_rate`）
- mock/device-independent な L0/L1 graph E2E + RT alloc テスト（node removal/plan-swap retirement を含む）
- video = NONRT 境界の明文化 + trigger 付き明示的延期。L1 video wire は確立しない。Issue #114 は real-adapter work のまま
- 退役 plugin リソースを RT コールバック経路上で drop しない（`audio-engine-snapshot-swap` ADR の non-RT retirement 期待に整合）

### コード観測事実（confirmed — 前提が最初に崩れる地点）

#### Send/Sync 制約（所有分離を強制する型証拠）

- `AudioNode: Send`（`crates/ruviform-audio/src/graph.rs:16`）— ノードは `Vec<Box<dyn AudioNode>>` として `Graph` に入り、`GraphPlan`（`Mutex<Graph>`、`plan.rs:15-17`）→ `Arc<RwLock<Arc<GraphPlan>>>`（`plan.rs:48,64`）→ `OutputDataCallback = Box<dyn FnMut + Send + 'static>`（`crates/ruviform-audio-core/src/device.rs:512`）の `Send` クロージャ内へ流れ込む。`GraphPlan: Send+Sync` が要求されるため **全ノードは `Send`**。
- `PluginHost` は `!Send`/`!Sync`（`composite.rs:205-209` doc）— L0 は `Box<dyn ErasedPlugin>`（erased object に auto-trait なし）、L1 は `shared_memory::Shmem`（raw pointer、`l1/shm.rs:256-257`）を保持。
- `PluginFactory::create() -> Box<dyn ErasedPlugin>`（`crates/ruviform-plugin-core/src/plugin.rs:215`）— `dyn ErasedPlugin` は `Send` supertrait を持たず（`plugin.rs:227`）、**L0 の処理対象そのものが `!Send`**。
- `PluginHost::process(&mut self, ...)`（`composite.rs:403-418`）は RT-suitable だが `&mut` ホスト状態を要求 — L0 は `InProcessHost::process` → `&mut dyn ErasedPlugin`（`host.rs:181-197`）、L1 は `L1Instance::process` → `rt: data::RtLeg`（`l1/mod.rs:1249-1257`）。
- 結論: **`PluginHost`（または `Mutex` 等で包んだもの）を `AudioNode` 内に置くことは型レベルで不可能** — `Mutex<T>: Sync` も `T: Send` を要求し、`Arc<T>: Send` は `T: Send + Sync` を要求する。所有分離は設計好みではなく型の強制。

#### RT 処理 endpoint の既存の形（L1 はほぼ `Send` 形状済み）

- `data::RtLeg`（`l1/data.rs:413-448`）= `Producer<Submit>` + `Consumer<u64>` + `Option<Arc<Staging>>` + 所有 `Vec` scratch — 全て `Send` 形状（`ruviform_rt::Producer`/`Consumer` は `T: Send` で `Send` — `spsc.rs:47,143`。`Staging` は atomic 群 + Vec — `data.rs:229-239`）。
- `L1Instance::process`（`l1/mod.rs:1249-1257`）= `shared.rt_state`（`AtomicU8`・Acquire）→ `mirror_decode` → `self.rt.process(mirror, ...)`。`rt_state` は RT leg 専用の lock-free lifecycle snapshot として設計済み（`l1/mod.rs:256` doc: "the RT leg never needs the mutex"）。
- `Activate` 成功時に `staging_generation += 1` → `Staging::new` → `shared.staging` 公開 + `self.rt.arm(staging)`（`l1/mod.rs:1124-1130`）— endpoint を分離する場合、`arm` の `Arc<Staging>` を non-RT → RT へ届ける経路が新規に要。
- `Shmem` は `!Send`（`l1/shm.rs:256-257`）— しかし RT leg は shm に触れない（driver/reader スレッドが各々 `shm::View` を key で開く — `l1/mod.rs:451-457`・`l1/mod.rs:792`・`l1/mod.rs:958`）。**L1 の `!Send` 因子は RAII `segment` のみであり、RT データプレーンとは既に分離済み**。

#### L0 の `Send` 壁（contract レベルの欠落）

- L0 の処理対象 `Box<dyn ErasedPlugin>` は `!Send` — `erase()` の返却型が `Box<dyn ErasedPlugin>` に固定（`plugin.rs:319`）。graph 内 L0 処理には **`Send` な erased 処理オブジェクト**が要 — plugin-core 契約レベルの論点（全域 `Send` 付与 vs opt-in capability）。
- `PluginProcessor::process`/`ErasedPlugin::process_host` は RT entry として設計済みだが `Send` 前提ではない（`plugin.rs:101`・`plugin-core/processor.rs:426`）。

#### graph/session/plan 構造

- `Graph` = `Vec<Box<dyn AudioNode>>` 順序リスト（`graph.rs:25-61`）。`Graph::pull` は各ノードを順に `&mut` 駆動 — RT contract は全ノード連立。
- `GraphPlan` = `Mutex<Graph>`（`plan.rs:15-39`）— `drive` は `try_lock`、競合時は silent no-op。
- `PlanSwap`/`RtPlan` = `Arc<RwLock<Arc<GraphPlan>>>` + RT 側 `cached`（`plan.rs:42-101`）— `swap` は non-RT、`active()` は `try_read` + last-good（≤1 block late）。
- `Session` = `OutputStream` + `PlanSwap`（`session.rs:38-113`）— callback が `RtContext::mint` → `plan.drive`。
- **plan-swap retirement の観測**: `PlanSwap::swap` は RwLock 内 `Arc` を writer スレッド（non-RT）で drop。`RtPlan.cached` の旧 `Arc` は RT 側 `active()` で差替時に drop — **最後の `Arc` なら `GraphPlan`（= 全ノード + plugin リソース）の Drop が RT スレッドで走る**。snapshot-swap ADR は「退役 Plan の Arc は非 RT 側が保持して自然 drop」を期待（`audio-engine-snapshot-swap.md` Decision Outcome）。`ruviform-rt::retire_channel`（`retire.rs`）が deferred-drop 機構として存在。

#### engine 境界ルール

- baseline: 「engines → 拡張層の直接依存は禁止（エンジンは `Processor` 等の下位ポート経由でプラグインを駆動する）」（`ruviform-architecture-design-baseline.md` 依存方向ルール）。
- `ruviform-audio` deps = `ruviform-rt`/`ruviform-audio-core`/`ruviform-media-core` のみ。`ruviform-plugin-host` deps = `ruviform-plugin-core`/`ruviform-rt`/`shared_memory`。**相互に相手の型を名指せない** — 接着は利用側（consumer）配線または port-level adapter の責務。
- baseline `ruviform-audio` 責務行には「プラグインチェーン接続」が既に含まれる — engine が消費するのは port（`Processor`/`RtProcessor`/`AudioNode`）であり plugin 型ではない。

#### L1 面（選択配線の対象）

- `OutOfProcessSpec`（`composite.rs:65-101`）: `descriptor`/`program`/`args`/`lookahead_blocks`（既定 2）/`command_timeout`（既定 10s）。`register_out_of_process` → `DuplicatePluginId` defined rejection（`composite.rs:295-314`）。
- 遅延契約: `lookahead_blocks × block_frames / sample_rate`（既定 ≈21.3ms @ 48kHz/512）。`lookahead_blocks = 0` は spawn で `Err(Ipc)`（`l1/mod.rs` spawn 検証）。
- `HostNotification::InstanceFailed { handle, exit }` — L1 `Failed` を once で通知（`composite.rs:143-153`・`try_recv_notification` は non-RT）。
- `lifecycle_state` は L1 で non-RT（mutex、`composite.rs:353-362`）。`plugin_mut` は L1 で `None`（`composite.rs:371-383`）。`kill_child` は `#[doc(hidden)]` 検証支援（`composite.rs:467`）。
- pass-through: 結果未着/`Failed` → 直前受信ブロック else 無音（`composite.rs:189-198`・`data.rs` `RtLeg::pass_through`）。

#### video 側（NONRT 境界）

- plugin-core `VideoPluginInstance`/`VideoProcessConfig` は全操作 NONRT（`crates/ruviform-plugin-core/src/video.rs:3-4` doc）。`ruviform-plugin-ofx` は NONRT L0 adapter（`ofx-host-video-contract-and-c-abi.md`）。
- L1 design deferred 行 L271: 「video/NONRT プラグインの L1 経路 | 本 Issue は audio `process` 経路のみ | NONRT video port の分離要求 | video plugin は L1 不可」。
- Issue #114 = 規格アダプタの子プロセス組込み・実プラグイン E2E（real-adapter work）— 本 Issue の範囲外。

## Alternatives and Comparison

### 軸 1: RT 側からの plugin `process` 到達形

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) host-in-node wrapper（`PluginHost`/`Arc<Mutex<PluginHost>>` を `AudioNode` 内に保持） | host API をそのまま利用 | **`Send` 契約で構造的に不可能** — `PluginHost`/`L1Instance`/`ErasedPlugin` が `!Send`。`Mutex<T>: Sync`/`Arc<T>: Send` の前提も `T: Send` | no（型エラー） |
| (b) transferable RT endpoint + non-RT control | `Processor→RtProcessor` の型レベル分離と一致（`rt/processor.rs:10-15`）。L1 は `RtLeg` が既に `Send` 形状・`rt_state` mirror 済み。`unsafe` 不要 | L0 は `Send` な処理オブジェクトの契約追加が要。endpoint の arm/退役配線が新設 | candidate |
| (c) full async service（全ブロックを non-RT host pump スレッド経由で marshal） | Send 契約変更不要・host 不変 | L0 の直接呼出し価値（IPC なし・低遅延）を喪失 — in-process を graph に置く意味がない | no |
| (d) `ErasedPlugin`/`Plugin` への全域 `Send` bound | 単純 | OFX/foreign-ABI adapter（`ruviform-plugin-ofx` 実 ABI）が満たせない可能性。過剰契約 | no（全域化は不採 — opt-in は (b) の下位論点） |

### 軸 2: L0 `Send` 処理オブジェクトの実現形（(b) 採用前提の下位論点）

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (i) opt-in `Send` erased 経路（factory/plugin が `Send` な erased 処理オブジェクトを提供できる capability。非対応は defined rejection） | 全域契約変更を回避・plugin 毎の選択 | plugin-core/host の API 追加。未対応 plugin は graph 不可（L0 out-of-graph or L1 経由） | candidate |
| (ii) `Arc<Mutex<…>>` 共有インスタンス（host 表に残し RT は `try_lock` — `GraphPlan::drive` 先例） | unified handle 意味を維持。non-RT verb が到達可能 | `Send` 前提は (i) と同じ。`try_lock` 失敗時の defined 出力が要 | maybe |
| (iii) インスタンスを node へ move（抽出） | 単純な所有 | host の `SaveState`/GUI 等 non-RT verb が handle 経由で届かなくなる | maybe |

（確定は design — (i) の opt-in `Send` capability はいずれの形でも必須）

### 軸 3: ノード/接着の crate 配置

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) engine が port-level 汎用 adapter（`RtProcessor`/endpoint → `AudioNode`）を持ち、plugin-host が endpoint を提供、consumer が配線 | engine は拡張層非依存のまま（baseline 整合）・plugin-host に新規依存なし | 2 箇所の API 追加 + consumer 配線 | candidate |
| (b) 新規 wiring crate | 境界明確 | workspace メンバー増・1 用途に過剰の恐れ | maybe |
| (c) `ruviform-edit`（利用側層）に配置 | 既存 consumer crate | 現在 `media-core` のみ依存・engine/host 双方への依存追加が必要 | maybe |

（確定は design — engine が plugin 型を名指さないことは確定済み制約）

### 軸 4: 退役 plugin リソースの drop 境界

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) `ruviform-rt::retire_channel` へ退役 endpoint/plan を offload | 既存機構・RT 検証済み（`retire.rs`） | plan/node に retirer 配線が要 | candidate |
| (b) non-RT 側が最終参照を保持（held `Arc`）し退役 drop を non-RT に限定 | 機構追加なし | 現行 `PlanSwap` API は retire 側 `Arc` の保持経路を持たない — API 面の検討が要 | maybe |
| (c) 現状維持 | — | `RtPlan.cached` の最終 `Arc` が RT で drop → node destructor（Vec 解放・L1 staging teardown）が RT 違反 | no |

## Tentative Conclusions and Hypotheses

- **confirmed**: `PluginHost` の graph 内在は `Send` 契約上不可能 — **non-RT 制御 / RT 処理の所有分離は型が強制する**。実アーキテクチャ分岐として ADR 化が妥当 — [`plugin-rt-ownership-split.md`](../adr/plugin-rt-ownership-split.md)（proposed）として記録済み
- **confirmed**: L1 の RT データプレーン（`RtLeg` + `rt_state` mirror）は既に `Send` 形状 — endpoint 化は抽出/分割レベルで実現可能
- **confirmed**: L0 graph 内処理は `Send` erased 処理オブジェクトを要 — plugin-core 契約論点（design 確定対象）
- **confirmed**: engine（`ruviform-audio`）が plugin-host 型を公開 API に露出してはいけない（baseline 依存方向ルール）
- **confirmed**: plan-swap 退役で plugin リソースを RT コールバック上で drop してはならない（snapshot-swap ADR の期待 + `retire_channel` 機構あり）
- **confirmed**: video 側は NONRT 境界の明文化 + trigger 付き延期。L1 video wire は確立しない（L1 design deferred L271 と整合）
- **hypothesis**: 構成面は default L0 / opt-in `OutOfProcessSpec`（`lookahead_blocks`/`command_timeout`）を consumer-facing non-RT API で露出 — `DuplicatePluginId`/`Err(Ipc)` の defined rejection を維持
- **deferred（Issue スコープ外として確認済み）**: 実規格アダプタ・実プラグイン E2E（#114）・L1 video wire・video plugin の RT 経路・自動再起動・GUI 配線

## Open Questions

> すべて design フェーズが答える論点 — architecture 未決ではなく実装設計の確定対象。**[2026-10-26 追記] 全項目は design `docs/designs/engine-plugin-host-integration.md`（draft、再レビュー反映済み）の決定サマリ a–k で確定済み**。

- L0 `Send` 処理オブジェクトの契約形（opt-in `+ Send` erased 経路 vs `Arc<Mutex>` 共有 vs 抽出）と非対応 plugin の defined rejection 面
- endpoint の公開形（host の `prepare` 系 verb で産むか、`RtLeg` を直接分割するか）と `RtProcessor`/`Processor` 契約への載せ方
- `Activate` の `Arc<Staging>` arm を non-RT → RT へ届ける経路（SPSC arm channel 等）と再 Activate/世代管理
- 退役 offload のシーム（`retire_channel` を plan/node どちらに配線するか vs non-RT 最終参照保持）
- ノード adapter の `Io`/バッファ写像（`AudioBufferViewMut` ↔ `HostInput`/`HostOutput` — plugin-core は audio-core 依存済みで写像の所有者は要確定）
- graph E2E の配置（dev-dep 方向 — engine↔host のテスト依存は循環 dev-dep 許容か、wiring crate か）
- L1 遅延報告 API の形（config surface での表現 — frames/秒/lookahead のどれで露出するか）
- graph-attached handle への non-RT verb（`SaveState`/`RestoreState`/`stop_processing`/GUI）の到達経路・defined 挙動
- video NONRT ノードの境界形（`VideoPluginInstance` を video engine 側にどう接続するか — 本 Issue は境界の明文化のみ）

## Promotion Candidates

- [x] spec-doc: [engine-plugin-host-integration.md](../specs/engine-plugin-host-integration.md)（**created — proposed**。confirmed scope・受入基準を formalize）
- [x] adr-doc: [plugin-rt-ownership-split.md](../adr/plugin-rt-ownership-split.md)（**created — proposed**。所有分離の実アーキテクチャ分岐 — rejected: host-in-node wrapper（型上不可能）・full async service・全域 `Send` bound）
- [x] design-doc: [engine-plugin-host-integration.md](../designs/engine-plugin-host-integration.md)（**created — draft**、再レビュー反映済み）。Open Questions の全項目を決定サマリ a–k で確定

## Sources and Relations

- Issue [#113](https://github.com/scarecrow173/ruviform/issues/113)（本探索対象）・[#76](https://github.com/scarecrow173/ruviform/issues/76)（L1 実装・CLOSED）・[#74](https://github.com/scarecrow173/ruviform/issues/74)（L0 実装・CLOSED）・[#114](https://github.com/scarecrow173/ruviform/issues/114)（real-adapter 後続）
- 一次文書: [plugin-l1-out-of-process.md](../designs/plugin-l1-out-of-process.md)（L268/L271 deferred 行 — trigger）・[plugin-l1-out-of-process.md](../specs/plugin-l1-out-of-process.md)（L95/L128）・[plugin-l1-out-of-process.md](../discovery/plugin-l1-out-of-process.md)（親探索）・[ruviform-architecture-design-baseline.md](../designs/ruviform-architecture-design-baseline.md)（依存方向ルール・A.2）・[plugin-host-l0.md](../specs/plugin-host-l0.md)/[plugin-host-l0.md](../designs/plugin-host-l0.md)・[audio-engine.md](../specs/audio-engine.md)/[audio-engine.md](../designs/audio-engine.md)・[audio-engine-snapshot-swap.md](../adr/audio-engine-snapshot-swap.md)・[plugin-isolation-strategy.md](../adr/plugin-isolation-strategy.md)/[plugin-isolation.md](../designs/plugin-isolation.md)
- ローカル一次証拠: `crates/ruviform-audio/src/graph.rs`（`AudioNode: Send`・`Vec<Box<dyn AudioNode>>`）・`plan.rs`（`GraphPlan`/`PlanSwap`/`RtPlan::active`）・`session.rs`（callback 配線）・`crates/ruviform-rt/src/processor.rs`（`Processor→RtProcessor` 分離）・`retire.rs`（`retire_channel`）・`spsc.rs`（`Producer`/`Consumer` Send）・`crates/ruviform-plugin-host/src/composite.rs`（`PluginHost` !Send・`process`/`execute`/`lifecycle_state`/`try_recv_notification`/`register_out_of_process`/`OutOfProcessSpec`/`HostNotification`）・`l1/mod.rs`（`L1Instance::process` → `rt` + `rt_state`）・`l1/data.rs`（`RtLeg`/`Staging`/`arm`/`process`）・`l1/shm.rs`（`Shmem` !Send・per-thread `View`）・`crates/ruviform-plugin-core/src/plugin.rs`（`ErasedPlugin`/`PluginFactory::create`・Send なし）・`video.rs`（`VideoPluginInstance` NONRT）・`crates/ruviform-audio-core/src/device.rs`（`OutputDataCallback: Send`）
