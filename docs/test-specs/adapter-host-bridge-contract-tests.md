---
id: "TSPEC-034MLRB5YDVNIaDpjhYknO"
type: "test-spec"
status: "approved"
title: "規格アダプタ host bridge 契約（descriptor/param 合成・lifecycle 写像・defined rejection・intern-once）テスト仕様"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/real-adapter-l1-e2e-tests.md"
    - "docs/test-specs/l1-real-plugin-lane-tests.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-114-real-adapter-l1-e2e.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-114-real-adapter-l1-e2e.md"
    - "docs/designs/issue-114-real-adapter-l1-e2e.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 規格アダプタ host bridge 契約（descriptor/param 合成・lifecycle 写像・defined rejection・intern-once）テスト仕様

> status: approved — Issue #114 の実装（Task 1–5・Full Gate 全 green）で契約が実結果により確認済み（`cargo test -p ruviform-plugin-clap/-vst3 --features host` — CLAP lib 63・VST3 lib 47 unit test 実 pass）。Refs #114。`docs/specs/issue-114-real-adapter-l1-e2e.md`（AC2・AC3 の bridge 層・AC7）を verify。配置: 各 adapter crate `src/host.rs` 内 `#[cfg(all(test, feature = "host"))]` unit test — **in-process dlopen** で実規格バイナリを load し bridge の合成写像を pin（L1 プロセス面は [real-adapter-l1-e2e-tests](real-adapter-l1-e2e-tests.md) が担う）。実行入口: `cargo test -p ruviform-plugin-clap --features host` / `cargo test -p ruviform-plugin-vst3 --features host`（`#[ignore]` なしの通常層 — lane では `test-l1-real-plugin` の明示 `--lib --bins` step（`cargo test -p ruviform-plugin-clap -p ruviform-plugin-vst3 --features host --lib --bins`）で実行される — `--test l1_real_adapter_e2e -- --ignored` 単独では本層は実行されない）。

## Purpose

host-side bridge（子プロセス内で実 `.clap`/`.vst3` cdylib を dlopen し `PluginFactory`/`Plugin` 中立契約へ写像する層）が、実バイナリから採取した metadata・params・lifecycle を正確に中立形へ写像することを pin する。目的は (a) E2E より低位の layer で写像欠陥を局所化する、(b) handshake field-equality の子側（実 scan → `PluginDescriptor` 合成）を、親側 const literal と独立した既知値で検証する、(c) defined rejection（dlopen/entry/scan/lifecycle/tresult 失敗 → `Err(HostError)` 系・panic/abort しない）を固定する、(d) intern-once の bounded/once 性を固定する。bridge が E2E 層（プロセス・wire・shm）に依存せず単独で検証できることを保証する。

## Feature

規格アダプタ host bridge の中立契約写像 — `ClapLibrary`/`Vst3Library` の load・descriptor 合成・`factories()`・`Plugin` lifecycle/params/process 写像・defined rejection。

## Rules

- `ClapLibrary::load(path)` は `PluginEntry::load` で fixture cdylib（`current_exe` sibling `deps/{DLL_PREFIX}ruviform_plugin_clap{DLL_SUFFIX}` — 欠落は `assert!` fail・silent skip なし）を開き、`descriptors()` が load 時 scan の cached slice を返す（複数回呼出で同一値 — 再 scan/leak 増殖なし）
- CLAP descriptor 合成は fixture 既知値と完全一致: `id: "org.ruviform.clap-gain-poc"`・`name: "Ruviform Gain PoC"`・`vendor: "Ruviform"`・`version: "0.1.0"`（`env!("CARGO_PKG_VERSION")` 由来）・`features: ["audio-effect", "stereo"]`（順序保持 — `Descriptor::matches` の field-equality 契約と同一比較）
- `ClapPluginFactory::descriptor()` は load 時 cached 値を返し、`create()` は `ErasedPlugin`（`ClapPlugin` — `create_sendable` は default `None`・`PluginInstance` は `!Send`/`!Sync`）を返す
- CLAP lifecycle 写像: `init` → `PluginInstance::new`（中立 `Initialize` が規格 instantiate+`plugin.init` を駆動 — `create()` 段では instance を作らない）・`prepare` → `activate(PluginAudioConfiguration)`・`start_processing`/`stop_processing` → `Stopped`/`Started` processor 遷移・**teardown は `Drop` 駆動** — 中立 `Deactivate`/`Destroy`/`Shutdown` に plugin hook はなく export Runner の lifecycle 遷移のみ（export/lib.rs:424-429・646-666）ため、`Drop` で best-effort `stop_processing` → `instance.deactivate(stopped)` → instance drop（CLAP `plugin.destroy`）を駆動する
- CLAP params 表は params extension の count/info から中立 `ParamInfo` へ写像（fixture: 1 param・`ParamId(0)`・`[0.0, 2.0]`・default `1.0`・`unit: ""` — CLAP `param_info` に unit 面なし: **`""` は honest synthetic 値**で、host discovery が unit を失うため `"x"` 等の filler に"修正"しない）・param 表は **factory 構築時に eager 構築+intern**（throwaway instance/controller 列挙）— `params()` は pre-`init` を含む全 lifecycle で cached `&[ParamInfo]` を返す・`HostThreadCheck` は host 側 `declare_extensions` に登録しない（fixture の `get_extension::<HostThreadCheck>()` → `None` graceful fallback で挙動不変 — 不実の `is_audio_thread` 応答をしない）
- `Vst3Library::load(path)` は `libloading::Library::new` → `GetPluginFactory` で `IPluginFactory` ComPtr を得て、`descriptors()` が `countClasses`/`getClassInfo2`/`getClassInfoUnicode` 列挙のうち `category == "Audio Module Class"` のみ採用する（`"Component Controller Class"` は独立プラグインとして列挙しない — fixture は component+controller の 2 クラスを持ち component のみ採用）
- VST3 descriptor 合成は fixture 既知値と完全一致: `id` = component CID の raw 16byte 小文字 hex — **platform 依存** — **expected は test が `GainComponent::CID`（`pub(crate)` — same-crate unit test から直接参照可）raw 16byte から実行時に derive**・`#[cfg]`-conditional literal は regression pin のみ: Windows GUID-order `"3d2c1b5a5f4e7b6a8c9d0e1f2a3b4c5d"`・非 Windows big-endian `"5a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d"`・`name: "Gain"`・`vendor: "Ruviform"`・`version: "0.0.0"`・`features: ["Fx"]`（`subCategories`）
- VST3 lifecycle 写像（fixture の必須順序）: `createInstance(component CID)` → QI `IComponent`/`IAudioProcessor` → `getControllerClassId` → `createInstance(controller CID)` → `IEditController` → component・controller 双方 `initialize(context)`（最小 `IHostApplication` 供給）→ `setComponentHandler`（最小 `IComponentHandler` — fixture は非保持 acknowledge）／`prepare` → **`setActive(true)` → `setupProcessing(ProcessSetup{processMode: kRealtime, symbolicSampleSize: kSample32, …})` の順**（fixture は Activated 未満で `kResultFalse` — component.rs:597）→ `setProcessing(true)` → `process(ProcessData)` → `setProcessing(false)` → **instance `Drop` で** `setActive(false)` → `terminate`（両 object）→ release（中立 `Deactivate`/`Destroy` に plugin hook なし — teardown は `Drop` で駆動）
- VST3 params 表は `IEditController::getParameterCount`/`getParameterInfo` → `ParamInfo`（fixture: 1 param・`ParamId(0)`・`[0.0, 1.0]`・default `1.0`・`unit: ""`）
- 両規格: `save_state`/`restore_state`/`open_gui`/`close_gui` は `Plugin` 既定の `Err(HostError::Unsupported)` を継承 — bridge は規格 state/GUI 動詞を**一切呼ばない**（VST3 controller の `setState`/`getState` は `kResultOk` no-op stub であり、呼ぶと偽の state 成功を観測する — 呼出し自体を禁止）
- intern-once: `PluginDescriptor`/`ParamInfo` の `&'static` 境界は `Library::load` scan 時 + factory 構築時に限り `String::leak`/`Vec::leak` で実体化 — `descriptors()`/`params()`/`descriptor()` は intern 済み値を返すのみ・per-call/per-instance 増殖なし
- defined rejection: 不在 path・規格 entry 欠落バイナリ・scan 失敗・lifecycle 非 Ok（`kResultFalse`/`kResultErr`/`kNotImplemented`）・dlopen 失敗は全て `Err(ClapHostError)`/`Err(Vst3HostError)` または `Plugin` 側 `Err(HostError)` で収束 — panic・abort・UB・応答なしハングなし
- unsafe は `host` module 内 scoped `#[expect(unsafe_code)]` + SAFETY のみ（`PluginEntry::load`・`libloading` symbol 解決・COM vtable 呼出し・ProcessData 組立）

## Examples

- Given fixture cdylib が `target/<profile>/deps/` に存在する、When `ClapLibrary::load` → `descriptors()`、Then 1 要素の `PluginDescriptor` が上記既知値と完全一致（`#[cfg(feature = "host")]` unit test・`assert_eq!` — test binary 自身は dlopen せず bridge 経由のみ）
- Given 同 fixture、When `Vst3Library::load` → `descriptors()`、Then component のみ 1 要素・`id` が `GainComponent::CID` bytes derive の platform 別 hex と一致（regression pin literal: Windows `"3d2c1b5a5f4e7b6a8c9d0e1f2a3b4c5d"` / その他 `"5a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d"`）・controller クラスが採用されない
- Given `Vst3Plugin` が `Initialize` 済、When `prepare(config)` → `start_processing` → `process_host` → `stop_processing` → instance drop、Then 規格層で `setActive(true)` → `setupProcessing` → `setProcessing(true)` → `process` → `setProcessing(false)` → `setActive(false)` → `terminate` の順で駆動（`setActive(false)`/`terminate` は `Drop` 駆動 — 中立 `Deactivate` に plugin hook なし・fixture の `Activated`/`Processing` 状態機械が拒否しない順序）
- Given `ClapPlugin`/`Vst3Plugin` が任意 lifecycle 状態、When `save_state`/`restore_state`/`open_gui`/`close_gui`、Then 常に `Err(HostError::Unsupported)` — 規格 state/GUI 動詞を子プロセス内で一度も呼ばない（state stub の `kResultOk` no-op を「成功」と誤観測しない）
- Given 不在 path・`GetPluginFactory`/`clap_entry` 欠落 fixture・規格外バイナリ、When `Library::load`/`descriptors()`/`create()`、Then `Err(ClapHostError)`/`Err(Vst3HostError)` — panic しない・partial instance が残らない
- Given `descriptors()` を複数回呼出、Then 同一値（`PartialEq`）かつ load 時 scan の cached 値を返す — 呼出毎の再 scan/leak 増殖なし（intern-once の once 性）

## Guarantees

- bridge の descriptor/param 合成は実規格バイナリの metadata からのみ駆動され、const literal（E2E の親側 pin）とは独立した既知値検証で固定される
- 規格 lifecycle の順序（CLAP `init`/`activate`/`start_processing`/`process`/`stop_processing`/`deactivate`・VST3 `initialize`/`setActive`/`setupProcessing`/`setProcessing`/`process`/`terminate`）が中立 `Plugin` 動詞に正確に写像され、fixture の状態機械が拒否しない順序で呼ばれる
- defined rejection の網羅: dlopen/entry/scan/lifecycle/tresult の全失敗面が `Err` 系で収束し、panic・abort・UB・ハングを残さない
- intern-once は bounded（descriptor ~5+features・param name+slice）・once-per-load・process-scoped（短命子プロセス）・feature `host` 封じの 4 性質を満たす — plugin-core の no-forged-statics 方針の sanctioned exception として固定
- `#[non_exhaustive]` error enum・scoped `#[expect(unsafe_code)]` + SAFETY・`create_sendable` default `None`・`unit: ""` の合成 pin が契約として固定される

## Non-goals

- L1 プロセス面（spawn・handshake・wire・shm block・`PluginHost`/`OutOfProcessSpec`・kill/timeout 異常系）は [real-adapter-l1-e2e-tests](real-adapter-l1-e2e-tests.md) が担う — 本 spec は in-process bridge 層のみ
- lane/mise/ci 機構は [l1-real-plugin-lane-tests](l1-real-plugin-lane-tests.md) が担う
- RT 制約の証明（`rt_` 測定窓）は対象外 — 子内 bridge の `process_host` は L1 経路で RT 検証の対象外（`rt_` 既存テストは不変グリーンのみ要求）
- `Send` 契約の全域検証（`create_sendable` default `None` の pin のみ — `!Send` 性の assert は行わない）
- CLAP `state` ext・VST3 `IBStream`/`setComponentState`・GUI・VST3 拡張面（`IConnectionPoint`・note/event bus・`IProgramListData` 系）は deferred — 実装しない
- OFX host bridge（video・NONRT — Issue #118 追跡）は本 spec 対象外

## Risk

- bridge 写像欠陥が unit test なしに E2E にのみ漏れると、欠陥層（bridge vs runner vs wire vs host）の局所化が不能になる — 本 spec が in-process 層の独立証拠を提供する
- `descriptors()`/params 表の intern-once 誤り（再 scan・leak 増殖・forged statics）は handshake field-equality の偽 fail や process-scoped leak の無限増殖に直結する — once 性の pin で防ぐ
- VST3 `uid()` の platform-order を単一 literal で pin すると platform 毎に偽 fail — `#[cfg]`-conditional の両 literal pin で防ぐ
- `HostThreadCheck` 登録・VST3 state 動詞呼出しを誤って入れると graceful fallback/no-op stub が「不実の応答」を返す — 呼出し禁止を Rules で固定
