---
id: "TASK-034Mrt1HhkO2vVKksdYxgA"
type: "task"
status: "todo"
title: "endpoint process/process_audio shell + グラフ E2E + rt_ レーン（bound clip・in-bus-first・source-position 慣行）"
created: "2026-10-26"
updated: "2026-10-26"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-113-engine-plugin-integration.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-113-engine-plugin-integration.md"
    - "docs/tasks/plugin-host-endpoint-surface.md"
    - "docs/tasks/audio-fnmut-node-and-plan-retirement.md"
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
    - "docs/test-specs/plugin-graph-e2e-tests.md"
    - "docs/test-specs/plugin-rt-retirement-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# endpoint process/process_audio shell + グラフ E2E + rt_ レーン（bound clip・in-bus-first・source-position 慣行）

## Work

Plan actions 17–19。`PluginEndpoint` の公開 `process`/`process_audio` shell（bound clip → per-block view → channel-major 複写 → `inner.process`）と、L0/L1 mock-device グラフ E2E・`rt_endpoint_*` no-alloc 実測・退役観測・`Send` compile-time 証拠を実装する。inner の variant gate（L0 3 段・L1 arm drain）は T2/T3 で実装済み — 本 task は**順序固定の shell と E2E/RT 検証面**を所有する。

- **`PluginEndpoint::process`**（`PluginHost::process` と同語彙 — `&[HostInput]`/`&mut [HostOutput]`/`events: EventList<'_>`（by-value — `EventList` は `Copy`）/`transport: Option<TransportInfo>`（by-value — `TransportInfo` は `Copy`）→ `Result<ProcessResult, HostError>` — ctx なし）: host 語彙一貫性を優先（`RtProcessor`/`Io` 準拠は design deferred）。`EventList::empty(block_frames)`（`block_frames: u32` — `event.rs` L192）/`None` transport の v1 既定・`ProcessResult.tail_frames` を保持（`ProcessStatus` ではない）
- **`PluginEndpoint::process_audio`**（`&mut AudioBufferViewMut<'_, f32>`・`events: EventList<'_>`（by-value — `Copy`）・`transport: Option<TransportInfo>`（by-value — `Copy`）・`ctx: &RtContext` → `Result<ProcessResult, HostError>` — **処理順序固定**。`ProcessResult` を返し `tail_frames` を保持 — `ProcessStatus` ではない）:
  1. **bound clip** — `out.channels() == bound.channels` かつ `out.frames() <= bound.max_frames` — 違反は `Err(HostError::Prepare(PrepareError::UnsupportedConfig))` で `out` 不変更（上界のみ — `bound` は上限定数）
  2. **per-block view 構築 + channel-major 複写** — `AudioBufferViewMut::try_new(&mut slab[..channels*frames], channels, frames)`（`data.len()==channels*frames` 前提 — `try_new` は `Result`・panic なし）で in-bus-first 規則の `frames`/`channels`（frames = in-bus → なければ out-bus・channels = 両側なら `min`）の実ブロック寸法 view を `in_scratch` から切出し、`out.data()` を channel-major で `in_scratch` へ複写
  3. **`inner.process(bound, …)`** — variant dispatch（L0 = 3 段経路・L1 = leg `process` + arm drain — T2/T3 の inner が所有）。activated gate の情報源は in-bus-first（`frames`/`channels` は実ビュー・`bound` は上限のみ）
  - `Err` は `out` 不変更（transform 位置は上流 bypass・source 位置は closure が呼出し前に `out.data_mut().fill(0.0)` — **adapter 契約の必須慣行**を crate doc/`endpoint.rs` doc に明記）
  - `Err(Ipc)` は L1 内部の wire 表現不能に留め、公開面に出さない（T4 統一表と整合 — `EndpointInner::L1` は `armed_frame_capacity` pre-gate で over-activated dims を `Err(Prepare(UnsupportedConfig))` で返し、leg `block_data` の `Err(Ipc)` に先行するため `Err(Ipc)` が `process`/`process_audio` の `Err` 経路として露出しない）
- **source-position `fill(0.0)` 慣行**: source 位置の consumer closure は `endpoint.process_audio` の前に `out.data_mut().fill(0.0)` を呼ぶ — `Err`/bypass 時に `out` が上流 stale でなく無音になる。transform 位置は `Err` 不変更（上流 bypass 経路のまま）— doc + E2E で pin
- **Send 証拠（`static_assertions` 採用）**: `expect_send::<PluginEndpoint>()`（`#[cfg(test)]` inline 型関数）は `PluginEndpoint` 具象型に使用 — **enum variant 自体は型ではない**ため `EndpointInner::L0`/`L1` の variant 名を assert の引数に書かず、具象 `L0Endpoint`/`L1Endpoint` 型（crate-private — `src/` 内 `#[cfg(test)]` または lib 配置・外部 `tests/` には置かない）を対象にする。endpoint を capture したノード closure の具体型・`GraphPlan`/`RtPlan`/`OutputDataCallback` capture — `unsafe impl Send` なし。**負側（`Send` でないことの証明）は `static_assertions` dev-dependency の `assert_not_impl_any!` を採用** — 通常の `expect_not_send` 型関数は「型が `Send` を実装しない」を compile 時に表明できないため不適。`Rc`/`RefCell`/`PluginHost`/`L1Instance`/裸 `ErasedPlugin` 参照の capture は `assert_not_impl_any!` で `!Send` を固定（trybuild なし — `ruviform-plugin-host` にも `static_assertions` dev-dep を追加）
- **`tests/plugin_graph_e2e.rs` 新設**（`ruviform-plugin-host` の dev-dep `ruviform-audio`・prod 依存は非循環 — prod `ruviform-audio` に plugin 依存を入れない）:
  - mock `DeviceEnumerator`/`Device`/`OutputStream` double — callback をテストスレッドから手動駆動（`ruviform-audio` 側の callback 取得口/test seam を使う — cpal/実デバイス非依存）
  - Send-capable gain mock factory（`create_sendable` 実装 — gain 値・panic 注入・DropFlag probe）
  - **L0 graph E2E（実 API 経路）**: `PluginHost::new()` → `host.register_factory(Box<dyn PluginFactory>)`（`plugin_id` を L0 に claim — `Route::InProcess` は private で公開 API ではない）→ `host.execute(HostCommand::Instantiate { plugin_id })`（plugin_id で route — caller は層を名指さない）→ `HostReply::Instantiated{handle}` → `Initialize`→`Activate{512}`→`StartProcessing`（`execute(HostCommand::*)` 経由）→ `prepare_endpoint(handle, bounds)` → `move |out: &mut AudioBufferViewMut<'_, f32>, ctx: &RtContext| { out.data_mut().fill(0.0); let _ = endpoint.process_audio(out, EventList::empty(out.frames() as u32), None, ctx); }` を `graph.push(Box::new(closure))`（既存 `Graph::push` — `push_node` という名は存在しない）→ `Session::open` → mock callback で固定入力ブロック → 出力が gain で変換されることを sample 値で検証・`latency_frames(h)==Some(0)`
  - **L1 graph E2E（実 API 経路）**: `CARGO_BIN_EXE_l1-test-worker`（crate 内 `env!` — `l1_proxy_e2e` 同方式）で spawn する `host.register_out_of_process(OutOfProcessSpec{lookahead_blocks:2, ..})`（`plugin_id` を L1 に claim — `Result<(), HostError>`）→ `host.execute(HostCommand::Instantiate { plugin_id })` → `HostReply::Instantiated{handle}` → lifecycle（`execute(HostCommand::*)` 経由）→ `prepare_endpoint(handle, bounds)` → `graph.push` → `Session::open` → per-block 駆動 → `Destroy`・`kill_child` → `Failed`+`InstanceFailed` once+以後無音・`latency_frames == Some(1024)`（`2 × 512`）・遅延未着 pass-through
  - **退役 E2E**: endpoint ノードを含む plan swap/node removal → DropFlag が drain 前に立たず → `Session::drain_retired()`/`teardown`（T5 の `Session` passthrough）で立つ・`Session::retired_overflows()==0`・endpoint drop → L1 leg が `return_tx` で host へ還流し再 attach `Ok`（`&mut` entry 経由）
  - **lifecycle 全体 E2E**: attach 中の `StopProcessing`/`Deactivate`/`Activate`（bound 内）/`Destroy` が defined 挙動で動く — `Deactivate` 後は `Err` bypass/無音・再 `Activate`+`StartProcessing` で再開
  - **source-position 慣行**: bound clip 違反の closure で `Err(Prepare(UnsupportedConfig))` → `fill(0.0)` 済み `out` が無音のまま
  - **graph-attached verb**: attach 中に `set_param`/`SaveState`/`StopProcessing` → `Ok`（callback は busy bypass し得るがハングしない）
- **`rt_endpoint_*` レーン**（`tests/` または inline `#[cfg(test)]` — fixture・warm-up は窓外）: `rt_endpoint_l0_process_no_alloc`（shared core `try_lock` 経路 0 alloc）・`rt_endpoint_l1_process_no_alloc`（`RtLeg` child なし fixture で `process` + arm drain 0 alloc）・`rt_endpoint_arm_apply_no_alloc`（別 generation `Arc<Staging>` の `arm_apply` 0 alloc・旧 ring `Retirer` 行き）・`rt_endpoint_busy_bypass`（busy/poisoned/非 `Processing` で定義済み出力 — alloc なし・ハングなし）
- **dev-dep 配線**: `crates/ruviform-plugin-host/Cargo.toml` の `[dev-dependencies]` に `ruviform-audio = { path = "../ruviform-audio" }` — `Cargo.lock` 差分を記録（prod 依存に `ruviform-audio` を入れない — 循環回避・AC3）

## Classification

- `normal-plan-task`（endpoint 公開 shell + E2E/RT 検証 — T4 + T5 に依存・実機構との接合点）

## Upstream Documents

- `docs/plans/implement-issue-113-engine-plugin-integration.md`（actions 17–19・Files/Ownership）
- `docs/designs/engine-plugin-host-integration.md`（`process`/`process_audio` 順序固定・in-bus-first 規則・bound clip・source-position 慣行・`Send` 証拠・mock device E2E 配置・`rt_` レーン）
- `docs/specs/engine-plugin-host-integration.md`（AC1・AC2・AC5・AC6・AC7・AC8・AC9 — 実グラフ消費・Send 証拠・error/fallback・latency・RT no-alloc・退役・E2E）
- `docs/designs/audio-engine.md`（`Session`/`OutputStream`/callback 形・mock device 前提）
- `docs/designs/plugin-l1-out-of-process.md`（L1 pass-through/遅延未着規則 — endpoint で継承）

## Dependencies

- `docs/plans/implement-issue-113-engine-plugin-integration.md`
- `docs/tasks/plugin-host-endpoint-surface.md`（`PluginHost::prepare_endpoint`/`latency_frames` 公開面・統一 rejection が前提）
- `docs/tasks/audio-fnmut-node-and-plan-retirement.md`（`FnMut` impl・plan 退役シーム・`drain_retired`/`retired_overflows` が前提）

## Affected Files

- `crates/ruviform-plugin-host/src/endpoint.rs` — `PluginEndpoint::process`/`process_audio` 公開 shell（bound clip → per-block view → channel-major 複写 → `inner.process`）+ source-position doc + `expect_send::<PluginEndpoint>()`（`#[cfg(test)]` inline）+ getter（`bounds`/`max_frames`/`latency_frames`/`channels`）+ `static_assertions` assert（`src/` 内 — 具象 `L0Endpoint`/`L1Endpoint`・`assert_not_impl_any!` 負側）
- `crates/ruviform-plugin-host/tests/plugin_graph_e2e.rs`（新規）— mock device double + L0/L1 graph E2E（`register_factory`/`register_out_of_process` + `execute(HostCommand::Instantiate{plugin_id})` + `graph.push`）+ lifecycle + 退役 + leg 還流 + source-position + graph-attached verb
- `crates/ruviform-plugin-host/Cargo.toml` — `[dev-dependencies] ruviform-audio` 追加 + `[dev-dependencies] static_assertions` 追加
- `Cargo.lock` — dev-dep 差分
- `crates/ruviform-plugin-host` 内 inline `#[cfg(test)] mod tests` または `tests/` — `rt_endpoint_*` + `expect_send`/`static_assertions` 型証明 + E2E 補助 unit

## Done When

- [ ] `PluginEndpoint::process`/`process_audio` が存在し、`process` が `events: EventList<'_>`・`transport: Option<TransportInfo>` を by-value（両 `Copy`）で受け `Result<ProcessResult, HostError>` を返し、`process_audio` が順序固定（(1) bound clip → (2) per-block `AudioBufferViewMut::try_new` view + channel-major 複写 → (3) `inner.process`）で `Result<ProcessResult, HostError>`（`tail_frames` 保持 — `ProcessStatus` ではない）を返す
- [ ] bound clip（`out.channels()==bound.channels`・`frames ≤ bound.max_frames`）違反が `Err(Prepare(UnsupportedConfig))` で `out` 不変更 — activated gate が in-bus-first（frames/channels は実ビュー・bound は上限のみ）
- [ ] `Err` 時 `out` 不変更 + source-position `fill(0.0)` 慣行が doc + E2E で pin される（transform 位置は上流 bypass・source 位置は無音）・`Err(Ipc)` が `process`/`process_audio` の公開面から露出しない（`armed_frame_capacity` pre-gate）
- [ ] `expect_send::<PluginEndpoint>()`（`#[cfg(test)]` inline）+ 具象 `L0Endpoint`/`L1Endpoint`・ノード closure 型・`GraphPlan`/`RtPlan`/callback capture が `Send` — `unsafe impl` なし・enum variant を型名として assert しない・`static_assertions` dev-dep の `assert_not_impl_any!` で `!Send` 側（`Rc`/`PluginHost`/`L1Instance` 等）も固定
- [ ] `tests/plugin_graph_e2e.rs` が mock device で L0/L1 graph E2E（`register_factory`/`register_out_of_process` + `execute(HostCommand::Instantiate{plugin_id})` の実 API 経路・`graph.push` — `push_node`/`PluginHost::instantiate(InProcess)` を使わない — gain 正当性・lifecycle・`kill_child`→`Failed`+通知 once+無音・latency 値・退役 DropFlag・leg 還流・source-position・verb 到達）をカバーし、`CARGO_BIN_EXE_l1-test-worker` が crate 内に閉じる
- [ ] `rt_endpoint_l0_process_no_alloc`/`rt_endpoint_l1_process_no_alloc`/`rt_endpoint_arm_apply_no_alloc`/`rt_endpoint_busy_bypass` が `assert_no_alloc` 窓内で通る（fixture 窓外・実一致件数記録）
- [ ] `ruviform-audio` の prod 依存に plugin クレートがない（dev-dep は `ruviform-plugin-host` 側のみ — AC3。`static_assertions`/`ruviform-audio` とも dev-dep のみ）
- [ ] narrow gate（下記 Verification）が exit 0 — E2E・`rt_`・Send 証拠が通る

## Verification

- [ ] `mise run check` — exit 0
- [ ] `mise run lint` — exit 0
- [ ] `mise run test` — exit 0（既存テスト無退行 + `plugin_graph_e2e` E2E + `expect_send` 型証明 + unit）
- [ ] `mise run test-rt` — exit 0・**実一致件数を記録**（`rt_endpoint_*` 4 本 + `rt_plan_*`/`rt_fn_node_*`（T5）+ 既存 `rt_` — 0 件スキップを成功と偽らない）
- [ ] `mise exec -- cargo test -p ruviform-plugin-host --test plugin_graph_e2e` — E2E 単体実行
- [ ] `mise exec -- cargo tree -p ruviform-audio` — DAG に `ruviform-plugin-*` が出現しない（AC3）
- [ ] 対象 test-spec: `docs/test-specs/plugin-graph-e2e-tests.md`（AC1/AC3/AC5/AC6/AC9 の E2E 面）・`docs/test-specs/plugin-rt-retirement-tests.md`（`rt_endpoint_*`・退役観測）・`docs/test-specs/plugin-endpoint-contract-tests.md`（`Send` 証拠・`process`/`process_audio` 契約）
