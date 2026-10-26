---
id: "TASK-034MLSwRAMOjT59ZH5kQFn"
type: "task"
status: "todo"
title: "CLAP host bridge（host feature・ClapLibrary/descriptor+param 合成・intern-once・lifecycle/params/error 写像・ClapPluginFactory）"
created: "2026-10-26"
updated: "2026-10-27"
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
    - "docs/plans/implement-issue-114-real-adapter-l1-e2e.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-114-real-adapter-l1-e2e.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/vst3-host-bridge.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-114-real-adapter-l1-e2e.md"
    - "docs/designs/issue-114-real-adapter-l1-e2e.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/adapter-host-bridge-contract-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# CLAP host bridge（host feature・ClapLibrary/descriptor+param 合成・intern-once・lifecycle/params/error 写像・ClapPluginFactory）

## Work

Plan actions 1–4。`ruviform-plugin-clap` に feature `host` で feature-sealed な host-side bridge を実装する — `clack-host` で実 `.clap` cdylib を dlopen・scan・lifecycle 写像し `PluginFactory`/`Plugin` 中立契約へ載せる。

- **`Cargo.toml`**: `[features] host = ["dep:clack-host", "dep:ruviform-plugin-export", "clack-extensions/clack-host"]` + `[dependencies] clack-host = { version = "=0.2.0", optional = true }` + `ruviform-plugin-export = { path = "../ruviform-plugin-export", optional = true }`。**`clack-extensions` は非 optional 既存 prod dep のまま**（`dep?/feat` weak 構文は optional dep 専用で適用不可 — feature 内は plain `dep/feat` 構文）。`[dev-dependencies] clack-host = { version = "=0.2.0", features = ["clack-plugin"] }`（`load_from_clack` typed-entry test 用）と `clack-extensions = { version = "=0.2.0", features = ["clack-host", "thread-check"] }` は現行維持 — 両 entry で feature union。`Cargo.lock` 更新 + dep 昇格理由 comment
- **`src/lib.rs`**: `#[cfg(feature = "host")] pub mod host;` を既存 `mod adapt/gain/plugin` と並置 + crate doc に host bridge 節
- **`src/host.rs`（新規）**: `ClapLibrary`（`PluginEntry` wrap — `PluginEntry::load(path)` は unsafe・scoped `#[expect(unsafe_code)]` + SAFETY）・`descriptors()`（load 時 scan: `get_plugin_factory()` → `plugin_descriptors()` → `clap_plugin_descriptor` id/name/vendor/version/features 順序保持 → `PluginDescriptor` 全 field 写像 + intern-once `String::leak`/`Vec::leak` — `&'static` 実体化・cached slice 返却）・`factories()` → `Vec<ClapPluginFactory>`・`ClapHostError`（`#[non_exhaustive]` — `PluginEntryError`/`PluginInstanceError` defined 写像）
- **`ClapPluginFactory`**: `PluginFactory` impl — `descriptor()` = load 時 cached 値・`create()` → `ClapPlugin`・`create_sendable` default `None`（`PluginInstance` は `!Send`/`!Sync` — `_no_send` PhantomData）
- **`ClapPlugin`**: `Plugin` impl — `init` → `PluginInstance::new`（CLAP instantiate+`plugin.init` 一体・中立 `Initialize` が駆動 — `create()` 段では instance を作らない・`Option<PluginInstance>` 保持）・`prepare` → `activate(|_, _| BridgeAudioProcessor, PluginAudioConfiguration{sample_rate, min_frames_count, max_frames_count}) -> Result<StoppedPluginAudioProcessor, PluginInstanceError>`（closure は `FnOnce(&Shared, &MainThread) -> AudioProcessor` 形・**戻り値 `StoppedPluginAudioProcessor` は `ClapPlugin` field に格納** — `start_processing` → `Started` / `stop_processing` → `Stopped` / `Drop` の `deactivate` が消費する対称形）・`start_processing`/`stop_processing` → `Stopped`↔`Started` processor 遷移・`process_host` → `StartedPluginAudioProcessor::process(&InputAudioBuffers, &mut OutputAudioBuffers, &InputEvents, &mut OutputEvents, steady_time: Option<u64>, transport: Option<&TransportEvent>)`（arity 6・`EventKind::ParamValue` → `EventBuffer` 内 `ParamValueEvent`・`ProcessResult` → `ProcessStatus` 読取り）・`params` → params extension `PluginParams::count`/`get_info` → `ParamInfo` 表（`unit: ""` — CLAP param_info に unit 面なし・intern-once）・**teardown は `Drop` 駆動**（中立 `Deactivate`/`Destroy`/`Shutdown` に plugin hook なし — export/lib.rs:424-429・646-666）: `Drop` で best-effort `stop_processing`（Started 時）→ `instance.deactivate(stopped)`（Activated 時）→ instance drop（CLAP `plugin.destroy`）・`save_state`/`restore_state`/`open_gui`/`close_gui` = `Plugin` 既定 `Err(Unsupported)` 継承
- **input mutability wall（bounce pin）**: `HostInput<'_>` は `&[f32]` のみ yield するが `InputChannel{buffer: &'a mut [f32]}`・`AudioPorts::with_input_buffers`（`ChannelIter32: IntoIterator<Item = InputChannel<'a, f32>>`）は mutable slice を要求する — bridge-owned **input scratch（`channels × max_frames`）を `prepare` で一括確保**し `process` は copy-in のみ（per-block alloc 禁止 — `process_host` は RT 経路）。`AudioPorts`・`EventBuffer`/`OutputEvents` の backing buffer は Vec-backed（realloc risk）のため `ClapPlugin` の **persistent field** として `prepare` で preallocate する。`InputAudioBuffers::from_raw_buffers` を手組立して immutable `&[f32]` を `*mut` として見せる const-cast 手続きは **REJECTED**（immutable input の可変偽装は UB-adjacent — scoped `#[expect(unsafe_code)]` + SAFETY でも正当化しない）
- **`process` の steady_time/transport 写像（v1 defined boundary）**: `steady_time`/`transport` は `None`/`None` — `TransportInfo`→`TransportEvent` 投影は `TransportInfo` snapshot（`position: MediaTime`・`musical: Option<MusicalTimeInfo>`・`loop_range` は media-time domain）から beat-domain loop field（`loop_start_beats`/`loop_end_beats`）や `tempo_inc` を honest に導出不能（tempo map 非保持）のため deferred — fixture は transport を読まず、半投影で 0 を詰めるより明示 `None` の defined boundary を採る
- **`params()` の可用性**: `params()` は pre-`init` を含む**全 lifecycle 地点**で呼べる必要があるため、param 表は **factory 構築時に eager 構築+intern**（throwaway `PluginInstance::new` で `PluginParams::count`/`get_info` を列挙 → drop — fixture plugin code が scan 時に 1 回走る bounded 形）— `create()` された `ClapPlugin` は cached 表を保持し `params()` は instance 非依存に `&[ParamInfo]` を返す（lazy 列挙は `PluginInstance` 不在の pre-`init` で成立しない）
- **`BridgeHandlers`**: `HostHandlers` impl — `Shared`/`MainThread`/`AudioProcessor` handler 型 + `declare_extensions` は `HostThreadCheck` を**登録しない**（fixture の `get_extension::<HostThreadCheck>()` → `None` graceful fallback で挙動不変 — 不実の `is_audio_thread` 応答をしない・plugin.rs:130・421）
- **unit test（`#[cfg(all(test, feature = "host"))]`・in-process dlopen）**: `descriptors()` 実値 == fixture 既知値 const literal（id `org.ruviform.clap-gain-poc`・name `Ruviform Gain PoC`・vendor `Ruviform`・version `0.1.0`（env!("CARGO_PKG_VERSION")）・features `[audio-effect, stereo]` 順序保持）・params 表（1 param `ParamId(0)`・`[0.0, 2.0]`・default `1.0`・`unit: ""` — **`""` は honest synthetic 値**: CLAP `param_info` に unit field がなく host discovery は unit を失うため `"x"` 等の filler に"修正"しない）・`params()` が pre-`init` の `create()` 直後 instance を含む全 lifecycle で cached 表を返すこと・`descriptors()` 複数回呼出の同一性（intern-once once 性）・error 写像（不在 path・非規格バイナリ → `Err(ClapHostError)` — panic しない）・fixture は `current_exe` sibling `deps/{DLL_PREFIX}ruviform_plugin_clap{DLL_SUFFIX}` で解決（欠落は `assert!` fail）

## Classification

- `normal-plan-task`（規格別 bridge — Task 2（VST3）と別 crate で論理上並行可能・Task 3（runner）の CLAP 側前提）

## Upstream Documents

- `docs/plans/implement-issue-114-real-adapter-l1-e2e.md`（actions 1–4・Files/Ownership・依存の確定）
- `docs/designs/issue-114-real-adapter-l1-e2e.md`（決定サマリ c/d・「CLAP bridge」「'static メタデータ実体化」「依存追加の判断」）
- `docs/specs/issue-114-real-adapter-l1-e2e.md`（AC1・AC2・AC7）
- `docs/adr/plugin-architecture-clap-vst3.md`（adapter 責務・bridge 配置）
- `docs/adr/plugin-binding-method-clack-for-clap-coupler-rs-vst3-for-vst3.md`（clack 採用根拠）
- `crates/ruviform-plugin-clap/src/plugin.rs`（L130・421 — `HostThreadCheck` `None` fallback）
- `crates/ruviform-plugin-clap/src/plugin.rs`（L197-203 — fixture 既知値 descriptor）

## Dependencies

- `docs/plans/implement-issue-114-real-adapter-l1-e2e.md`（起点 task — 他 task 前提なし）

## Affected Files

- `crates/ruviform-plugin-clap/Cargo.toml` — `[features] host` + `clack-host`(optional) + `ruviform-plugin-export`(optional) + `clack-extensions/clack-host`（plain 構文）+ `[[bin]]`（T3 で追加）+ `[dev-dependencies] ruviform-plugin-host`（T4 で追加）
- `crates/ruviform-plugin-clap/src/lib.rs` — `#[cfg(feature = "host")] pub mod host;` + crate doc
- `crates/ruviform-plugin-clap/src/host.rs`（新規）— `ClapLibrary`/`ClapPluginFactory`/`ClapPlugin`/`ClapHostError`/`BridgeHandlers` + intern-once helper + `#[cfg(test)]` unit test
- `Cargo.lock` — `dep:` optional 昇格の差分

## Done When

- [ ] `[features] host` が定義され、`cargo build -p ruviform-plugin-clap --features host` が通る（`clack-host`/`ruviform-plugin-export`/`clack-extensions/clack-host` が `host` 指定時のみ dep 木に出現 — `cargo tree` で `host` off 時に新規 dep が non-optional 木に出ないことを確認）
- [ ] `ClapLibrary::load` が fixture cdylib を `PluginEntry::load` で開き、`descriptors()` が load 時 scan の cached `PluginDescriptor` を返す（`id`/`name`/`vendor`/`version`/`features` 全 field・`features` 順序保持・intern-once）
- [ ] `ClapPluginFactory::descriptor()` が cached 値・`create()` が `ClapPlugin` を返し、`create_sendable` が default `None`
- [ ] `ClapPlugin` が `init`（`PluginInstance::new`）/`prepare`（`activate` — `StoppedPluginAudioProcessor` 戻り値を field 格納 + input scratch・`AudioPorts`/`EventBuffer`/`OutputEvents` persistent buffer を `prepare` で一括確保）/`start_processing`/`stop_processing`/`process_host`（input bounce copy-in・`from_raw_buffers` const-cast 不採用・`steady_time`/`transport` = `None`/`None` defined boundary）/`params`（factory 構築時 eager 列挙の cached 表を pre-`init` を含む全 lifecycle で返す）を中立動詞に写像し、`Drop` で best-effort `stop_processing`→`deactivate`→destroy を駆動、`save_state`/`restore_state`/`open_gui`/`close_gui` が `Err(Unsupported)` 既定
- [ ] `BridgeHandlers::declare_extensions` が `HostThreadCheck` を登録しない（fixture `None` fallback 維持）
- [ ] unit test が `--features host` で pass — descriptor/param 既知値・`descriptors()` 複数回同一性・error 写像・fixture 欠落 `assert!` fail
- [ ] narrow gate（下記 Verification）が exit 0 — `cargo test`/`clippy`/`check`/`lint`/`test` で default 構成不変グリーン

## Verification

- [ ] `cargo test -p ruviform-plugin-clap --features host` — exit 0（bridge unit test + `load_from_clack` typed-entry test + 既存 unit test 無退行）
- [ ] `cargo clippy -p ruviform-plugin-clap --all-targets --features host -- -D warnings` — exit 0（host feature の scoped `#[expect(unsafe_code)]` + SAFETY 箇所のみ・0 warning）
- [ ] `mise run check` — exit 0（`cargo check --workspace --all-targets` — `host` 未指定で `host.rs` 非 compile・default 構成不変）
- [ ] `mise run lint` — exit 0（`cargo clippy --workspace --all-targets -D warnings` — `host` 未指定の default 木で無退行）
- [ ] `mise run test` — exit 0（`cargo test --workspace` — `host` 未指定の default 木で無退行・`l1_real_adapter_e2e.rs` は `#![cfg]` で空）
- [ ] `cargo tree -p ruviform-plugin-clap` — `clack-host`/`libloading`/`ruviform-plugin-export` が `host` off で non-optional 木に出ない・`clack-extensions` は既存 non-optional のまま
- [ ] `cargo tree -p ruviform-plugin-clap --features host` — `clack-host`/`libloading`/`ruviform-plugin-export`/`clack-extensions/clack-host` が `host` 指定時のみ出現
