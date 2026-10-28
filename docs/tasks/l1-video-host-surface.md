---
id: "TASK-034NXuq6kT7i5UmNviPjy6"
type: "task"
status: "todo"
title: "L1 video host surface（video() façade・Route/Slot video arm・descriptors kind-tag・create_video_instance・Creating ゲート・vshm::Segment・VideoL1Instance）"
created: "2026-10-28"
updated: "2026-10-29"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/118"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/video.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/vshm.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/video.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/composite.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/mod.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/Cargo.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-118-video-l1.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-118-video-l1.md"
    - "docs/tasks/l1-wire-v3-video-codec.md"
  blocks:
    - "docs/tasks/l1-video-fixture-and-e2e.md"
    - "docs/tasks/l1-video-docs-sweep.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/l1-video-runner-and-erased-surface.md"
    - "docs/tasks/l1-child-instance-split.md"
    - "docs/tasks/l1-route-pool-and-isolation-surface.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-118-video-l1.md"
    - "docs/designs/issue-118-video-l1.md"
    - "docs/adr/issue-118-video-l1-host-surface.md"
    - "docs/adr/issue-118-video-l1-create-path-segment.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-118-video-failed-parity-tests.md"
    - "docs/test-specs/issue-118-video-e2e-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/76"
    - "https://github.com/scarecrow173/ruviform/issues/117"
  defers: []
  deferred-by: []
---

# L1 video host surface（video() façade・Route/Slot video arm・descriptors kind-tag・create_video_instance・Creating ゲート・vshm::Segment・VideoL1Instance）

## Work

host 側の video 表面 — `video()` 借用 façade・`Route::OutOfProcessVideo`/`VideoSpawnSpec`・`Slot::OutOfProcessVideo` 全 match-site arm・`descriptors` kind-tag・`InstKind` 分離・`create_video_instance`・**`Creating` ゲート拡張**・`request_timed`/`render_timeout`・`l1/vshm.rs`・`VideoL1Instance`。audio 公開面（`HostCommand`/`execute`/`OutOfProcessSpec`/`OutOfProcessIsolation`）は無変更。T2 とは file 非交差で並行可能。

1. `crates/ruviform-plugin-host/src/video.rs`（新規 module）: `VideoHostCommand`/`VideoHostReply`（両 `#[non_exhaustive]` — design「host 公開面」の variant 表どおり: `Scan`/`Instantiate{plugin_id, config: VideoProcessConfig}`/`Destroy`/`Initialize`/`Activate`/`StartProcessing`/`Render{handle, input: VideoFrame, at: MediaTime}`/`StopProcessing`/`Deactivate`/`GetParams`/`GetParamInfo{handle, id}`/`GetParamValue{handle, id}`/`SetParamValue{handle, id, value}`）。`VideoOutOfProcessSpec{descriptor, program, args, command_timeout(10s), render_timeout(60s)}` — **`isolation`/`lookahead_blocks` 字段を持たない**（Dedicated 固定の構造 pin — Bundled video を公開面上で表現不能にする）。`VideoPluginHost<'a>{ &'a mut PluginHost }` — `register_out_of_process(spec)`・`execute(VideoHostCommand)`・`lifecycle_state`/`kill_child`/`try_recv_notification` delegate。`descriptor(handle)` — `PluginHost` に同名メソッドは存在しないため `descriptors` 登録表から route 非依存に引く薄い lookup として video view 側に実装する（plan Open Question — 提供する場合はここに、提供しない場合は doc に「v1 未提供」と明記）
2. `composite.rs` — `Route::OutOfProcessVideo{spec: VideoSpawnSpec}` variant 追加（`pool` 字段なし — Dedicated のみ）。`VideoSpawnSpec` は新規 minimal 型（`{descriptor, program, args, command_timeout, render_timeout}` — `SpawnSpec` を再利用しない・`lookahead_blocks`/`instance_depth()` は audio doorbell ring の概念で dead field）。登録衝突は map key で `Err(DuplicatePluginId)`（双方向）
3. `composite.rs` — **`descriptors` kind-tag**: `Scan` reply は `self.descriptors.clone()` 全量返却のため、各 entry に登録面の kind を持たせる（`register_factory`/`register_out_of_process` → audio-kind・video `register_out_of_process` → video-kind）。audio `HostCommand::Scan` は audio-kind のみ・video `Scan` は video-kind のみ列挙（shadowed 登録は自分の登録面の kind で残る — 現行意味論を kind 内に維持）
4. `composite.rs` — **`Slot::OutOfProcessVideo(Box<VideoL1Instance>)` 追加 + 全 match-site arm**（design「`Slot` match site 全網羅」表どおり — 12 site）: `execute`→`HostCommand::Scan`（kind filter・item 3）・`instantiate`→`Route` match（`Route::OutOfProcessVideo` arm → `Err(UnknownPluginId)` — audio surface から instantiate 不能）・`dispatch`→audio `HostCommand`（video slot → `Err(Unsupported)`）・`destroy`（動く — mirror transition + destroy + slot 解放は kind 非依存）・`kill_child`（動く — Dedicated で per-instance=per-child）・**`try_recv_notification`（必須 arm — video `notify_rx` を drain し `InstanceFailed{handle, exit}` を mint。現行 `let Some(Slot::OutOfProcess(_))` は video slot を静かに skip し通知が永遠に失われる）**・`lifecycle_state`（`Some(mirror)`）・`latency_frames`（`None`）・`plugin_mut`（`None`）・`process`（`Err(Unsupported)`）・`prepare_endpoint`/`attach_endpoint`（`Err(EndpointUnavailable)` — prelude の slot 分岐で即拒否）・`evict_emptied_child` pool scan（無変更 — `Route::OutOfProcess{pool}` のみ摘まむパターンで video route は自然に skip）
5. `composite.rs` — `PluginHost::video(&mut self) -> VideoPluginHost<'_>` accessor + video `execute` dispatch（`VideoHostCommand` が audio slot を指す → `Err(Unsupported)` — cross-kind 動詞は slot 分岐で wire 非送信）。audio `Instantiate`/`Scan`/`register_out_of_process` は video route を「存在しない route」として扱う
6. `l1/mod.rs` — **`InstShared` の `InstKind` 分離**: `enum InstKind { Audio { shm: SegmentInfo, staging: Option<Arc<data::Staging>>, result_tx: Mutex<ruviform_rt::Producer<u64>> }, Video { config: VideoProcessConfig } }` を `InstShared` に持たせ、`shm`/`staging`/`result_tx` を `InstKind::Audio` 内へ移す（`phase`/`mirror`/`rt_state`/`notify_tx`/`notify` は共通字段のまま）。audio data-plane 字段を読む全 site（`AddInstance`/`RemoveInstance`/demux/reader/drain）を `InstKind::Audio` arm 経由に写す — video entry は audio data-plane 字段を持たないことを型で pin
7. `l1/mod.rs` — **`L1Child::create_video_instance(spec, config)`**（`create_instance` L1746-1875 と同型の流れ）: `vshm::Segment::create(slot, config)`（`frame_bytes` 算出 → `header + 2×frame_bytes` named segment + header 書込）→ `InstShared{kind:Video{config}, phase:Creating}` insert → `CreateVideoInstance{plugin_id, shm_key, config}` request（`command_timeout`）→ `Ok` で `Live` flip + façade mint。失敗時 rollback（`remove_instance` + `Err(Ipc)`/`InstanceLimit` 透過）は verbatim reuse。**`DriverCmd::AddInstance`/`RemoveInstance` は送らない**（driver 側に video の per-instance 状態は存在しない — submit/result チャネルも作らない）
8. `l1/mod.rs` — **`request` の `InstPhase::Creating` ゲート拡張（必須 touch-point・L1914-1915）**: `InstPhase::Creating if matches!(cmd, wire::Command::CreateInstance { .. })` を `CreateInstance | CreateVideoInstance` 両方受理へ拡張 — 現状のままでは `Creating` 位相の `CreateVideoInstance` request が `_ => Err(HostError::Ipc)` に落ち create が恒に失敗する。`InstPhase::Failed|Destroying` の `DestroyInstance` ゲート（L1916-1917）は共有動詞で無変更
9. `l1/mod.rs` — **`request_timed` 一般化**: `request(instance, cmd)` が固定 `self.command_timeout` を使う現状を `request_timed(instance, cmd, timeout)` に一般化（既存呼出しは `command_timeout` を渡す薄い wrapper として互換）。`VideoRender` のみ `render_timeout` を渡す。timeout→waiter 除去→`converge_child`→`Err(Ipc)` の写像は不変
10. `crates/ruviform-plugin-host/src/l1/vshm.rs`（新規 — `l1/shm.rs` と同型）: `vshm::Segment` — `unique_key` 共用・`create(slot, config)` が `frame_bytes` を算出して `header + 2×frame_bytes` の named segment を作り header を書込。`in_region_mut()`/`out_region()` byte slice accessor（`bytes`/`bytes_mut` に SAFETY コメント + `#[expect(unsafe_code)]` — 書込単位は request→reply 窓で直列化される論証。audio `shm.rs` と同じ 2 箇所パターン）
11. `crates/ruviform-plugin-host/src/l1/video.rs`（新規 — `L1Instance` と parallel な façade）: `VideoL1Instance{shared, child, instance, notify_rx, rt_state, segment: vshm::Segment, config, render_timeout}` — RT leg・staging・arm 系・audio segment は一切持たない。`request`/`destroy`/`kill`/`lifecycle_state`/`try_recv_notification` の収束・通知経路は `L1Instance` と同一機構を共有。**`Render` path**: façade が `input.format()/width()/height()` を cached config と照合（不一致 → `Err(HostError::InvalidValue)` — wire 非送信）→ mirror `Processing` 確認（不一致 → `Err(Lifecycle)` — `transit(Process)` 前置の写像）→ plane rows を in 領域に canonical repack → `VideoRender{at, input:desc}` を `render_timeout` で request → `Rendered{output:desc}` を検証（config dims/format・canonical・`out_bytes` 内 — semantic 違反は**帰属可能 per-instance `Failed`** 収束で caller は waiter drain 経由で `Err(Ipc)`）→ out 領域から `VideoFrame::new_owned` + row copy + pts/duration/color を desc から復元。`Err` reply/timeout/子死亡時は out 領域を読まない（commit-on-success）
12. `l1/video.rs` — **mirror 写像**（`err_committed_state` 踏襲 — `Err` reply で mirror 不変の L0-faithful 契約）: `VideoInitialize` → Initialized（Err → `InitFailed` → Created 維持）・`VideoActivate` → Activated（Err → `ActivateFailed` → Initialized 維持）・`VideoStartProcessing` → Processing（Err → pre-commit で Activated 維持）・`VideoStopProcessing`/`VideoDeactivate` → Activated/Initialized（**Err でも commit 済み** — commit-then-advisory）・`VideoRender`/param 動詞 → 変化なし。`Err(Lifecycle)` は常に pre-commit で mirror 不変
13. `l1/mod.rs` + `l1/` mod 宣言 — `pub(crate) mod vshm;`・`pub(crate) mod video;`（`tests/l1_proxy_e2e.rs` の `#[path]` harness は `crate::` path なし制約を既に課している — video facade が同じ制約内で組めるか確認し、必要なら同じ `dead_code` allowance 方針を適用。`video.rs` は `l1::video` として l1 module 内に置く形を優先 — design は `l1/` 内と記述）
14. `composite.rs` — **encode obligation (a)**: video façade が `SetParamValue` dispatch 前に `Double` 値の finiteness を検証 → 非有限は **wire 非送信の `Err(HostError::InvalidValue)`**（`frame_bytes`/`check_uniform_bounds` 系 early 拒否と同クラス — local defined `Err`・wire に出ない）
15. `composite.rs` — **façade param verbs**: `GetParams`/`GetParamInfo`/`GetParamValue`/`SetParamValue` は mirror `Failed` 検査（`Err(Failed)` — `Destroy` 以外の全動詞で同一）→ wire request の pass-through で lifecycle gate は掛けない（parameter 意味は adapter 責務）。未知 `ParamId` の答えは runner 側 reply をそのまま透過（`ParamInfo(None)`・`Err(InvalidValue)` — façade 側に param list cache は持たない）
16. `composite.rs` — **`Instantiate` bound 拒否**: `frame_bytes(config) ≤ WIRE_VIDEO_MAX_FRAME_BYTES`・`segment_len ≤ WIRE_VIDEO_MAX_SEGMENT_BYTES`・dims ≤ `WIRE_VIDEO_MAX_DIM`・`format` 既知・`frame_rate` 非零 — 違反は `Err(HostError::Prepare(PrepareError::UnsupportedConfig))`（`check_uniform_bounds` と同語彙 — wire 非送信の defined 拒否）
17. `crates/ruviform-plugin-host/src/lib.rs` — `video` mod + `VideoHostCommand`/`VideoHostReply`/`VideoOutOfProcessSpec`/`VideoPluginHost` re-export
18. `crates/ruviform-plugin-host/Cargo.toml` — `ruviform-video-core = { path = "../ruviform-video-core" }` を `[dependencies]` に新規追加（`VideoFrame` — `Render{input}`/`Rendered` 復元で公開型を名指す）・`ruviform-media-core` を dev-dep → `[dependencies]` 昇格（`MediaTime` — `Render{at}`。現 `Cargo.toml` L36 dev-dep → `[dependencies]` — `ruviform-audio-core` の #113 昇格と同経路）
19. unit test（`composite.rs`/`l1/mod.rs` test module — 既存 unit test の配置に合わせる）: kind filter Scan（audio `Scan` が video route を返さない・video `Scan` が audio route を返さない）・`DuplicatePluginId` 双方向・cross-kind `Unsupported`・`EndpointUnavailable`・`Instantiate` bound 拒否・`Creating` ゲート拡張（`CreateVideoInstance` が `Creating` で受理される — `matches!` arm の unit pin または fixture 経由）・`request_timed` の timeout 分離・`Rendered.output` semantic 違反 → `converge_instance`（合成 frame で demux 判定を pin — audio demux unit test と同じパターン）・video `Err` reply → mirror 不変・`vshm` header 書込/field 配置

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-118-video-l1.md](../plans/implement-issue-118-video-l1.md)（本 task は T3 — T2 と並行可）
- [../designs/issue-118-video-l1.md](../designs/issue-118-video-l1.md) L243-295（host 公開面 — `VideoHostCommand`/`VideoHostReply`/`VideoOutOfProcessSpec`/`VideoPluginHost`・route・descriptors kind-tag・cross-kind・`Render`/param verbs・mirror 写像・`Err` reply mirror 不変）・L297-316（`Slot` match site 全網羅表 — **12 site の確定値**）・L318-326（host 内部 — `VideoL1Instance`/`InstKind`/`create_video_instance`/`Creating` ゲート/`request_timed`/`vshm::Segment`/dep edge）
- [../adr/issue-118-video-l1-host-surface.md](../adr/issue-118-video-l1-host-surface.md)（借用 façade・単一 plugin_id 名空間・Dedicated 構造 pin・`descriptors` kind-tag・cross-kind 語彙）
- [../adr/issue-118-video-l1-create-path-segment.md](../adr/issue-118-video-l1-create-path-segment.md)（`create_video_instance` 手順・segment sizing・canonical packing・attach 検証）
- [../specs/issue-118-video-l1.md](../specs/issue-118-video-l1.md) AC2・AC5・R-F5・R-V5・R-B5
- [issue-118-video-failed-parity-tests.md](../test-specs/issue-118-video-failed-parity-tests.md)・[issue-118-video-e2e-tests.md](../test-specs/issue-118-video-e2e-tests.md)（本 task の oracle — host 側分岐）

## Dependencies

- 先行タスク: `l1-wire-v3-video-codec`（wire v3 codec・`wire::vshm`・`InvalidValue`・`PlainParamInfoOwned`・`VideoFrameDesc`/`ColorInfo` codec 型）
- T2 `l1-video-runner-and-erased-surface` とは file 非交差（plugin-core の `video_plugin.rs` は T2 側 file — host surface は T1 の wire 契約のみ共有）
- `shared_memory` crate は既存依存（`Cargo.toml` — `l1/shm.rs` と同じ scoped unsafe パターン）

## Affected Files

- `crates/ruviform-plugin-host/src/video.rs`（新規）— `VideoHostCommand`/`VideoHostReply`/`VideoOutOfProcessSpec`/`VideoPluginHost<'a>`/`descriptor` lookup
- `crates/ruviform-plugin-host/src/composite.rs` — `Route`/`Slot` video variant・全 match-site arm・`descriptors` kind-tag + Scan filter・`video()` accessor・cross-kind dispatch・`Instantiate` bound 拒否・obligation (a)
- `crates/ruviform-plugin-host/src/l1/mod.rs` — `InstKind` 分離・`create_video_instance`・**`Creating` ゲート拡張（L1914-1915）**・`request_timed`・`VideoSpawnSpec`・mod 宣言
- `crates/ruviform-plugin-host/src/l1/vshm.rs`（新規）— `vshm::Segment` create/own/unlink + region accessor
- `crates/ruviform-plugin-host/src/l1/video.rs`（新規）— `VideoL1Instance` façade + render path + mirror 写像
- `crates/ruviform-plugin-host/src/lib.rs` — `video` mod + re-export
- `crates/ruviform-plugin-host/Cargo.toml` — `ruviform-video-core` 追加・`ruviform-media-core` 昇格

## Done When

- [ ] `PluginHost::video(&mut self) -> VideoPluginHost<'_>` が存在し、`VideoHostCommand`/`VideoHostReply`/`VideoOutOfProcessSpec`/`VideoPluginHost` が `#[non_exhaustive]` を維持して re-export される
- [ ] `VideoOutOfProcessSpec` の struct 定義に `isolation`/`lookahead_blocks` 字段が**存在しない**（compile-time pin — ソースレビューで確認）
- [ ] `Route::OutOfProcessVideo`/`Slot::OutOfProcessVideo` が存在し、**12 match-site 全て**が design 表どおりの答えを返す（`try_recv_notification` の video arm を含む — 静かに skip しない）
- [ ] `descriptors` kind-tag で audio `Scan` が video route を返さず video `Scan` が audio route を返さない（shadowed 登録の意味論を kind 内に維持）
- [ ] cross-kind `Instantiate` → `Err(UnknownPluginId)`・登録衝突 → `Err(DuplicatePluginId)`・cross-kind 動詞 → `Err(Unsupported)`・`prepare_endpoint` → `Err(EndpointUnavailable)`・direct `PluginHost` メソッドの video 答え（`process`→`Err(Unsupported)`・`latency_frames`/`plugin_mut`→`None`・`lifecycle_state`→`Some`）
- [ ] **`InstPhase::Creating` ゲートが `CreateVideoInstance` も受理する**（`matches!` arm 拡張 — L1914-1915）
- [ ] `request_timed` が一般化され `VideoRender` のみ `render_timeout` を使い、他動詞は `command_timeout` を維持
- [ ] `create_video_instance` が `vshm::Segment::create` → `Creating` insert → `CreateVideoInstance` request → `Live` flip/rollback を `create_instance` と同型で実行し、`DriverCmd::AddInstance`/`RemoveInstance` を送らない
- [ ] `Render` が input/config 照合（`Err(InvalidValue)`・wire 非送信）→ mirror `Processing` 検査 → repack → `render_timeout` request → `Rendered` 検証（帰属可能 `Failed`/`Err(Ipc)`）→ out 領域復元 → commit-on-success で動く
- [ ] `SetParamValue` 非有限 `Double` が wire 非送信 `Err(InvalidValue)`（obligation (a)）
- [ ] `Err` reply で video instance mirror が不変（`Err(PluginFailed)`/`Err(Lifecycle)`/defined 拒否 — audio doorbell `Err` 収束との意図的 divergence）

## Verification

- [ ] `cargo test -p ruviform-plugin-host` — kind filter・`DuplicatePluginId`・cross-kind・`EndpointUnavailable`・`Creating` ゲート・`request_timed`・demux 合成 frame・mirror 不変の unit test が pass
- [ ] `cargo test -p ruviform-plugin-host --test l1_proxy_e2e` — 既存 audio suite が無変更で green（audio 無退行）
- [ ] `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` が全 pass
- [ ] `cargo doc -p ruviform-plugin-host --no-deps` が warning 0
- [ ] `grep -n "isolation\|lookahead" crates/ruviform-plugin-host/src/video.rs` で `VideoOutOfProcessSpec` 定義に該当字段がない（compile-time pin のソースレビュー — `isolation` 語は doc 記述のみ）
