---
id: "IMPL-034IpnJFvFDzopcBVBBB4M"
type: "impl"
status: "completed"
title: "Issue #113 L1 endpoint arm + leg return — Task 3 implementation"
created: "2026-10-26"
updated: "2026-10-26"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-113-l1-endpoint-arm-and-leg-return-task-3-implementation.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/composite.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/endpoint.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/host.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/data.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/mod.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs"
      - type: "file"
        path: "docs/designs/engine-plugin-host-integration.md"
      - type: "file"
        path: "docs/plans/implement-issue-113-engine-plugin-integration.md"
      - type: "file"
        path: "docs/tasks/l1-endpoint-arm-and-leg-return.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/test-specs/plugin-endpoint-contract-tests.md"
      - type: "file"
        path: "docs/test-specs/plugin-rt-retirement-tests.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/l1-endpoint-arm-and-leg-return.md"
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

# Issue #113 L1 endpoint arm + leg return — Task 3 implementation

## Summary

Issue #113 Task 3 完了 — plan actions 8–11（L1 out-of-process
経路の endpoint 所有分離）を全範囲実施。(a) `L1Instance.rt`
の `Option<RtLeg>` 化 + `Shared::rt_state` の `Arc<AtomicU8>` 化
+ per-attach field（`arm_tx`/`pending_arm`/`return_rx`/
`arm_collector`/`endpoint_bound`/`activated_sample_rate`）、
(b) `prepare_endpoint` — pinned 拒否順序（state → `rt` presence
→ bounds-before-take → take → `endpoint_channel_setup`）で leg
を `EndpointLegParts`（crate-free プリミティブ束）として移出、
(c) `endpoint.rs` — `L1Endpoint`（leg + mirror + `arm_rx`/
`arm_retirer`/`return_tx` のみを保持・Send は型構造のみ）+
`EndpointInner::L1(Box<…>)` variant 充填 + `process` の 4 段経路
（arm drain → mirror gate → dims pre-gate → `RtLeg::process`）、
(d) `RtLeg::arm_apply`（ring 束縛のみ reset・seq 空間簿記持続
・displaced ring → `Retirer`）+ `armed_generation`/
`armed_frame_capacity`/`reserve_last_output` accessor、
(e) `drain_endpoint_channels` 2 層構成（wait-free 半分 =
resend + restore のみ・non-RT 半分 = collector drain + re-arm
+ field clear）+ `Drop` → cap-1 return channel、
(f) composite `attach_endpoint` route-aware inner + L1 arm
（post-take 失敗は endpoint drop + drain で leg を復元する
restore-via-drop）。独立レビュー 2 件（A=APPROVE-WITH-FIXES・
B=adversarial）の確定修正 F1–F7 を全て適用 — wait-free drain
分割・restored leg re-arm・共有 `Mutex<Producer<u64>>` result
channel（synthesized `mark_failed` completion）・over-bound
arm の pass-through 収束・`channels == bound.channels` gate +
`min_frames` child 委譲・`Destroyed` restore gate・composite
doc + `arm_retire_overflows` 観測点。単体テスト新規 25 件 +
`rt_` 4 件。全 gate green 実 log。

## Context

approved plan
`docs/plans/implement-issue-113-engine-plugin-integration.md` の
Task 3（`docs/tasks/l1-endpoint-arm-and-leg-return.md`・actions
8–11）。upstream: approved design
`docs/designs/engine-plugin-host-integration.md`（決定サマリ
i–k — L1 leg 移出・arm channel 5 点固定・`RtLeg::arm_apply`
簿記分割・`pending_arm`/tier 境界・leg 返却 +
drain-before-early-return・bounded 性）・spec
`docs/specs/engine-plugin-host-integration.md`（AC2・AC4・AC5・
AC8）・ADR `docs/adr/plugin-rt-ownership-split.md`（leg を
graph node 所有へ移す確定分岐）・TSpec
`docs/test-specs/plugin-endpoint-contract-tests.md`・
`docs/test-specs/plugin-rt-retirement-tests.md`。前提は Task 1
（`EndpointUnavailable` local-only variant）と Task 2（
`src/endpoint.rs` の `EndpointInner` 骨格・`EndpointBounds`
公開型・`not(test)` tripwire）が供給。後続: Task 4（
`PluginHost::prepare_endpoint` 公開面 — 本 task の
`attach_endpoint` route-aware inner を消費）・Task 6（
`process_audio` shell — `in_scratch` を消費）。

## Implementation

### Changes

- `crates/ruviform-plugin-host/src/l1/mod.rs`（+976/-76 行）:
  - `rt: data::RtLeg` → `rt: Option<data::RtLeg>` — endpoint
    attach 生存の authoritative 判定は `self.rt.is_none()`。
    `process` は wait-free drain 後に `None` →
    `Err(EndpointUnavailable)`（処理面が endpoint に移った
    defined 拒否 — `Ipc` と区別）。
  - `Shared::rt_state: AtomicU8` → `Arc<AtomicU8>` —
    `set_mirror` の store 点は不変・endpoint が同じ `Arc` を
    clone 共有し lock-free に mirror を観測する。
  - **`Shared::result_tx: Mutex<ruviform_rt::Producer<u64>>`**
    （F3 — review 修正）: result-channel producer を 2 non-RT
    スレッドで共有 — reader が real `BlockDone`/rejection
    completion を、driver が doorbell-less drop した seq の
    **synthesized `mark_failed` completion** を push する。
    `Producer` は single-owner のため `Mutex`（leaf lock —
    `inner`/`child` 取得中は絶対に取らない・CS は bounded
    `try_push` 1 発・poison-tolerant）。drop された seq は
    `block_in_flight` に入らず real result を生まない —
    応答なしでは `cleared[i] >= last_pushed[i]` の strict-
    alternation gate が永久に wedge する（全 slot 占有で
    leg が 100% pass-through に固着）。synthesized completion
    は 1 seq = 1 element で channel cap `D` を in-protocol
    で溢れさせない。`reader_main`/`spawn_reader` の
    `result_tx` 引数を除去し `Shared::push_result` に統一。
  - per-attach field: `arm_tx: Option<Producer<Arc<Staging>>>`
    （cap 2）・`pending_arm: Option<Arc<Staging>>`（latest-wins
    退避）・`return_rx: Option<Consumer<RtLeg>>`（cap 1）・
    `arm_collector: Option<DropCollector<Arc<Staging>>>`（cap
    4 — endpoint の `Retirer` 対応）・`endpoint_bound:
    Option<(u32, u32)>`（`(max_frames, channels)` 簿記 —
    公開 `EndpointBounds` を l1 から名指さない crate-free
    discipline）・`activated_sample_rate: Option<u32>`（
    composite が endpoint `in_scratch` を size するための
    最後の成功 Activate 記録）。
  - `prepare_endpoint(&mut self, bound_max_frames,
    bound_channels) -> Result<data::EndpointLegParts,
    HostError>`（`pub(crate)` — T4 の public caller 未配線
    のため `not(test)` tripwire）: **pinned 順序** — (1) drain
    + state check（`Failed` → `Err(Failed)`・`Activated`/
    `Processing` 未満 → `Err(Lifecycle)`）→ (2) `rt.is_none()`
    → `Err(EndpointUnavailable)`（単一 endpoint 規則）→
    (3) bounds validation（非 0・`≤ WIRE_MAX_FRAMES`(8192)/
    `≤ WIRE_MAX_CHANNELS`(16)・`bound_max_frames ≥ armed
    staging.frame_capacity` — `Err(Prepare(UnsupportedConfig))`
    — **take 前なので bad bounds は leg 不動**）→ (4) `rt.take()`
    → (5) `endpoint_channel_setup`（post-take 契約: 将来の
    fallible step は `self.rt = Some(leg)` + field clear で
    復元してから返す — 現行 helper は infallible・restore
    契約は doc pin）。
  - `endpoint_channel_setup`（pinned 順序 (4)–(8)）:
    `channel::<Arc<Staging>>(2)`・`retire_channel::<Arc<
    Staging>>(4)`・`channel::<RtLeg>(1)` を mint → `Inner::
    staging` clone を **seed arm** として `try_push` →
    `leg.reserve_last_output(bound_max_frames ×
    WIRE_MAX_CHANNELS)`（`arm_apply` の `resize` が常に
    capacity 内 — RT path で realloc しない）→ per-attach
    field 配線 → `EndpointLegParts` 返却。
  - `arm_staging(staging, config)` — `Activate` 成功時の ring
    publish を集約: `Inner::staging` 差替 + `activated_sample_
    rate` 記録 + `(Some(rt), _)` → `rt.arm`（従来経路）/
    `(None, Some(arm_tx))` → `try_push`（`Full` → `pending_arm`
    latest-wins 退避）。
  - 再 `Activate` bound gate: `self.rt.is_none()` が
    authoritative 判定（`endpoint_bound` は比較値のみ —
    liveness 判定に使わない）で `config.max_frames() >
    endpoint_bound.0` → `Err(Prepare(UnsupportedConfig))`（
    arm 生成・wire 到達より前 — `execute` の `request` 呼出し
    前に配置）。
  - `drain_endpoint_channels`（全 `&mut` non-RT entry point
    冒頭 — `execute`/`prepare_endpoint`/`destroy`/`Drop`/
    composite `try_recv_notification(&mut)`）: wait-free 半分
    → `arm_collector.drain()` → **leg 復元の仕上げ**（F2 —
    復元 leg が `Inner::staging` より古い generation を装着
    していれば `rt.arm(inner.staging)` で re-arm — そのまま
    では以後の submit が driver の `armed_for` gate で全 drop
    され恒久的 pass-through + `latency_frames` 乖離。wait-free
    restore から本 drain までの一過性 stale submit は
    pass-through で自然回復）→ `clear_endpoint_fields`（
    `arm_tx`/`pending_arm`/`return_rx`/`arm_collector`/
    `endpoint_bound` = None — rtrb ring free・`Arc<Staging>`
    destructor は non-RT のみ。検出子は `rt.is_some() &&
    return_rx.is_some()`）。
  - `drain_endpoint_channels_wait_free`（F1 — `process` が
    呼ぶ wait-free 半分のみ分離）: `pending_arm` resend（
    `try_push` — `Full` なら pending に戻す）+ `return_rx.
    try_pop` → `self.rt = Some(leg)` の **restore のみ** —
    `arm_collector.drain()` も `clear_endpoint_fields` も
    含まない（destructor ゼロ・stale channel ends は non-RT
    drain が片付ける）。**restore は非 terminal gate**（F6 —
    `rt_state` mirror ≠ `Destroyed` — Destroy 済み instance へ
    は復元せず queued leg は channel ends と共に teardown で
    bounded drop）。
  - `latency_frames(&self) -> Option<u32>`（`not(test)`
    tripwire）: `inner.staging` が `Some` なら
    `Some((depth-1) × staging.frame_capacity)`・`None` なら
    `None` — `&self` defined non-drain 観測点。
  - `activated_sample_rate(&self)`（`not(test)` tripwire）・
    `arm_retire_overflows(&self)`（F7 — arm retire channel の
    leak-on-full 飽和観測点・`PlanSwap::retired_overflows` と
    同じ observation point）。
  - `Drop::drop` 冒頭で `drain_endpoint_channels` — bounded
    `try_*` teardown・endpoint liveness に待たない。
  - `#[cfg(test)] impl L1Instance::test_fixture(mirror,
    activated_frames)` — child-free fixture（real `Shared`・
    channel legs・shm segment・driver/reader は `None`）—
    `l1`/`composite` 両 test module から利用。
- `crates/ruviform-plugin-host/src/l1/data.rs`（+247/-19 行）:
  - `RtLeg::arm_apply(&mut self, arm: Arc<Staging>, retirer:
    &mut Retirer<Arc<Staging>>)` — endpoint 側 ring swap（
    `RtLeg::arm` の RT-suitable twin — host 側 `Activate` 用の
    現行 `arm` は不変・別メソッド）。**簿記分割**: ring 束縛の
    み reset（`frame_capacity`・`last_output` clear + `resize`
    （`prepare_endpoint` reserve 内・realloc なし —
    `debug_assert` で pin）・`staging` 差替・`last_frames`/
    `last_channels`/`have_output`）+ displaced ring（
    `self.staging.replace(arm)` の旧値）→ `retirer.retire`。
    seq 空間簿記（`last_pushed`/`cleared`/`held`/`submit_dims`/
    `next_block`/`depth`/`lookahead`）は arm を跨いで持続 —
    旧 ring の in-flight block が resolve できるよう
    strict-alternation 不変条件 `cleared[i] ≥ last_pushed[i] ⇒
    slot i 空` を維持し、stale done が再 arm 後 seq と alias
    しない。
  - `RtLeg::armed_generation(&self) -> u64`（未装着は 0 —
    `staging_generation` は 1 起点なので最初の arm を受理）・
    `RtLeg::armed_frame_capacity(&self) -> usize`（未装着は
    0）— private `staging` field を l1 外（`endpoint.rs`）へ
    見せない accessor。endpoint drain の `generation > armed`
    単調 gate と dims pre-gate が読む。
  - `RtLeg::reserve_last_output(&mut self, sample_count)` —
    `prepare_endpoint` が `bound.max_frames × WIRE_MAX_CHANNELS`
    で一度だけ呼ぶ reserve（`saturating_sub` で縮小方向は
    no-op）。
  - `RtLeg::pass_through` を `pub(crate)` 化 — endpoint の
    over-bound arm 収束（F4）が同じ defined 出力へ畳み込む。
  - `armed_for`/`Submit`/module doc の更新（F3）: dropped seq
    は「result が来ない」ではなく driver が synthesized
    `mark_failed` で complete する — alternation gate が
    slot を解放する語義へ修正。
  - `EndpointLegParts`（`pub(crate)` struct — `leg`/`mirror`/
    `arm_rx`/`arm_retirer`/`return_tx`）: l1 が返せる名指し
    可能なプリミティブ束（`data::RtLeg`/`ruviform_rt` 型/
    `std` 型のみ — `L1Endpoint`/`EndpointBounds` を含まない
    crate-free 境界型。`#[path]` include される `tests/
    l1_proxy_e2e.rs` の module tree 内でも解決する）。
- `crates/ruviform-plugin-host/src/endpoint.rs`（+613/-15 行）:
  - `EndpointInner::L1(Box<L1Endpoint>)` variant 充填 —
    **Boxed**（leg の channel ends + staging state は L0 arm
    の ~10 倍 — attach は non-RT なので `EndpointInner` を
    compact に保つため `large_enum_variant` 対応として Box。
    task doc の `L1(L1Endpoint)` 素置きからの deviation）。
  - `struct L1Endpoint { leg: Option<RtLeg>, mirror:
    Arc<AtomicU8>, arm_rx: Consumer<Arc<Staging>>, arm_retirer:
    Retirer<Arc<Staging>>, return_tx: Producer<RtLeg> }` —
    child process・shm segment・`Mutex`・`L1Instance` 参照・
    I/O thread・notification receiver を一切持たない（Send
    を型構造のみで担保・`unsafe impl` なし — `expect_send`
    witness を test に追加）。`leg` が `Option` なのは `Drop`
    の `take` のみ — live 区間は常に `Some`。
  - `ARM_DRAIN_MAX = 3` — 1 `process` pass の arm pop 上限
    （arm channel cap 2 + resend が drain と race し得る 1）。
  - `L1Endpoint::process` 4 段経路: **(1) arm drain** —
    bounded ≤3 pop・`arm.generation > leg.armed_generation()`
    のみ `arm_apply`（同 generation = 正規化 no-op・stale =
    拒否）・**drain した arm は適用・不適用・seed を問わず
    全て `arm_retirer.retire`**（`Arc<Staging>` destructor は
    RT thread で走らない）。`frame_capacity > bound.max_frames`
    の arm は apply せず retire + `leg.pass_through(outputs)`
    + `Ok(CONTINUE)` で収束（F4 — control 側 `Activate` gate
    で in-protocol 不到達の内部バックストップ — `Err(Ipc)`
    は endpoint 公開面に一切露出しない）。**(2) mirror gate**
    — `Failed` 観測は `fetch_max(Failed, Release)` で単調
    publish + 無音 + `Ok(CONTINUE)`（第 2 の書き手 — L0 と
    同じ収束）・非 `Processing` は `Err(Lifecycle)` で `out`
    不変更。**(3) dims pre-gate** — `frames > leg.armed_frame_
    capacity()`（armed ring の capacity = activated
    `max_frames`）または `channels != bound.channels`（F5 —
    L0 gate の mirror として追加）→ `Err(Prepare(
    UnsupportedConfig))`・`out` 不変更 — leg の `block_data`
    が同種条件を `Err(Ipc)` で返すのに先行。`min_frames` は
    **child 側 plugin 契約に委譲**（armed `Staging` が保持
    しない deliberate divergence — code/test-spec/design で
    pin）・`out.channels() == bound.channels`/`frames ≤
    bound.max_frames` の bound clip は T6 の `process_audio`
    が所有。**(4) `leg.process`** — `Err(Ipc)` は pre-gate
    漏れの wire 表現不能ブロックのみ（`EndpointInner::L1::
    process`/`process_audio` 公開面から露出しない契約 —
    `Err(HostError::Ipc)` → `Err(Prepare(UnsupportedConfig))`
    remap・scope は L1 arm のみ・L0 arm は untouched）。
  - `From<EndpointLegParts> for L1Endpoint` — crate-free 境界
    で parts bundle を endpoint へ組立てる（`endpoint.rs` は
    `crate::l1::data` を利用可 — l1 木の外）。
  - `Drop for L1Endpoint` — `leg.take()` → `return_tx.
    try_push`（cap 1）— `TryPushError::Full`（double-drop
    anomaly のみ到達）のみ失敗形で `into_inner` 回収した leg
    は drop スレッド上でそのまま drop（bounded teardown —
    どのスレッドからでも安全）。leg 喪失（host 側 channel
    endpoint 既 drop）は再 attach 不能の defined 終端。
  - `bound_layout`/`in_scratch` を `host.rs` から移設 —
    L0/L1 両 attach path で共有する bound-shaped scratch
    確保（`pub(crate)`・`not(test)` tripwire）。
- `crates/ruviform-plugin-host/src/host.rs`（+9/-30 行）:
  `bound_layout`/`in_scratch` の `endpoint.rs` 移設に伴う L0
  `attach_endpoint` の call site 更新（`crate::endpoint::
  in_scratch(bounds, activated.sample_rate())`）— L0 側の
  attach 順序・失敗時 epoch release は不変。
- `crates/ruviform-plugin-host/src/composite.rs`（+280/-1 行）:
  - `PluginHost::attach_endpoint(&mut self, handle, bounds) ->
    Result<PluginEndpoint, HostError>`（`pub(crate)` —
    route-aware inner・T4 public prelude の唯一の in-crate
    caller になるまで `not(test)` tripwire）: `Slot::InProcess`
    → `l0.attach_endpoint` へ delegate・`Slot::OutOfProcess`
    → `instance.prepare_endpoint`（pinned 拒否順序は L1 側
    所有）→ `L1Endpoint::from(parts)` → `activated_sample_rate`
    + `in_scratch` 組立て → `PluginEndpoint::new(
    EndpointInner::L1(Box::new(endpoint)), bounds, in_scratch,
    latency_frames)`。**post-take restore-via-drop**: scratch
    組立ての `Err` 経路（現行は両 `Err` leg とも理論上不到達
    — `Activated`/`Processing` ⟹ armed ring ⟹ recorded
    sample rate・bound-validated shape の `in_scratch` は
    失敗しない — `debug_assert!(false)` で pin）は endpoint
    を drop（leg を return channel へ push）→
    `drain_endpoint_channels` で `self.rt` 復元 — post-take
    `Err` が leg を orphan 化しない `prepare_endpoint` 契約の
    composite 側実装。
  - `PluginHost::try_recv_notification(&mut)` — slot iter 内で
    `instance.drain_endpoint_channels` を呼ぶ配線（`&mut`
    entry の drain 呼出し点 — `&self` 版 queries は defined
    non-drain 維持）。
  - `process` doc — detached L1 slot の `EndpointUnavailable`
    行を追加。
- `crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs`（+4/-3
  行・comment のみ）: stale-submit gate の doc を F3 語義へ
  修正（dropped seq → driver synthesizes `mark_failed` →
  alternation gate frees slot）。crate-free 証人は `#[path]`
  include で新 `#[cfg(test)] mod tests`（`l1::tests` 10 件・
  `l1::data::tests` 2 件）も取り込み e2e バイナリの test 数が
  42 → 54 に増えたことを確認。
- `docs/designs/engine-plugin-host-integration.md`・
  `docs/plans/implement-issue-113-engine-plugin-integration.md`・
  `docs/test-specs/plugin-endpoint-contract-tests.md`・
  `docs/test-specs/plugin-rt-retirement-tests.md`: review 修正
  F1–F7 を upstream 文書へ in-place 反映（wait-free drain
  分割・re-arm 仕上げ・synthesized completion・pass-through
  収束・`channels == bound` gate + `min_frames` 委譲・
  `Destroyed` gate・`Shared::result_tx` 記述）— design の
  確定決定が implementation と乖離しない同期。

### Review findings and resolutions（独立レビュー 2 件 — A=APPROVE-WITH-FIXES・B=adversarial — 確定修正 F1–F7 を全て適用）

- **F1: wait-free drain の分割 — RT 側で destructor を走らせない
  （fixed — B 指摘・最重要）**: 初版は `process` の drain が
  `arm_collector.drain()` と `clear_endpoint_fields`（stale
  channel ends の drop = rtrb ring free + `Arc<Staging>`
  destructor）を含み、leg 復元のたびに RT thread 上で dealloc
  burst が走り得た。`drain_endpoint_channels_wait_free`（
  resend + `return_rx` pop → `self.rt = Some(leg)` の restore
  のみ — **destructor を一切含まない**）へ分割し、collector
  drain・re-arm・field clear は non-RT `drain_endpoint_channels`
  のみが行う。wait-free restore 後の stale ends は bounded・
  inert で残り、次の `&mut` entry で片付く。
- **F2: restored leg の re-arm を non-RT drain が担う（fixed —
  B 指摘）**: detached-`Activate` で `Inner::staging` が新
  generation へ進み、endpoint がその arm を未消費のまま leg が
  `return_rx` で還流すると、復元 leg は旧 generation を装着
  したまま — 以後の全 submit が driver の `armed_for` gate で
  drop され恒久的 pass-through + `latency_frames` 乖離に
  なった。non-RT drain が `staging.generation > rt.
  armed_generation()` なら `rt.arm(inner.staging)` で re-arm
  して自己修復（`RtLeg::arm` の `last_output` resize は attach
  時 reserve 内 — realloc なし）。これにより **再 attach の
  seed arm は常に同 generation の正規化 no-op** となり、
  endpoint 側 `generation > armed` apply が新 ring を実際に
  装着するのは endpoint 生存中に arm channel へ届いた arm の
  み — task doc の「detached-Activate 後は seed が legitimately
  apply される」記述を修正（design/task/test-spec も同期）。
  wait-free restore から次 drain までの一過性 stale submit は
  driver drop + synthesized completion で pass-through として
  自然回復する。
- **F3: 共有 `Mutex<Producer<u64>>` result channel — driver が
  doorbell-less drop の completion を synthesize（fixed — B 指摘
  ·最重要）**: driver が `armed_for`（stale generation）または
  terminal/shutdown gate で seq を drop すると、従来は result
  が一切来ず `cleared[i] >= last_pushed[i]` gate が永久に
  wedge — 全 slot 占有で leg が恒久的 pass-through に固着
  し得た。`Shared::result_tx: Mutex<Producer<u64>>` を新設し、
  drop した seq には `push_result(mark_failed(seq))` で
  completion を synthesize — **1 seq = 1 completion element**
  の accounting を維持して alternation slot を解放する（
  `reader_main` の push も同 producer 経由に統一 — reader が
  real completion・driver が synthesized completion を push
  する 2 書き手 non-RT 構造・leaf mutex）。
- **F4: over-bound arm は `Err(Ipc)` ではなく pass-through 収束
  （fixed — A 指摘）**: `frame_capacity > bound.max_frames` の
  arm で `Err(Ipc)` を返す初版は「`Err(Ipc)` が endpoint 公開
  面に出ない」契約違反だった。retire + `leg.pass_through(
  outputs)` + `Ok(CONTINUE)` の defined pass-through row へ
  収束（control 側 `Activate` gate で in-protocol 不到達の内部
  バックストップのまま）。`RtLeg::pass_through` を `pub(crate)`
  化して同じ defined 出力へ畳み込む。
- **F5: dims pre-gate に `channels == bound.channels` を追加 +
  `min_frames` 委譲を pin（fixed — A/B 指摘）**: 初版の pre-gate
  は `frames ≤ armed_frame_capacity` のみ — stereo block
  against mono bound が素通りして wire へ届き得た。L0 gate の
  mirror として `channels != bound.channels` を追加。`min_
  frames` enforcement は armed `Staging` が保持しないため
  **child 側 plugin 契約に委譲** — deliberate divergence として
  code comment・task doc・test-spec・design の 4 面で pin。
  `Err(HostError::Ipc)` → `Err(Prepare(UnsupportedConfig))` の
  remap scope は L1 `process` 経路のみに限定（`Err(Ipc)` は
  L1 wire 内部の「wire が表現不能なブロック」意味に留める）。
- **F6: leg restore を `!Destroyed` gate（fixed — B 指摘）**:
  `Destroy` 後に `return_rx` へ leg が返った場合、従来は
  Destroyed instance に `self.rt` を復元し得た（terminal 状態
  の半復活）。wait-free drain の restore を mirror
  `is_destroyed()` gate 化 — queued leg は channel ends と共に
  teardown で bounded drop され、Destroyed instance は復元
  しない（`leg_returned_after_destroy_is_not_restored` で pin）。
- **F7: composite doc + `arm_retire_overflows` 観測点（fixed —
  A low）**: `PluginHost::attach_endpoint` doc に route/pinned
  順序の所有権（`prepare_endpoint` が state→bounds→take を
  所有・composite は route + scratch 組立てのみ）と post-take
  restore-via-drop を明記。`arm_collector` の `DropCollector::
  overflows` を `L1Instance::arm_retire_overflows(&self)` で公開
  — arm retire channel の leak-on-full 飽和を `PlanSwap::
  retired_overflows` と同型の観測点で pin（`&self` defined
  non-drain — atomic load のみ）。

### Decisions During Implementation

- **`EndpointInner::L1` は `Box<L1Endpoint>`**（deviation —
  deliberate）: task doc/design の `L1(L1Endpoint)` 素置きは
  `large_enum_variant` lint を trigger する（leg の channel
  ends + staging state は L0 arm の ~10 倍）。attach は non-RT
  なので Box 化で `EndpointInner` を compact に保つ —
  `ProcessResult` dispatch・`Send` 契約は不変。
- **`endpoint_channel_setup` は infallible-today + restore 契約
  doc**（deviation — deliberate）: channel mint・seed push・
  reserve・簿記は現行全て失敗しないため helper は `->` 値を
  返す（`Result` ではない）。task doc の「(3) 以降の全 `Err`
  経路は `self.rt = Some(leg)` で復元」契約は **post-take
  restore contract** として doc pin — 将来の fallible step は
  `self.rt = Some(leg)` + `clear_endpoint_fields` で復元して
  から返す。composite 側は endpoint を組立ててから失敗し得る
  `in_scratch` 組立てを **restore-via-drop**（endpoint drop →
  return channel → `drain_endpoint_channels`）で同契約を
  実装する。
- **wait-free restore は stale ends を残す**（F1 の帰結 —
  deliberate）: `process` 経路で leg が復元されても
  `arm_tx`/`return_rx`/`arm_collector`/`endpoint_bound` は
  そのまま残る（clear の検出子 `rt.is_some() &&
  return_rx.is_some()` は次の non-RT drain で立つ）。stale
  ends は bounded・inert — `&mut` entry が一度も走らない
  session でも drop/teardown で channel ends と一緒に回収
  される。
- **detached-Activate → restore re-arm は non-RT のみ**（
  follow-up — transient pass-through は defined）: wait-free
  restore から次 `&mut` drain までの window では leg が旧
  generation を装着し submit が driver drop + synthesized
  completion で pass-through する — この一過性 divergence は
  defined で自己修復する（test で pin）。endpoint 生存中に
  detached-Activate する caller がこの transient を嫌うなら
  `&mut` entry（`execute`/`try_recv_notification` 等）を一度
  呼ぶことで re-arm を確定できる。
- **`activated_sample_rate` field を L1Instance に追加**: bound
  -shaped `in_scratch` は `AudioBuffer<f32>` を activated
  sample rate で確保する必要があり、`ProcessConfig` は wire
  command の payload としてしか残らないため、最後の成功
  `Activate` の `sample_rate().get()` を `arm_staging` で記録
  する（`Some` ⟺ armed ring 存在 — composite は `ok_or(
  HostError::Ipc)` で不到達 `Err` に写像）。
- **`min_frames` enforcement は child 側契約**（F5 — defined
  divergence pin）: armed `Staging` は `frame_capacity` のみ
  保持し `min_frames` を知らない — L1 dims gate は `frames ≤
  capacity` + `channels == bound` の 2 条件のみ。min-frames
  未到達 block の defined 挙動（zero-pad か rejection か）は
  child-side plugin contract に委譲 — test-spec の defined-
  rejection 表が pin する deliberate divergence。
- **`cfg_attr(not(test), expect(dead_code))` tripwire 継続**:
  `prepare_endpoint`/`endpoint_channel_setup`/`latency_frames`/
  `activated_sample_rate`/`arm_retire_overflows`/`in_scratch`/
  composite `attach_endpoint`/`EndpointInner` 等は T2 と同じ
  `not(test)` パターン — 非 test build でのみ dead_code を
  expect し、T4/T6 の配線時に lint が自動再燃する自己消去
  tripwire。

## Related Experiments

### Adopted

なし — approved design/plan の確定実装 + review 確定修正のみ
（実験ログなし）。

### Rejected

- **wait-free `process` 経路での `arm_collector.drain()`/field
  clear** — F1 で却下: rtrb ring free・`Arc<Staging>`
  destructor は RT 契約違反の destructor work — non-RT drain
  のみが実行する。
- **endpoint drop 後の leg 喪失の自動復旧** — host 側 channel
  endpoint が先に drop された場合の leg 喪失は defined 終端と
  する（再 attach 不能 — design 確定どおり）。
- **interior-mutability による `&self` drain** — design 時点で
  却下済み（`try_push`/`try_pop`/`retire`/`drain` は全て `&mut`
  signature — `&self` query を defined non-drain observation
  point とする確定分岐を維持）。

## Validation

- gate（実 log・全 exit 0）:
  - `mise run fmt-check` — exit 0
  - `mise run check` — exit 0（`cargo check --workspace
    --all-targets`）
  - `mise run lint` — exit 0（`cargo clippy --workspace
    --all-targets -- -D warnings` clean — `EndpointInner::L1`
    の `Box` 化は `large_enum_variant` 対応・`not(test)`
    tripwire の expectation が非 test build で fulfilled）
  - `mise run test` — exit 0（`cargo test --workspace` —
    **55 test binaries 全 pass・0 fail**）
  - `mise run test-rt` — exit 0（`rt_` フィルタ実一致: lib
    **8** 件 = 新規 `rt_l1_endpoint_process_no_alloc`・
    `rt_l1_endpoint_arm_drain_no_alloc`・`rt_l1_detached_process_
    no_alloc`・`rt_l1_arm_apply_no_alloc` の 4 + 既存
    `rt_endpoint_process_no_alloc`・`rt_endpoint_busy_bypass_
    no_alloc`・`rt_process_no_alloc` + 副次一致 `shared_start_
    failure_rolls_back_to_activated`（`start_` 内 `rt_` —
    T2 既知）。e2e バイナリ **4** 件 = `rt_l1_process_path_
    no_alloc`・`rt_l1_result_leg_try_ops` + `#[path]` include
    経由の `l1::tests::rt_l1_detached_process_no_alloc`・
    `l1::data::tests::rt_l1_arm_apply_no_alloc`）
  - `mise run build` — exit 0（`cargo build --workspace`）
  - `mise exec -- cargo test -p ruviform-plugin-host` — exit 0
    （lib **55** + `l1-test-worker` **0** + `l1_proxy_e2e`
    **54** + doctest **1** — 全 pass。新規単体 **25** 件を
    含む: l1/mod.rs 10・l1/data.rs 2・endpoint.rs 11・
    composite.rs 2。e2e 42 → 54 は `#[path]` include が新
    `#[cfg(test)]` module を取り込んだため。**load-sensitive
    2 件**（`process_roundtrip_gain_and_lookahead_delay` —
    paced caller の `matched >= 16` 閾値・`wedged_child_write_
    converges_and_teardown_is_bounded` — `NOTIFY_TIMEOUT`
    teardown 閾値）**も本 run で pass** — flake 未発現）
- 構造確認（実コマンド）:
  - `rg -n "fn rt_" crates/ruviform-plugin-host/{src,tests}` —
    新規 `rt_` 4 件の実在を確認（上記 `test-rt` 実一致件数と
    整合 — e2e 側は `#[path]` include 経由で `l1::` module
    tests が実行される）
  - `rg -n "crate::|ruviform_plugin_host::" crates/ruviform-
    plugin-host/src/l1` — crate-free discipline 維持（
    `EndpointLegParts`/`armed_*` accessor/`ProcessConfig`
    import は `ruviform_plugin_core`/`ruviform_rt`/`super::`/
    `data::` のみ — `L1Endpoint`/`EndpointBounds` を l1 から
    名指さない）

## Non-Goals

- `PluginHost::prepare_endpoint`/`latency_frames` 公開面 —
  Task 4 の scope。`attach_endpoint`/`L1Instance::
  prepare_endpoint` は `pub(crate)` のまま（unit test が sole
  consumer — `not(test)` tripwire で pin）。
- `process_audio` shell（`in_scratch` 消費・bound clip・
  source-position 慣行）— Task 6 の scope。`in_scratch` は
  attach 時の確保のみ（`#[expect]` pin 継続）。
- test-spec `plugin-endpoint-contract-tests.md`/
  `plugin-rt-retirement-tests.md` の `proposed → confirmed`
  更新 — endpoint 機構全体（T4/T6）完了後の final
  verification で実施（plan の規律どおり・本 task は review
  修正の in-place 反映のみ）。
- driver-side submit drop の `mark_failed` synthesize は driver
  thread 内完結（endpoint 機構とは独立して機能 — F3 は既存
  wedge hole の修正でもある）。
- `PluginEndpoint` crate doc 完成・`lib.rs` stale 遅延式 doc
  修正 — Task 4 の scope。

## Risks

- **wait-free restore 後の transient stale-generation submit** —
  leg が wait-free 経路で復元されても、復元仕上げ（re-arm）が
  non-RT drain でしか走らないため、`&mut` entry が一度も呼ば
  れない限り旧 generation で submit し続ける（driver drop +
  synthesized completion で pass-through — defined で自己修復
  する transient）。`&mut` entry を呼ばない session は理論上
  pass-through が継続し得るが、arm 経路の arm は host 側
  `arm_tx` に積まれたまま（bounded cap 2 + `pending_arm`
  latest-wins — wedge しない）。
- **detached-Activate → restore re-arm は non-RT のみ**（F2 の
  帰結・follow-up 記録）: `process` だけを呼ぶ caller は re-arm
  が確定するまで pass-through window を観測し得る — doc pin
  済みの transient。
- **`Mutex<Producer<u64>>` は leaf lock のみ許容** — `push_
  result` の CS は `try_push` 1 発で `inner`/`child` 取得中は
  絶対に取らない（module doc で disjoint-sections 規則を pin）。
  将来ここへ `lock()` を挟む変更は RT 契約ではない（push は
  non-RT thread のみ）が deadlock 規則を破る — comment で pin。
- **`l1_proxy_e2e` の load-sensitive 2 件** — paced caller の
  `matched >= 16`・`NOTIFY_TIMEOUT` teardown 閾値は CI 高負荷
  下で flake し得る既知リスク（本 run では両方 pass）。閾値は
  headroom 込みで既に設計されている — flake が再現する場合は
  閾値の見直し（本 task scope 外）。
- **`test_fixture` は `#[cfg(test)]` のみ** — child-free
  fixture（driver/reader `None`）は `request` が `Err(Ipc)` を
  返すため wire 越えの挙動は e2e でのみ検証される（単体
  test の fixture 契約として意図的 — real child を spawn
  する経路は `l1_proxy_e2e` が担保）。

## Follow-ups

- Task 4 `docs/tasks/plugin-host-endpoint-surface.md` —
  `PluginHost::prepare_endpoint`/`latency_frames` 公開面・統一
  defined-rejection 表・bounds prelude（presence/占有 →
  bounds の順 — T2 watch note 転記済み）・本 task の `not(test)`
  tripwire 除去（caller 配線で expectation が非 test build
  でも fulfilled になり attribute 自体を外す）・crate doc。
- Task 6 `docs/tasks/plugin-graph-e2e-and-rt-lane.md` —
  `process_audio` shell が `in_scratch` を消費（`#[expect]` が
  自動で再燃）・bound clip・source-position 慣行・`min_frames`
  enforcement（child 側契約の host 側残務 — `bound` clip row
  は `process_audio` が所有）。
- **L1 dims `min_frames` enforcement = child-side contract**
  （pinned deliberate divergence — F5）: armed `Staging` は
  `min_frames` を保持しないため L1 gate は上界 + channels
  等価のみ。child-side plugin contract での enforcement は
  wire protocol の child 側責務として残る — plugin-export
  runner（T 系別 task）が contract を具体化する際に整合確認。
- **detached-activate → restore re-arm は non-RT のみ**（transient
  pass-through は defined — F1/F2 の帰結）: `process` のみを
  呼ぶ caller は `&mut` entry まで re-arm 未確定の transient
  window を観測し得る — doc pin 済み・変更要求があれば別
  issue で検討。
- Issue #114 — `EndpointBounds::channels` と activated main bus
  の等価性は plugin channel query 契約（未整備）に依存 — v1
  は `channels == bound.channels` の per-block 規則のみ（T2
  から継続・F5 で `!=` gate を追加しても同前提）。
