---
id: "DESIGN-034MIeiBWXpLjwgA1yolIT"
type: "design"
status: "approved"
title: "L1 規格アダプタの子プロセス組込み・実プラグイン E2E（CLAP + VST3 host-side bridge — Issue #114）"
created: "2026-10-26"
updated: "2026-10-27"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/114"
    - "https://github.com/scarecrow173/ruviform/issues/76"
    - "https://github.com/scarecrow173/ruviform/issues/113"
    - "https://github.com/scarecrow173/ruviform/issues/118"
    - "https://github.com/scarecrow173/ruviform/issues/123"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-114-real-adapter-l1-e2e.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/plugin-architecture-clap-vst3.md"
    - "docs/adr/plugin-binding-method-clack-for-clap-coupler-rs-vst3-for-vst3.md"
    - "docs/adr/plugin-rt-ownership-split.md"
    - "docs/adr/plugin-isolation-strategy.md"
    - "docs/adr/ofx-host-video-contract-and-c-abi.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/select-audio-device-backend.md"
    - "docs/specs/plugin-l1-out-of-process.md"
    - "docs/specs/engine-plugin-host-integration.md"
    - "docs/specs/ofx-host-adapter.md"
    - "docs/specs/rt-verification-lane.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-114-real-adapter-l1-e2e.md"
    - "docs/discovery/issue-114-real-adapter-l1-e2e.md"
    - "docs/designs/plugin-l1-out-of-process.md"
    - "docs/designs/engine-plugin-host-integration.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  derived-by:
    - "docs/plans/implement-issue-114-real-adapter-l1-e2e.md"
    - "docs/tasks/clap-host-bridge.md"
    - "docs/tasks/vst3-host-bridge.md"
    - "docs/tasks/real-adapter-runner-bins.md"
    - "docs/tasks/real-adapter-e2e-and-lane.md"
    - "docs/tasks/real-adapter-final-verification.md"
    - "docs/test-specs/adapter-host-bridge-contract-tests.md"
    - "docs/test-specs/real-adapter-l1-e2e-tests.md"
    - "docs/test-specs/l1-real-plugin-lane-tests.md"
  verifies: []
  verified-by:
    - "docs/test-specs/adapter-host-bridge-contract-tests.md"
    - "docs/test-specs/real-adapter-l1-e2e-tests.md"
    - "docs/test-specs/l1-real-plugin-lane-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# L1 規格アダプタの子プロセス組込み・実プラグイン E2E（CLAP + VST3 host-side bridge — Issue #114）

## Context

Issue #114 — #76 で確立した L1 アウトプロセス経路（`ruviform_plugin_export::run(factories)` runner + `PluginHost` コンポジット proxy）の検証が `GainPlugin`/`l1-test-worker`/`UnderwritePlugin`/`PanicPlugin` のモック範囲に留まる。本書は discovery `docs/discovery/issue-114-real-adapter-l1-e2e.md` の Open Questions と spec `docs/specs/issue-114-real-adapter-l1-e2e.md` の受入基準を、**2026-10-26 のユーザー決定**（規格スコープ CLAP+VST3 両方・runner オーサリング compile-time 固定・CI lane = 専用 mise task + `#[ignore]` ゲート）に基づき確定する。

上流確定事項（再決定しない）: 1 子 = 1 インスタンス（per-`Instantiate` spawn）・`--ruviform-plugin-id`/`--ruviform-shm-key` の argv 規約・`Hello`→`Scan` handshake（`spec.descriptor` field-equality）・`Failed`+once 通知+pass-through・自動再起動なし・`command_timeout` 既定 10s・wire/shm は planar `f32` audio 専用・video/`VideoPluginInstance` は L1 非対象・`run` の argv 規約不変（未知引数は defined rejection）・`CARGO_BIN_EXE_*` 利用は各 crate 内に閉じる（L1 design 規約）。

ブリッジは**子プロセス内**で動作する — runner バイナリが規格バイナリを dlopen して `PluginFactory`/`ErasedPlugin` へ写像し、host 側（親プロセス）が見るのは L1 wire のみ。本 Issue は親プロセス側のプラグイン load に触れない。

## 決定サマリ（discovery Open Questions + ユーザー決定 → 本書の結論）

| # | 論点 | 決定 |
| --- | --- | --- |
| a | runner オーサリング形 | **compile-time 固定 factory 構成**（ユーザー決定 — 軸 1 (a)）。規格毎の `[[bin]]` が `main` で sibling-of-exe の規格 fixture を load → `run(factories)`。`run` argv 規約・`OutOfProcessSpec` 不変。**rejected alternative**: 軸 1 (b) `run` argv 規約拡張（`spec.args` 経由の plugin path 受け渡し）・軸 1 (c) 汎用 loader runner（#76 判断継続 + ユーザー決定） |
| b | 規格別スコープ | **CLAP + VST3 両方を本 Issue に含める**（ユーザー決定 — 軸 3 (a) CLAP 先行 lean を上書き）。OFX は video・NONRT・`VideoPluginInstance` 構造上 L1 非対応 — **Issue #118 で追跡**（軸 4 (a) 確定。本書で境界を明文化） |
| c | bridge 配置 | **既存 adapter crate 内の `host` feature**（`ruviform-plugin-clap`/`ruviform-plugin-vst3` に `#[cfg(feature = "host")] pub mod host` + `required-features` runner bin）。新規 `*-host` crate は棄却 — 同一規格の双方向 ABI 知識は ADR `plugin-architecture-clap-vst3`（ホスト・制作双方）と OFX 先例（per-standard crate に host adapter を収容）の責務境界に整合し、baseline A.2 表の行追加も不要。feature gate で plugin-side cdylib の依存木・既定構成を不変に保つ |
| d | CLAP host binding | **`clack-host` `=0.2.0` を dev-dep → optional prod dep 昇格**（feature `host` で `dep:clack-host`、default features = `libloading` により `PluginEntry::load(path)` dlopen 経路）。`clack-extensions` に `clack-host` feature を有効化（host 側 event/param 型）— `clack-extensions` は非 optional の既存 prod dep のため plain 構文 `clack-extensions/clack-host`（`dep?/feat` weak 構文は optional dep 専用で適用不可）。dev-dep 側は現行 `clack-plugin` feature を維持（`load_from_clack` typed entry 経路は test 用のまま） |
| e | VST3 host binding | **`vst3` 0.3.0 raw COM bindings 直駆動 + `libloading` 0.8.9**（`vst3` は plugin-side で既存 prod dep・`libloading` は `ruviform-plugin-ofx` と同一バージョン — ともに feature `host` で optional 化はしない: vst3 は既存必須 dep のまま、libloading のみ optional）。外部 crate `vst3-host` 0.10.0 は評価の上**不採用**（依存追加の判断を参照）。host-side COM object は adapter 内で `Class`/`ComWrapper` 実装 |
| f | fixture 供給源 | **workspace 生成 cdylib をそのまま規格 fixture に使う** — `target/<profile>/ruviform_plugin_clap.dll`・`ruviform_plugin_vst3.dll`（実 `clap_entry`/`GetPluginFactory` export・dlopen 可能）。新規 fixture crate・外部ダウンロード・bundle 化は不要（`.clap`/`.vst3` 配布形の再現は deferred） |
| g | E2E トポロジー | **adapter crate 内 `tests/l1_real_adapter_e2e.rs`（`#![cfg(feature = "host")]` + 各テスト `#[ignore = "…"]`）** — `env!("CARGO_BIN_EXE_<runner>")` で runner 解決（same-crate 規約）・fixture 既知 metadata の const `PluginDescriptor` を `spec.descriptor` に供給（handshake field-equality が子側の実 metadata 合成写像を検証 — §`'static` メタデータ実体化）→ `PluginHost`/`OutOfProcessSpec` で `l1_proxy_e2e` 同型の spawn→lifecycle→shm block→state 動詞→destroy。device-independent（音声デバイス不使用 — 入出力はテストベクトル） |
| h | CI lane | **専用 mise task + `#[ignore]` ゲート**（ユーザー決定）。`test-l1-real-plugin`（`cargo build --features host` → `cargo test --features host --lib --bins` → `cargo test --features host --test l1_real_adapter_e2e -- --ignored` — 実装確定の 3 段）+ `lint-l1-real-plugin`（clippy `--features host`）。env-gated skip は不採用（repo に慣行なし・silent skip 禁止 — 欠落時は fail）。`RUVIFORM_L1_REAL_PLUGIN_DIR` を runner 側の任意 fixture dir override として定義。CI 反映は既存 `ci.yml`（workflow_dispatch）内の `l1-real-verify` job — **別 workflow ファイルは作らない** |
| i | descriptor 合成 | **bridge が実バイナリ metadata から `PluginDescriptor` を合成**（CLAP: `clap_plugin_descriptor` の id/name/vendor/version/features を順序保持で写像。VST3: `IPluginFactory2::getClassInfo2`/`IPluginFactory3::getClassInfoUnicode` — `"Audio Module Class"` category のみ採用・id = CID 16byte hex・name/vendor/version・subCategories → features）。`PluginDescriptor` の `'static` 境界は §`'static` メタデータ実体化の intern-once で満たし、host 側 `spec.descriptor` は fixture 既知値の const literal — handshake field-equality が合成写像の実検証となる |
| j | `unsafe` 封じ込め | `PluginEntry::load`/`libloading`/COM vtable 呼出しは `host` モジュール内の scoped `#[expect(unsafe_code)]` + SAFETY コメント — OFX `ffi/loader.rs` の先例型 |

### spec 修正記録（本書レビューで確定した spec 差分）

- spec は 2026-10-26 にユーザー決定反映済み（AC1–AC8 改訂・OFX → #118・argv loader 棄却・lane 形確定）。本書が追加する確定差分: **state 動詞は defined 応答往復として扱う** — workspace fixture は実 state 面を持たない（CLAP PoC は `audio-ports`/`params` extension のみ。VST3 fixture は `IComponent::getState`/`setState` が `kNotImplemented` stub・`IEditController::setState`/`getState` が `kResultOk` no-op stub — component.rs / controller.rs）ため、**bridge は規格の state 動詞を一切呼ばず**、`SaveState`/`RestoreState` は `Plugin` trait 既定の `Err(HostError::Unsupported)` を返す defined rejection の往復を E2E で観測する（verb が wire に乗る経路の実証 — spec AC2/AC3 の「state 動詞」文言と整合・追加 spec 差分なし）

## Architecture

### クレート構成と依存方向（baseline A.2 適合）

| クレート | 追加要素 | 追加依存 |
| --- | --- | --- |
| `ruviform-plugin-clap` | feature `host`・`src/host.rs`（`ClapLibrary`/`ClapPluginFactory`/`ClapPlugin`/`ClapHostError`）・`src/bin/ruviform-plugin-clap-l1-runner.rs`（`required-features = ["host"]`）・`tests/l1_real_adapter_e2e.rs` | prod（optional・`host`）: `dep:clack-host` `=0.2.0`・`dep:ruviform-plugin-export`・`clack-extensions/clack-host`（plain 構文 — `clack-extensions` は非 optional の既存 prod dep）。dev: `ruviform-plugin-host` |
| `ruviform-plugin-vst3` | feature `host`・`src/host.rs`（`Vst3Library`/`Vst3PluginFactory`/`Vst3Plugin`/`Vst3HostError` + host COM objects）・`src/bin/ruviform-plugin-vst3-l1-runner.rs`（`required-features = ["host"]`）・`tests/l1_real_adapter_e2e.rs` | prod（optional・`host`）: `dep:libloading` `0.8.9`・`dep:ruviform-plugin-export`。dev: `ruviform-plugin-host` |
| `ruviform-plugin-export` | **変更なし** — `run` argv 規約・exit 意味を不変のまま利用 | — |
| `ruviform-plugin-host` | **変更なし** — `PluginHost`/`OutOfProcessSpec`/`L1Instance` は #76 確定面のまま消費 | — |
| `ruviform-plugin-core` | **変更なし** — `PluginFactory`/`ErasedPlugin`/`wire` 契約は不変 | — |

**依存方向の検証**: adapter → `ruviform-plugin-export`・`ruviform-plugin-host`（dev）は baseline A.2 クレート表（adapter の許可依存 = Domain foundations + Primitives + 拡張層 core/host/export）に適合。新たな逆向き依存（export/host → adapter、adapter → adapter）は発生しない。feature `host` は default feature 集合に含めない（暫定規約「default feature は最小限」— 実バイナリ lane 専用のため）。

### host bridge の配置確定（c の確定）

同一規格の **plugin-side adapter と host-side bridge を 1 crate に収容**する — `ruviform-plugin-clap`/`ruviform-plugin-vst3` は規格の ABI 知識を持つ crate であり、ADR `plugin-architecture-clap-vst3`（規格アダプタはホスト・制作双方）と `ruviform-plugin-ofx`（host adapter が per-standard crate に存在する先例）の責務境界に整合する。

- **`#[cfg(feature = "host")] pub mod host`** — default 構成（cdylib plugin artifact + 既定 dep 木）は不変。bridge 部品は feature 有効時のみコンパイルされる
- **runner bin は adapter crate 内** — `[[bin]] required-features = ["host"]`。#76 design「adapter が runner bin を持つ」配置（baseline A.2 で adapter → export 合法）と `CARGO_BIN_EXE_*` same-crate 規約を同時に満たす
- **棄却した代替**: 新規 `ruviform-plugin-clap-host`/`-vst3-host` crate — baseline 表の行追加を要し、同一規格の双方向知識を 2 crate に分割する。adapter → adapter 依存禁止のため plugin-side 内部参照も不可能で、新規 crate の利点が薄い（責務は規格単位で既に分離済み）
- **`ruviform-plugin-export`/`plugin-host`/`plugin-core` は無変更** — wire/host/runner 基盤の公開契約に触れない

### CLAP bridge（`ruviform-plugin-clap::host`）

`clack-host` 0.2.0 の host API（`PluginEntry::load(path)` dlopen・`get_plugin_factory()`・`PluginInstance::new`/`activate`/`StoppedPluginAudioProcessor::start_processing`/`StartedPluginAudioProcessor::process`/`stop_processing`/`deactivate`）を `PluginFactory`/`ErasedPlugin` へ写像する薄い写像層。

```rust
// 概念形（feature = "host"）。シグネチャの正本は plan/実装で確定
pub struct ClapLibrary { /* PluginEntry — loaded .clap shared object */ }
impl ClapLibrary {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ClapHostError>;
    pub fn descriptors(&self) -> &[PluginDescriptor];        // entry factory scan → 合成
                                                             //   （'static 実体化は §`'static` メタデータ実体化）
                                                             //   — 実装確定: load 時 scan の cached 表を
                                                             //     slice で返す（Vec ではない）
    pub fn factories(&self) -> Vec<ClapPluginFactory>;        // descriptor 毎の bridge factory
}
#[non_exhaustive]
pub enum ClapHostError { /* Library/Entry/Descriptor/... */ }
pub struct ClapPluginFactory { /* PluginFactory — cached descriptor + create() */ }
struct ClapPlugin { /* PluginInstance<BridgeHandlers> — !Send・子 single-thread */ }
```

- **lifecycle 写像**: `init` → `PluginInstance::new`（clap `plugin.init` 経路）・`prepare` → `activate(PluginAudioConfiguration)`・`start_processing`/`stop_processing` → clap 同名動詞・`process_host` → `StartedPluginAudioProcessor::process`（`HostInput`/`HostOutput` view → `InputAudioBuffers`/`OutputAudioBuffers` 変換、`EventKind::ParamValue` → `params` extension の in-events、`ProcessResult` → CLAP status 読取り）
- **buffer 受け渡し（実装確定差分 — Task 1）**: `HostInput`/`HostOutput` ともに **bridge-owned scratch 経由の bounce**（input copy-in・output copy-back）。planning 時点では output を `HostOutput.channel_mut` の `&mut [f32]` 直接 alias に許容したが、中立 `process_host` の outputs は `&mut dyn BufferViewMut` で 1 live borrow につき 1 `&mut [f32]` しか yield しない — multi-channel 直接 alias は raw-pointer 組立（追加 `unsafe`）を要し、unsafe 境界を `PluginEntry::load` 1 箇所に保つ方針（決定 j）と衝突するため output も対称 bounce に確定（unconnected output channel は defined drop・declared block と channel slice の長さ不一致は `ProcessStatus::Error` の defined rejection）。VST3 側は `channelBuffers32` への直接指しを維持 — 両 bridge の非対称は意図的（IR `issue-114-clap-host-bridge-task-1-implementation.md`）
- **params**: `params` extension の count/info → 中立 `ParamInfo` 表（denormalize 含む — `name`/`unit` の `'static` 実体化は §`'static` メタデータ実体化）
- **descriptor**: `clap_plugin_descriptor` の id/name/vendor/version/features（順序保持）→ `PluginDescriptor` 全 field 写像（`'static` 実体化は §`'static` メタデータ実体化）— handshake field-equality が写像の実検証となる（親側は const pin — §descriptor 合成と handshake 整合）
- **thread-check 方針**: fixture は `plugin.init` で `HostThreadCheck` extension を解決し、host が `is_audio_thread == Some(false)` と応答した `process` を defined rejection する（plugin.rs）。runner は単一 command thread で `process` を駆動するため、bridge の `HostHandlers::declare_extensions` は **`HostThreadCheck` を登録しない** — extension 不在は `host.get_extension::<HostThreadCheck>()` が `None` を返す fixture の graceful fallback（check 不実施・挙動不変）に落ちる。`is_audio_thread = true` と応答する代替は棄却 — command thread は RT audio thread ではなく、不実の capability 申告は thread 契約違反を覆い隠す。fixture が解決を試みる host extension は `thread-check` のみ（plugin.rs: `get_extension` 呼出は init の 1 箇所）
- **error 写像**: `PluginInstanceError`/`PluginEntryError` → `HostError`/`ClapHostError` への defined 写像。load/instantiate 失敗は panic-free の `Err` か `create()` 内 panic（`run` が catch → exit 1 → spawn `Err(Ipc)`）のいずれかの defined rejection に収束
- **`Send`**: `PluginInstance` は `!Send`（clack-host 定義）— 子は single-thread、`create_sendable` は default `None`（L1 経路で不要求）
- **teardown（実装確定差分 — Task 1）**: 中立 `Plugin` 面には deactivate/destroy hook がないため、`Drop for ClapPlugin` が best-effort に `stop_processing`（still-started processor の `into_stopped`）→ `deactivate` → instance drop（`plugin.destroy`）を駆動する。`entry`（clack 内部 Arc-backed）を factory/instance が clone するため module unmap は最後の instance drop まで発生しない（OFX `OfxLibrary::share` と同型の lifetime 保証）
- **unsafe**: `PluginEntry::load` のみ（dlopen）— `host.rs` 内 scoped `#[expect(unsafe_code)]` + SAFETY

### VST3 bridge（`ruviform-plugin-vst3::host`）

`vst3` 0.3.0 は raw COM bindings のみ（`ComPtr`/`ComRef`/`Class`/`ComWrapper` — host API なし）のため、adapter が**最小 host** を自前実装する。`libloading` で dlopen → `GetPluginFactory` 取得 → COM 駆動。

```rust
// 概念形（feature = "host"）
pub struct Vst3Library { /* libloading::Library + IPluginFactory ComPtr */ }
impl Vst3Library {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Vst3HostError>;
    pub fn descriptors(&self) -> &[PluginDescriptor];        // class-info scan → 合成
                                                             //   （'static 実体化は §`'static` メタデータ実体化）
                                                             //   — 実装確定: load 時 scan の cached 表を
                                                             //     slice で返す（Vec ではない）
    pub fn factories(&self) -> Vec<Vst3PluginFactory>;
}
#[non_exhaustive]
pub enum Vst3HostError { /* Library/Entry/ClassInfo/Instantiate/Com(tresult)/... */ }
pub struct Vst3PluginFactory { /* PluginFactory */ }
struct Vst3Plugin { /* component + controller pair + host objects — !Send */ }
```

- **load**: `libloading::Library::new` → `GetPluginFactory` → `IPluginFactory` ComPtr。platform module hook（Windows `InitDll`・Linux `ModuleEntry`・macOS `bundleEntry`）は export されていれば呼ぶ（fixture は全て trivially-true 実装 — best-effort）
- **scan/descriptor**: `countClasses` + `getClassInfo2`/`getClassInfoUnicode` を列挙し `category == "Audio Module Class"` をプラグインとして採用（`Component Controller Class` は pair 対象・独立プラグインではない）。id = CID の 16byte hex（実装確定 — `tuid_hex` が raw `TUID` byte を lowercase hex 化。`vst3::uid()` の byte layout は **platform 依存**: Windows は先頭 GUID field が反転するため Windows/非 Windows で id hex が異なる・`TUID` 要素は `c_char`（本 platform では `i8`）のため符号拡張なし reinterpret（`cast_unsigned`）が必須 — E2E 側は prod とは独立に `uid(...)` bytes から hex を derive し `#[cfg]`-conditional literal で両形を regression pin）・name/vendor/version/`subCategories` → `PluginDescriptor`（`'static` 実体化は §`'static` メタデータ実体化）
- **instantiate / minimal-host call 列**: `createInstance(component CID)` → QI `IComponent`/`IAudioProcessor`・`getControllerClassId` → `createInstance(controller CID)` → `IEditController` → component・controller 双方の `IPluginBase::initialize(context)` に最小 `IHostApplication` を `ComWrapper` で供給 → `IEditController::setComponentHandler(handler)` で最小 `IComponentHandler` を controller に注入。handler 注入経路は `initialize(context)` ではない — context が運ぶのは `IHostApplication` で、`IComponentHandler` は `setComponentHandler` 経由が spec の定める面（fixture: controller.rs — handler 非保持の `kResultOk` acknowledge）。bus 情報問合せ（`getBusCount`/`getBusInfo`/`getBusArrangement`）は fixture では optional（stereo 既定受理）。`IPluginFactory3::setHostContext` は fixture が trivial acknowledge（`kResultOk`）のため呼んでも省略してもよい。実装面は「fixture 形状 + spec の必須面」に scope する（component.rs test の `TestQueue`/`TestChanges` が同型先例）
- **lifecycle 写像**: `prepare`（`Activate` 内）→ **`IComponent::setActive(true)` → `IAudioProcessor::setupProcessing(ProcessSetup{kRealtime, kSample32, maxSamplesPerBlock, sampleRate})` の順** — fixture は `setupProcessing` に Activated 済みを要求し、未活性なら `kResultFalse`（component.rs — fixture 自身の `activate()` test helper も `setActive(1)` → `setupProcessing` の順）。`setBusArrangements` は fixture では不要（stereo 既定受理・呼出 gate なし）。`start_processing` → `setProcessing(true)`（negotiated config を要求）・`process_host` → `IAudioProcessor::process(ProcessData)`（`HostInput`/`HostOutput` → `AudioBusBuffers` channel pointer array・`EventKind::ParamValue` → bridge 所有の `IParameterChanges`/`IParamValueQueue` COM object — 子内 host 実装として `Class`/`ComWrapper` で提供）・`stop_processing` → `setProcessing(false)`・`Deactivate` → `setActive(false)`（`Processing` 中は `kResultFalse` 拒否のため stop 先行の対称形）。再 Activate では `setActive(true)` → `setupProcessing` の再交渉を行う（activation 毎の config — fixture の re-activation test と同型）
- **params**: `IEditController::getParameterCount`/`getParameterInfo` → 中立 `ParamInfo` 表（`'static` 実体化は §`'static` メタデータ実体化）。実装確定 — 列挙は **`Vst3Library::load` 時点に hoist**（`factories()` は複数回呼出可のため per-call 列挙は呼出し毎に throwaway pair を instantiate し intern 済み表を再 leak する — load 時 1 回 + `factories()` は cache 読取りのみで「once-per-load・no per-call growth」の intern-once 不変条件を構造的に保持。CLAP bridge と同構造 — IR `issue-114-vst3-host-bridge-task-2-implementation.md`）
- **teardown**: `setProcessing(false)` → `setActive(false)` → `terminate`（両 object）→ release → `ExitDll`/`ModuleExit`/`bundleExit`（export 時のみ）→ `Library` drop — fixture の drop 順序は instance → library（factory/instance が `Arc<Vst3Library>` 参照を保持して unload 順を保証）。実装確定 — `Vst3Plugin`/`LibraryInner` の `Drop` と `LoadGuard` で同順を RAII 駆動し、`init` 途中失敗は `Initialized` 済み object のみ逆順 `terminate` で rollback（`ComPtr` drop が release を担い leak なし）
- **v1 の defined 境界**: 単一 audio in/out bus・`kSample32`・`kRealtime` のみ受理（fixture の形）。event bus・note expression・`IConnectionPoint`・`IBStream` state・`IProgramListData` 等は未対応（defined rejection or 未実装 — fixture が要求しない面）。**state 動詞は呼ばない** — `IComponent::getState`/`setState` は `kNotImplemented` stub、`IEditController::setState`/`getState` は `kResultOk` no-op stub（component.rs / controller.rs）。controller 経由では `Ok(State(empty))` が返り defined rejection にならず、component 経由は `IBStream` object の供給が要る — よって bridge は規格 state 動詞を一切呼ばず、`save_state`/`restore_state` は `Plugin` 既定の `Err(HostError::Unsupported)` を継承する。E2E はその defined rejection を観測する
- **unsafe**: `libloading` + 全 vtable 呼出しは `host.rs`/`host/` モジュール内の scoped `#[expect(unsafe_code)]` + SAFETY — OFX `ffi/loader.rs` 先例

### `'static` メタデータ実体化（descriptor/params の lifetime 決定）

`PluginDescriptor`・`ParamInfo` の文字列 field は `&'static str`（`features` は `&'static [&'static str]` — plugin-core descriptor.rs / param.rs）。`PluginFactory::descriptor`・`Plugin::descriptor`/`params`・`OutOfProcessSpec.descriptor` の公開契約がすべて `'static` を要求する一方、dlopen した規格バイナリの metadata は runtime の `String`/`Vec` でしか得られない — 変換機構を決めなければ上記 sketch（`descriptors() -> Vec<PluginDescriptor>`・`params() -> &[ParamInfo]`）は実装不能である。

**決定: bounded intern-once** — `Library::load` の scan 時（descriptor 合成: per descriptor で id/name/vendor/version + features 各要素 + features slice 自身 ≈ 5+N 本）と factory 構築時（param 表: `name`/`unit` 2 本/param + table slice — factory 共有で 1 回）に限り、`String::leak`/`Vec::leak` で `'static` 化する。以後 `descriptor()`/`params()` は intern 済みキャッシュ値を返すだけで、leak は load・構築時の 1 回限り。

**「no `Box::leak` / forged statics」方針との整合** — plugin-core は descriptor.rs（`PluginDescriptorView` 節）と wire.rs（`Descriptor` コメント）で forged statics を明示禁止している。この方針が狙うのは hot/per-instance 経路と wire 越境（child は `'static` を受け取れないため wire は owned `Descriptor` に変換 — 逆変換は存在しない）。test-lane bridge の intern-once は以下の性質により本方針の sanctioned exception とする: (i) **bounded** — library 単位で固定小数の小文字列（descriptor あたり ~5 + features・param あたり 2 本）、per-block/per-instance で増殖しない、(ii) **once-per-load** — steady state での増加なし、(iii) **process-scoped** — runner は per-`Instantiate` spawn の短命子プロセスで exit 時に全回収される、(iv) **feature 封じ** — intern は `host` module 内に閉じ、default 構成の plugin-side cdylib 経路に影響しない。

**rejected alternatives**: (a) owned/view 型への API 変更（`PluginDescriptor` の owned 化・`PluginFactory::descriptor`/`OutOfProcessSpec` の view 型化）— plugin-core/host 無変更の上流確定事項に矛盾（#76 確定面を実プラグイン lane のために破壊する）。(b) 子側 descriptor の const-pinning — 実バイナリ metadata 合成の目的そのものを否定（spec の「adapter が実バイナリ metadata から合成する経路」要件に反する）。(c) `PluginDescriptorView<'a>`（OFX が使う owner-borrowed 形）— `PluginFactory::descriptor`/`OutOfProcessSpec` boundary が要求するのは `PluginDescriptor`（`'static`）そのものであり、view 型は boundary に適合しない。

**親側 `spec.descriptor` の materialization**: `OutOfProcessSpec::new` も `PluginDescriptor`（`'static`）を要求する。**決定: parent-side test は fixture が宣言する既知 metadata の `const`/`static` `PluginDescriptor` literal を供給する**（`Descriptor::matches` の doctest が `const GAIN: PluginDescriptor` で同型先例 — wire.rs）。子側は実 metadata から合成するため、handshake の field-equality が合成写像そのものの実検証となる — 同一 scan API を両側で呼ぶ形は同一関数の同値で field-equality を構造的に自明にし、写像の検証力を持たないため棄却する。test binary は fixture を dlopen せず（`spec.descriptor` は const）、`descriptors()` の in-process 単体検証（実値 == 期待値 const）は adapter crate 側の `--features host` test が担う（§検証戦略）。

### runner バイナリ（compile-time 固定 — a の確定）

各 adapter crate に規格毎の runner bin を置く（`required-features = ["host"]` — default build では生成されず、lane task が `--features host` で生成する）。

```rust
// src/bin/ruviform-plugin-clap-l1-runner.rs — VST3 側も同形
fn main() -> std::process::ExitCode {
    // fixture 解決: env RUVIFORM_L1_REAL_PLUGIN_DIR（任意 override — §CI lane の escape hatch）
    //   → unset 時は current_exe() sibling
    //     （target/<profile>/{DLL_PREFIX}ruviform_plugin_clap{DLL_SUFFIX}）
    let dir = std::env::var_os("RUVIFORM_L1_REAL_PLUGIN_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| exe_dir());
    // Unix cdylib は lib 接頭辞付き: DLL_PREFIX+name+DLL_SUFFIX で組立
    // （fixture_lib_name() 級の小 helper — test 側も同規約で解決）
    let path = dir.join(format!(
        "{}ruviform_plugin_clap{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    ));
    let lib = match ruviform_plugin_clap::host::ClapLibrary::load(&path) {
        Ok(lib) => lib,
        Err(e) => { eprintln!("ruviform-plugin-clap-l1-runner: {e}"); return 1.into(); }
    };
    ruviform_plugin_export::run(
        lib.factories().map(|f| Box::new(f) as _).collect(),
    )
}
```

- **compile-time 固定の意味**: runner が serve する規格・バイナリ・登録形は compile/link 時に確定 — `--plugin-path` 等の実行時指定は argv 規約に存在しない（`run` の argv 規約・未知引数 reject は不変。`RUVIFORM_L1_REAL_PLUGIN_DIR` は子プロセスへの env 注入であり argv 規約を拡張しない — lane 運用のための fixture dir override で、プラグインの実行時スキャン機構ではない。exercise は runner 直接起動のみ — §CI lane）
- **fixture 解決**: runner は自身の exe sibling（`target/<profile>/`）を見る — `runner_program()`（`l1_proxy_e2e.rs:1591`）と同一の `current_exe` 相対解決規約（#123 が解決形を強化すればそれに追従 — 本 Issue は現行規約を踏襲）
- **失敗意味**: binary 不在/dlopen 失敗/entry symbol 欠落/scan 失敗 → stderr diagnostic + `exit(1)`（`run` 到達前 = Hello 送出前）→ 親の handshake `Hello` 待ちが EOF で失敗 → spawn `Err(HostError::Ipc)`。instantiate 失敗（`create()` 内）→ panic → `run` が catch → `exit(1)` → handshake/Spawn 系 `Err(Ipc)`。いずれも defined rejection でハングしない
- **`Scan` 応答**: bridge factory の `descriptor()` は load 時 scan のキャッシュ値を返す（キャッシュ値 = §`'static` メタデータ実体化の intern-once 済み値。`Scan`/`argv` 解決で dlopen を再実行しない — `descriptor()` が plugin バイナリ依存の失敗を持たない構造）

### fixture 供給（f の確定）

- **fixture = 既存 workspace cdylib** — `cargo build`/`cargo test` が `target/<profile>/` へ規格 cdylib を生成済み（実測確認は Windows で実施: `ruviform_plugin_clap.dll`/`ruviform_plugin_vst3.dll` が `clap_entry`/`GetPluginFactory` を export。Unix 系の cdylib 名は `lib{name}.so`/`lib{name}.dylib` と `lib` 接頭辞付きのため、runner・test 双方とも `{DLL_PREFIX}{name}{DLL_SUFFIX}` — `std::env::consts` 経由で解決する）。新規 fixture crate・外部取得・bundle 化は不要
- CLAP fixture は `org.ruviform.clap-gain-poc`（`SinglePluginEntry<GainClapPlugin>` — `audio-ports`/`params`/`thread-check` extension）。VST3 fixture は 2-class module（`Audio Module Class` = GainComponent `IComponent`+`IAudioProcessor`、`Component Controller Class` = GainController `IEditController` — component+controller pair の実 COM 接線を E2E で通る）
- **`--features host` ビルドの注意**: feature 統一上、fixture cdylib 自体も `host` feature 付きでビルドされる（bridge コードを内包するが規格 entry の契約は不変 — dlopen から見れば依然として真の規格バイナリ）。配布形（`.clap` bundle/`.vst3` 構造）は本 Issue 範囲外 — raw cdylib の dlopen で E2E は成立する

### descriptor 合成と handshake 整合（i の確定）

`OutOfProcessSpec.descriptor`（host 側の registration 値）と子の `Scanned` 報告は field-equality が必須（不一致は spawn `Err(Ipc)`）。**子は実 metadata から合成し、親は既知値を pin する**ことで、field-equality が合成写像の検証を兼ねる:

- 子側: runner が `Library::load` → `descriptors()` で実バイナリ metadata から合成（`'static` 実体化は §`'static` メタデータ実体化の intern-once）→ `Scan`/`Scanned` 応答
- 親側（テスト）: fixture が宣言する既知 metadata の `const`/`static` `PluginDescriptor` literal を `OutOfProcessSpec::new` に渡す — test binary は fixture を dlopen しない。同一 scan API を両側で呼ぶ形は同一関数の同値で field-equality を構造的に自明にし、写像の検証力を持たないため棄却（§`'static` メタデータ実体化）
- 合成写像: CLAP `clap_plugin_descriptor`（id/name/vendor/version/features 配列・順序保持）→ `PluginDescriptor`。VST3 `PClassInfo2`/`PClassInfoW`（cid → hex id、name、vendor、version、subCategories → features）

### E2E トポロジー（g の確定）

`ruviform-plugin-clap/tests/l1_real_adapter_e2e.rs`・`ruviform-plugin-vst3/tests/l1_real_adapter_e2e.rs` — 先頭 `#![cfg(feature = "host")]`（default build では空 binary・0 test — lane が唯一の実行者）・各テスト関数に `#[ignore = "…"]`（理由付き形 — §CI lane）。

```rust
// 概念形
let runner = env!("CARGO_BIN_EXE_ruviform-plugin-clap-l1-runner");  // same-crate
// fixture_lib_name: format!("{}{}{}", DLL_PREFIX, name, DLL_SUFFIX)
//   — Unix cdylib は lib 接頭辞付き
let fixture = target_profile_dir().join(fixture_lib_name("ruviform_plugin_clap"));
assert!(fixture.is_file());                                     // 欠落は fail（skip しない）
// spec.descriptor = fixture 既知 metadata の const literal（'static）。
// 子の合成値との handshake field-equality が写像検証を兼ねる — test binary は dlopen しない
const EXPECTED: PluginDescriptor = PluginDescriptor {
    id: "org.ruviform.clap-gain-poc",
    name: "Ruviform Gain PoC",
    vendor: "Ruviform",
    version: "0.1.0",  // fixture の env!("CARGO_PKG_VERSION") 宣言値
    features: &["audio-effect", "stereo"],  // features::AUDIO_EFFECT/STEREO・順序保持
};
let mut spec = OutOfProcessSpec::new(EXPECTED, OsString::from(runner));
// PluginHost: Scan → Instantiate → Initialize → Activate → StartProcessing
//   → N ブロック（ParamValue event 込み）→ delivered block == gain × earlier input
//   （lookahead 既定 2・underrun/pass-through 意味不変）
//   → SaveState（bridge は規格 state 動詞を呼ばず `Plugin` 既定の defined
//     `Unsupported`）→ RestoreState 同
//   → StopProcessing → Deactivate → Destroy → 子 exit(0)
// + 1 本: kill_child → Failed + InstanceFailed once + 以後 defined 拒否
```

- `env!("CARGO_BIN_EXE_<runner>")` は `required-features = ["host"]` を満たす `--features host` ビルドでのみ設定される — テストファイルの `#![cfg(feature = "host")]` と組で compile 時整合
- fixture path はテスト側も `current_exe` → `deps/` → `target/<profile>/` 解決（`runner_program()` 同型）。欠落時は `assert!(path.is_file())` で **fail**（skip しない — `runner_program()` 先例）
- `spec.descriptor` は fixture 既知 metadata の const literal — 期待値 pin（CLAP `org.ruviform.clap-gain-poc`・VST3 GainComponent CID hex 等）自体が `OutOfProcessSpec` に乗り、handshake field-equality が子側合成写像の検証を兼ねる（§`'static` メタデータ実体化）

### 失敗・timeout 意味（L1Instance 既存契約の継承）

#76 確定面をそのまま継承する（本 Issue の新規面は子内の規格 load に限定）:

- per-`Instantiate` spawn・`Hello`/`Scan` handshake・`command_timeout`（既定 10s）で bounded・`Failed`+`InstanceFailed` once+pass-through・自動再起動なし・`Shutdown`/`Destroy` → exit(0)・stdin EOF normalization
- 新規 defined rejection（子内）: 規格バイナリ不在・dlopen/entry-symbol/scan 失敗 → `exit(1)` pre-Hello → spawn `Err(Ipc)`・`create()` 内失敗 → panic catch → `exit(1)` post-Hello → `Err(Ipc)`。bridge の foreign panic（規格 plugin コードの crash）は子プロセス crash として既存 `Failed` 契約に吸収（bridge 自体は Rust panic-safe — `catch_unwind` は `run` 既存機構が plugin 境界で適用）

### CI lane（h の確定 — ユーザー決定のメカニズム）

```toml
# mise.toml（既定タスク群と並置 — icc/ofx lane と同型）
[tasks.test-l1-real-plugin]
description = "Build real CLAP/VST3 plugin fixtures + L1 runner binaries (--features host), then run the #[ignore]-gated real-binary L1 E2E tests (opt-in lane — Issue #114)"
run = [
  "cargo build -p ruviform-plugin-clap -p ruviform-plugin-vst3 --features host",
  # 実装確定（Task 4 review 修正）— `--lib --bins` 段を挟み、in-process
  # bridge 契約層 + runner bin の `#[cfg(test)]` fixture 解決 unit test も
  # lane/CI に載せる（`--test l1_real_adapter_e2e -- --ignored` 単独では
  # 両者は実行されない）
  "cargo test -p ruviform-plugin-clap -p ruviform-plugin-vst3 --features host --lib --bins",
  "cargo test -p ruviform-plugin-clap -p ruviform-plugin-vst3 --features host --test l1_real_adapter_e2e -- --ignored",
]

[tasks.lint-l1-real-plugin]
description = "Lint the CLAP/VST3 host-bridge code and test targets (--features host)"
run = "cargo clippy -p ruviform-plugin-clap -p ruviform-plugin-vst3 --all-targets --features host -- -D warnings"
```

- **ゲート本体は `#[ignore = "…"]`** — default の `cargo test`/`test-ci` では決して実行されず、opt-in は `-- --ignored` で明示的に引く。`#![cfg(feature = "host")]` と `required-features` の組で default build にコード・成果物が出ない。理由付きの `#[ignore = "…"]` 形を使う（video-proc icc.rs `generate_fixtures` の先例 — 手動実行条件を attribute に明示）
- **silent skip しない設計**: テスト・runner・fixture の欠落は `assert!`/spawn `Err` で **fail**（env-gated skip 慣行は repo に存在せず導入しない）。`#[ignore]` は gate であって skip 機構ではない
- **手動実行（runbook）**: ローカルは `mise run test-l1-real-plugin`（`--features host` build → `--ignored` test。lint は `mise run lint-l1-real-plugin`）。CI 側は `ci.yml` の `workflow_dispatch` で `l1-real-verify` job を含む dispatch を手動発火する
- **`RUVIFORM_L1_REAL_PLUGIN_DIR`** — runner 側の任意 env: unset → sibling-of-exe の workspace fixture（既定）。out-of-tree fixture 向けの override knob であり、ゲートとしては使わない（ユーザー決定の「env/`#[ignore]` ゲート」の env 側面を override knob として具体化）。なお `OutOfProcessSpec` に env field はなく子 env は親プロセスの純継承のため、本 override は **runner 直接起動（shell env 注入）でのみ作用する developer escape hatch** であり `PluginHost` 経路の検証対象外とする — lane test は既定の sibling 解決のみを通す（test からの `std::env::set_var` は edition 2024 で `unsafe` + process-global mutation のため持ち込まない）
- **CI 反映**: 既存 `.github/workflows/ci.yml`（`workflow_dispatch` 専用）に `l1-real-verify` job を追加（`rt-verify`/`icc-verify` と同型 — `mise run lint-l1-real-plugin` + `mise run test-l1-real-plugin`）。**別 workflow ファイルは作らない**。OS coverage は **3-OS matrix**（fail-fast: false — dlopen/spawn/COM ABI の実経路は OS 毎に異なり、実バイナリ lane の価値は platform 差の検出にある。dispatch-only のためコストは手動実行時のみ）

## 依存追加の判断（暫定規約の理由記録）

- **`clack-host = "=0.2.0"`（`ruviform-plugin-clap`・optional・`dep:` under `host`）** — dev-dep → optional prod dep 昇格。MIT OR Apache-2.0（prokopyl/clack）。採用済み clack 族（`clack-plugin`/`clack-common`/`clack-extensions` — binding-method ADR 系）と同一 family・同一 exact-pin 戦略。default features（`libloading`）で `PluginEntry::load(path)` の file-load 経路が得られる。dep tree 追加: `libloading` + `objc2-foundation`（macOS のみ）。dev-dep entry（`clack-plugin` feature）は `load_from_clack` typed-entry test のため現行のまま残す（両 entry で feature union）
- **`libloading = "0.8.9"`（`ruviform-plugin-vst3`・optional・`dep:` under `host`）** — ISC（nagisa/rust_libloading）。`ruviform-plugin-ofx` が既に同一バージョンを採用 — 新規バージョン導入なし・dlopen の実績ある実装
- **`vst3` 0.3.0** — 既存 prod dep のまま（optional 化しない — plugin-side が既に必須とする）。MIT OR Apache-2.0（coupler-rs）。host-side COM object 実装は `Class`/`ComWrapper`/`Interface` で可能（component.rs test の `TestQueue`/`TestChanges` が同型）
- **`ruviform-plugin-export`（両 adapter crate・optional・`dep:` under `host`）** — runner bin のみが使う内部 dep。adapter → export は baseline 許可方向
- **`ruviform-plugin-host`（両 adapter crate・dev-dep）** — E2E test の `PluginHost`/`OutOfProcessSpec` のため。adapter → host は baseline 許可方向・test 対象のみ
- **外部 crate 評価 — `vst3-host` 0.10.0（不採用）**: crates.io に実在する host-side VST3 crate（MIT・2026-06 公開・0.10.0 は 2026-10-09 — ≥7d 経過・同一 `vst3 ^0.3.0` bindings 使用）。不採用理由: (i) dep tree が重い（serde/serde_json/thiserror/rtrb/crossbeam-queue/log/winapi/**xcb**(Linux normal — CI の ubuntu に libxcb dev header 追加を要)・macOS objc2 系、optional cpal/egui/midir）。(ii) 抽象レベルが不一致 — app 向け API（`load_plugin`/`play`/`process_audio`/MIDI/GUI editor/独自 process-isolation）であり、`PluginFactory`/`ErasedPlugin` の中立契約写像には二重の適応層が要る。(iii) 成熟度 — 公開 ~4 ヶ月・単一 owner・dependents 0 で foundation 層 dep の採用基準に届かない。(iv) 本 Issue 範囲の最小 host（fixture が要求する面のみ）が既存 vendored bindings + libloading で完結する。**再評価 trigger**: vst3-host が成熟・採用実績を積んだ場合、bridge 内部実装の置換は `PluginFactory` 面を変えずに可能（後続 Issue 級）
- **`vst3-sys`/`vst3_com`** — 旧世代の bindings（host abstraction なし）。`vst3` 0.3.0 で既に採用済みの bindings と重複 — 不採用

## 検証戦略（mock/fixture — device-independent）

- **単体（adapter crate・`--features host`・通常 lane で実行可能な層）**: descriptor 合成写像（CLAP/VST3 metadata → `PluginDescriptor` 全 field — in-process `descriptors()` 実値 vs fixture 既知値 const literal）・param 表写像・error 写像・dlopen/entry 解決の defined rejection（不在 path・非規格バイナリ・entry 欠落 fixture あれば）
- **子側**: `l1_runner_e2e` の `MiniHost` 形で adapter runner を直接駆動する補助検証（必要に応じて — 主証拠は host 側 E2E）
- **host 側 E2E（主証拠・`#[ignore]` lane）**: `l1_proxy_e2e.rs` `runner_bin_serves_real_gain_plugin` 同型を規格毎 — spawn→`Scan`（実 descriptor）→instantiate→lifecycle→shm block（gain データ正当性・lookahead/pass-through 意味不変）→state 動詞 defined 応答→destroy/exit(0)。異常系 1 本/規格（`kill_child` → `Failed`+once 通知+以後拒否）
- **異常系**: 既存 `l1-test-worker`/`l1_runner_e2e` が wire 契約の大部分を担う（不変）— adapter runner では kill 系を代表観測し、binary 不在は spawn `Err(Ipc)` を別途 pin
- **RT レーン**: 実装確定 — 子内 bridge は runner が非 RT 側だが、bridge の `process_host` 経路の alloc-free を `assert_no_alloc` で pin する `rt_process_drives_fixture_block_allocation_free` を feature `host` 内に追加（Task 1 review 修正 — opt-in lane `--lib` 段でのみ実行）。default `cargo test --workspace rt_`（`mise run test-rt`）の一致件数は feature-gate により不変・不変グリーン
- **lane 運用**: `mise run test-l1-real-plugin`（build → `--lib --bins` unit test → `--ignored` E2E の実装確定 3 段）・`lint-l1-real-plugin`・ci.yml `l1-real-verify` job

## Risks and Trade-offs

- **VST3 COM host 手実装が本 Issue の最大新規コード**（component+controller pair・`IParameterChanges`/`IParamValueQueue`/`IHostApplication`/`IComponentHandler` 実装・`ProcessData` 組立・teardown 順）— 軽減: PoC component.rs test の `TestQueue`/`TestChanges` が host-object 実装の同型先例・fixture が要求する面への限定（single bus・kSample32・kRealtime で defined 境界化）・`vst3` bindings の `ComPtr`/`ComRef` が refcount 管理を担う
- **clack-host `PluginInstance` は `!Send`** — 子は single-thread で非問題（discovery confirmed）。bridge が `Send` 前提を誤ると `create_sendable` 誤実装になる → default `None` を維持
- **`#[ignore]` lane は実行されないと腐る** — `l1-real-verify` job（workflow_dispatch 専用）での手動 dispatch 時検出に限られる。dispatch 以外では走らないため手動実行の抜けは運用リスクとして残る（opt-in の性質上不可避 — spec AC6 の lane 形と整合）
- **drop 順序（instance → library unload）** — foreign object が unload 済み lib の vtable を触ると crash。`Arc<Library>` で instance → library の生存順を保証（OFX `OfxLibrary::share` 先例）
- **feature union の複雑さ** — `clack-extensions/clack-host`（非 optional dep への plain feature 有効化）・dev/prod 両 entry の union で feature 面が絡む → plan フェーズで feature 集合を最終確定・`cargo tree` で依存木確認
- **macOS bundle 注意** — raw cdylib dlopen は動作するが規格準拠の発見経路（`.clap` bundle/`.vst3` bundle）ではない — deferred 行に記録済み
- **foreign crash は bridge で catch できない** — 規格 plugin 自体の SEGV/abort は子プロセス crash → `Failed` 契約（実はこれが L1 分離の検証対象そのもの）

## Deferred Design Concerns

- `.clap`/`.vst3` 規格準拠配布形（bundle・拡張子改名・ディレクトリ構造）での load・発見経路 | raw cdylib dlopen で E2E 成立 | 規格準拠ホストとしての発見経路検証要求 | bundle 経路は未検証のまま
- video/NONRT プラグイン（OFX `VideoPluginInstance`）の L1 経路 | video port は全操作 NONRT・wire/shm は audio 専用（#113 design 決定 `k`） — **Issue [#118](https://github.com/scarecrow173/ruviform/issues/118) で追跡（ユーザー決定）** | NONRT video port の分離要求（L1 design deferred と整合） | video plugin は L1 不可
- 実行時プラグインバイナリ指定（loader 型 runner・argv/`spec.args` 経由 path 受け渡し） | **rejected alternative（ユーザー決定）** — compile-time 固定 registration 採用・`run` argv 規約不変 | 実行時にプラグインバイナリを差し替える要求（third-party スキャン等） | runner は組込み fixture のみ serve
- third-party/外部取得プラグインバイナリ E2E | 本 Issue は workspace 生成 fixture | 外部バイナリ取得・固定の要求発生 | fixture カバレッジは自前 plugin に留まる
- VST3 拡張面（`IConnectionPoint`・note/event bus・`IProgramListData`・`IBStream` state） | fixture が要求しない面 | 実 VST3 plugin で同面が要求された時 | bridge は fixture 形状に限定される
- stateful plugin の実 state round-trip（CLAP `state` ext・VST3 `IComponent::getState`/`setState` + `IBStream`） | 現 fixture が state 面を持たない — bridge は規格 state 動詞を呼ばず `Plugin` 既定の defined `Unsupported` 往復で wire 経路は実証済み | stateful fixture/実プラグインの導入 | state 写像の実経路は未検証
- `vst3-host` 等外部 host crate への bridge 内部置換 | 現状は成熟度・dep tree・抽象不一致で不採用（依存追加の判断） | 成熟・採用実績の蓄積 | 手実装 host の保守コストが残る
- runner_program/fixture path 等 `target/<profile>/` 解決の強化（Issue #123） | 本 E2E ハーネスも同一解決規約に依存 | #123 が解決形を決定 | 現行の `current_exe` 相対解決に依存し続ける

## Verification Notes

- [ ] spec AC1: CLAP・VST3 各 adapter crate に `required-features = ["host"]` の compile-time 固定 runner bin が存在し、`run` argv 規約不変で `--ruviform-plugin-id` 選択が機能する
- [ ] spec AC2/AC3: `PluginHost`/`OutOfProcessSpec` で実 `.clap`/`.vst3` 形バイナリを載せた子が spawn→handshake→lifecycle→shm block→state 動詞→destroy を完走し、規格 ABI 経由の出力がデータ正当性を持つ（`l1_real_adapter_e2e` 両ファイル）
- [ ] spec AC4: 子 kill → `Failed`+once 通知+pass-through・binary 不在 → spawn `Err(Ipc)`・ハングなし
- [ ] spec AC5: OFX 境界が本書・spec に明文化され #118 参照・trigger が記録される
- [ ] spec AC6: `test-l1-real-plugin`/`lint-l1-real-plugin` task が存在し `--ignored` で opt-in 実行・欠落時 fail・ci.yml に `l1-real-verify` job（既存ファイル内・workflow_dispatch・3-OS）
- [ ] spec AC7: `run`/`OutOfProcessSpec`/wire/`RtLeg` の公開契約に互換破壊なし・新規公開 enum `#[non_exhaustive]`・`unsafe` scoped 許可内
- [ ] spec AC8: `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` + `test-l1-real-plugin`/`lint-l1-real-plugin` グリーン・doc 監査 findings 0
- [ ] bridge の `descriptor()` が dlopen/scan を遅延せず load 時キャッシュ値を返す（`Scan` 応答が plugin バイナリ依存の失敗を持たない）
- [ ] `descriptors()`/`params()` の `'static` 実体化が §`'static` メタデータ実体化の intern-once（load/factory 構築時の bounded `String::leak`/`Vec::leak`）で実装され、per-block/per-instance で増殖しない
- [ ] 親側 `spec.descriptor` が fixture 既知 metadata の const `PluginDescriptor` literal で、handshake field-equality が子側合成写像を実検証する（test binary は fixture を dlopen しない）
- [ ] VST3 lifecycle が `setActive(true)` → `setupProcessing` → `setProcessing(true)` → `process` → `setProcessing(false)` → `setActive(false)` → `terminate`（両 object）の順で駆動される（fixture が要求する Activated-before-setupProcessing 順序）
- [ ] CLAP bridge の `HostHandlers::declare_extensions` が `HostThreadCheck` を登録しない（fixture の `None` fallback 経路 — 不実の `is_audio_thread` 応答をしない）
- [ ] bridge は VST3 state 動詞（`IComponent`/`IEditController` の `setState`/`getState`/`setComponentState`）を呼ばず、`save_state`/`restore_state` が `Plugin` 既定の `Err(HostError::Unsupported)` を継承する
- [ ] fixture 解決が `{DLL_PREFIX}{name}{DLL_SUFFIX}` で組立てられ Unix の `lib` 接頭辞 cdylib も解決する
- [ ] `Arc<Library>` で instance → library の drop 順序が保証される（unload 後 vtable アクセスなし）
