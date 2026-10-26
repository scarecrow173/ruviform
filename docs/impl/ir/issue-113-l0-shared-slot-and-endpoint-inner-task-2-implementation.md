---
id: "IMPL-034PFxBChBNCubLGV6GoGB"
type: "impl"
status: "completed"
title: "Issue #113 L0 shared slot + endpoint inner — Task 2 implementation"
created: "2026-10-26"
updated: "2026-10-26"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-113-l0-shared-slot-and-endpoint-inner-task-2-implementation.md"
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
      - type: "file"
        path: "docs/tasks/l0-shared-slot-and-endpoint-inner.md"
      - type: "file"
        path: "docs/tasks/plugin-host-endpoint-surface.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/l0-shared-slot-and-endpoint-inner.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #113 L0 shared slot + endpoint inner — Task 2 implementation

## Summary

Issue #113 Task 2 完了 — plan actions 4–7（`ruviform-plugin-host`
の L0 endpoint 機構）を全範囲実施。(a) `Instance` の
`Plain`/`Shared` スロット enum 化 + `SharedSlot`/`SendCore` 共有
セル（`Arc<Mutex<SendCore>>` + `AtomicU8` mirror + `AtomicU64`
attach epoch + host 側 bookkeeping）、(b) `Instantiate` の
`create_sendable()` dispatch（`Some` → `Shared`・`None` → `Plain`
— 非 Send factory の L0 経路不変）、(c) `shared_verb` ヘルパに
よる全 verb の 3 段 lock 形態（entry pre-check → `lock()`/poison
収束 → in-CS mirror 再読 — commit→`set_state`→hook が同一 CS）、
(d) `src/endpoint.rs` 新設（`PluginEndpoint`/`EndpointBounds`/
`EndpointInner`/`L0Endpoint`）+ epoch CAS attach/`Drop` 返却 +
L0 inner `process`（mirror fast-skip → `try_lock` → in-lock
再読 → activated gate → `catch_unwind` → panic/`Failed` は
`fetch_max`/`store` + 無音 + `Ok(CONTINUE)`、busy は
`Err(EndpointUnavailable)`）+ `attach_endpoint` の pin 済み拒否
順序 + 再 Activate bound gate。単体テスト 19 件。独立レビュー
2 件: A=APPROVE（低優先候補 3 件）・B=adversarial 全攻撃面クリア
— 確定修正を全て適用。全 gate green 実 log。

## Context

approved plan `docs/plans/implement-issue-113-engine-plugin-integration.md`
の Task 2（`docs/tasks/l0-shared-slot-and-endpoint-inner.md`・
actions 4–7）。upstream: approved design
`docs/designs/engine-plugin-host-integration.md`（決定サマリ
e–h — L0 shared セル・3 段 lock 形態・commit 順序・`destroy`
例外・epoch CAS・`catch_unwind`+`fetch_max`・`PluginHost` のみ
公開面）・spec `docs/specs/engine-plugin-host-integration.md`
（AC2・AC4・AC5）・ADR `docs/adr/plugin-rt-ownership-split.md`
（L0 opt-in `Send` + shared cell）・TSpec
`docs/test-specs/plugin-endpoint-contract-tests.md`（proposed —
「L0 Shared セルの verb 到達」「verb 3 段 lock 形態」「`&self`
照会は観測 union のみ」「L0 endpoint `process` 経路」「単一
endpoint 規則（L0 側）」）。前提は Task 1 が供給（
`EndpointUnavailable` local-only variant・
`create_sendable`/`erase_sendable`・`AudioBufferView*` の
`BufferView*` impl）。後続: Task 3（`EndpointInner::L1`
variant 充填 — 本 task の enum 骨格を消費）・Task 4（
`PluginHost::prepare_endpoint` 公開面 — `attach_endpoint` の
caller を配線）・Task 6（`process_audio` shell — `in_scratch`
を消費）。

## Implementation

### Changes

- `crates/ruviform-plugin-host/src/host.rs`（+1884/-91 行）:
  - `enum InstanceSlot { Plain { plugin, context },
    Shared(SharedSlot) }` — `Instantiate` で
    `factory.create_sendable()` が `Some` を返せば `Shared`
    （`SendCore { plugin: Box<dyn ErasedPlugin + Send>, context,
    activated: Option<ProcessConfig> }` を `Arc<Mutex>` 化・
    mirror=`Created`・`attached=0`・`bound=None`・
    `next_epoch=1`）、`None` なら従来 `Plain`（`factory.create()`
    — 非 Send factory の L0 利用に退行なし）。`SharedSlot` は
    `l1::Shared`（L1 プロキシ状態）との衝突を避ける命名。
  - `shared_verb(slot, lifecycle, body)` — `Shared` slot で
    plugin/context に触れる全 verb（`initialize`/`activate`/
    `start_processing`/`stop_processing`/`deactivate`/
    `save_state`/`restore_state`/`open_gui`/`close_gui`）の
    **3 段 lock 形態**: (1) entry pre-check — mirror `Failed`
    なら lock せず `converge_failed`（bookkeeping lifecycle に
    `Failed` を冪等 commit + `Err(HostError::Failed)`）、
    (2) `slot.core.lock()` — `Err(PoisonError)`（verb-CS hook
    または host 側 `process` Shared arm を unwind した plugin
    panic が起源 — `catch_unwind` される endpoint 経路は poison
    しない旨を comment で pin）は同じ収束、(3) in-CS mirror
    再読 — `Failed` なら hook/transition/`set_state` を一切
    実行せず guard 解放して収束。`body` は CS 内で合法性検査 →
    transition commit → `set_state` mirror store（`Release`）→
    plugin hook — `try_lock` 観測は常に完全 commit 済みのみ
    （half-committed mirror は構造上存在しない）。blocking
    `lock()` の starvation note（endpoint `process` 中の verb
    は待機 — wedged plugin は stream を既に壊しているので許容）
    を doc に明記（レビュー修正 — 後述）。
  - `destroy` の `into_inner` 例外 — `Shared` は mirror `Failed`
    観測または `core.is_poisoned()` で bookkeeping を
    `Failed → Destroyed` へ先に収束し、`lock()` のみ
    `unwrap_or_else(PoisonError::into_inner)` で teardown
    （panic 済み plugin の destroy hook は呼ばない・
    `activated=None` + mirror=`Destroyed` store + core 内容物
    は slot と共に drop）。
  - `converge_failed(&mut Lifecycle)` — `Failed` 観測の集約点:
    bookkeeping lifecycle に `Failed` を冪等 commit し
    `Err(HostError::Failed)`。mirror 自体は書かない —
    host 側 mirror store は CS 内（`set_state`）のみで、
    out-of-CS writer は endpoint の `fetch_max(Failed)` 単独
    （単調 — 後続 `Destroyed` store を regress しない）。
  - `&self` 照会は観測 union のみ — `lifecycle_state(&self)`:
    `Plain` は `instance.lifecycle.state()`、`Shared` は
    `slot.mirror.load(Acquire)` のみ（`core.lock()` 不許可・
    `lifecycle.state()` 非更新 — commit は `&mut` entry point
    で行う）。`plugin_mut`: `Shared` → `None`（`Mutex` 越しに
    `&mut` を貸せない — L1 と同じ defined 挙動）。
  - host 側 `process` の `Shared` arm — mirror pre-check →
    `core.try_lock()`（`WouldBlock` → `Err(EndpointUnavailable)`
    = 一時 bypass・`Poisoned` → `Failed` 収束）→ in-lock 再読
    → `Process` transition commit → `set_state` →
    `process_host`。**`catch_unwind` なしは設計通り** —
    直接呼出し経路は panic を伝播させ（`Plain` と同じ）、
    `Shared` では guard 越しの unwind が cell を poison する
    （次観測が `Failed` へ収束 — panic 済み plugin を再 enter
    しない asymmetric 収束を doc で pin。panic isolation が
    要る caller は endpoint 経路を使う）。
  - `attach_endpoint(&mut self, handle, bounds) ->
    Result<PluginEndpoint, HostError>`（`pub(crate)` —
    `PluginHost::prepare_endpoint` の L0 内側。pin 済み拒否
    順序: **(1) state check**（`Shared` は mirror=観測 union・
    `Plain` は bookkeeping — `Destroyed`/stale は
    `Err(InvalidHandle)`、`Failed` は `Err(Failed)` 収束、
    `Activated`/`Processing` 未満は `Err(Lifecycle)`）→
    **(2) slot-shape**（`Plain` → `Err(EndpointUnavailable)`
    — 非 Send factory は endpoint 不可）→ **(3) activated 層
    bounds 検査**（`core.lock()` 下で `SendCore.activated` を
    読み、非 0・`≤ WIRE_MAX_FRAMES`(8192)/`≤ WIRE_MAX_CHANNELS`(16)
    ・`≥ activated.max_frames` — 違反は
    `Err(Prepare(UnsupportedConfig))`。T4 が所有する uniform
    prelude と同規則の defense-in-depth 再検査 — 矛盾する
    結果は出ない）→ **(4) epoch CAS attach**（
    `attached.compare_exchange(0, epoch, AcqRel, Acquire)` —
    非 0 なら `Err(EndpointUnavailable)` = 単一 endpoint 規則。
    `next_epoch` は採番後 `+=1` — post-claim 失敗が claim を
    release しても bump は残るため値を skip し得る旨 comment
    で pin）→ **(5) `in_scratch` 事前確保**（`bound_layout` で
    `channels` を `ChannelLayout` へ写像 — named variant または
    `custom` mask — し `AudioBuffer::try_new(BufferSpec{layout,
    frames: bound.max_frames, sample_rate: activated.sample_rate()})`
    で確保。失敗は claim 済み epoch を `attached.store(0,
    Release)` で release して `Err(Prepare(UnsupportedConfig))`
    — 「a rejected attach consumes nothing」）。
  - **attach 順序の inner/prelude 非対称 doc**（レビュー修正 —
    後述）: 内側は bounds→CAS の順（bounds 拒否は `attached`
    を触らない = release 不要）。これは T4 public prelude の
    順序（state → presence/slot → bounds — 占有中に不正 bounds
    を渡した公開 Err は `EndpointUnavailable`）と**意図的に
    逆**である旨を `attach_endpoint` doc に明記し、同じ
    watch note を `docs/tasks/plugin-host-endpoint-surface.md`
    へ 補足 line として転記（T4 実装時の order pin）。
  - **再 Activate bound gate** — `Activate` の Shared arm が
    `attached != 0`（authoritative liveness — `bound` field は
    比較用の簿記値のみ）かつ `config.max_frames() >
    bound.max_frames`（attach 時簿記値）なら
    `Err(Prepare(UnsupportedConfig))` を transition commit・
    `activated` 更新より前に返す（`in_scratch` は attach 時
    bound で確保済みのため scratch 契約維持。bound 内の再
    Activate は合法 — `activated` は同一 lock 経由で新値へ
    更新され RT 可視になる。L1 側 `rt.is_none()` gate の
    mirror）。
  - `activated` 不変条件の担い手 — `Activate` commit で
    `core.activated = Some(*config)`、`Deactivate`/`Destroy`
    で `None`、全て CS 内 — `mirror ∈ {Activated, Processing}
    ⟹ activated.is_some()`。u8 順
    `Created0/Initialized1/Activated2/Processing3/Failed4/
    Destroyed5` では `Failed`/`Destroyed` ≥ `Activated` のため
    `≥` 比較を不変条件に使わず列挙で書く規則を遵守（
    `is_failed()`/`accepts_processing()` 経由）。
  - `bound_layout(channels) -> Result<ChannelLayout, HostError>`
    — `in_scratch` 用の `ChannelLayout` 写像（1/2/4/6/8 →
    named、それ以外 → `SpeakerMask` custom mask・`Err` は
    `Prepare(UnsupportedConfig)` 写像 — `channels ≤
    WIRE_MAX_CHANNELS` は bounds 検査済みの後置き）。
  - `#[cfg(test)] mod tests` — 18 件（fixture: `SendGain` +
    `SendGainFactory`（`create_sendable` opt-in）・`PlainGain`
    + `PlainGainFactory`（default `None` — 正直な非 Send
    fixture）+ `SliceIn`/`SliceOut` view plumbing + `run_endpoint`
    /`run_host` helper）: `shared_cell_types_are_send`（
    `expect_send::<SendCore>`/`expect_send::<SharedSlot>`）・
    `instantiate_selects_slot_by_create_sendable`・
    `plain_slot_keeps_direct_path`・`shared_plugin_mut_is_none`・
    `shared_verbs_reach_plugin_while_attached`（attach 中の全
    verb 到達 — `SaveState`/`RestoreState`/`StopProcessing`/
    `Deactivate`/`OpenGui`/`CloseGui` が `Ok`）・
    `shared_prepare_failure_keeps_initialized`・
    `shared_start_failure_rolls_back_to_activated`・
    `attach_rejects_in_pinned_order`（拒否順序の網羅表 — state
    系 → slot-shape → bounds 各種 → 2 本目）・
    `attach_epoch_drop_and_stale_endpoint`（drop で自分の epoch
    のみ返却・再 attach で新 epoch）・
    `endpoint_process_applies_gain_and_gates_blocks`（gain 正当性
    + 非 `Processing`/`Failed`/bad dims の defined 拒否）・
    `endpoint_panic_converges_failed_silences_output`・
    `poisoned_verb_converges_and_destroy_tears_down`・
    `busy_cell_bypasses_endpoint_and_host_process`（別スレッド
    guard 保持で `try_lock` busy → 両経路
    `Err(EndpointUnavailable)`）・
    `lifecycle_state_reads_mirror_while_core_held`（lock 保持中
    も `&self` 照会は mirror のみ読む）・`reactivate_bound_gate`・
    `endpoint_outlives_instance_teardown`・
    `rt_endpoint_process_no_alloc`・`rt_endpoint_busy_bypass_no_alloc`
    （`assert_no_alloc` 窓内 — warm-up は窓外: 初回
    `catch_unwind` が per-thread panic state を登録するため）。
- `crates/ruviform-plugin-host/src/endpoint.rs`（新規・401 行）:
  - `pub struct EndpointBounds { pub max_frames: u32, pub channels:
    u32 }` — `Copy` 値型。`bound` は scratch ceiling であり、
    per-block gate は activated `ProcessConfig`（`min_frames ≤
    frames ≤ max_frames`）+ `channels == bound.channels` —
    2 者の役割を doc で分離。`channels` は v1 では activated
    main bus との等価を要求しない（plugin に channel query が
    ない — Issue #114 contract work）旨を pin。
  - `pub struct PluginEndpoint { inner: EndpointInner, bound:
    EndpointBounds, in_scratch: AudioBuffer<f32>,
    latency_frames: u32 }` — `Send`、RT-side owner へ 1 度だけ
    move される endpoint。`bounds()`/`latency_frames()`（const・
    RT-suitable）+ `process`（`PluginHost::process` と同じ語彙 —
    erased bus views + `EventList` + `Option<TransportInfo>`）を
    公開。`in_scratch` は `process_audio` shell（T6）が消費する
    bound-shaped 事前確保 slab — `#[expect(dead_code)]` で未消費
    を pin。module doc に per-block 出力表（`Failed`/poison/panic
    → 無音 + `Ok(CONTINUE)`・非 `Processing` →
    `Err(Lifecycle)`・activated/bound 違反 → `Err(Prepare)`・
    busy → `Err(EndpointUnavailable)`・全 `Err` は outputs
    不変更）を定義。
  - `enum EndpointInner { L0(L0Endpoint) }` — variant dispatch
    形（`process(bound, in, out, events, transport)`）をここで
    定義。`L1` variant は T3 で充填（`l1::data::EndpointLegParts`
    から `endpoint.rs` 側で組立て — `src/l1/` 木の crate-free
    discipline を維持）。
  - `struct L0Endpoint { core: Arc<Mutex<SendCore>>, mirror:
    Arc<AtomicU8>, attached: Arc<AtomicU64>, epoch: u64 }` —
    全 field が `Send`/`Sync` handle（`Mutex<SendCore>` は
    `SendCore: Send` なら `Sync` — `unsafe impl Send` 不要の
    構造的証拠）。
  - `L0Endpoint::process` — pin 済み pipeline: (1) mirror
    Acquire **fast-skip**（`Failed` → `converge_failed`・非
    `Processing` → `Err(Lifecycle)` — pre-lock 読みは最適化で
    あり half-committed 観測は構造上起きないが stale には
    なり得るため gate 判定は in-lock 再読が authoritative）、
    (2) `core.try_lock()` — `WouldBlock` →
    `Err(EndpointUnavailable)`（non-RT verb mid-flight の一時
    bypass・`outputs` 不変更）、`Poisoned` → `Failed` 収束
    （poison 起源は verb-CS hook / host `process` arm —
    endpoint 自身の `catch_unwind` 済み call は poison しない
    旨 comment で pin）、(3) in-lock mirror Acquire 再読、
    (4) activated gate — `SendCore.activated` が lock 内の
    source of truth（`mirror==Processing ⟹ activated.is_some()`
    は commit-order 不変条件 — `None` は
    `debug_assert`+defined 拒否）、`block_dims`（in-bus-first:
    `frames` は first input bus else first output bus・
    `channels` は `min(in,out)` else whichever）が
    `min_frames ≤ frames ≤ max_frames` かつ `channels ==
    bound.channels` でなければ
    `Err(Prepare(UnsupportedConfig))`、(5) `process_host` を
    `catch_unwind(AssertUnwindSafe)` で同一 CS 内駆動 — panic
    捕捉時は `mirror.store(Failed, Release)`（CS 内なので
    store で安全）+ `silence(outputs)` + `Ok(CONTINUE)`、
    lock 外観測の収束は `fetch_max(Failed, Release)`（単調 —
    後続 `Destroyed` store を regress しない）+ 無音 +
    `Ok(CONTINUE)`（`converge_failed` — endpoint は mirror の
    第 2 writer）。
  - `impl Drop for L0Endpoint` — `attached.compare_exchange(
    epoch, 0, AcqRel, Acquire)` で**自分の epoch のみ**返却
    （stale drop — e.g. collector スレッド上の遅延退役 — が
    後続 attach の新 epoch を掴み消さない）。
  - `process_rejected`（非 `Processing` → `LifecycleError::
    IllegalTransition{from, event: Process}` — host `process`
    と同形）・`block_dims`（in-bus-first 規則 — T6 shell が
    frames/channels を供給するまでの endpoint 内規則）・
    `silence` helper。
  - `#[cfg(test)]` — `endpoint_types_are_send`（`expect_send` で
    `SendCore`/`L0Endpoint`/`EndpointInner`/`PluginEndpoint`）+
    `assert_not_impl_any!` 負側 pin（`Rc`: `!Send`/`!Sync`・
    `RefCell`: `!Sync` — `T: Send` なら `RefCell<T>` は `Send`
    のため `!Send` ではなく **`!Sync`** 側で pin（deviation —
    後述）・`Box<dyn ErasedPlugin>`: `!Send`・`L1Instance`:
    `!Send`・`PluginHost`: `!Send` — endpoint が capture しては
    いけない型を compile-time で拒否）。
  - `#[cfg_attr(not(test), expect(dead_code))]` tripwire —
    `PluginEndpoint::new`・`EndpointInner`（`L0` variant 構築）
    は in-crate caller が T4（`prepare_endpoint`）/T3（`L1`
    variant）まで存在せず test が唯一の consumer — bare
    `#[expect]` は test で exercise される現時点では unfulfilled
    expectation で fail するため **`not(test)` gate 付き**で
    pin（T3/T4 が真の caller を配線した時点で非 test build が
    `dead_code` lint を再燃させる自己消去 tripwire — deviation
    詳細は後述）。
- `crates/ruviform-plugin-host/src/lib.rs`（+2 行）: `mod
  endpoint;` + `pub use endpoint::{EndpointBounds,
  PluginEndpoint};`（crate doc の完成は T4 — re-export 骨格のみ）。
- `crates/ruviform-plugin-host/Cargo.toml`: `ruviform-audio-core`
  を dev-dep → **正式 dep** へ昇格（`endpoint.rs` が
  `AudioBuffer`/`AudioBufferViewMut` を名指す — Domain
  foundations で A.2 許可範囲・既に plugin-core 経由の推移 dep
  だったため dependency tree 不変の旨 comment で記録）+
  `static_assertions = "1.1"` を dev-dep 追加（Send 証拠の負側
  — `assert_not_impl_any!` が `expect_send` 型関数で表現不能な
  `!Send`/`!Sync` を pin — test-spec §Send 証拠の要求）。
- `Cargo.lock` — `static_assertions 1.1.0` entry 追加 +
  `ruviform-plugin-host`/`ruviform-audio` の dependencies 反映。
  **本 commit は Task 5 分の `static_assertions` dev-dep entry
  （`ruviform-audio` 側）も同時に運ぶ** — lockfile は単一 file
  で分割不可能なため、T2 commit に全量含めた旨を deviation と
  して記録（T5 IR 側にも同 note）。
- `docs/tasks/l0-shared-slot-and-endpoint-inner.md` — status
  `done`・`implemented-by` 本 IR・changes 記録・checkbox 全 tick
  + 補足 line。
- `docs/tasks/plugin-host-endpoint-surface.md` — 補足 line 追加
  （T4 watch note: public prelude は presence/占有 → bounds の
  順 — inner の bounds→CAS 順とは別層の契約、占有中に不正
  bounds を渡した公開 Err は `EndpointUnavailable`）。
- `docs/tasks/README.md`・`docs/impl/ir/README.md` — status/索引
  同期。

### Review findings and resolutions（独立レビュー 2 件 — A=APPROVE・B=adversarial 全攻撃面クリア — 確定修正を全て適用）

- **poison 起源の明記（fixed — A low candidate / B 指摘）**:
  `SharedSlot::core`・`shared_verb`・`destroy`・host `process`
  Shared arm・`L0Endpoint::process` の `Poisoned` arm 各所で、
  poison の起源が「verb-CS hook または host 側 `process` arm を
  unwind した plugin panic」である旨を comment で pin —
  endpoint の `catch_unwind` 済み経路は poison しない（非対称の
  根拠を読者が追える形に）。
- **`#[cfg_attr(not(test), expect(dead_code))]` tripwire
  （fixed — B 提案採用・deviation 併記）**: 初版は T4 未配線の
  `attach_endpoint`/`next_epoch`/`bound_layout` と T3 未充填の
  `EndpointInner`/`PluginEndpoint::new`/`in_scratch` を bare
  `#[expect(dead_code)]` で pin する設計だったが、**unit test が
  これらを exercise する現時点で bare `#[expect]` は
  unfulfilled expectation で lint gate fail** する — semantics
  （T3/T4 の配線時に自動で再燃する自己消去 tripwire）を保った
  まま `not(test)` gate 付きへ修正。`in_scratch` のみ真に誰も
  読まないため bare `#[expect]` のまま。
- **attach 順序の inner/prelude 非対称 doc + T4 watch line
  （fixed — A 指摘）**: inner の bounds→CAS 順と T4 public
  prelude の state→presence→bounds 順が逆である旨を
  `attach_endpoint` doc の Order note に明記し、同趣旨の補足
  line を `docs/tasks/plugin-host-endpoint-surface.md` へ追加
  — T4 実装時に順序 pin を取り違えないための証跡。
- **verb starvation note（fixed — B 指摘）**: `shared_verb` の
  blocking `lock()` は in-flight endpoint `process` の後に
  queue するため wedged plugin は verb を starvation させる —
  「wedged plugin は stream を既に壊している」ので許容する
  設計判断を doc に明記。
- **epoch skip comment（fixed — A low candidate）**: post-claim
  失敗（`in_scratch` 確保 `Err`）が claim を release しても
  `next_epoch` の bump は残るため epoch 値は skip し得る — 値は
  opaque で gap は無害である旨 comment で pin。

### Decisions During Implementation

- **`attach_endpoint`/`EndpointInner`/`PluginEndpoint::new`/
  `next_epoch`/`bound_layout` は `#[cfg_attr(not(test),
  expect(dead_code))]`**（deviation・deliberate — 上記）:
  task doc は「未使用 API を `#[expect(dead_code)]` で pin」と
  するが、bare `#[expect]` は test 経路が item を使う現時点で
  unfulfilled expectation として `-D warnings` を fail させる。
  `not(test)` gate で「非 test build でのみ dead_code を
  expect」する形にし、T3/T4 配線後に自動的に lint が再燃する
  tripwire semantics を維持した。
- **`RefCell` の負側 pin は `!Sync`**（deviation・deliberate —
  test-spec は `Rc`/`RefCell` を `!Send` 証拠の例に挙げるが、
  `RefCell<T>` は `T: Send` なら `Send` — 真の拒否特性は
  `!Sync`）: `assert_not_impl_any!(RefCell<()>: Sync)` で pin
  し、comment で理由を記録。`Rc`（`!Send` かつ `!Sync`）が
  `!Send` 側の canonical witness を兼ねる。
- **`InProcessHost::process` の panic 伝播は設計通り維持**
  （documented・design-intentional）: 直接呼出し経路は
  `catch_unwind` を持たず panic を caller へ伝播（`Plain` と
  同じ）。`Shared` arm では guard 越し unwind が cell を
  poison し、次観測が `Failed` 収束する asymmetric 収束 —
  panic isolation が要る RT caller は endpoint 経路を使う
  旨を `process` doc に明記（endpoint 側のみ `catch_unwind`
  — 「a panicking plugin must not be re-entered」）。
- **panic 収束は CS 内 `store`・lock 外観測は `fetch_max`**:
  `catch_unwind` で捕捉した panic は core guard 保持中（CS 内）
  に `mirror.store(Failed, Release)` — store で安全。lock 外
  観測（fast-skip・`Poisoned`）は `fetch_max(Failed, Release)`
  で単調化し、host 側の後続 `Destroyed` store を regress し
  ない — writer 種別で store/fetch_max を使い分ける。
- **post-claim 失敗の release は `store(0)`**: `in_scratch`
  確保失敗時の epoch release は `attached.store(0, Release)`
  — claim は同じ `&mut self` 排他下で直前に行ったため CAS
  不要（endpoint 側 writer は attach 済み epoch のみ触れる
  構造上、この store は自分の claim だけを消す）。
- **`attach_endpoint` は `&mut self` + blocking `lock()` を使う
  non-RT 経路**: RT contract として「non-RT only — allocates
  and takes `core.lock()`」を doc で pin（caller 側 prepare
  手順 — endpoint が生存する前の配線なので blocking でよい）。
- **`bound` field は簿記値・`attached` が authoritative**:
  `SharedSlot::bound: Option<EndpointBounds>` は最新 attach の
  bounds 記録（再 Activate gate の比較対象）で、endpoint
  生存の判定には `attached != 0` を使う（design「`rt.is_none()`
  が authoritative — 簿記値は liveness に使わない」の L0
  mirror）。

## Related Experiments

### Adopted

なし — approved design/plan の確定実装のみ（実験ログなし）。

### Rejected

なし — 代替実装の評価は design の確定事項をそのまま実装した
（named wrapper・`&mut` lend-out 等の却下分岐は全て design 時
決定）。

## Validation

- gate（実 log・全 exit 0）:
  - `mise run fmt-check` — exit 0
  - `mise run check` — exit 0（`cargo check --workspace
    --all-targets`）
  - `mise run lint` — exit 0（`cargo clippy --workspace
    --all-targets -- -D warnings` clean — `#[cfg_attr(not(test),
    expect(dead_code))]` の expectation が非 test build で
    fulfilled である実証）
  - `mise run test` — exit 0（`cargo test --workspace` —
    **55 test binaries 全 pass**）
  - `mise run test-rt` — exit 0（新規 `rt_` =
    `rt_endpoint_process_no_alloc`・`rt_endpoint_busy_bypass_no_alloc`
    の 2 件 — plugin-host lib 内では既存 `rt_process_no_alloc`
    と文字列フィルタの副次一致 `shared_start_failure_rolls_back_
    to_activated`（`start_f` 内に `rt_` を含む）を含め 4 件
    pass、`l1_proxy_e2e` 側 `rt_l1_process_path_no_alloc`/
    `rt_l1_result_leg_try_ops` も pass — 0 件バイナリなし）
  - `mise run build` — exit 0（`cargo build --workspace`）
  - `mise exec -- cargo test -p ruviform-plugin-host` — exit 0
    （lib **30** + `l1-test-worker` 0 + `l1_proxy_e2e` **42** +
    doctest **1** — 新規 19 件（host.rs 18 + endpoint.rs 1）
    を含む全 pass）
- 構造確認（実コマンド）:
  - `rg -n "fn rt_" crates/ruviform-plugin-host/src` —
    `rt_endpoint_process_no_alloc`/`rt_endpoint_busy_bypass_no_alloc`
    の実在を確認（`mise run test-rt` の実一致件数と整合）
  - `rg -n "cfg_attr\(not\(test\)" crates/ruviform-plugin-host/src`
    — tripwire の pin 箇所（`next_epoch`・`attach_endpoint`・
    `bound_layout`・`PluginEndpoint::new`・`EndpointInner`）実在

## Non-Goals

- `PluginHost::prepare_endpoint`/`latency_frames` 公開面 — Task
  4 の scope。`attach_endpoint` は `pub(crate)` のままで、唯一の
  in-crate caller が T4 で配線されるまで unit test が sole
  consumer（`not(test)` tripwire で pin）。standalone
  `InProcessHost` に公開 endpoint API は作らない（design 確定 —
  v1 の公開面は `PluginHost` コンポジットのみ）。
- `EndpointInner::L1` variant・`EndpointLegParts`・arm/返却
  channel — Task 3 の scope。本 task は enum 骨格 + `L0`
  variant のみ（dispatch 形の定義まで）。
- `process_audio` shell（`in_scratch` 消費・bound clip・
  source-position 慣行）— Task 6 の scope。`in_scratch` は
  事前確保のみ（`#[expect(dead_code)]` pin）。
- `PluginEndpoint` の crate doc 完成・`lib.rs` L44-49 付近の
  stale 遅延式 doc 修正 — Task 4 の scope（re-export 骨格のみ
  本 task で供給）。
- test-spec `plugin-endpoint-contract-tests.md` の `proposed →
  confirmed` 更新 — endpoint 機構全体（T3/T4/T6）完了後の
  final verification で実施（plan の規律どおり）。
- `gui_open` field の write-only 状態 — pre-existing（T1 以前
  から書き込みのみ・読み取り consumer なし）。本 task の
  `Shared` arm でも同形状を維持 — 別 issue で追跡（Follow-ups
  参照）。

## Risks

- **`shared_verb` は blocking `lock()`** — endpoint `process` が
  cell を保持している間、全 verb（`destroy` の `into_inner`
  teardown 除く）は待機する。wedged plugin は verb を starvation
  させ得るが、stream を既に壊しているため許容（doc 明記済み）。
- **endpoint `Drop` は CAS のみだが `Arc` dealloc を含む** —
  `L0Endpoint::drop` は `compare_exchange` 1 発（wait-free）だが
  endpoint 自体の dealloc は `Arc` 参照カウントの連鎖を起こし
  得る — RT スレッド上での drop は避け、engine 側の plan-swap
  collector（Task 5 の退役経路）で退役させるのが契約（
  `PluginEndpoint` doc で pin 済み）。
- **`process` の `Shared` arm は `catch_unwind` なし** — panic
  isolation が要る RT caller は endpoint 経路を使う設計（doc
  で pin 済み）。直接呼出しで panic した plugin は cell を
  poison し、以後の全経路が `Failed` 収束する — `Plain` の
  「再 enter しない」語義との非対称は意図的。
- **`static_assertions` は dev-dep のみ** — 本番依存木に新規
  外部依存なし（`ruviform-audio-core` 昇格は既存推移 dep の
  顕在化で tree 不変）。
- **`in_scratch` の確保は attach 時点の snapshot** —
  `sample_rate` は `activated` 時点の値で確保。再 Activate が
  sample_rate を変えても `in_scratch` は追随しない（bound gate
  は `max_frames` のみ検査 — v1 契約の範囲・design 確定）。
- **`Cargo.lock` が T5 分の entry を含有** — 本 commit の
  lockfile は `ruviform-audio` の `static_assertions` entry を
  既に含む（T5 のコードは別 commit — lockfile 分割不可の
  ため deviation として記録）。

## Follow-ups

- Task 3 `docs/tasks/l1-endpoint-arm-and-leg-return.md` —
  `EndpointInner::L1` variant 充填・`EndpointLegParts` 組立て・
  arm/返却 channel・leg 返却 `Drop`（本 task の enum 骨格と
  `not(test)` tripwire を消費 — `EndpointInner` の expect が
  充填時に自動で再燃する）。
- Task 4 `docs/tasks/plugin-host-endpoint-surface.md` —
  `PluginHost::prepare_endpoint`/`latency_frames` 公開面・統一
  defined-rejection 表・bounds prelude（presence/占有 →
  bounds の順 — watch note 転記済み）・`attach_endpoint` 等の
  `not(test)` tripwire 除去（caller 配線で expectation が非
  test build でも fulfilled になり attribute 自体を外す）。
- Task 6 `docs/tasks/plugin-graph-e2e-and-rt-lane.md` —
  `process_audio` shell が `in_scratch` を消費（`#[expect]`
  が自動で再燃する）・bound clip・source-position 慣行。
- `gui_open` write-only — pre-existing の write-only field
  （T2 で `Shared` arm も同形状で書き込むが読み取り consumer
  は依然不在）。Issue #123 で追跡。
- Issue #114 — `EndpointBounds::channels` と activated main
  bus の等価性は plugin channel query 契約（未整備）に依存 —
  v1 は `out.channels() == bound.channels` の per-block 規則のみ
  （doc pin 済み）。
