---
id: "TASK-034NXupvMzQElYkWehw8ln"
type: "task"
status: "todo"
title: "L1 video runner + erased surface（erase_video・run_video・VideoEntry・vshm attach・reference bin・runner E2E）"
created: "2026-10-28"
updated: "2026-10-29"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/118"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-core/src/video_plugin.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/src/vshm.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/src/bin/ruviform-plugin-l1-video-runner.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/tests/l1_video_runner_e2e.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/Cargo.toml"
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
    - "docs/tasks/l1-video-host-surface.md"
    - "docs/tasks/plugin-export-crate-and-l1-runner.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-118-video-l1.md"
    - "docs/designs/issue-118-video-l1.md"
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

# L1 video runner + erased surface（erase_video・run_video・VideoEntry・vshm attach・reference bin・runner E2E）

## Work

子プロセス側の video 表面 — `ErasedVideoPlugin`/`VideoPluginFactory`/`erase_video`（plugin-core 新規 `video_plugin.rs`）と `run_video`/`VideoRunner`/`VideoEntry`/`vshm` attach（plugin-export）。`run`/audio registry は無変更（R-B6）。T3（host 側）とは crate・file とも非交差で並行可能 — 共有するのは T1 の wire 契約のみ。

1. `crates/ruviform-plugin-core/src/video_plugin.rs`（新規 — `video.rs` と隣接）: `ErasedVideoPlugin` trait（`VideoPluginInstance` の dyn-safe mirror — `type Error` 関連型を持たず全 hook は `Result<_, HostError>`。`state()`/`descriptor() -> PluginDescriptorView<'_>`/`parameters() -> &[ParamId]`/`parameter(id) -> Option<PlainParamInfo<'_>>`/`parameter_value`/`set_parameter`/`initialize(&VideoProcessConfig)`/`activate`/`start_processing`/`render(&VideoFrame,&mut VideoFrame,MediaTime)`/`stop_processing`/`deactivate`/`destroy`）。`VideoPluginFactory{descriptor() -> PluginDescriptor; create() -> Box<dyn ErasedVideoPlugin>}`（audio `PluginFactory` 対の最小形 — `create_sendable` は video に不要・RT endpoint 経路がない）。`erase_video<P: VideoPluginInstance>(p) -> Box<dyn ErasedVideoPlugin>` adapter — plugin `Error` を `HostError::PluginFailed` に正規化（L0 で adapter error が host surface に出る写像と同型）
2. `video_plugin.rs` — **encode obligations (b)(c)(d) を `erase_video` adapter 境界に集約**: (b) `parameter_value()` が `Double(非有限)` を返したら `Err(PluginFailed)` に正規化（非有限値を wire に書かせない — plugin の wart は protocol violation でなく plugin error）・(c) `parameters()` が `WIRE_MAX_ELEMENTS`（1024）超の `ids`、`parameter()` が choices 件数・各 string byte 長が `WIRE_MAX_*` を超える `PlainParamInfo` を報告したら `Err(PluginFailed)` に正規化（truncate しない — verbatim encode → 相手 decode `LimitExceeded` → 子死の経路を封じる）・(d) `parameter()` の非有限 `display_*` を clamp（`[min,max]` 内に収まるならそこへ・さもなくば `±f64::MAX` — OFX `±DBL_MAX` 慣行の写像。decode は lenient 受理なので wire 拒否ではなく host 側 sanity 正規化）
3. `crates/ruviform-plugin-core/src/lib.rs` — `video_plugin` mod + `ErasedVideoPlugin`/`VideoPluginFactory`/`erase_video` re-export
4. `crates/ruviform-plugin-export/src/vshm.rs`（新規 — `shm.rs` と同型・`wire::vshm` 定盤を使用）: `Segment::attach(key, config)` の 2 層検証 — (a) 構造: `magic=="RVLV"`・`wire_version==WIRE_VERSION`・`in_off==HEADER_LEN(28)`・`out_off==HEADER_LEN+in_bytes`・`reserved==0`・`declared_len >= out_off+out_bytes`; (b) config 照合 + bound 再検査: `in_bytes`/`out_bytes` と `frame_bytes(config)` 再計算の一致・`frame_bytes(config) <= WIRE_VIDEO_MAX_FRAME_BYTES`（**対称防御** — host 側 early check を信頼しない）。`bytes`/`bytes_mut` accessor は audio `shm.rs` と同じ 2 箇所の `#[expect(unsafe_code)]` + SAFETY コメントパターン
5. `crates/ruviform-plugin-export/src/lib.rs` — `pub fn run_video(factories: Vec<Box<dyn VideoPluginFactory>>) -> ExitCode`（`run` と parallel な別 entry — framing/`Hello`/exit semantics/argv 拒否（引数なし・`--ruviform-*` → exit 2）/`Shutdown`+EOF normalize は `serve` 骨格を共有）。`VideoRunner{factories, instances: HashMap<u32, VideoEntry>}` + `VideoEntry{instance: Box<dyn ErasedVideoPlugin>, lifecycle: Lifecycle, config: VideoProcessConfig, segment: vshm::Segment}`。dispatch: `Scan` → video descriptor 列（child-scope）・`CreateVideoInstance{i,plugin_id,shm_key,config}` → audio `create_instance` と同一手順（`plugin_id ∈ factories` → `Err(UnknownPluginId)`・`registry.len() < WIRE_MAX_INSTANCES` → `Err(InstanceLimit)`・`i` 未登録 → dup は `Err(Ipc)`・vshm attach → `Err(Ipc)`・`create()` → insert — capacity は duplicate より先）。`DestroyInstance` → video entry normalize + registry 解放（`Failed`/`Destroying` 宛も許可 — audio と同一 contract）
6. `export/lib.rs` — `VideoInitialize` は **payload 無し** — `CreateVideoInstance` で stash した `config` を `initialize(&config)` hook に再生（trait 契約の写像 — wire に config を 2 回載せない）。lifecycle dispatch: `VideoActivate`/`VideoStartProcessing`/`VideoStopProcessing`/`VideoDeactivate` は `lifecycle.transition()` が hook 前に走る形（commit-then-advisory — `stop_processing`/`deactivate` は Err でも commit 済み、audio `StopProcessing` 同型）。違法遷移は `Err(Lifecycle)`
7. `export/lib.rs` — `VideoRender{at, input: desc}` dispatch: `transition(Process)` 前置 → desc post-decode semantic 検証（canonical 配置・`offset+len ≤ in_bytes`・config dims/format 一致 — 非 canonical は `Err(HostError::Ipc)`・config 不一致は `Err(HostError::Prepare(PrepareError::UnsupportedConfig))`・いずれも `Err` reply で子は生きる）→ in 領域から `VideoFrame` 復元（`new_owned` + per-plane row copy）→ **出力 frame を input desc の metadata（`pts`/`duration`/`color`）で seed** → hook 呼出し → `Ok` なら out 領域へ書込 + `Rendered{output: desc}`（v1 は config dims/format・canonical を報告）・hook `Err`/panic → 出力不 commit + `Err(PluginFailed)`（`catch_unwind` 正規化は audio と同じ — `factory.descriptor()`/`create()` も dispatch 内で guard）
8. `export/lib.rs` — param dispatch: `id ∈ instance.parameters()` 先行検査（hook を呼ばない）— `VideoGetParamInfo` → `ParamInfo{info: None}`（`parameter(id)` の `Option` 契約を素通し）・`VideoGetParamValue`/`VideoSetParamValue` → **`Err(HostError::InvalidValue)` reply**（値 domain の defined 拒否 — `Err(PluginFailed)` に畳まない）。audio 動詞（tag 1-9）を登録済み entry 宛に受けた場合は entry catch-all で `Err(Unsupported)`・未登録 handle 宛は runner 層で `Err(InvalidHandle)`（`CreateVideoInstance`/`CreateInstance` cross-kind create も `Err(InvalidHandle)` — 新規 arm ゼロ）
9. `export/lib.rs` — inbound `BlockIn` は **静かに drop**（direction-legal だが video 子には帰属不能な foreign doorbell — `normalize_all`/死亡に畳まない・返信なし）。inbound `BlockDone`/`Reply`/`Hello` は direction-illegal → `normalize_all` + `ExitCode::FAILURE`（audio `serve` L271-282 と同規則）。`Shutdown`/stdin EOF → 全 video entry normalize → exit 0
10. `crates/ruviform-plugin-export/src/bin/ruviform-plugin-l1-video-runner.rs`（新規 reference bin — `ruviform-plugin-l1-runner` 対の video 版）: `run_video(vec![...])` を `main` で呼ぶ。mock factories: deterministic pixel-transform plugin（反転/fill — host E2E の pixel oracle・`org.ruviform.test` 系 id または `dev.ruviform.mock.*` — audio bin の命名慣例に倣う）+ panic fixture（env-gated `RUVIFORM_PANIC_*` — `descriptor()`/`create()`/video hook panic → `Err(PluginFailed)` で子生存の pin）+ over-limit params fixture（obligation (c) — `WIRE_MAX_ELEMENTS` 超 `ids`/超長 choices 報告で `Err(PluginFailed)`）
11. `crates/ruviform-plugin-export/tests/l1_video_runner_e2e.rs`（新規 — `l1_runner_e2e.rs` の MiniHost 構造を video wire に写した suite）: `CARGO_BIN_EXE_ruviform-plugin-l1-video-runner` を bare spawn・`Hello` → `Scan` → `CreateVideoInstance{i,id,key,config}` handshake → video lifecycle + `VideoRender`（in/out 領域 byte 検証 — frame 内容一致・pts/duration/color pass-through）→ `DestroyInstance` → `Shutdown`。検証: `InvalidHandle` flagged reply・dup `CreateVideoInstance` → `Err(Ipc)`・`InstanceLimit`・attach 拒否行列（magic/version/in_off/out_off/reserved/len/in_bytes/frame_bytes cap — `Err(Ipc)` reply + 子生存）・stash config 再生（`VideoInitialize` で hook が stash config を受け取る — plugin が config を echo する形で観測）・param 先行検査（`ParamInfo(None)`/`Err(InvalidValue)`）・panic 正規化（`Err(PluginFailed)` + 子生存）・cross-kind `CreateInstance` → `Err(InvalidHandle)`・inbound `BlockIn` 静かに drop・`BlockDone`/`Reply`/`Hello` → 子終了・argv `--ruviform-*` → exit 2
12. `crates/ruviform-plugin-export/Cargo.toml` — `ruviform-video-core = { path = "../ruviform-video-core" }` を `[dependencies]` に新規追加（`VideoEntry` が `VideoFrame` を構築）・`ruviform-media-core` を dev-dep → `[dependencies]` 昇格（`MediaTime`/`MediaDuration` — `ruviform-audio-core` の #113 昇格と同経路・transitive 増加なし）

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-118-video-l1.md](../plans/implement-issue-118-video-l1.md)（本 task は T2 — T3 と並行可）
- [../designs/issue-118-video-l1.md](../designs/issue-118-video-l1.md) L328-335（runner 側 — `ErasedVideoPlugin`/`run_video`/`VideoEntry`/stash replay/param dispatch/registry 動詞/inbound `BlockIn`）・L162-180（encode obligations (b)(c)(d)・責務分離）・L238-241（attach 検証・`Rendered.output` 信頼性）
- [../adr/issue-118-video-l1-create-path-segment.md](../adr/issue-118-video-l1-create-path-segment.md)（`CreateVideoInstance` 手順・attach 2 層検証・stash → `initialize(&config)` 再生）
- [../specs/issue-118-video-l1.md](../specs/issue-118-video-l1.md) R-B7・AC6・AC7・R-V2（runner 側）
- [issue-118-video-failed-parity-tests.md](../test-specs/issue-118-video-failed-parity-tests.md)（runner normalize/inert/attach 拒否の oracle）
- [issue-118-video-e2e-tests.md](../test-specs/issue-118-video-e2e-tests.md)（param 透過・frame pass-through の E2E oracle）

## Dependencies

- 先行タスク: `l1-wire-v3-video-codec`（wire v3 codec・`wire::vshm`・`InvalidValue`・`PlainParamInfoOwned`）
- T3 `l1-video-host-surface` とは非交差（plugin-core の `video_plugin.rs` 1 file のみ共有 crate・file は別）
- `shared_memory` crate は既存依存（`export/Cargo.toml` — audio `shm.rs` と同じ scoped unsafe パターン）

## Affected Files

- `crates/ruviform-plugin-core/src/video_plugin.rs`（新規）— `ErasedVideoPlugin`/`VideoPluginFactory`/`erase_video` + encode obligations (b)(c)(d)
- `crates/ruviform-plugin-core/src/lib.rs` — `video_plugin` mod + re-export
- `crates/ruviform-plugin-export/src/lib.rs` — `run_video` + `VideoRunner`/`VideoEntry` + dispatch + inbound kind 規則
- `crates/ruviform-plugin-export/src/vshm.rs`（新規）— attach 2 層検証 + region accessor
- `crates/ruviform-plugin-export/src/bin/ruviform-plugin-l1-video-runner.rs`（新規）— reference bin + mock/panic/over-limit factories
- `crates/ruviform-plugin-export/tests/l1_video_runner_e2e.rs`（新規）— MiniHost runner E2E
- `crates/ruviform-plugin-export/Cargo.toml` — `ruviform-video-core` 追加・`ruviform-media-core` 昇格

## Done When

- [ ] `erase_video`/`ErasedVideoPlugin`/`VideoPluginFactory` が plugin-core に存在し、全 hook が `Result<_, HostError>` で plugin `Error` → `PluginFailed` 正規化・encode obligations (b)(c)(d) が adapter 境界に集約されている
- [ ] `run_video(factories)` が `run` と parallel に存在し、`run`/`ErasedPlugin`/audio registry が無変更
- [ ] `CreateVideoInstance` の検証 chain（`UnknownPluginId` → `InstanceLimit` → dup `Ipc` → attach `Ipc` → create → insert）と stash config の `initialize(&config)` 再生が動作する
- [ ] `VideoRender` が `transition(Process)` 前置 + desc semantic 検証（非 canonical → `Err(Ipc)`・config 不一致 → `Err(Prepare(UnsupportedConfig))`）+ output metadata seed + commit-on-success で動く
- [ ] param dispatch が `id ∈ parameters()` 先行検査を行い `ParamInfo(None)`/`Err(InvalidValue)` を返し、audio 動詞宛は `Err(Unsupported)`・未登録は `Err(InvalidHandle)`（cross-kind create 含む — 新規 arm ゼロ）
- [ ] inbound `BlockIn` が静かに drop され、`BlockDone`/`Reply`/`Hello` が protocol violation で子終了、`Shutdown`/EOF が全 entry normalize + exit 0
- [ ] reference bin + MiniHost E2E が上記分岐を全て pass する

## Verification

- [ ] `cargo test -p ruviform-plugin-export --test l1_video_runner_e2e` — MiniHost video runner E2E が pass
- [ ] `cargo test -p ruviform-plugin-export` — 既存 audio runner E2E（`l1_runner_e2e`）が無変更で green
- [ ] `cargo test -p ruviform-plugin-core` — `erase_video` adapter の unit test（正規化・obligation 行列）が pass
- [ ] `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` が全 pass
- [ ] `cargo doc -p ruviform-plugin-export -p ruviform-plugin-core --no-deps` が warning 0
