---
id: "IMPL-034MGZ77fXX8R8KvK7aHuy"
type: "impl"
status: "completed"
title: "Issue #113 plugin graph E2E + RT lane — Task 6 implementation"
created: "2026-10-26"
updated: "2026-10-26"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/plugin_graph_e2e.rs"
      - type: "file"
        path: "docs/impl/ir/issue-113-plugin-graph-e2e-and-rt-lane-task-6-implementation.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/endpoint.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/host.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/lib.rs"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "docs/tasks/plugin-graph-e2e-and-rt-lane.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/plugin-graph-e2e-and-rt-lane.md"
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

# Issue #113 plugin graph E2E + RT lane — Task 6 implementation

## Summary

Issue #113 Task 6 完了 — plan actions 17–19（endpoint 公開
`process`/`process_audio` shell + グラフ E2E + `rt_` レーン）。
(a) `PluginEndpoint::process_audio(&mut self, &mut
AudioBufferViewMut<'_, f32>, EventList<'_>,
Option<TransportInfo>, &RtContext) -> Result<ProcessResult,
HostError>` を実装 — 順序固定の 3 段 shell: (1) **bound
clip**（`out.channels() == bound.channels` かつ `out.frames()
<= bound.max_frames` — 違反は `Err(HostError::Prepare(
PrepareError::UnsupportedConfig))` で `out` 不変更・clip は
全てに先行）、(2) **per-block view + channel-major stage-in**
（attach 時 prealloc の `in_scratch` slab の先頭 `channels ×
frames` を `AudioBufferViewMut::try_new` で実ブロック寸法の
入力バスとして切出し、`out.data()` を channel-major で複写）、
(3) **`inner.process(HostInput(&input), HostOutput(out),
events, transport)`** — plugin は `out` へ**直接書込み**
（copy-back なし。uniform copy-in は spec-pin 済み v1 規則 —
in-place 最適化は deferred）。`Err` は `out` 不変更・
`ProcessResult`（`tail_frames` 保持）を透過。(b) attach 時
getter `max_frames()`/`channels()`（`const`・`#[must_use]`・
RT-contract doc 付き）追加・`in_scratch` の `#[expect(
dead_code)]` 除去（reader が着弾）。(c) 新規 e2e suite
`tests/plugin_graph_e2e.rs`（13 テスト）— mock
`DeviceEnumerator`/`Device`/`OutputStream` が `Session::open`
の**実 callback** を capture しテストスレッドから手動駆動
（cpal/実デバイス非依存）。(d) dev-dep 配線 `ruviform-audio =
{ path = "../ruviform-audio" }` を `[dev-dependencies]` のみに
（prod 非循環 — AC3 を `cargo tree` で実証）。独立レビュー
確定修正を全適用（dead recorder の assert 化・`Gain.dropped`
probe の配線と drain 時点の意味論修正・L1 rt_ 窓内 matched
branch 化・`transform_node` Mutex の test-harness 明記）。
全 gate green 実 log。

## Context

approved plan
`docs/plans/implement-issue-113-engine-plugin-integration.md`
の Task 6（`docs/tasks/plugin-graph-e2e-and-rt-lane.md`・
actions 17–19）。upstream: approved design
`docs/designs/engine-plugin-host-integration.md`
（`process`/`process_audio` 順序固定・in-bus-first 規則・
bound clip・source-position `fill(0.0)` 慣行・Send 証拠・
mock device E2E 配置・`rt_` レーン）・spec
`docs/specs/engine-plugin-host-integration.md`（AC1・AC2・
AC3・AC5・AC6・AC7・AC8・AC9）・TSpec
`docs/test-specs/plugin-graph-e2e-tests.md` /
`docs/test-specs/plugin-rt-retirement-tests.md` /
`docs/test-specs/plugin-endpoint-contract-tests.md`。
前提完了 task: T4（`plugin-host-endpoint-surface` —
`prepare_endpoint`/`latency_frames`/統一拒否表）・T5
（`audio-fnmut-node-and-plan-retirement` — `FnMut` blanket
impl・plan 退役シーム・`Session::drain_retired`/
`retired_overflows` passthrough）。inner の variant gate
（L0 3 段・L1 arm drain）は T2/T3 済み — 本 task は**順序
固定の shell と E2E/RT 検証面**のみ所有する。

## Implementation

### Changes

- `crates/ruviform-plugin-host/src/endpoint.rs`:
  - module doc — `process_audio` 段落を追加: transform
    semantics（`pull` は read-modify-write in place）と
    adapter 契約の source-position 慣行（`Err` は「`out`
    不変更」のみを意味し、transform 位置では上流 bypass、
    **source 位置では consuming closure が
    `out.data_mut().fill(0.0)` を先に呼ぶ**ことで bypass/
    reject 時に stale でなく無音になる）を明記。
  - `in_scratch` field — `#[expect(dead_code)]` 除去
    （`process_audio` が reader として着弾）。
  - `max_frames()`/`channels()` — `pub const fn`・
    `#[must_use]`・「RT contract: RT-suitable — const, pure」
    doc。attach 時 bound の ceiling/channel 契約を report
    （graph 側が `process_audio` を呼ばずに寸法を引ける）。
  - **`process_audio`** — signature は task doc 確定形
    （`events`/`transport` は by-value — 両 `Copy`・`_ctx`
    は RT-section capability token として受け取るが読まない
    — `pull` signature を通じて RT 契約節に留める慣行）。
    disjoint field borrow（`inner`/`bound`/`in_scratch`）で
    stage-in と dispatch を両立。(1) clip は slice の in-bounds
    前提そのものなので全てに先行。(2) `try_new` は
    `channels * frames` 長の slice で不敗だが `Err` を
    `Prepare(UnsupportedConfig)` へ写像して定義済みのまま
    （panic しない）。(3) `inputs = [HostInput(&input)]`・
    `outputs = [HostOutput(out)]` — plugin は `out` へ直接
    書き、`Err` 経路は `out` untouched。
  - `#[cfg(test)]` — `L1HostEnds` fixture に `submit_rx:
    Consumer<Submit>`・`result_tx: Producer<u64>`・`staging:
    Arc<Staging>` を追加（以前は `_` 破棄 — review 修正で
    child-sim 駆動に必要）+ `complete_submits(&mut
    L1HostEnds)` helper（doorbell drain → publish-last 規則で
    out-slot の `frames`/`channels`/pattern samples → `seq`
    tag を store → `result_tx.try_push(seq)` — 次の
    `expected` seq が **matched** 解決する branch を窓内で
    踏む。bounded `try_pop`/`try_push` + atomic store のみで
    `assert_no_alloc` 窓内安全）。
  - 新規 unit/`rt_` テスト 5 件（下記 Tests 節）。
- `crates/ruviform-plugin-host/src/host.rs`:
  - `run_audio` helper（mono/stereo slab 経由の
    `process_audio` 1 ブロック — graph node `pull` 形状）。
  - `rt_endpoint_l0_process_no_alloc`（bound clip →
    `in_scratch` copy → mirror read → `try_lock` →
    `catch_unwind` → plugin call が 0 alloc — warm-up は窓外・
    gain ×2 で 0.5→1.0 を検証）と `rt_endpoint_busy_bypass`
    （held-lock `Err(EndpointUnavailable)`・poisoned cell の
    `Failed` 収束 `Ok(CONTINUE)`+silence・非 `Processing`
    `Err(Lifecycle)` の 3 行を各 `assert_no_alloc` 窓内で —
    alloc なし・ハングなし）。
- `crates/ruviform-plugin-host/src/lib.rs` — crate doc の
  RT 契約表を更新: `process_audio` を「`process` の
  graph-facing sibling（single-bus `pull`-shaped shell・同一
  RT 契約）」と名指し + attach-time getter
  （`bounds`/`max_frames`/`channels`/`latency_frames`）を
  const-cheap と明記（review 修正 — RT-contract 語彙で
  process_audio+getter を crate doc から引けるように）。
- `crates/ruviform-plugin-host/tests/plugin_graph_e2e.rs`
  （新規・1177 行・13 テスト）:
  - mock `DeviceEnumerator`/`Device`/`OutputStream` —
    `open_output_stream` が `OutputDataCallback` を
    `Arc<Mutex<Option<…>>>` slot に capture し、
    `take_callback` でテストスレッドから手動駆動（cpal/実
    デバイス非依存・**実 `Session::open` callback 経路**を
    通る）。
  - `Gain`/`GainFactory` — `create_sendable` 実装の
    Send-capable mock（gain 値・`panic` 注入 flag・
    `dropped: Option<Arc<AtomicBool>>` teardown probe）。
  - `transform_node` — `out` を上流→結果へ read-modify-write
    する closure。`results: Arc<Mutex<Vec<Result<
    ProcessResult, HostError>>>>` recorder 付き —
    **test-harness recorder であることを doc 明記**
    （uncontended by construction — テストスレッドが唯一の
    callback driver。production RT pattern ではない —
    review 修正）。
  - `source_node` — `fill(0.0)` 慣行を `process_audio` 前に
    実行する source-position closure。
  - `EndpointNode` — endpoint + `Drop` probe
    （`Arc<AtomicBool>`）を持つ具象 `AudioNode` — 退役 plan
    の destructor が**いつ**走るかを pin（RT pull 上では
    決して走らない）。
  - Send 証拠 — `static_assertions::assert_impl_all!` で
    `PluginEndpoint: Send`・`GraphPlan: Send+Sync`・
    `RtPlan: Send`・`PlanSwap: Send+Sync`・`Session: Send`・
    `OutputDataCallback: Send`（負側 `assert_not_impl_any!`
    は `src/` 内に既存 — T4/T5）。`expect_send::<T>` 型関数
    で endpoint-capturing closure 具象型の `Send` も検証
    （`endpoint_capturing_closure_is_send`）。`unsafe impl`
    なし。
  - L0 e2e — `register_factory` + `execute(HostCommand::
    Instantiate{plugin_id})` + lifecycle verb（`execute`
    経由）+ `prepare_endpoint` + `graph.push` + `Session::
    open` の実 API 経路: `l0_endpoint_node_drives_gain_
    through_session`（0.5→1.0 gain・8 ブロック全
    `Ok(CONTINUE)`・attach 時 report 値 bounds/max_frames/
    channels/latency_frames=0・`host.latency_frames ==
    Some(0)`）・`l0_panic_converges_failed_with_silence`
    （panic → `Ok(CONTINUE)`+silence・`lifecycle_state ==
    Failed`・`SaveState` が `Err(Failed)` — `Err` でも
    ハングでもない defined row）・`l0_lifecycle_while_
    attached`（StopProcessing → `Err` bypass・Deactivate →
    同・bound 内 Activate+StartProcessing → 再開・over-bound
    Activate → `Err(Prepare)`・Destroy 後 `Err(Lifecycle)`）・
    `l0_graph_attached_verbs_answer`（attach 中に
    `SaveState`/`RestoreState`/`StopProcessing`/
    `StartProcessing` → `Ok`・callback は hang しない —
    **deviation: v1 surface に `SetParam` は存在しない**
    ため task doc の `set_param` 行は Save/Restore/Stop/
    Start で代替カバー）。
  - bound-clip/adapter 慣行 — `bound_clip_err_leaves_out_
    unchanged`（mono vs stereo bound・1024 > 512 frames の両
    方で `Err(Prepare(UnsupportedConfig))`+`out` 不変更・
    recorder が `Err` 2 件を検証）・`source_position_err_
    yields_silence_not_stale`（9.0-stale が `fill(0.0)` 済み
    `out` で無音に — **9.0-stale vs 0.0-silence の区別**を
    transform/source 両位置で pin）。
  - 退役 e2e — `plan_swap_retires_endpoint_node_on_drain`
    （`Session::swap` で endpoint ノード退避 → `active()` が
    displaced+cached retire → drain 前 `!dropped` →
    `drain_retired()==1` で node+endpoint destructor 走破・
    `retired_overflows()==0`・L0 epoch 解放で再 attach Ok。
    **`Gain.dropped` probe を配線 — deviation**（後述））・
    `session_teardown_collects_retired_plan`（callback drop
    → `Drop for RtPlan` retire → collector drop で session
    teardown 回収 — 明示 drain なし）・`l1_leg_returns_
    through_retired_plan_drain`（checkout 中の二重 attach は
    `Err(EndpointUnavailable)` → drain で endpoint drop が
    `return_tx` へ leg 還流 → `prepare_endpoint` の `&mut`
    drain で復帰・再 attach Ok・`latency_frames()==1024`）。
  - L1 e2e — `CARGO_BIN_EXE_l1-test-worker` で spawn する
    `register_out_of_process(OutOfProcessSpec{lookahead_
    blocks:2, ..})` + `Instantiate` + lifecycle 実 API 経路:
    `l1_endpoint_node_drives_worker_gain_through_session`
    （latency `2 × 512 = 1024`・序盤 pass-through silence・
    32 ブロック paced（5ms/block — `l1_proxy_e2e` 同様の
    real-time-scale pace）で `matched >= 16`・全 sample は
    defined value（gained input か silence — torn なし）・
    全 block `Ok(CONTINUE)`）・`l1_kill_child_converges_
    failed_once_then_silence`（`kill_child` → `Failed` 収束
    → 以後 `Ok(CONTINUE)`+silence・`InstanceFailed` 通知が
    **ちょうど 1 回**・post-Failed `SaveState` →
    `Err(Failed)`・`Destroy` は clean）。
  - `rt_endpoint_graph_pull_no_alloc` — `RtPlan::active` →
    `GraphPlan::drive` → `Graph::pull` → endpoint-capturing
    `FnMut` node → `process_audio` の**全 RT block 経路**が
    `assert_no_alloc` 窓内 0 alloc（fixture・`RtContext`・
    `PlanSwap`・warm-up は窓外）。
- `crates/ruviform-plugin-host/Cargo.toml` —
  `[dev-dependencies]` に `ruviform-audio = { path =
  "../ruviform-audio" }` 追加（AC3 の非循環方向を comment で
  pin — engine 側に plugin 依存を入れない）。
- `Cargo.lock` — `ruviform-plugin-host` deps に
  `ruviform-audio` edge 1 行のみ（dev edge — 最小差分を実
  diff で確認）。
- `docs/tasks/plugin-graph-e2e-and-rt-lane.md` — status
  `done`・`implemented-by` 本 IR・changes 記録・checkbox 全
  tick + 実数値の補足 line。
- `docs/tasks/README.md`（T6 行 `done`）・
  `docs/impl/ir/README.md`（本 IR の index 行）。

### Tests（新規: lib unit/rt_ 5 件 + host rt_ 2 件 + e2e 13 件）

- `endpoint.rs` `#[cfg(test)]`（+5）:
  - `process_audio_bound_clip_rejects_with_out_unchanged` —
    channels 不一致・frames 超過の両 row で
    `Err(Prepare(UnsupportedConfig))`+`out` 不変更。
  - `process_audio_err_leaves_out_unchanged` — 非
    `Processing` mirror で `Err(Lifecycle)`+`out` 不変更
    （transform-position bypass row）。
  - `process_audio_stages_and_writes_back` — stage-in 経路の
    end-to-end（upstream が leg input へ届き defined output が
    `out` へ書戻る — child なし leg の pass-through 行は
    初受信前 silence）。
  - `rt_endpoint_l1_process_no_alloc` — `process_audio`
    shell の L1 leg 全経路（clip・slab copy・arm drain・
    mirror gate・dims pre-gate・stage/doorbell・**matched
    解決**）が 0 alloc — warm-up 2 ブロック +
    `complete_submits` interleave で lookahead を満たし
    matched branch を窓内で踏む（review 修正 — 従来は
    pass-through のみ）。
  - `rt_endpoint_arm_apply_no_alloc` — 別 generation
    `Arc<Staging>` の `arm_apply`（in-capacity resize +
    旧 ring `Retirer` 行き）が 0 alloc — gen 2 適用・
    collector `drain()==1` を窓外で検証。
  - `rt_l1_endpoint_process_no_alloc`（既存）— 同じく
    `complete_submits` interleave で matched branch を窓内
    カバーへ強化 + `all_eq(&output, 0.25)` で pipeline 解決を
    検証。
- `host.rs` `#[cfg(test)]`（+2）: `rt_endpoint_l0_process_no_
  alloc`・`rt_endpoint_busy_bypass`（上記 Changes 節）。
- `plugin_graph_e2e.rs`（13 — 上記 Changes 節の一覧）。
- dead recorder の assert 化（review 修正）— panic/verb/
  bound-clip/kill の各 e2e で `results` recorder を記録する
  だけでなく内容を assert（`Ok(CONTINUE)` 列・`Err` 列・
  「全 block Ok で Err なし」の pinned claim）。

### Review findings and resolutions（実装レビュー確定修正 — 全適用）

- **crate doc の RT-contract 名指し（fixed）** — `lib.rs`
  RT 契約表が `process` のみ言及だったため、
  `process_audio`（graph-facing sibling・同一 RT 契約）と
  attach-time getter 群（const-cheap）を明記。
- **dead `results` recorder（fixed）** — 記録のみで assert
  されていなかった recorder を全て内容検証へ（e2e の
  「defined output row」claim を data で pin）。
- **`Gain.dropped` probe の配線と意味論修正（fixed —
  deviation）** — plugin 側 probe を plan-swap 退役テストへ
  配線。ただし plugin flag は drain 時点では**立てられない**
  — `SendCore` cell は host slot が `Arc` 共有しており、
  retired plan の drop は endpoint 側 handle を 1 つ減らす
  だけで plugin instance は生存する。したがって assert は
  「drain 時点 `!dropped`（host slot が生存 — teardown
  chain が RT pull 上で走らないことの pin）+ clean
  lifecycle teardown（StopProcessing→Deactivate→Destroy）
  後に `dropped`（最後の `SendCore` handle が非 RT
  command path で解放）」の 2 点に分離。node 側
  `EndpointNode.dropped` probe が drain 即時性を担う。
- **L1 `rt_` 窓内 matched branch 化（fixed）** —
  `rt_l1_endpoint_process_no_alloc` /
  `rt_endpoint_l1_process_no_alloc` が pass-through 解決のみを
  計測していた（child 不在で result が届かない）問題を、
  `L1HostEnds` へ `submit_rx`/`result_tx`/`staging` を保持 +
  `complete_submits` child-sim（publish-last 規則で実
  out-slot data を公開）+ warm-up で lookahead 充足により、
  **matched-result branch（drain → out-slot check →
  `copy_out` → `last_output`）を窓内で実測**する形へ修正。
- **`transform_node` の `Mutex` を test-harness 明記
  （fixed）** — recorder `Mutex` は uncontended by
  construction（テストスレッドが唯一の callback driver）で
  あり production RT pattern ではない旨を doc で pin。

### Decisions During Implementation

- **uniform copy-in（stage-in）は spec-pin 済み v1 規則** —
  `out` → `in_scratch` の channel-major 複写を常に行い、
  plugin は `out` へ直接書く（copy-back なし）。upstream
  と同一バッファで plugin が in-place 処理する最適化は
  adapter 層の契約拡張を要するため deferred。
- **`try_new` の `Err` 写像は `Prepare(UnsupportedConfig)`**
  — slice 長が正確に `channels * frames` で不敗だが、
  panic-free の公開契約を保つため defined `Err` を維持。
- **`_ctx: &RtContext` は読まない** — RT-section capability
  token として受け取るのみ（`pull` signature 経由で RT 契約
  節内に留める慣行 — design 確定）。
- **mock stream は callback capture のみ** — 実 `Session::
  open` 経路を通る一方、negotiated buffer size の強制は
  しない（follow-up 記録）。
- **L1 e2e の pacing/assert は `l1_proxy_e2e` 慣行を継承**
  — 5ms/block の real-time-scale pace・`matched >= 16` の
  generous bound（loaded CI host 向け headroom）・
  `is_defined_l1_sample`（silence または gained-input のみ
  許可）— flake surface も含めて既存慣行に揃える。

## Related Experiments

### Adopted

なし — approved design/plan の確定実装のみ（実験ログなし）。

### Rejected

なし — copy-in 省略の in-place 最適化等の代替は design
段階で v1 不採（spec-pin 済み uniform copy-in）。本 task で
新規評価なし。

## Validation

- gate（実 log・全 exit 0 — 本 IR 記録時に再実行）:
  - `mise run fmt-check` — exit 0
  - `mise run check` — exit 0（`cargo check --workspace
    --all-targets`）
  - `mise run lint` — exit 0（`cargo clippy --workspace
    --all-targets -- -D warnings` clean）
  - `mise run test` — exit 0（`cargo test --workspace` —
    **56 test binaries・1125 tests 全 pass・0 failed**）
  - `mise run test-rt` — exit 0・**実一致件数 129**（workspace
    全体。ruviform-plugin-host 内訳: lib **12** — `rt_` 真
    prefix 11 + substring 一致 `shared_start_failure_rolls_
    back_to_activated` 1・`l1_proxy_e2e` **4**・
    `plugin_graph_e2e` **1** = `rt_endpoint_graph_pull_no_
    alloc`。新規 `rt_endpoint_*` は spec 4 本（`rt_endpoint_
    l0_process_no_alloc`/`rt_endpoint_l1_process_no_alloc`/
    `rt_endpoint_arm_apply_no_alloc`/`rt_endpoint_busy_
    bypass`）+ graph pull 1 本 — 0 件スキップ・0 件
    バイナリなし）
  - `mise exec -- cargo test -p ruviform-plugin-host` —
    exit 0（lib **72** + l1-test-worker bin **0** +
    `l1_proxy_e2e` **55** + `plugin_graph_e2e` **13** +
    doctest **2** = **142**）
  - `mise exec -- cargo test -p ruviform-plugin-host --test
    plugin_graph_e2e` — exit 0（13 全 pass・上記の内訳）
  - `mise exec -- cargo doc -p ruviform-plugin-host
    --no-deps` — exit 0・warning 0
- 依存監査（実コマンド）:
  - `mise exec -- cargo tree -p ruviform-audio` — deps は
    `ruviform-audio-core`/`ruviform-media-core`/`ruviform-rt`
    のみで `ruviform-plugin-*` 出現なし（**AC3 実証**。
    dev-deps は `assert_no_alloc` + `static_assertions` のみ）
  - `mise exec -- cargo tree -p ruviform-plugin-host
    --edges normal` — `ruviform-audio-core`（+
    `ruviform-media-core`）・`ruviform-plugin-core`（+
    media/video-core）・`ruviform-rt`（arc-swap/rtrb）・
    `shared_memory` のみ。**`ruviform-audio` は normal
    edges に出現しない**（dev-only を実証 — `--edges
    normal` を使用したのは `cargo tree` に `--no-dev`
    flag が存在しないため・deviation 記録）。
- 構造確認:
  - `rg "fn rt_" crates/ruviform-plugin-host` — src 内 11 件
    （新規 4 件: `rt_endpoint_l0_process_no_alloc`/
    `rt_endpoint_l1_process_no_alloc`/`rt_endpoint_arm_
    apply_no_alloc`/`rt_endpoint_busy_bypass`）+ tests/
    3 件（`rt_endpoint_graph_pull_no_alloc` 新規 +
    `l1_proxy_e2e` 2 件既存）。
  - `Cargo.lock` diff — `ruviform-plugin-host` deps に
    `ruviform-audio` 1 行のみ（最小 dev edge）。

## Non-Goals

- `RtProcessor`/`Io` 準拠の `process_audio` 形 — design で
  deferred と確定（host 語彙一貫性を優先）。
- copy-in 省略の in-place 最適化 — spec-pin 済み uniform
  copy-in の v1 規則を維持。
- `process_audio` の multi-bus 化 — v1 は single-bus shell。
- mock device 側の negotiated buffer size 強制 — mock は
  callback capture seam のみ（follow-up）。
- test-spec の `proposed → confirmed` 更新 — final
  verification（Task 7）の scope。
- `Route::InProcess` の公開 — private のまま（task doc
  確定 — caller は `plugin_id` で route し層を名指さない）。

## Risks

- **`cargo tree --edges normal` は dev-dep を隠す** — 監査は
  `cargo tree -p ruviform-audio`（plugin 非出現）と
  `--edges normal`（`ruviform-audio` 非出現）の両面で実施
  済みだが、CI への恒常組込みはない（Task 7 での full gate
  再監査が担保）。
- **L1 e2e の timing 依存** — 実 worker process + 5ms/block
  pace + `matched >= 16` は環境負荷に対する flake surface
  （`l1_proxy_e2e` 継承の慣行 — 過去実績で headroom は
  充分）。
- **`shared_start_failure_rolls_back_to_activated` が `rt_`
  filter に substring 一致** — `test-rt` の「実一致件数」に
  ノイズとして 1 件混入する（`rt_` prefix 規約の限界 —
  filter は substring。既知の計数クセとして記録）。

## Follow-ups

- mock `OutputStream` が negotiated buffer size を強制しない
  — callback config を検査する assertion 追加の余地。
- L1 `matched >= 16` の flake surface — `l1_proxy_e2e` 継承
  慣行。閾値/pace の根拠は T7 の spec confirm 時に再評価。
- `RtPlan` の `Option` wrap（session.rs）・`DropCollector::
  take_queued`・`GraphPlan::drive` の poison 永久無音の根治
  — T5 IR 由来の follow-up を継続（本 task では変更なし）。
- test-spec 3 本（graph-e2e・rt-retirement・endpoint-
  contract）の `proposed → confirmed` — Task 7。
