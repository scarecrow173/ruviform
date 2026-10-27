---
id: "IMPL-034MRCMPz6M4DGPJGY0ZgX"
type: "impl"
status: "completed"
title: "Issue #114 VST3 host bridge — Task 2 implementation"
created: "2026-10-27"
updated: "2026-10-27"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-vst3/src/host.rs"
      - type: "file"
        path: "docs/impl/ir/issue-114-vst3-host-bridge-task-2-implementation.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-vst3/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/src/lib.rs"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "docs/tasks/vst3-host-bridge.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/vst3-host-bridge.md"
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

# Issue #114 VST3 host bridge — Task 2 implementation

## Summary

Issue #114 Task 2 完了 — plan actions 5–9（VST3 host bridge 全範囲 —
**本 Issue の最大新規コード**: COM host objects 手実装・ProcessData
組立・pinned teardown 順）を実施。`ruviform-plugin-vst3` に feature
`host` で feature-sealed な host-side bridge `src/host.rs`（+2,214
行）を新設し、`vst3 0.3.0` raw COM bindings + `libloading` で実
`.vst3` cdylib を dlopen・class scan・COM host objects・lifecycle
写像して plugin-core の中立契約 `PluginFactory`/`Plugin` へ載せた。

- `Cargo.toml`: `[features] host = ["dep:libloading",
  "dep:ruviform-plugin-export"]` + `libloading = { version =
  "0.8.9", optional = true }`（OFX adapter と同 version — 新規
  version 導入なし）+ `ruviform-plugin-export`（path・optional）。
  `vst3 = "0.3.0"` は既存必須 dep のまま optional 化しない
  （`Class`/`ComWrapper`/`Interface`/constants は `vst3::`
  re-export 経由で共用）。
- `src/lib.rs`: `#[cfg(feature = "host")] pub mod host;`（既存
  `mod block/component/controller/factory/gain` と `mod strings`
  の間に配置）+ crate doc に host bridge 節。
- `src/host.rs`: `Vst3Library`（`libloading::Library` +
  `GetPluginFactory` → `IPluginFactory` ComPtr — platform module
  entry/exit hook・`LoadGuard` RAII・load 時 class+param scan・
  intern-once）・`descriptors()`/`factories()`・`Vst3PluginFactory`
  （cached descriptor + `create()` — `create_sendable` は raw COM
  pointer `!Send` のため default `None` 維持）・`Vst3Plugin`
  （`Phase` enum による Dormant→Initialized→Activated→Processing
  の ordered lifecycle + `Drop` 駆動 pinned teardown）・
  `Vst3HostError`（`#[non_exhaustive]` — `Library`/`ModuleInit`/
  `MissingEntryPoint`/`NullFactory`/`Com(tresult)`）・COM host
  objects 4 種（`BridgeHostApplication`/`BridgeComponentHandler`/
  `BridgeParamQueue`/`BridgeParameterChanges` — `Class`/`ComWrapper`
  実装・component.rs test の `TestQueue`/`TestChanges` 同型先例）・
  `fixture_lib_name` + `#[cfg(all(test, feature = "host"))]` unit
  test 16 件。

敵対的レビュー 1 ラウンド適用 — 確定修正 8 件（LoadGuard exit-hook
pairing・single-component cast branch・`restartComponent`
`kNotImplemented`・drop tresult `debug_assert`s・
`continousTimeSamples` dead-write removal・count caps・load-hoisted
param tables・transport coverage）を全て実装に反映（「Review
findings and resolutions」参照）。全 gate green 実績。

## Context

approved plan `docs/plans/implement-issue-114-real-adapter-l1-e2e.md`
の Task 2（`docs/tasks/vst3-host-bridge.md`・actions 5–9）。
upstream: approved design
`docs/designs/issue-114-real-adapter-l1-e2e.md`（決定サマリ c/e —
「VST3 bridge」「COM host objects」「lifecycle 写像」「'static
メタデータ実体化」「依存追加の判断」）・spec
`docs/specs/issue-114-real-adapter-l1-e2e.md`（proposed・AC1・AC3・
AC7）・ADR `docs/adr/plugin-architecture-clap-vst3.md`（adapter
責務・bridge 配置）・`docs/adr/plugin-binding-method-clack-for-clap-coupler-rs-vst3-for-vst3.md`
（raw `vst3` bindings 採用根拠・`vst3-host`/`vst3-sys`/`vst3_com`
不採用）・TSpec `docs/test-specs/adapter-host-bridge-contract-tests.md`
（`verified-by` edge）。Task 1（CLAP bridge）と別 crate で構造的に
独立・Task 3（runner bin）の VST3 側前提。

## Implementation

### Changes

- `crates/ruviform-plugin-vst3/Cargo.toml`（+19 行）: `[features]
  host` 節新設 + `libloading`/`ruviform-plugin-export` optional dep
  追加。`vst3` 必須維持・`libloading` version と runner 前提を
  comment で記録。
- `crates/ruviform-plugin-vst3/src/lib.rs`（+16 行）: `pub mod
  host;` + crate doc に「Host bridge (`feature = "host"`)」節
  （dlopen・class scan・COM bindings・feature-sealed・unsafe 境界
  の明示）。
- `crates/ruviform-plugin-vst3/src/host.rs`（新規・2,214 行）:
  - **`Vst3Library`** — `Library::new`（dlopen — crate の unsafe
    境界の一つ・scoped `#[expect(unsafe_code)]` + SAFETY）→
    `module_entry`（platform hook `InitDll`/`ModuleEntry`/
    `bundleEntry` — exported されていれば呼出・decline は
    `Err(ModuleInit)`・absent は defined no-op・other platforms は
    hook 自体 no-op）→ **`LoadGuard::armed`**（entry 成功後の全
    early return を paired exit hook + unmap で被覆する RAII —
    `disarm` で `LibraryInner` へ移譲）→ `GetPluginFactory`
    symbol 解決（`MissingEntryPoint`）→ `ComPtr::from_raw`
    （`NullFactory`）→ `scan_classes` + `enumerate_params`（load
    時 1 回）→ `LibraryInner{factory, exit_hook, _lib}` を
    `Arc` 化。`descriptors()`/`factories()` は cache 読取りのみ。
    `LibraryInner::drop` は factory release → exit hook → unmap
    の順序保証（`_lib` は最後に drop — OFX `OfxLibrary::share`
    precedent）。
  - **`LoadGuard`** — `module_entry` 成功後に `Library`+exit hook
    を所有し、scan/param 列挙の失敗でも module の exit hook を
    **unmap 前**に実行する RAII guard（entry/exit pairing — VST3
    platform module 契約）。`disarm` で `(Library, exit_hook)` を
    `LibraryInner` へ移し義務を継承。
  - **`scan_classes`** — `countClasses` を `MAX_SCAN_CLASSES=
    1_024` で clamp → `getClassInfoUnicode`（`IPluginFactory3`）
    → `getClassInfo2`（`IPluginFactory2`）→ `getClassInfo`
    （base）の fallback 順で列挙（非 `kResultOk` は hard
    `Err(Com)` — 記述不能な class は broken scan）→
    `category == "Audio Module Class"` のみ採用（`"Component
    Controller Class"` は pair 対象で独立 plugin ではない —
    `controller_class_is_not_a_plugin` が pin）。`ClassInfo` が
    `PClassInfoW`/`PClassInfo2`/`PClassInfo` の 3 spelling を
    正規化（base `PClassInfo` は vendor/version/subCategories
    非保持で honest 空文字）。
  - **descriptor `id`** — `tuid_hex`: raw `TUID` 16 byte
    （`vst3::uid` platform-order layout — 文字列 GUID ではない）
    を小文字 hex 32 文字へ。test は `GainComponent::CID` raw
    bytes から `expected_id()` を独立導出（production `tuid_hex`
    を restate せず検証）+ `#[cfg]` literal は regression pin
    （Windows GUID-order `"3d2c1b…"`・非 Windows big-endian
    `"5a1b2c…"`）。
  - **COM host objects（`Class`/`ComWrapper` 手実装）**:
    `BridgeHostApplication`（`IHostApplication` — `getName` は
    "Ruviform VST3 Host"・`createInstance` は `kNotImplemented`
    — bridge は shared service を host しない honest 拒否）・
    `BridgeComponentHandler`（`IComponentHandler` — acknowledge-
    only: `beginEdit`/`performEdit`/`endEdit` は `kResultOk`
    （edit 拒否は plugin の UI thread を wedge し得る）・
    **`restartComponent` は `kNotImplemented`** — `kResultOk`
    は flag-requested resend（param values/component state/
    latency）を promise するが connection point が v1 に無く
    実行されない — 不実の ack より honest 拒否）・
    `BridgeParamQueue`（`IParamValueQueue` — `RefCell<Vec<
    (i32,f64)>>`・`getParameterId`/`getPointCount`/`getPoint`/
    `addPoint`=`kNotImplemented`（plugin からは read-only）・
    `try_borrow` の `kResultFalse` が re-entrant FFI の defined
    応答 — RefCell panic は ABI を越えない）・
    `BridgeParameterChanges`（`IParameterChanges` — fixed queue
    set・`getParameterCount`/`getParameterData`/`addParameterData`
    =null）。全 vtable method が scoped `#[expect(unsafe_code)]`
    + SAFETY。
  - **`enumerate_params`** — params は distributed controller が
    所有するため load 時に class 毎 1 回 throwaway
    component+controller pair を instantiate（
    `create_instance::<IComponent>` → `getControllerClassId` →
    **controller_cid == component_cid なら同一 object への
    `cast::<IEditController>`** — single-component plugin。
    `createInstance` では disconnected clone を作ってしまう）→
    `BridgeHostApplication` context で `initialize`（shared
    object では 1 回のみ）→ `getParameterCount` を
    `MAX_SCAN_PARAMS=4_096` で clamp → `getParameterInfo` 列挙
    （index 毎の失敗は deliberate skip — 記述不能 param は
    fabricated entry ではなく describable subset）→
    `normalizedParamToPlain` で min/max/default を plain domain
    へ（normalization law を honest に反映）→ `terminate`+
    release 後 intern 済み表を返す。instantiate 不能・CID 取得
    失敗は honest 空表 — 実失敗は `init` で表面化。
  - **`map_param_flags`** — `kCanAutomate`→AUTOMATABLE・
    `kIsReadOnly`→READONLY・`kIsHidden`→HIDDEN・`kIsBypass`→
    BYPASS・`stepCount>0 || kIsList`→STEPPED。`kIsWrapAround`/
    `kIsProgramChange` は中立 counterpart 無しで drop。
  - **`Vst3Plugin`** — `Phase` enum（`Dormant < Initialized <
    Activated < Processing`・`PartialOrd/Ord` derive）で
    lifecycle を型付き遷移:
    `init` — `Phase::Dormant` gate → `createInstance(component
    cid)` → `cast::<IAudioProcessor>` → `getControllerClassId`
    → single-component 分岐（`cast`）or distributed
    `createInstance::<IEditController>` → component
    `initialize(context)` → `!single_component` なら controller
    `initialize`（shared object は 1 回のみ — 失敗時は
    component `terminate` で rollback）→ `BridgeComponentHandler`
    を `setComponentHandler` で注入（途中失敗は全て terminate
    を逆順 rollback — ComPtr drop が release を担い leak 無し）
    → `host_app`/`handler` を field 保持（plugin が context/
    handler を retain し得るため instance 寿命まで生存）。
    `prepare` — `Phase::Initialized` gate → `max_frames >
    WIRE_MAX_FRAMES` は `UnsupportedConfig`（scratch 確保前に
    oversized alloc を遮断 — CLAP bridge と同 bound）→
    **`setActive(1)` → `setupProcessing(ProcessSetup{kRealtime,
    kSample32, maxSamplesPerBlock, sampleRate})` の順**（fixture
    は `Activated` 未満で `kResultFalse` — pinned order。`setup
    Processing` 失敗時は `setActive(0)` で rollback → component
    を合法 `Initialized` に戻す）→ `Phase::Activated` →
    `bus_channels`（`getBusCount`/`getBusInfo` — plugin-reported
    `channelCount` は `MAX_SCAN_CHANNELS=64` clamp・照会不能は
    `CHANNELS_PER_BUS=2` fallback）で negotiated topology 確定 →
    input scratch（`in_channels × max_frames`）・output scrap・
    channel pointer arrays（`Vec<*mut f32>` — prealloc・per-block
    repoint のみ）・param 毎の `BridgeParamQueue`（capacity
    `MAX_EVENTS_PER_BLOCK=1_024` で予約）+ `IParameterChanges`
    shell を一括確保 — `process` は clear/push/repoint のみで
    alloc しない。
    `process` — `Phase::Processing` gate・`frames > max_frames`
    defined `Error`（v1 では分割不能）→ `block_frames`（declared
    block と wired channel 最短の defensive min）→ input bounce
    （`HostInput` `&[f32]` → owned scratch — `channelBuffers32`
    は `*mut *mut Sample32` で immutable 偽装禁止・unconnected
    channel は silence）→ output は `HostOutput.channel_mut` の
    実 `&mut [f32]` へ `channelBuffers32` を直接 repoint（mutable
    既知 — task 明示許容の alias・unwired channel は owned scrap
    で defined drop）→ `fill_param_queues`（`EventKind::ParamValue`
    normalized → `(sample_offset, value)` point — foreign id・
    out-of-block offset・cap 超過は defined drop・queue 内で再
    borrow 失敗も skip）→ `ProcessData` 組立（`numInputs=1`・
    `numOutputs=1` — 単一 in/out audio bus defined 境界・
    `kSample32`・`kRealtime`・`outputParameterChanges`/
    `inputEvents`/`outputEvents` = null — v1 非対応）→
    `process(&mut data)` → `kResultOk` 時は `getTailSamples` を
    `ProcessResult` tail へ、非 Ok は `Error`。
    `map_process_context` — `TransportInfo` snapshot →
    `ProcessContext`: `Playing`→`kPlaying`・`position()`→
    `projectTimeSamples`（`TimeBase::sample_rate` convert・
    `Rounding::NearestEven`）・musical snapshot →
    `projectTimeMusic`/`barPositionMusic`（ticks/ppqn）・`tempo`
    （bpm_milli/1000）・time signature + 対応 `*Valid` flag。
    `None` transport → null context（fixture は null 許容の
    free-running boundary）。**`continousTimeSamples` は常 0・
    `kContTimeValid` 非設定** — `position()` は transport
    playhead で engine-continuous time ではなく、flag 無しの
    dead write は spec-conforming reader が ignore する領域。
    `start_processing`/`stop_processing` — `Phase` gate +
    `setProcessing(1)`/`(0)` — fixture は activated 未満・
    processing 外を `kResultFalse` で拒否するため phase 遷移は
    `kResultOk` 時のみ commit。
    `Drop` — **pinned 対称 teardown**: `Phase::Processing` 残留
    なら先に `setProcessing(0)`（fixture は Processing 中の
    `setActive(0)` を `kResultFalse` 拒否 → stop 先行が対称）→
    `Activated` 以上なら `setActive(0)` → `Initialized` 以上なら
    `!single_component` 時 controller `terminate` → component
    `terminate`（shared object の terminate は 1 回 — 2 回目は
    fixture が拒否）。各 tresult を `debug_assert_eq!` で assert
    — 順序違反は test 時 panic・release は best-effort。field
    宣言順で `library: Arc<LibraryInner>` が最後 — 全 ComPtr
    release（mapped vtable 経由）→ factory release → exit hook
    → unmap の順序保証。
  - **`Vst3HostError`** — `#[non_exhaustive]`。`Library`（
    `libloading::Error` verbatim — `source()` 供給）・
    `ModuleInit`・`MissingEntryPoint(&'static str)`（symbol 名
    搬送）・`NullFactory`・`Com(tresult)`（raw result code —
    vtable 失敗の defined 搬送）。
  - **`fixture_lib_name`** — `{DLL_PREFIX}ruviform_plugin_vst3
    {DLL_SUFFIX}` — T3 runner/T4 E2E 共有の fixture 解約規約。
  - **unit test 16 件**（`#[cfg(all(test, feature = "host"))]`・
    in-process dlopen — fixture は `current_exe` sibling `deps/`
    解決・欠落は `assert!` fail）: descriptor 既知値（id は
    `GainComponent::CID` bytes derive・cfg literal pin・name
    `Gain`・vendor `Ruviform`・version `0.0.0`・features `["Fx"]`
    順序保持）・`"Component Controller Class"` filter・cached
    同一性（ptr-eq）・missing path `Err(Library)`・non-VST3
    binary `Err(MissingEntryPoint|Library)`・factory cached
    descriptor + `create_sendable` `None`・params 全 lifecycle
    同一（pre-`init` 含む・`ParamId(0)` `[0.0,1.0]` default
    `1.0` unit `""`・`AUTOMATABLE` flag）・lifecycle roundtrip
    passthrough・param event normalized 到達（0.25→gain 0.25 —
    fixture は normalized/plain identity）・`process` before
    start `Error`・oversized block `Error`（negotiate 32・drive
    64）・`drop` mid-Processing teardown 完了（debug_assert が
    順序違反を検出）・state/GUI verbs `Err(Unsupported)`・
    `max_frames > WIRE_MAX_FRAMES` `UnsupportedConfig`・
    **`transport_snapshot_maps_to_process_context`**（Some arm
    field-by-field pin — flag 集合・projectTimeSamples 1:1・
    PPQ・`continousTimeSamples==0`・`kContTimeValid` 非設定・
    `None` arm）・**`process_with_transport_snapshot_is_defined`**
    （Some(transport) 経路で processContext 非 null の process
    往復）。

### Review findings and resolutions（敵対的レビュー 1 ラウンド — 確定修正 8 件全適用）

- **`LoadGuard` exit-hook pairing（fixed）**: `module_entry` 成功後
  に `GetPluginFactory` 解決・factory null・class/param scan で
  失敗すると、`Library` drop の unmap だけが走り module の exit
  hook（`ExitDll`/`ModuleExit`/`bundleExit`）が実行されなかった
  — platform module 契約の entry/exit pairing 違反。`LoadGuard`
  RAII を導入し entry 成功後の全 early return を paired exit
  hook で被覆（`disarm` で `LibraryInner` へ義務移譲 —
  `LibraryInner::drop` も exit hook → unmap の同順序を保持）。
- **single-component cast branch（fixed）**: component が
  `getControllerClassId` で自分の CID を返す plugin（component
  と edit controller が同一 object）に `createInstance` を使うと
  disconnected な controller clone が作られ、`initialize` が別
  object に走る → `controller_cid == component_cid` 時は
  `component.cast::<IEditController>()` で同一 object の interface
  を解決。`initialize`/`terminate` も shared object では 1 回のみ
  （2 回目の `initialize`/`terminate` は legitimately 拒否され
  得る）— `init` と `enumerate_params` の両経路に適用。
- **`restartComponent` `kNotImplemented`（fixed）**: 初版は
  `IComponentHandler` の全 method が `kResultOk` ack だった —
  `restartComponent(flags)` の `kResultOk` は flag-requested
  resend（param values/component state/latency）の実行を promise
  するが、v1 には plugin への connection point が無く resend は
  行われない。不実の ack より honest な `kNotImplemented` 拒否に
  変更（`beginEdit`/`performEdit`/`endEdit` は `kResultOk` 維持 —
  edit 拒否は plugin の UI thread を wedge し得る）。
- **drop tresult `debug_assert`s（fixed）**: `Drop` の teardown は
  best-effort で tresult を無視していた — しかし fixture は
  `Processing` 中の `setActive(0)` や `Activated` 中の
  `terminate` を `kResultFalse` で拒否するため、順序違反は silent
  pass になり得た。`debug_assert_eq!(result, kResultOk)` を各段に
  追加 — 順序違反は debug/test 時に panic で検出（
  `drop_mid_processing_completes_teardown` が mid-Processing drop
  を pin）・release では best-effort 継続。
- **`continousTimeSamples` dead-write removal（fixed）**: 初版は
  `position()` を `continousTimeSamples` にも書いていた —
  `kContTimeValid` flag は設定せず。flag 無しの値は spec 上
  undefined で spec-conforming reader が ignore する dead write
  に過ぎない上、`position()` は transport playhead で
  engine-continuous time ではない語義の誤り。field を 0 初期化の
  まま残し `kContTimeValid` を立てない honest boundary に修正
  （test が `continousTimeSamples == 0` を pin）。
- **count caps（fixed）**: plugin-reported な count
  （`countClasses`・`getParameterCount`・`BusInfo.channelCount`）
  が garbage 値を返すと `Vec::with_capacity`/`vec![…]` が abort
  し得る → `MAX_SCAN_CLASSES=1_024`・`MAX_SCAN_PARAMS=4_096`・
  `MAX_SCAN_CHANNELS=64` で scan loop・allocation 双方の前に
  clamp（CLAP bridge と同 `MAX_SCAN_*` 規約 — magnitude は
  honest plugin より十分大きい defined truncation）。
- **load-hoisted param tables（fixed）**: 初版は `params()` を
  instance 経由の lazy 列挙としていたが、中立 `params()` は
  pre-`init` を含む全 lifecycle で呼べる必要がある → load 時に
  class 毎 throwaway component+controller pair で eager 列挙し
  intern 済み `&'static [ParamInfo]` を `params_tables` cache に
  保持（`factories()`/`create()` は cache 読取りのみ）。test が
  pre-`init`〜activated〜stopped の全地点で同一表を pin。
- **transport coverage（fixed）**: `map_process_context` の
  `Some(transport)` arm が test で一度も駆動されていなかった →
  `transport_snapshot_maps_to_process_context`（flag 集合・
  `projectTimeSamples` convert・PPQ 各 field・`continousTime`
  dead-write 非存在・`None` arm の field-by-field pin）と
  `process_with_transport_snapshot_is_defined`（`Some` 経由の
  非 null `processContext` で `process` 往復 — fixture の
  `component.rs::transport` 写像と整合）を追加。

### Decisions During Implementation

- **param 表列挙は `Vst3Library::load` 時点へ hoist** — task doc
  の「factory 構築時 eager 列挙」をさらに一段早い load 時点へ。
  `factories()` は複数回呼べるため、per-call 列挙だと呼出し毎に
  throwaway pair を instantiate し intern 済み表を再 leak する。
  load 時 1 回 + `factories()` は cache 読取りのみにすることで
  「once-per-load・no per-call growth」の intern-once 不変条件を
  構造的に保持（CLAP bridge と同構造 — review の load-hoisted
  param tables 修正と同一解）。
- **`block_frames` は defensive min クランプ** — 中立
  `BufferView` 契約は equal-length を promise するため mismatch
  は本来到達不能だが、declared `block_frames` と実 channel 長の
  最小値で floor を置き `numSamples` が実バッファを超えない防御
  層にした（CLAP bridge は同一条件を `Error` で拒否 — 両者とも
  契約内では到達不能な防御層で、VST3 側は `numSamples` 過大を
  plugin に見せないことを優先）。両 bridge の非対称は意図的 —
  CLAP は equal-length 契約の厳格検査、VST3 は ProcessData
  numSamples の健全性 floor と責務が異なる。
- **`enumerate_params` の失敗は空表** — instantiate 不能・CID
  解決失敗・`initialize` 拒否は honest な `&[]` を返し、実失敗は
  `init` で表面化（scan と instantiate の責務分離 — CLAP と同
  policy）。
- **`init` の途中失敗は逆順 terminate** — `createInstance` 成功
  後の途中 `Err`（controller `initialize` 拒否・`setComponent
  Handler` 拒否）は既に `Initialized` に達した object のみ
  `terminate` で rollback。`ComPtr` drop が release を担うため
  leak 無し・`Initialize` を終えていない object に `terminate`
  は呼ばない（VST3 契約上 `Initialized` 未満への `terminate`
  は不正）。
- **`#![allow(clippy::unnecessary_cast)]` は file-level** —
  generated bindings の enum constants は `DefaultEnumType`
  （Windows `c_int`・他 `c_uint`）宣言で struct field は
  `int32`/`uint32` — `as` cast が片 platform で必要・片 platform
  で no-op になるため `factory.rs`/`component.rs` と同じ解決
  （file-level allow）。
- **`fixture_lib_name` を `pub` 公開** — T3 runner・T4 E2E が
  同一命名規約を共有するため（CLAP と同型・feature `host` 内に
  限定）。

## Related Experiments

### Adopted

なし — approved design/plan の確定実装のみ（実験ログなし）。

### Rejected

なし — 代替実装の評価は design の確定事項（raw `vst3` bindings・
`vst3-host`/`vst3-sys`/`vst3_com` 不採用・COM host objects 手実装）
をそのまま実装した。

## Validation

- gate（実績・全 green）:
  - `mise run fmt-check` — exit 0（`cargo fmt --all -- --check`）
  - `mise run check` — exit 0（`cargo check --workspace
    --all-targets` — `host` 未指定の default 構成で `host.rs`
    非 compile・無退行）
  - `mise run lint` — exit 0（`cargo clippy --workspace
    --all-targets -- -D warnings` clean）
  - `mise run test` — exit 0（`cargo test --workspace` —
    default 構成無退行）
  - `cargo test -p ruviform-plugin-vst3 --features host` —
    **47 unit test 全 pass**（bridge test 16 件 + 既存
    component/controller/factory/gain test 無退行）
  - `cargo clippy -p ruviform-plugin-vst3 --all-targets --features
    host -- -D warnings` — exit 0（COM vtable/libloading 箇所の
    scoped `#[expect(unsafe_code)]` + SAFETY のみ・0 warning）
  - `cargo doc` — 0 warning
- 構造確認（実コマンド）:
  - `cargo tree -p ruviform-plugin-vst3` — `host` off で
    `libloading`/`ruviform-plugin-export` が non-optional dep 木
    に非出現・`vst3` は既存必須のまま
  - `cargo tree -p ruviform-plugin-vst3 --features host` —
    `libloading`/`ruviform-plugin-export` が `host` 指定時のみ
    出現
- 対象 test-spec:
  `docs/test-specs/adapter-host-bridge-contract-tests.md`
  （`verified-by` edge — descriptor 既知値・controller filter・
  cached 同一性・params 全 lifecycle・error 写像・fixture 欠落
  `assert!` 規則を担保）

## Non-Goals

- **規格別 runner bin（Task 3）** — `[[bin]]
  ruviform-plugin-vst3-l1-runner`（`required-features=["host"]`・
  `RUVIFORM_L1_REAL_PLUGIN_DIR` 解決）の実装は T3 scope。本 task
  は `factories()`/`fixture_lib_name` の供給のみ。
- **L1 E2E・lane 配線（Task 4）** — `tests/l1_real_adapter_e2e.rs`
  ・`test-l1-real-plugin`/`lint-l1-real-plugin` mise task・
  `l1-real-verify` ci job・`[dev-dependencies]
  ruviform-plugin-host` は T4 scope。
- **規格 state 動詞** — `setState`/`getState`/`setComponentState`
  を一度も呼ばない（`save_state`/`restore_state`/`open_gui`/
  `close_gui` は中立 `Plugin` default `Err(Unsupported)` 継承）。
- **`IComponent::activateBus`/`IAudioProcessor::setBusArrangements`**
  — plugin の default-active bus layout のみ駆動（非 default bus
  state を要求する plugin は v1 定義境界外）。
- **`ProcessData.outputParameterChanges`/`inputEvents`/
  `outputEvents`** — 常 null（plugin 発行 param change と VST3
  note/event list は v1 非対応 — 中立面は `EventList` の param
  event **入力**のみ搬送）。
- **multi-bus・kSample64・non-realtime process mode** — 単一
  in/out audio bus・kSample32・kRealtime の defined 境界
  （`setupProcessing` で kSample64 等は fixture 側が reject）。
- **macOS `.vst3` bundle 解決** — `Library::new` は raw dynamic
  library path を受ける（bundle directory の component 解約は
  runner 側の `RUVIFORM_L1_REAL_PLUGIN_DIR` 規約の範囲 —
  Windows/Linux で検証・macOS は 3-OS lane での実測対象）。

## Risks

- **in-process dlopen + raw COM vtable は広い unsafe 境界** —
  `Library::new`・全 vtable 呼出し・`ProcessData` 組立は scoped
  `#[expect(unsafe_code)]` + SAFETY で封じ込め（OFX
  `ffi/loader.rs` 先例型の範囲内）。実害範囲は L1 runner child
  process に限定 — 短命・exit 回収。
- **COM object の refcount は `ComPtr`/`ComWrapper` に依存** —
  plugin が host object（context/handler/queue）を retain する
  場合は `host_app`/`handler` field と queue `ComPtr` が instance
  寿命まで生存を保証。plugin が vtable 外参照を不正に行う場合は
  規格違反として検出対象外（dlopen 自体のリスクに包含）。
- **`RefCell` + `try_borrow` の queue** — re-entrant FFI（plugin
  が `process` 内から host queue を再呼出）は `kResultFalse` の
  defined 応答。borrow conflict は単一 thread 駆動の child
  contract 下で re-entrant のみ — cross-thread race は構造的に
  排除。
- **intern-once leak は bounded だが不可逆** — descriptor 毎
  ~5+N strings・param 毎 2 strings（name/unit）が process 寿命
  まで解放されない。once-per-load・process-scoped・feature-sealed
  の sanctioned exception（`descriptors()`/`params()` の
  per-call growth は cache で構造的に排除）。
- **throwaway pair が load 時に plugin code を実行** — param 列挙
  のため class 毎に createInstance+`initialize`+`terminate` が走る
  （scan 1 回 bounded・失敗は空表で退避）。
- **single-component plugin は fixture 未カバー** —
  `controller_cid == component_cid` の cast 分岐は契約上正しい
  が、fixture は distributed pair のため単体 test 未経由（実
  plugin でのみ発火 — `init`/`enumerate_params` 両経路に同分岐
  を実装済み）。

## Follow-ups

- Task 3 `docs/tasks/real-adapter-runner-bins.md` —
  `factories()`/`fixture_lib_name`/`#[feature = "host"]` 公開面を
  消費する最初の下游（`depends-on` edge 宣言済み）。
- Task 4 `docs/tasks/real-adapter-e2e-and-lane.md` — bridge unit
  test 層（`--lib`）+ E2E（`--ignored` gate）+ lane mise task +
  `l1-real-verify` job（3-OS — macOS `bundleEntry`/`bundleExit`
  hook と dlopen 経路の実測を含む）。
- **single-component plugin の実機カバレッジ** — `controller_cid
  == component_cid` cast 分岐は契約実装済みだが fixture 未経由。
  v1 以降に single-component fixture（または synthetic COM
  fixture）を別途検討。
- **`l1_proxy_e2e` flake の継続観測** — Issue #113 で記録済みの
  `wedged_child_write_converges_and_teardown_is_bounded` /
  `rejected_block_replies_resolve_pass_through` の spawn/handshake
  競合 flake は本 task 非起因・既存リスクとして継続観測（再発時
  は handshake timeout または test 直列化で別途 triage）。
