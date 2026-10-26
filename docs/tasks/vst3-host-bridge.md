---
id: "TASK-034MLSxnXfu3lnbTX3LQzI"
type: "task"
status: "todo"
title: "VST3 host bridge（host feature・Vst3Library/dlopen+class scan・COM host objects・lifecycle 写像・Vst3PluginFactory）"
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
    - "docs/tasks/clap-host-bridge.md"
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

# VST3 host bridge（host feature・Vst3Library/dlopen+class scan・COM host objects・lifecycle 写像・Vst3PluginFactory）

## Work

Plan actions 5–9。`ruviform-plugin-vst3` に feature `host` で feature-sealed な host-side bridge を実装する — `vst3 0.3.0` raw COM bindings + `libloading` で実 `.vst3` cdylib を dlopen・class scan・COM host objects・lifecycle 写像し `PluginFactory`/`Plugin` 中立契約へ載せる。**本 Issue の最大新規コード**（COM host objects 手実装・ProcessData 組立・teardown 順）。

- **`Cargo.toml`**: `[features] host = ["dep:libloading", "dep:ruviform-plugin-export"]` + `[dependencies] libloading = { version = "0.8.9", optional = true }` + `ruviform-plugin-export = { path = "../ruviform-plugin-export", optional = true }`。**`vst3 = "0.3.0"` は既存必須 dep のまま**（optional 化しない — `Class`/`ComWrapper`/`Interface`/constants は `vst3::` re-export で参照・lib.rs:65）・`vst3-host`/`vst3-sys`/`vst3_com` は不採用（design「依存追加の判断」評価済み）。`Cargo.lock` 更新 + dep 追加理由 comment
- **`src/lib.rs`**: `#[cfg(feature = "host")] pub mod host;`（既存 `block/component/controller/factory/gain/strings` 並置）+ crate doc に host bridge 節
- **`src/host.rs`/`src/host/`（新規）**: `Vst3Library`（`libloading::Library::new` → `GetPluginFactory` symbol → `IPluginFactory` ComPtr — platform module hook `InitDll`/`ModuleEntry`/`bundleEntry` は export されていれば best-effort 呼出し）・`descriptors()`（`countClasses` → `getClassInfoUnicode`（`IPluginFactory3`）/`getClassInfo2`（`IPluginFactory2`）列挙 → `category == "Audio Module Class"` のみ採用（`"Component Controller Class"` は pair 対象・独立プラグインではない）→ `PluginDescriptor { id: cid 16byte 小文字 hex（raw bytes・platform-order — `uid()` 由来）, name, vendor, version, features: subCategories }` + intern-once）・`factories()` → `Vec<Vst3PluginFactory>`・`Vst3HostError`（`#[non_exhaustive]` — tresult 値を運ぶ `Com` variant 含む）
- **COM host objects**（bridge 内 `Class`/`ComWrapper` 実装 — component.rs test の `TestQueue`/`TestChanges` 同型先例）: 最小 `IHostApplication`（`IPluginBase::initialize(context)` の context 供給）・最小 `IComponentHandler`（`setComponentHandler` 経由注入 — fixture は非保持 `kResultOk` acknowledge・controller.rs:226）・`IParameterChanges`/`IParamValueQueue`（`ProcessData.inputParameterChanges` 供給 — `EventKind::ParamValue` を normalized `ParamValue` のまま per-frame offset point として積む bridge 所有 object）
- **`Vst3PluginFactory`**: `PluginFactory` impl — `descriptor()` = load 時 cached 値・`create()` → `Vst3Plugin`・`create_sendable` default `None`（raw COM pointer 保持で `!Send`）
- **`Vst3Plugin`**: `Plugin` impl — `init`/`create` 段: `createInstance(component CID)` → QI `IComponent`/`IAudioProcessor` → `getControllerClassId` → `createInstance(controller CID)` → `IEditController` → component・controller 双方の `initialize(context)`（`IHostApplication` 供給）→ `setComponentHandler`（`IComponentHandler` 供給）／`prepare` → **`setActive(true)` → `setupProcessing(ProcessSetup{processMode: kRealtime, symbolicSampleSize: kSample32, maxSamplesPerBlock, sampleRate})` の順**（fixture は `Activated` 未満に `kResultFalse` — component.rs:597・`maxSamplesPerBlock ∈ (0, MAX_NEGOTIATED_FRAMES]`・`sampleRate ≥ 1.0`・kSample64 reject）→ `setProcessing(true)`（`start_processing`）→ `process(ProcessData)`（`HostInput`/`HostOutput` → `AudioBusBuffers{numChannels: 2, channelBuffers32}` — `channelBuffers32` は `*mut *mut Sample32` の writable 型: **input は bounce 固定** — bridge-owned input scratch + channel pointer 配列を `prepare` で `channels × max_frames` 一括確保（per-block alloc なし）し `HostInput` を copy-in ・`&[f32]` を `*mut` 経由で alias するのは禁止（component.rs:1195-1201 の bounce 先例 — plugin が input を書き換える in-place 形状を host 側が許さない）・**output は `HostOutput.channel_mut`（`&mut [f32]`）に `channelBuffers32` を直接指させてよい**（mutable 既知 — alias 安全）・**単一 in/out audio bus・kSample32・kRealtime defined 境界**・`numSamples`・`inputParameterChanges` = bridge 所有 object・`processContext` = transport 写像 or null）→ `setProcessing(false)`（`stop_processing`）→ **teardown は `Drop` 駆動**（中立 `Deactivate`/`Destroy`/`Shutdown` に plugin hook なし — export/lib.rs:424-429）: `Drop` で best-effort `setProcessing(false)`（Processing 残留時・fixture は Processing 中の `setActive(false)` を `kResultFalse` 拒否のため stop 先行対称）→ `setActive(false)`（Activated 時）→ `terminate`（両 object）→ release → module exit hooks → `Library` drop — `Arc<Vst3Library>` で instance → library drop 順序保証（OFX `OfxLibrary::share` 先例）・`params` → `IEditController::getParameterCount`/`getParameterInfo` → `ParamInfo` 表（**factory 構築時に eager 構築+intern** — `params()` は pre-`init` を含む全 lifecycle で呼べる必要があるため、throwaway `createInstance` → `IEditController` で列挙 → release（必要なら最小 `initialize` 付き）し、`create()` された `Vst3Plugin` は cached `&[ParamInfo]` を instance 非依存に返す — fixture: 1 param `ParamId(0)`・`[0.0, 1.0]`・default `1.0`・`unit: ""`）・**state 動詞（`IComponent`/`IEditController` の `setState`/`getState`/`setComponentState`）は一切呼ばない** — `save_state`/`restore_state`/`open_gui`/`close_gui` = `Plugin` 既定 `Err(Unsupported)`
- **unsafe**: 全 vtable 呼出し + `libloading` symbol 解決 + `ProcessData` 組立は `host` module 内 scoped `#[expect(unsafe_code)]` + SAFETY（OFX `ffi/loader.rs` 先例型を超えない）
- **unit test（`#[cfg(all(test, feature = "host"))]`・in-process dlopen）**: `descriptors()` == 既知値 const（component のみ採用: name `Gain`・vendor `Ruviform`・version `0.0.0`・features `["Fx"]`・id = CID hex — **`uid()` byte order は platform 依存**のため expected id は test が `GainComponent::CID`（`pub(crate)` — same-crate unit test から直接参照可）raw 16byte から実行時に derive・`#[cfg]`-conditional literal は regression pin のみ: Windows GUID-order `"3d2c1b5a5f4e7b6a8c9d0e1f2a3b4c5d"`・非 Windows big-endian `"5a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d"`）・`"Component Controller Class"` filter（controller が独立プラグインとして列挙されない）・params 表（`ParamId(0)`・`[0.0,1.0]`・default `1.0`・`unit: ""`）・error 写像（不在 path・`GetPluginFactory` 欠落 fixture・tresult 非 Ok → `Err(Vst3HostError)` — panic しない）・fixture は `current_exe` sibling `deps/{DLL_PREFIX}ruviform_plugin_vst3{DLL_SUFFIX}` で解決（欠落は `assert!` fail）

## Classification

- `normal-plan-task`（規格別 bridge — Task 1（CLAP）と別 crate で論理上並行可能・Task 3（runner）の VST3 側前提・**本 Issue の最大新規コード** — COM host objects 手実装）

## Upstream Documents

- `docs/plans/implement-issue-114-real-adapter-l1-e2e.md`（actions 5–9・Files/Ownership・依存の確定）
- `docs/designs/issue-114-real-adapter-l1-e2e.md`（決定サマリ c/e・「VST3 bridge」「COM host objects」「lifecycle 写像」「'static メタデータ実体化」「依存追加の判断」）
- `docs/specs/issue-114-real-adapter-l1-e2e.md`（AC1・AC3・AC7）
- `docs/adr/plugin-architecture-clap-vst3.md`（adapter 責務・bridge 配置）
- `docs/adr/plugin-binding-method-clack-for-clap-coupler-rs-vst3-for-vst3.md`（raw `vst3` bindings 採用根拠・`vst3-host` 不採用）
- `crates/ruviform-plugin-vst3/src/component.rs`（L597 — `Activated` 未満で `kResultFalse`・L509-516 — `setState`/`getState` `kNotImplemented` stub・test `TestQueue`/`TestChanges` 先例）
- `crates/ruviform-plugin-vst3/src/controller.rs`（L226 — `setComponentHandler` 非保持 acknowledge・L87-93 — `setState`/`getState` `kResultOk` no-op stub）
- `crates/ruviform-plugin-vst3/src/factory.rs`（`getClassInfo2`/`getClassInfoUnicode` — `"Audio Module Class"`/`"Component Controller Class"` 両 class・`Gain`/`Gain Controller`・vendor `Ruviform`・version `0.0.0`・subCategories `Fx`）

## Dependencies

- `docs/plans/implement-issue-114-real-adapter-l1-e2e.md`（起点 task — 他 task 前提なし・Task 1 と論理上並行）

## Affected Files

- `crates/ruviform-plugin-vst3/Cargo.toml` — `[features] host` + `libloading`(optional) + `ruviform-plugin-export`(optional) + `[[bin]]`（T3 で追加）+ `[dev-dependencies] ruviform-plugin-host`（T4 で追加）
- `crates/ruviform-plugin-vst3/src/lib.rs` — `#[cfg(feature = "host")] pub mod host;` + crate doc
- `crates/ruviform-plugin-vst3/src/host.rs` または `src/host/`（新規）— `Vst3Library`/`Vst3PluginFactory`/`Vst3Plugin`/`Vst3HostError` + COM host objects（`IHostApplication`/`IComponentHandler`/`IParameterChanges`/`IParamValueQueue`）+ intern-once helper + `#[cfg(test)]` unit test
- `Cargo.lock` — `dep:` optional 昇格の差分（`libloading` 新規 optional・`ruviform-plugin-export` 新規 optional）

## Done When

- [ ] `[features] host` が定義され、`cargo build -p ruviform-plugin-vst3 --features host` が通る（`libloading`/`ruviform-plugin-export` が `host` 指定時のみ dep 木に出現 — `cargo tree` で `host` off 時に新規 dep が non-optional 木に出ないことを確認）
- [ ] `Vst3Library::load` が fixture cdylib を `libloading` で開き `GetPluginFactory` で `IPluginFactory` ComPtr を得て、`descriptors()` が `countClasses`/`getClassInfo2`/`getClassInfoUnicode` で `"Audio Module Class"` のみ採用した cached `PluginDescriptor` を返す（`id` = cid 16byte 小文字 hex・platform-order）
- [ ] COM host objects（`IHostApplication`/`IComponentHandler`/`IParameterChanges`/`IParamValueQueue`）が `Class`/`ComWrapper` で実装され、`initialize(context)`/`setComponentHandler`/`inputParameterChanges` 経路で fixture に供給される
- [ ] `Vst3Plugin` が `init`（`createInstance` chain + `initialize` + `setComponentHandler`）/`prepare`（`setActive(true)`→`setupProcessing` の順 + input scratch/channel pointer 配列を `channels × max_frames` で一括確保）/`start_processing`（`setProcessing(true)`）/`process_host`（`process(ProcessData)` 単一 bus・kSample32・kRealtime・input は bounce copy-in・output は `HostOutput.channel_mut` 直接参照）/`stop_processing`（`setProcessing(false)`）/`params`（`getParameterCount`/`getParameterInfo` — factory 構築時 eager 列挙の cached 表を pre-`init` を含む全 lifecycle で返す）を写像し、`Drop` で best-effort `setProcessing(false)`→`setActive(false)`→`terminate`→release を駆動（`Arc<Vst3Library>` で drop 順序保証）
- [ ] `save_state`/`restore_state`/`open_gui`/`close_gui` が `Err(Unsupported)` 既定で、規格 state 動詞（`setState`/`getState`/`setComponentState`）を一度も呼ばない
- [ ] unit test が `--features host` で pass — descriptor（id hex は `GainComponent::CID` bytes derive・literal は regression pin）/param 既知値・`"Component Controller Class"` filter・error 写像・fixture 欠落 `assert!` fail
- [ ] narrow gate（下記 Verification）が exit 0 — `cargo test`/`clippy`/`check`/`lint`/`test` で default 構成不変グリーン

## Verification

- [ ] `cargo test -p ruviform-plugin-vst3 --features host` — exit 0（bridge unit test + 既存 component/controller/factory unit test 無退行）
- [ ] `cargo clippy -p ruviform-plugin-vst3 --all-targets --features host -- -D warnings` — exit 0（COM vtable/libloading 箇所の scoped `#[expect(unsafe_code)]` + SAFETY のみ・0 warning）
- [ ] `mise run check` — exit 0（`cargo check --workspace --all-targets` — `host` 未指定で `host.rs` 非 compile・default 構成不変）
- [ ] `mise run lint` — exit 0（`cargo clippy --workspace --all-targets -D warnings` — `host` 未指定の default 木で無退行）
- [ ] `mise run test` — exit 0（`cargo test --workspace` — `host` 未指定の default 木で無退行）
- [ ] `cargo tree -p ruviform-plugin-vst3` — `libloading`/`ruviform-plugin-export` が `host` off で non-optional 木に出ない・`vst3` は既存必須のまま
- [ ] `cargo tree -p ruviform-plugin-vst3 --features host` — `libloading`/`ruviform-plugin-export` が `host` 指定時のみ出現
