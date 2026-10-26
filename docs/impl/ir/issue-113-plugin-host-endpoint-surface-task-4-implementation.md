---
id: "IMPL-034ME2yPd0TjTbxGaWidey"
type: "impl"
status: "completed"
title: "Issue #113 plugin-host endpoint surface — Task 4 implementation"
created: "2026-10-26"
updated: "2026-10-26"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-113-plugin-host-endpoint-surface-task-4-implementation.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/composite.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/endpoint.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/host.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/mod.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs"
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
    - "docs/tasks/plugin-host-endpoint-surface.md"
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

# Issue #113 plugin-host endpoint surface — Task 4 implementation

## Summary

Issue #113 Task 4 完了 — plan actions 12–13（`PluginHost`
コンポジットの endpoint **公開面**集約）を全範囲実施。機構自体は
T2/T3 で実装済みであり、本 task は (a) public
`PluginHost::prepare_endpoint`（handle → state → presence →
uniform bounds の pinned 拒否順序を持つ prelude + slot dispatch →
`pub(crate)` 内側 `attach_endpoint`）、(b) public
`PluginHost::latency_frames`（L0 `Some(0)` lifecycle gate・L1
mirror gate・それ以外 `None`）、(c) 統一 defined-rejection 表の
公開 API doc 化（`Err(Ipc)` defensive row 含む）、(d) crate
doc 完成（endpoint 面・遅延契約・video NONRT 境界・RT 契約）+
`composite.rs`/`lib.rs` の stale 遅延式修正、(e) `plugin_mut`
L0-Shared `None` parity doc、を所有した。T2/T3 で埋め込まれた
「T4 が呼ぶまでの橋」`expect(dead_code)` 11 件を全て除去し、
残余 2 件（`in_scratch` field・`arm_retire_overflows`）は意図的
残存。実装レビュー確定修正 5 件（F1–F5）を全て適用。新規テスト
10 件 + doctest 1 件 + e2e assertion 2 箇所。全 gate green 実
log。

## Context

approved plan `docs/plans/implement-issue-113-engine-plugin-integration.md`
の Task 4（`docs/tasks/plugin-host-endpoint-surface.md`・
actions 12–13）。upstream: approved design
`docs/designs/engine-plugin-host-integration.md`（「エラー・拒否面」
統一表・`PluginHost` のみ公開面の確定・`latency_frames` 修正版式
`lookahead_blocks × activated max_frames`・video NONRT 境界）・
spec `docs/specs/engine-plugin-host-integration.md`（AC4・AC6・
AC10）・ADR `docs/adr/plugin-rt-ownership-split.md`（
`PluginHost` コンポジットのみが endpoint 面を持つ確定）。前提は
T2（`docs/tasks/l0-shared-slot-and-endpoint-inner.md` —
`InProcessHost::attach_endpoint`・`Shared` slot・epoch・inner
bounds 検査）と T3（`docs/tasks/l1-endpoint-arm-and-leg-return.md`
— `L1Instance::prepare_endpoint`・leg 移出・`latency_frames`
内側）の `pub(crate)` 機構 — 本 task はその上に public API
形状・統一 rejection・doc を重ねる最後の面付け task。後続は
Task 5（audio seam — 別 task で先行完了済み）と Task 6
（endpoint `process`/`process_audio` public shell + graph E2E）。

## Implementation

### Changes

- `crates/ruviform-plugin-host/src/composite.rs`（+757/-25 行）:
  - **`pub fn PluginHost::prepare_endpoint(&mut self, handle,
    bounds) -> Result<PluginEndpoint, HostError>`** — 公開 mint
    面。pinned prelude order: `self.slot_mut(handle)?`（stale/
    unknown → `InvalidHandle` が handle step で先勝ち）→ route
    別の per-route state gate → presence/occupancy →
    `check_uniform_bounds` → `self.attach_endpoint(handle,
    bounds)`（既存 `pub(crate)` inner — per-route の pinned
    順序は activated 層の defense-in-depth 再検査）。L0 arm は
    `l0.check_attachable_state(inner)?` +
    `l0.check_endpoint_free(inner)?`、L1 arm は
    `drain_endpoint_channels()`（`&mut` entry-point drain —
    presence 読みが post-return の truth を見るため）+
    `check_attachable_state()?` + `leg_detached()` →
    `Err(EndpointUnavailable)`。
  - **統一 defined-rejection 表**を `prepare_endpoint` doc に
    公開: handle（`InvalidHandle`）→ (1) state（`Lifecycle`/
    `Failed`）→ (2) presence（`EndpointUnavailable` — L0
    `Plain`/2 本目 attach/L1 leg checkout）→ (3) bounds
    （`Prepare(UnsupportedConfig)` — 非 0・`≤ WIRE_MAX_FRAMES`
    (8192)・`≤ WIRE_MAX_CHANNELS`(16)・`≥ activated`）+
    internal 行（L1 post-take assembly anomaly —
    `debug_assert`'d 未到達・leg restore 済みで `Err(Ipc)`
    返却）。video 行は存在しない（`PluginHost` が video
    handle を発行しない dead row — `Err(Unsupported)` 分岐
    不設置）。
  - **`fn check_uniform_bounds`**（private・free fn）—
    prelude の uniform 半分のみ担当: `max_frames == 0`・
    `channels == 0`・`> WIRE_MAX_FRAMES`・`> WIRE_MAX_CHANNELS`
    → `Err(Prepare(UnsupportedConfig))`。`u32::MAX` も同じ
    defined rejection で allocation abort に到達しない。
    `max_frames ≥ activated max_frames` の activated 半分は
    route 内側に残す（bounds split — Decisions 参照）。
  - **`pub fn PluginHost::latency_frames(&self, handle) ->
    Option<u32>`** — `#[must_use]`。generation check → slot
    dispatch: L0 は `l0.lifecycle_state(*inner)` が
    `Activated`/`Processing` で `Some(0)`（direct call は
    pipeline latency を加えない — mirror/lifecycle 読みのみで
    `core` mutex の blocking lock なし）、L1 は
    `instance.latency_frames()`（T3 内側 — mirror gate 版）。
    未 `Activate`/無効 handle/`Destroyed` は `None`。`&self`
    non-draining（live endpoint の leg は復元しない）。
  - **`plugin_mut` doc 修正** — L0 `Shared` slot でも `None`
    を返す旨を追記（shared mutex が plugin を保持し `&mut`
    を貸せない — L1 と同じ defined 回答。T2 挙動の doc
    parity）。
  - **`process` doc 修正** — L0 `Shared` cell busy（
    `try_lock` `WouldBlock` — non-RT verb mid-flight）が
    `EndpointUnavailable` bypass である旨を追記（review F3）。
  - **stale 遅延式 doc 修正** — `OutOfProcessSpec::
    lookahead_blocks` doc と `Slot` enum doc の旧式
    （`lookahead_blocks × block_frames / sample_rate`）を
    修正版式（`lookahead_blocks × activated max_frames`
    frames・秒換算 `frames / sample_rate`）へ同期 +
    `PluginHost` struct doc に Endpoint surface 節・RT
    contract 行（`latency_frames` 追加・`prepare_endpoint`
    non-RT 明記）追記。
  - `attach_endpoint` doc 更新（public prelude から呼ばれる
    inner である旨・内側の順序は activated 層再検査）+
    `#[cfg_attr(not(test), expect(dead_code))]` 除去 + L1
    arm の `latency_frames().unwrap_or(0)` を
    `armed_latency_frames().unwrap_or(0)` へ（review F1 —
    attach 簿記は convergence 中でも armed-ring 値を取る）。
  - inline tests +**9** 件（後述 Tests 節）+ fixture 群
    （`PlainGainFactory`・`PanicGain`/`PanicGainFactory` —
    `Shared` union `Failed` 観測の第 2 writer・`l0_instance`/
    `l1_instance`/`run_endpoint` helper・`std::panic::
    catch_unwind` import）。
- `crates/ruviform-plugin-host/src/host.rs`（+99/-32 行）:
  - **`pub(crate) fn check_attachable_state(&mut self, handle)`**
    新設 — L0 attach 側 lifecycle gate。`Shared` は
    mirror load を観測 union とし（endpoint の
    `fetch_max(Failed)` が bookkeeping lifecycle より先に見える）、
    **`slot.core.is_poisoned()` を union の第 2 half として合成**
    （review F2 — panic unwind で guard poison し mirror が
    last-committed のまま残るケースを `Failed` として観測 —
    `lock()` の `Err` arm・`destroy` と同じ判定を non-locking
    flag read で実現）。`Activated`/`Processing` → `Ok`・
    `Failed` → `converge_failed` + `Err(Failed)`・それ以下 →
    `Err(Lifecycle(IllegalTransition{Process}))`。
  - **`pub(crate) fn check_endpoint_free(&self, handle)`** 新設
    — L0 attach 側 presence gate。`Plain` →
    `Err(EndpointUnavailable)`（非 `Send` factory = endpoint
    incapable）・`Shared` の `attached != 0` →
    `Err(EndpointUnavailable)`（epoch CAS は内側の
    authoritative claim のまま — これは pre-check）。lock-free
    atomic read。
  - `attach_endpoint` 内側の step (1) を
    `check_attachable_state(handle)?` 呼出しへ置換（prelude
    と inner が同じ gate を共有する DRY — 二重実装を排除し
    inner 側は defense-in-depth 再検査として機能）+
    `expect(dead_code)` 除去（`next_epoch` field 含む 2 件）。
- `crates/ruviform-plugin-host/src/l1/mod.rs`（+116/-26 行）:
  - **`pub(crate) fn check_attachable_state(&self)`** 新設 —
    L1 attach 側 lifecycle gate。`shared.lock().mirror`
    （committed authority — L1 側は convergence 不要）で
    `Activated`/`Processing` → `Ok`・`Failed` → `Err(Failed)`・
    それ以下 → `Err(Lifecycle(IllegalTransition{Process}))`。
    prelude の L1 arm と `L1Instance::prepare_endpoint` 自身が
    共有（DRY — 内側は defense-in-depth 再検査）。
  - **`pub(crate) fn leg_detached(&self) -> bool`** 新設 —
    `rt.is_none()` の occupancy 読み（leg 返却は
    `return_rx` drain のみ — authoritative fact）。prelude が
    bounds 前に presence を判定するための read。
  - `prepare_endpoint` 内側の step (1) を
    `check_attachable_state()?`・occupancy を
    `self.leg_detached()` 呼出しへ置換 + `expect(dead_code)`
    除去（`prepare_endpoint`・`endpoint_channel_setup`・
    `latency_frames`・`activated_sample_rate` の 4 件）。
  - **`pub(crate) fn latency_frames` に mirror gate 追加**
    （review F1）— `inner.mirror` が
    `Activated`/`Processing` 以外なら `None`。armed ring は
    `Deactivate`/`Failed` 後も `staging` に残るため、gate なし
    では pipeline 停止後も stale horizon を報告し続ける —
    L0 の lifecycle gate と parity。
  - **`pub(crate) fn armed_latency_frames(&self) ->
    Option<u32>`** 新設（review F1）— mirror gate なしの
    armed-ring 値。composite の attach 経路が endpoint 簿記
    値として読む: endpoint は leg が保持する armed horizon
    を mint 時に引き継ぐべきで、mid-attach の mirror flip
    （convergence）で公開 query の gate が簿記値へ漏れると
    `Some` であるべき値が `None`/0 に化ける。公開 gate と
    内部 read を分離。
  - `fn armed_latency(inner, depth)` 抽出 — `(depth − 1) ×
    staging.frame_capacity` の shared read を
    `latency_frames`（gated）と `armed_latency_frames`
    （ungated）が共用。
  - inline test +**1** 件: `latency_frames_gate_vs_armed_read`
    （後述）。
- `crates/ruviform-plugin-host/src/lib.rs`（+41/-7 行）:
  - crate doc に **Endpoint surface (Issue #113)** 節追加 —
    `PluginHost::prepare_endpoint` が activated instance の
    processing leg を `PluginEndpoint`（`Send`・RT-side owner
    へ 1 度 move）へ detach・endpoint 面は composite のみ
    （standalone `InProcessHost` は direct-call のみ）・単一
    endpoint 規則と `EndpointUnavailable`（host-local
    topology fact — wire encoding 不載）・**video NONRT
    境界**（`VideoPluginInstance` は全操作 non-RT —
    endpoint/RT 経路は存在しない、AC10）。
  - `latency_frames` 遅延契約 doc — `Some(0)` L0・
    `Some(lookahead_blocks × activated max_frames)` L1・
    `None` 未 `Activate`/無効 handle・re-`Activate` で真値は
    変わり得る（endpoint の stored 整数は追跡しない）。
  - **stale 遅延式修正** — L44-49 付近の旧式
    （`lookahead_blocks × block_frames / sample_rate`）を
    frames 整数式へ同期（`2 × 512 = 1024 frames ≈ 21.3 ms
    at 48 kHz`・秒換算 `frames / sample_rate`）。
  - RT contract 節更新 — `plugin_mut` の L0 `Shared` `None`
    追記・`lifecycle_state`/`latency_frames` を L0
    const-cheap・L1 shared mutex（non-RT 扱い）として併記・
    `PluginEndpoint::process` が RT leg（`Send`・try-only・
    block 経路 no alloc — `rt_` test で担保）・
    `prepare_endpoint` は non-RT mint、の行を追加（review
    F3 — `latency_frames` の RT 行は新設）。
- `crates/ruviform-plugin-host/src/endpoint.rs`（+7/-24 行）:
  module doc の「Task 4's public surface」先読み記述を確定形へ
  （`prepare_endpoint` が prelude を所有）+ `PluginEndpoint::
  new` doc の caller 記述更新（`pub(crate)` で残り、公開 mint
  面は `PluginHost::prepare_endpoint`）+ **消費済み
  `expect(dead_code)` 4 件除去**（`PluginEndpoint::new`・
  `EndpointInner`・`bound_layout`・`in_scratch` fn — T4 が
  実 caller を配線したため）。
- `crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs`（+7 行）:
  `command_parity_lifecycle_and_state` に assertion 2 箇所追加
  — real `Activate` 後に `instance.latency_frames() ==
  Some(1024)`（armed ring の lookahead horizon・`(depth − 1)
  × frame_capacity` = 2 × 512）・real `Deactivate` 後に
  `None`（armed ring は `staging` に残るが公開 query は
  mirror gate で `None` — L0 parity。fixture ではなく実
  lifecycle 遷移で F1 の gate を end-to-end 実証）。
- `docs/tasks/plugin-host-endpoint-surface.md` — 補足 2 行追加
  （T4 watch note: prelude は presence/占有 → bounds の順で
  評価すること・bounds-split note: uniform/activated 分割は
  意図的 — review F5）+ status/relations/Done When/
  Verification の closeout 更新。
- `expect(dead_code)` 残存 2 件（意図的 — 除去対象外）:
  `PluginEndpoint::in_scratch` **field**（`process_audio` の
  bound-shaped input copy target — T6 public shell が読む
  までの所有物、T6-owned）・`L1Instance::arm_retire_overflows`
  （観測点 — diagnostic 用途で T6 所有ではない恒常的な
  observation-only API）。`in_scratch` **fn** は本 task で
  attach 経路の実 caller ができたため attr 除去済み。

### Review findings and resolutions（実装レビュー確定修正 5 件 — 全適用）

- **F1 — `latency_frames` mirror gate + `armed_latency_frames`
  簿記 read（fixed）**: 初版の `L1Instance::latency_frames` は
  `staging` 在否のみを読み、`Activated` 以降は常に `Some` を
  返した — しかし armed ring は `Deactivate`/`Failed`/
  `Destroyed` 後も `Inner::staging` に残るため、pipeline 停止
  後も stale horizon を報告し続ける設計だった（L0 の
  lifecycle gate と非 parity）。公開 query に mirror gate
  （`Activated`/`Processing` 以外 → `None`）を追加し、同時に
  **ungated な `armed_latency_frames` を新設** — composite
  attach 経路が endpoint 簿記値として要求するのは armed-ring
  値であり、mid-attach の convergence flip で公開 gate が
  簿記へ漏れると mint された endpoint の
  `latency_frames` が 0 に化ける。gate 版（public）と
  ungated 版（internal bookkeeping）を分離し、共有 read を
  `armed_latency(inner, depth)` へ集約。
- **F2 — `is_poisoned` union half（fixed）**: L0
  `check_attachable_state` の初版は mirror のみを観測 union
  としたが、host-side `process`（`catch_unwind` なし）で
  panicking plugin が `core` guard を抜けて unwind すると
  cell が poison しつつ mirror は last-committed
  `Processing` のまま残る — 内側 `lock()` `Err` arm や
  `destroy` が `Failed` 相当として扱う観測を prelude が
  見落とし、pinned state-before-bounds 順序に反して bad
  bounds が `Err(Prepare)` に落ち得た。`slot.core.
  is_poisoned()` を non-locking flag read で union の第 2
  half として合成し、`Err(Failed)` + bookkeeping
  convergence を union 全体で保証（
  `prepare_endpoint_observes_poisoned_cell_as_failed` で
  pin — bounds 0 でも `Failed` が勝つ）。
- **F3 — doc completeness（fixed）**: (a) `PluginHost::
  process` doc が `EndpointUnavailable` の L1 leg checkout
  のみ記し L0 `Shared` cell busy（`try_lock` `WouldBlock`）
  を欠いていた → bypass 行追加。(b) `prepare_endpoint`
  拒否表が internal 行（L1 post-take assembly anomaly —
  `debug_assert`'d 未到達・leg restore 済み `Err(Ipc)`）と
  RT contract 文（L1 shared mutex + L0 `Shared` `core`
  cell lock の both-locks 記述）を欠いていた → 表へ row
  追加 + RT 節へ both-locks 明記。(c) `lib.rs` crate doc の
  RT contract が `latency_frames` の行を欠いていた →
  `lifecycle_state` と同じ non-RT-for-L1 行として追加。
- **F4 — L1 bounds matrix 行（fixed）**: 初版の
  `prepare_endpoint_bounds_matrix` は L0 経路のみで 7 条件
  を pin していた → L1 経路へ同 matrix を拡張（rejected
  attach は leg を動かさない — 各拒否後に同一 instance が
  `bounds(512,2)` で attach 可能であることを pin。公開面の
  rejection 契約は route 非依存であるべき）。
- **F5 — task-doc bounds-split note（fixed）**: task doc が
  prelude 所有の bounds 検査に `≥ activated max_frames` を
  含めて記述していたが、L0 では `activated` が
  `core.lock()` 下でしか読めず public prelude では検査不能
  — uniform 半分（非 0・wire 上限）のみが prelude の担当で
  activated 半分は route 内側に残る旨の補足行を task doc
  へ追記（実装と語彙の整合）。

### Tests（新規 10 件 + doctest 1 件 + e2e assertion 2 箇所）

- `composite.rs`（**9** 件）:
  - `prepare_endpoint_mints_both_routes` — L0 `Shared` attach
    と L1 leg checkout の双方で `Ok` + `bounds()`/
    `latency_frames()` が route 値（L0 `0`・L1 `(depth−1)×512
    = 1024`）。
  - `prepare_endpoint_pins_the_rejection_order` — handle →
    state → presence → bounds の pinned 順序: `Created`/
    `Initialized` + bad bounds → `Lifecycle`・占有 L0/L1 +
    bad bounds → `EndpointUnavailable`・`Plain` + bad
    bounds → `EndpointUnavailable`・stale/never-minted
    handle → `InvalidHandle`（全て先行判定として pin）。
  - `prepare_endpoint_rejects_failed` — L1 `Failed` mirror →
    `Err(Failed)`・L0 は panicking endpoint `process` の
    `fetch_max(Failed)`（union 第 1 writer）観測で
    `Err(Failed)` + bookkeeping converge。
  - `prepare_endpoint_observes_poisoned_cell_as_failed` —
    host-side `process` からの panic unwind が `core` cell
    を poison（mirror は `Processing` 残留）した状態で
    `Err(Failed)` — bounds 0・正値ともに state が勝つ（F2
    regression pin）。
  - `prepare_endpoint_bounds_matrix` — L0/L1 各 7 条件
    （`0`・`> WIRE_MAX_*`・`u32::MAX`・`< activated`）全て
    `Err(Prepare(UnsupportedConfig))` + 拒否は何も消費し
    ない（直後に attach 可能 — L1 は leg 不動を実証）。
  - `endpoint_unavailable_stays_local` — L0 2 本目・L1 leg
    checkout 中の attach・detached leg への host-side
    `process` が全て `EndpointUnavailable`（`Ipc`/
    `Unsupported` に潰れない）+ endpoint Drop → leg 返却
    → drain 後に再 attach 可能。
  - `latency_frames_reports_per_route` — L0 `Created`/
    `Initialized` `None` → `Activated`/`Processing` `Some(0)`・
    L1 armed `Some(lookahead × cap)`・unarmed `None`・stale
    handle `None`・`&self` non-drain（live endpoint の leg
    は復元しない）。
  - `latency_frames_gates_l1_on_the_mirror` — `Initialized`/
    `Failed`/`Destroyed` mirror + armed ring → `None`（F1
    regression pin — L0 parity）。
  - `plugin_mut_none_for_shared_and_l1` — L0 `Shared` `None`・
    L1 `None`・L0 `Plain` `Some` の 3 分岐 parity。
- `l1/mod.rs`（**1** 件）: `latency_frames_gate_vs_armed_read`
  — `Initialized`/`Failed`/`Destroyed` で公開 query `None`
  vs `armed_latency_frames` `Some(1024)`・`Processing` で
  `Some(1024)`・unarmed で両方 `None`（F1 の gate/ungated
  分離を pin）。
- doctest（**1** 件）: `PluginHost::prepare_endpoint` の
  `# Examples` — `create_sendable` opt-in factory →
  Activate → `prepare_endpoint` → `endpoint.latency_frames()
  == 0` + `host.latency_frames(handle) == Some(0)`。
- `tests/l1_proxy_e2e.rs`（assertion **2** 箇所 —
  `command_parity_lifecycle_and_state` 内）: real `Activate`
  後 `Some(1024)`・real `Deactivate` 後 `None`（F1 gate の
  実機 end-to-end 実証 — fixture でなく実 lifecycle 遷移）。

### Decisions During Implementation

- **task doc「新 accessor は不要」記述は superseded**
  （deviation・deliberate）— Work 節は「composite 側の
  state 読みに新 accessor は不要」としたが、L0 の
  mirror/`attached`/lifecycle・L1 の `mirror`/`rt` は全て
  private フィールドであり、prelude が同じ判定を呼ぶには
  `pub(crate)` accessor が構造的に必要だった（公開 API
  増なし — `check_attachable_state`/`check_endpoint_free`/
  `leg_detached` は crate-internal）。代替案（prelude 内で
  `instance_mut` + slot match を内職）は inner の step (1)
  と同じ知識を二重実装するため却下 — accessor 共有が
  DRY で authoritative gate の single home を保つ。
- **bounds 検査の uniform/activated 分割**（deviation・
  deliberate）— prelude は uniform 半分（非 0・
  `≤ WIRE_MAX_*`）のみを `check_uniform_bounds` で検査し、
  `max_frames ≥ activated max_frames` は route 内側の
  activated 層再検査に残す。L0 では `activated` が
  `core.lock()` 下でのみ読め、公開 prelude（no-lock 規律・
  `&mut` であっても state gate は bookkeeping/mirror read
  のみ）では検査不能なため — 観測される拒否順序・`Err`
  型はどちらの層でも同一（task doc 補足として文書化）。
- **`Err(Ipc)` defensive row を表内に保持**（deviation・
  deliberate）— Work 節は「`Err(Ipc)` は公開面の拒否と
  して露出しない」とするが、L1 inner の post-take assembly
  anomaly 経路（`debug_assert`'d 未到達・leg restore 済み）
  は防御的に `Err(Ipc)` を返し得る。表から隠すより
  internal 行として in-table で記録する方が honest —
  公開面の語義「`Ipc` を潰さない = `EndpointUnavailable`
  を wire 型に変換しない」と矛盾しない（この row は
  `EndpointUnavailable` 由来ではなく internal anomaly）。
- **L1 arm の endpoint 簿記値は `armed_latency_frames`**
  （F1 派生）— mint 時の `PluginEndpoint::latency_frames`
  は leg が保持する armed horizon であり、public query の
  mirror gate を簿記 read へ漏らさない。
- **L1 arm は attach 前に `drain_endpoint_channels` を呼ぶ**
  — `&mut` entry-point drain を prelude の presence 判定
  より前に置き、`leg_detached()` が post-return の truth
  を見る（inner の `prepare_endpoint` 冒頭 drain と同じ
  規律 — 二重 drain は安価な no-op）。

## Related Experiments

### Adopted

なし — approved design/plan の確定実装のみ（実験ログなし）。

### Rejected

なし — 代替実装の評価は design の確定事項をそのまま実装した
（新規 experiment なし）。

## Validation

- gate（実 log・全 exit 0）:
  - `mise run fmt-check` — exit 0
  - `mise run check` — exit 0（`cargo check --workspace
    --all-targets`）
  - `mise run lint` — exit 0（`cargo clippy --workspace
    --all-targets -- -D warnings` clean — 消費済み
    `expect(dead_code)` 11 件除去後も `unfulfilled_lint_
    expectations` 残留なし）
  - `mise run build` — exit 0（`cargo build --workspace`）
  - `mise run test` — exit 0（`cargo test --workspace` —
    **55 test binaries 全 pass・0 fail・計 1105 tests**）
  - `mise run test-rt` — exit 0（`cargo test --workspace rt_`
    — plugin-host: lib `rt_` 一致 **8** 件 + `l1_proxy_e2e`
    **4** 件全 pass — 本 task は非 RT mint のため新規 `rt_`
    テストなし・既存 lane 無退行）
  - `mise exec -- cargo test -p ruviform-plugin-host` — exit
    0（lib **65**・`l1-test-worker` **0**・`l1_proxy_e2e`
    **55**・doctest **2** — 新規 10 件 + doctest 1 件を含む）
  - `mise exec -- cargo doc -p ruviform-plugin-host --no-deps`
    — exit 0・**0 warnings**（crate doc 完成形の rustdoc
    健全性 — link/table/section 全て解決）
- 構造確認（実コマンド）:
  - `git grep -n "expect(dead_code)"` — src 内残存は
    `endpoint.rs` `in_scratch` field（T6-owned）と
    `l1/mod.rs` `arm_retire_overflows`（observation-only）の
    2 件のみ — T2/T3 が残した「T4 が呼ぶまで」の橋 11 件
    は全て除去
  - `mise exec -- node .agents/skills/doc-status/scripts/
    doc_status.js audit --json` — `{"blocking":0,"documents":
    332,"findings":[]}` clean
- flake 記録: `l1_proxy_e2e` 系の load-sensitive test（spawn
  handshake timeout 系 — `wedged_child_write_converges_and_
  teardown_is_bounded` 等）が検証セッション中に 1 度 timeout
  失敗を観測、単独/全量再実行で pass（本 task の変更面と
  無関係 — T1/T3 でも同系の観測記録あり。Risks 参照）。

## Non-Goals

- endpoint `process`/`process_audio` の **public shell** —
  Task 6（`docs/tasks/plugin-graph-e2e-and-rt-lane.md`）の
  scope。`PluginEndpoint::in_scratch` field が
  `#[expect(dead_code)]` のまま残るのはこのため（
  `process_audio` の bound-shaped input copy target — T6 が
  消費するまでの意図的残存）。
- `arm_retire_overflows` の caller 配線 — T6 所有ではない
  恒常的な observation-only 観測点（diagnostic 用 —
  `#[cfg_attr(not(test), expect(dead_code))]` 維持が正しい形）。
- video endpoint 経路 — design 確定どおり video は全操作
  non-RT で endpoint 面の対象外（dead row — 表に「
  `PluginHost` が video handle を発行しない」と記すのみ）。
- `bounds.channels` の activated main bus 等値規則 — v1 で
  は channel query 未存在のため non-0・wire 上限 + per-block
  `out.channels() == bound.channels` 契約のみ（design §
  bounds.channels 確定どおり）。
- test-spec `plugin-endpoint-contract-tests.md` の
  `proposed → confirmed` 更新 — endpoint 機構全体（T6
  含む）完了後の final verification で実施（plan の規律
  どおり — 本 task は規則を実装側で担保するのみ）。

## Risks

- **公開 API surface の追加** — `PluginHost::prepare_endpoint`
  /`latency_frames` は public API（pre-1.0 許容・plan 開示
  済み）。`PluginHandle`/`EndpointBounds`/`PluginEndpoint`/
  `HostError` 経由の型は既存公開型のみ — 新規公開型なし。
- **prelude と inner の検査二重性** — state/presence/uniform
  bounds が prelude・activated bounds が inner と層分割
  （uniform/activated split は deliberate）。将来 inner 側
  の bounds 規則が変わると公開面と乖離し得る — 拒否表の
  正本は公開 doc であり、inner 変更時は表との照合が必要。
- **`is_poisoned` union の読み筋** — `Shared` slot の
  `Failed` 観測は mirror load + poison flag の 2 源合成。
  将来 `Shared` cell の locking 規約が変わると union 定義
  を再確認する必要あり（現状は `lock()` `Err` arm・
  `destroy` と同じ判定で一貫）。
- **`l1_proxy_e2e` load-sensitive flake** — spawn handshake
  timeout 系が 1 度観測（再実行で pass・本 task 非起因 —
  T1/T3 でも同系記録）。再発頻度が上がる場合は timeout 値
  または直列化を別途 triage（Follow-ups）。
- **`latency_frames` の L1 mirror gate** — `Activated`/
  `Processing` 以外は `None` を返すため、endpoint が leg
  を持ちつつ mirror が mid-convergence の瞬間は public
  query が `None` を返し得る。簿記値は `armed_latency_
  frames` で分離済み（endpoint 自身の stored 値は影響
  なし — mint 時に固定）。

## Follow-ups

- Task 6 `docs/tasks/plugin-graph-e2e-and-rt-lane.md` —
  `PluginEndpoint::process`/`process_audio` public shell が
  `in_scratch` field（bound-shaped input copy target）と
  `EndpointInner`/`L1Endpoint` の残りの面を消費する —
  `#[expect(dead_code)]` の `in_scratch` field は T6 で除去
  見込み。
- `docs/tasks/engine-plugin-final-verification.md` —
  endpoint 機構全体（T1–T6）完了後の AC-to-test 証跡・
  test-spec `proposed → confirmed`・Full Gate（rustdoc
  warning 0 検証含む）。
- `l1_proxy_e2e` spawn-timeout flake — 再発した場合は
  handshake timeout 値または test 直列化を別途 triage
  （T1/T3 IR に同系記録 — 本 task でも 1 度観測・再実行
  pass）。
- `docs/designs/engine-plugin-host-integration.md` —
  「エラー・拒否面」統一表が public `prepare_endpoint`
  doc として実在することを確認済み。実装と表の drift が
  将来生じた場合は公開 doc の表を正本として同期。
