---
id: "TASK-034MrsLrqKnu9N3YfxnSGl"
type: "task"
status: "done"
title: "L0 Shared スロット + endpoint inner（SharedSlot/SendCore・3 段 lock・epoch attach・catch_unwind 収束）"
created: "2026-10-26"
updated: "2026-10-26"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/endpoint.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/host.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/lib.rs"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-113-engine-plugin-integration.md"
  implemented-by:
    - "docs/impl/ir/issue-113-l0-shared-slot-and-endpoint-inner-task-2-implementation.md"
  depends-on:
    - "docs/plans/implement-issue-113-engine-plugin-integration.md"
    - "docs/tasks/plugin-core-endpoint-primitives.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-113-engine-plugin-integration.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/plugin-endpoint-contract-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# L0 Shared スロット + endpoint inner（SharedSlot/SendCore・3 段 lock・epoch attach・catch_unwind 収束）

## Work

Plan actions 4–7。`ruviform-plugin-host` の L0（in-process）経路に endpoint 所有分離の機構を導入する — `Instance` のスロット enum 化・`SharedSlot`/`SendCore`・全 verb の 3 段 lock 形態・commit 順序・`Failed` 収束・`src/endpoint.rs` 新設（`PluginEndpoint`/`EndpointBounds`/`EndpointInner`/`L0Endpoint`）・epoch attach/`Drop` 返却・L0 inner `process`・`InProcessHost` の `pub(crate)` attach 内側。**endpoint 公開面は v1 では `PluginHost` コンポジットのみ** — standalone `InProcessHost` に公開 endpoint API は作らない（design 確定）。

- **`mirror_encode`/`mirror_decode` は `l1::data` に維持（`src/mirror.rs` は新設しない）**: `src/l1/` 木は `crate::`/`ruviform_plugin_host::` パス禁止の import discipline（`l1/mod.rs` L19-26 — `tests/l1_proxy_e2e.rs` が `#[path]` で同木を include するため）にあり、`crate::mirror` 等の crate 直下モジュールは l1 側から**到達不能**。`l1::data` は既に `pub(crate)` で sibling `data::` パスからも `crate::l1::data::` パスからも到達できる「l1 から到達可能な crate 内共有位置」— design の「`l1::data` に残し crate 内共有で参照」と一致（移動なし・L0 側は `crate::l1::data::mirror_encode`/`mirror_decode` を利用）。u8 順 `Created0/Initialized1/Activated2/Processing3/Failed4/Destroyed5`（`Failed`/`Destroyed` ≥ `Activated` なので `≥` 比較を不変条件に使わず列挙で書く）
- **`host.rs` `Instance` スロット enum**: `enum InstanceSlot { Plain { plugin: Box<dyn ErasedPlugin>, context: ProcessContext }, Shared(SharedSlot) }` — `Instance { slot, lifecycle, generation, gui_open }`。`SharedSlot { core: Arc<Mutex<SendCore>>, mirror: Arc<AtomicU8>, attached: Arc<AtomicU64>, bound: Option<EndpointBounds>, next_epoch: u64 }`・`SendCore { plugin: Box<dyn ErasedPlugin + Send>, context: ProcessContext, activated: Option<ProcessConfig> }`（`activated` は `Activate` commit で保持した `ProcessConfig` の値コピー — mirror ∈ {Activated,Processing} ⟹ `activated.is_some()` 不変条件の担い手・`Deactivate`/`Destroy` でクリア）
- **`Instantiate` 分岐**: `factory.create_sendable()` が `Some` → `Shared` slot（`SendCore` 内に Send plugin + context・mirror=`Created`・`attached=0`・`bound=None`・`next_epoch=1`）・`None` → `Plain` slot（`factory.create()` — 現行の `Box<dyn ErasedPlugin>`）。非 Send factory の L0 利用は損なわない
- **全 verb の 3 段 lock 形態**（`Shared` slot で `plugin`/`context` に触れる全 verb — lifecycle 動詞 + `SaveState`/`RestoreState`/`OpenGui`/`CloseGui`）:
  1. entry pre-check — `slot.mirror.load(Acquire)` で `Failed` なら lock せず収束（`LifecycleEvent::Failed` + `Err(HostError::Failed)`）
  2. `match slot.core.lock()` — `Err(PoisonError)` なら同じ収束（poisoned lock は Failed 観測として扱う）
  3. lock 内で `slot.mirror.load(Acquire)` 再読 — `Failed` なら hook/transition/`set_state` を一切実行せず解放して収束
  成功時は lock 内で transition 合法性検査 → commit → `set_state`（mirror store）→ plugin hook を同一クリティカルセクション内で実行 — `try_lock` 観測は常に完全 commit 済みのみ（half-committed mirror は構造上存在しない）
- **`destroy` の `into_inner` 例外**: `destroy` のみ `unwrap_or_else(PoisonError::into_inner)` — host 側 `Failed→Destroyed` transition + mirror=`Destroyed` store + core 内容物 drop（panic 済み plugin の destroy hook は呼ばない・teardown のみ）
- **`&self` 照会は観測 union のみ**: `lifecycle_state(&self)` — `Plain` は従来どおり `instance.lifecycle.state()`、`Shared` は `slot.mirror.load(Acquire)` のみ（`core.lock()` 不許可・`lifecycle.state()` 非更新 — commit は `&mut` entry point で行う）。`plugin_mut` — `Shared` で `None`（`Mutex` 越しに `&mut` を貸せない — L1 と同じ defined 挙動）。host 側 `process` — `Shared` は `try_lock`、busy は `Err(EndpointUnavailable)`（blocking `lock()` しない — RT-suitable 維持）
- **`src/endpoint.rs` 新設**: `pub struct PluginEndpoint { inner: EndpointInner, bound: EndpointBounds, in_scratch: AudioBuffer<f32>, latency_frames: u32 }`・`pub struct EndpointBounds { pub max_frames: u32, pub channels: u32 }`（design の正本どおり `u32` — `ProcessConfig`/`WIRE_MAX_*` 語彙と整合）・`enum EndpointInner { L0(L0Endpoint), L1(L1Endpoint) }`（L1 variant は T3 で充填 — ここでは enum 骨格 + `L0` のみ可。`EndpointInner::process(bound, in, out, …)` の variant dispatch 形をここで定義）・`L0Endpoint { core: Arc<Mutex<SendCore>>, mirror: Arc<AtomicU8>, attached: Arc<AtomicU64>, epoch: u64 }`
- **epoch attach/detach**: `InProcessHost::attach_endpoint`（`pub(crate)` — `prepare_endpoint` の L0 内側）が `slot.next_epoch`（初期値 1・attach 毎に単調増）を採番し `slot.attached.compare_exchange(0, epoch, AcqRel, Acquire)` — 非 0 なら `Err(EndpointUnavailable)`（単一 endpoint 規則）。`L0Endpoint::drop` は `compare_exchange(epoch, 0, AcqRel, Acquire)` で**自分の epoch のみ**返却（後続 attach の新 epoch を掴み消さない）
- **L0 inner `process`**（`EndpointInner::L0` arm）: mirror Acquire fast-skip（`Failed`/非 `Processing` で早期 short-circuit）→ `core.try_lock()`（busy → `Err(EndpointUnavailable)` = `out` 不変更の一時 bypass）→ lock 内 mirror Acquire 再読 → `activated` gate（`min_frames ≤ frames ≤ max_frames`・channels 一致 — in-bus-first 規則の frames/channels は T6 の shell が供給）→ `plugin.process_host(...)` を `catch_unwind`（`AssertUnwindSafe`）で駆動 — panic 捕捉・poisoned `try_lock`（`Err(TryLockError::Poisoned)`）・`Failed` 観測のいずれでも `slot.mirror.fetch_max(Failed, Release)`（endpoint は第 2 の書き手）+ `Ok(CONTINUE)` + `last_output`/out への無音書込み（終端条件）
- **bounded 検証（attach 内側 — 拒否順序の pin）**: `attach_endpoint` の拒否は **state 系を slot-shape 系より先に**評価する — (1) state check（`Activated`/`Processing` 要求 — `Destroyed`/stale は `Err(InvalidHandle)`・`Failed` は `Err(Failed)`・それ以下は `Err(Lifecycle)` — 写像の全表は T4 統一表）→ (2) slot-shape check（`Plain` slot → `Err(EndpointUnavailable)` — 非 Send factory・`create_sendable() == None`）→ (3) L0 activated 層の bounds 検査（`bound.max_frames`/`channels` 非 0・`max_frames ≥ SendCore.activated.max_frames`・`≤ WIRE_MAX_FRAMES`(8192)/`≤ WIRE_MAX_CHANNELS`(16) — 違反は `Err(Prepare(UnsupportedConfig))`）→ (4) epoch CAS attach（非 0 → `Err(EndpointUnavailable)` — 2 本目拒否）→ (5) `in_scratch` 事前確保。**所有の分割**: L0/L1 共通の uniform bounds prelude（非 0・`≤ WIRE_MAX_*`・`≥ activated` — slot 毎の activated 照会を行い inner dispatch より前に評価）は **T4 の `PluginHost::prepare_endpoint` が所有**し、本 task の内側検査は L0 activated 層チェックとして残る（`SendCore.activated` を所有する層での再検査 — prelude と同規則で defense-in-depth・矛盾する結果は出ない）。`u32::MAX` 級の bound は prelude/内側とも `Err(Prepare(UnsupportedConfig))` で defined rejection — cap（8192×16ch=512KiB 上限）により巨大確保の alloc abort は起こらない。`in_scratch` は `AudioBuffer::try_new(BufferSpec { layout: <bound.channels から導く `ChannelLayout` — named variant または `custom` mask>, frames: bound.max_frames, sample_rate: <activated `ProcessConfig` の sample_rate> })` で事前確保（`buffer.rs` L214 の実 signature — `(channels, frames)` 引数形ではない）し、`Err`（`InvalidValue`）を `Err(Prepare(UnsupportedConfig))` へ写像
- **再 Activate bound gate（L0 側 — L1 `rt.is_none()` gate の mirror）**: `Shared` slot で `attached` epoch ≠ 0（endpoint attach 生存の authoritative 判定 — `bound` field は比較用の簿記値のみで liveness 判定には使わない）かつ `Activate` の `new_config.max_frames > bound.max_frames`（attach 時に `SharedSlot::bound` へ保持した簿記値 — 現行 activated config ではない）なら `Err(HostError::Prepare(UnsupportedConfig))` で拒否 — transition commit・`activated` 更新・mirror store より前に拒否し `SharedSlot`/`SendCore` は不変のまま（scratch 契約の維持 — `in_scratch` は attach 時 bound で事前確保済みのため、それを上回る `max_frames` の再 Activate は受け付けない）。bound 内の再 Activate は合法 — `activated` は同一 lock 経由で新値へ更新され RT 可視になる（design §再 Activate — L1 側は `rt.is_none()` authoritative + `endpoint_bound` 簿記で同規則）
- **dep 配線**: `ruviform-plugin-host` の `ruviform-audio-core` を dev-dep → **正式 dep** へ昇格（`endpoint.rs` が `AudioBuffer`/`AudioBufferViewMut` を名指す — Domain foundations で A.2 許可範囲）。`Cargo.lock` 差分を記録
- **`lib.rs` re-export 骨格**: `PluginEndpoint`/`EndpointBounds` を pub export（crate doc の完成は T4）
- **単体テスト**: Shared slot dispatch（`create_sendable` → `Shared`・default → `Plain`）・全 verb 到達（attach 中も `SaveState`/`RestoreState`/`StopProcessing`/`Deactivate`/`OpenGui`/`CloseGui` が `Ok`）・epoch 二重 attach 拒否 + 自分の epoch のみ返却 + 再 attach で新 epoch・`Failed` 収束（panic fixture → `fetch_max(Failed)` → 以後 verb/`process` が `Err(Failed)`/`Ok(CONTINUE)`+無音）・poisoned lock → `destroy` のみ `Ok` teardown・busy bypass（別スレッド guard 保持で `try_lock` busy → `Err(EndpointUnavailable)`）・`plugin_mut`→`None`・`lifecycle_state(&self)` が mirror のみ読む（`core.lock()` 不取得 — `Failed` 観測 union）

## Classification

- `normal-plan-task`（L0 endpoint 機構の中核 — design の決定サマリ e–h 実装）

## Upstream Documents

- `docs/plans/implement-issue-113-engine-plugin-integration.md`（actions 4–7・Files/Ownership）
- `docs/designs/engine-plugin-host-integration.md`（L0 shared セル・3 段 lock 形態・commit 順序・`destroy` 例外・epoch CAS・`catch_unwind`+`fetch_max`・`PluginHost` のみ公開面の確定）
- `docs/specs/engine-plugin-host-integration.md`（AC2・AC4・AC5 — Send 証拠・bounds/error 面）
- `docs/adr/plugin-rt-ownership-split.md`（L0 opt-in `Send` + shared cell の確定分岐）
- `docs/designs/plugin-host-l0.md`（Issue #74 上流 — `Instance`/`InProcessHost` の既存構造）

## Dependencies

- `docs/plans/implement-issue-113-engine-plugin-integration.md`
- `docs/tasks/plugin-core-endpoint-primitives.md`（`EndpointUnavailable`・`create_sendable`/`erase_sendable`・view impl が前提）

## Affected Files

- `crates/ruviform-plugin-host/src/host.rs` — `Instance` スロット enum・`SharedSlot`/`SendCore`・`Instantiate` 分岐・全 verb 3 段 lock・commit 順序・`destroy` `into_inner` 例外・`lifecycle_state`/`plugin_mut`/`process` の `Shared` 対応・`pub(crate) attach_endpoint` 内側
- `crates/ruviform-plugin-host/src/endpoint.rs`（新規）— `PluginEndpoint`/`EndpointBounds`/`EndpointInner`/`L0Endpoint` + epoch attach/`Drop` + L0 inner `process` 骨格
- `crates/ruviform-plugin-host/src/lib.rs` — `endpoint` モジュール宣言 + `PluginEndpoint`/`EndpointBounds` re-export
- `crates/ruviform-plugin-host/src/l1/data.rs` — **変更なし（`mirror_encode`/`mirror_decode` は既存位置に維持）** — L0 側は `crate::l1::data::` 経路で利用（`src/mirror.rs` は新設しない — `crate::` 直下モジュールは l1 から到達不能・`l1/mod.rs` L19-26 の import discipline）
- `crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs` — **変更なし（制約 witness）** — `src/l1/` 木の crate-free discipline が l1-proxy E2E の `#[path]` include を維持する前提として記録（T3 が l1 木を編集する際の制約でもある）
- `crates/ruviform-plugin-host/Cargo.toml` — `ruviform-audio-core` dev-dep → 正式 dep
- `Cargo.lock` — dep 昇格差分
- `crates/ruviform-plugin-host` 内 inline `#[cfg(test)] mod tests` — 新規単体テスト

## Done When

- [x] `Instance` が `Plain`/`Shared` スロットを持ち、`create_sendable` 有無で dispatch される — 非 Send factory の L0 利用（`plugin_mut`/`process`）に退行なし
- [x] `Shared` slot の全 verb（lifecycle + `SaveState`/`RestoreState`/`OpenGui`/`CloseGui`）が 3 段 lock 形態（entry pre-check → `match lock` → lock 内再読）で、transition commit → `set_state` mirror store → plugin hook が同一クリティカルセクション内にある
- [x] `mirror ∈ {Activated, Processing} ⟹ activated.is_some()` 不変条件が構造的に保持される（`Failed`/`Destroyed` ≥ `Activated` の u8 順 — `≥` 比較禁止・列挙で書く）
- [x] `destroy` のみ `into_inner` で teardown する（hook 不呼出）・`&self` 照会（`lifecycle_state`）が mirror のみ読み `core.lock()` を取らない・`plugin_mut` が `Shared` で `None`・host `process` が busy で `Err(EndpointUnavailable)`
- [x] `src/endpoint.rs` に `PluginEndpoint`/`EndpointBounds`/`EndpointInner`/`L0Endpoint` が存在し、epoch CAS attach（`next_epoch` 単調増・`attached` 非 0 → `Err(EndpointUnavailable)`）と `Drop` での自分の epoch のみ返却（`compare_exchange(epoch, 0)`）が実装される
- [x] L0 inner `process` が mirror fast-skip → `try_lock` → 再読 → `activated` gate → `catch_unwind` の経路で、panic/poisoned/`Failed` を `fetch_max(Failed)` + `Ok(CONTINUE)` + 無音へ収束させる
- [x] `attach_endpoint` が拒否順序を守る（state 系 `InvalidHandle`/`Failed`/`Lifecycle` → slot-shape `EndpointUnavailable` → activated 層 bounds 検査 → epoch CAS → scratch 確保）— L0 activated 層の bounds 検査（非 0・`≥ SendCore.activated.max_frames`・`≤ WIRE_MAX_*`）+ `AudioBuffer::try_new(BufferSpec{..})` `Err` → `Prepare(UnsupportedConfig)` 写像が実装される（uniform prelude は T4 所有・本層は activated 層の再検査）
- [x] 再 Activate bound gate — `Shared` slot で `attached` epoch ≠ 0 かつ `new_config.max_frames > bound.max_frames`（`SharedSlot::bound` 簿記値との比較・`attached` が authoritative 判定）の `Activate` が `Err(Prepare(UnsupportedConfig))` で拒否される（commit・`activated` 更新より前 — L1 `rt.is_none()` gate の mirror）・bound 内の再 Activate は合法で `activated` が新値へ更新される
- [x] `mirror_encode`/`mirror_decode` が `l1::data` に維持され `crate::l1::data::` 経路で L0 側から利用される（`src/mirror.rs` 新設なし・l1 crate-free discipline 維持）
- [x] narrow gate（下記 Verification）が exit 0 — L0 単体テスト一式が通る

## Verification

- [x] `mise run check` — exit 0
- [x] `mise run lint` — exit 0
- [x] `mise run test` — exit 0（既存 L0/host テスト無退行 + 新規 L0 単体テスト）
- [x] `mise exec -- cargo test -p ruviform-plugin-host` — L0 単体一式（Shared dispatch・verb 到達・epoch・`Failed` 収束・busy bypass・`plugin_mut` `None`・poisoned destroy）
- [x] 対象 test-spec: `docs/test-specs/plugin-endpoint-contract-tests.md` — 規則「L0 Shared セルの verb 到達」「verb 3 段 lock 形態」「`&self` 照会は観測 union のみ」「L0 endpoint `process` 経路」「単一 endpoint 規則（L0 側）」を担保

補足（task doc の Verification 範囲外・IR に記録）: `mise run fmt-check`/`mise run build` exit 0・`mise run test-rt` exit 0（新規 `rt_` 実一致 = `rt_endpoint_process_no_alloc`・`rt_endpoint_busy_bypass_no_alloc` の 2 件 — plugin-host lib 内の既存 `rt_process_no_alloc` + 文字列フィルタの副次一致 `shared_start_failure_rolls_back_to_activated` を含め lib 4 件、`l1_proxy_e2e` 側 `rt_l1_*` 2 件も pass）。レビュー（A=APPROVE・B=全攻撃面クリア）の確定修正（poison 起源 comment・`#[cfg_attr(not(test), expect(dead_code))]` tripwire・attach 順序 doc + T4 watch line・verb starvation doc・epoch skip comment）適用済み — 詳細は `docs/impl/ir/issue-113-l0-shared-slot-and-endpoint-inner-task-2-implementation.md`。
