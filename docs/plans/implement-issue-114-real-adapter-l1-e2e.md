---
id: "PLAN-034MLRinXTGEtewrMHtDSh"
type: "plan"
status: "approved"
title: "Issue #114 実施計画 — 規格アダプタ host-side bridge（CLAP/VST3）+ 実プラグイン L1 E2E（compile-time runner + opt-in lane）"
created: "2026-10-26"
updated: "2026-10-27"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/114"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-114-real-adapter-l1-e2e.md"
  implemented-by:
    - "docs/tasks/clap-host-bridge.md"
    - "docs/tasks/vst3-host-bridge.md"
    - "docs/tasks/real-adapter-runner-bins.md"
    - "docs/tasks/real-adapter-e2e-and-lane.md"
    - "docs/tasks/real-adapter-final-verification.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/implement-issue-76-plugin-l1-out-of-process.md"
    - "docs/plans/implement-issue-113-engine-plugin-integration.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-114-real-adapter-l1-e2e.md"
    - "docs/designs/issue-114-real-adapter-l1-e2e.md"
    - "docs/designs/plugin-l1-out-of-process.md"
    - "docs/designs/engine-plugin-host-integration.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/adr/plugin-architecture-clap-vst3.md"
    - "docs/adr/plugin-binding-method-clack-for-clap-coupler-rs-vst3-for-vst3.md"
    - "docs/adr/ofx-host-video-contract-and-c-abi.md"
  derived-by:
    - "docs/tasks/clap-host-bridge.md"
    - "docs/tasks/vst3-host-bridge.md"
    - "docs/tasks/real-adapter-runner-bins.md"
    - "docs/tasks/real-adapter-e2e-and-lane.md"
    - "docs/tasks/real-adapter-final-verification.md"
  verifies: []
  verified-by:
    - "docs/test-specs/adapter-host-bridge-contract-tests.md"
    - "docs/test-specs/real-adapter-l1-e2e-tests.md"
    - "docs/test-specs/l1-real-plugin-lane-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #114 実施計画 — 規格アダプタ host-side bridge（CLAP/VST3）+ 実プラグイン L1 E2E（compile-time runner + opt-in lane） Implementation Plan

> status: approved — Refs #114。approved design `docs/designs/issue-114-real-adapter-l1-e2e.md`（DESIGN-034MIeiBWXpLjwgA1yolIT・決定サマリ a–j）を正本とし、spec `docs/specs/issue-114-real-adapter-l1-e2e.md`（proposed・AC1–AC8）と 2026-10-26 のユーザー決定（規格スコープ CLAP+VST3 両方・runner オーサリング compile-time 固定・CI lane = 専用 mise task + `#[ignore]` ゲート）を実装する。**5 buildable task**（task 文書は起票済み）。全 API 名・dep version・fixture metadata は実コード照合済み（vendored `clack-host 0.2.0`/`clack-common 0.2.0`/`clack-extensions 0.2.0`/`vst3 0.3.0`・`Cargo.lock`・両 adapter crate・`ruviform-plugin-export`/`plugin-host`/`plugin-core`）。

## Goal

`ruviform-plugin-clap`/`ruviform-plugin-vst3` に feature `host` で feature-sealed な **host-side bridge**（実規格バイナリの dlopen → `PluginFactory`/`ErasedPlugin` 写像）を実装し、規格毎の **compile-time 固定 runner bin**（`required-features = ["host"]`）が `run(factories)` を駆動する形で L1 子プロセスへ組込む。`PluginHost`/`OutOfProcessSpec` 経由の実 `.clap`/`.vst3` cdylib E2E（spawn→handshake→lifecycle→shm block→state 動詞 defined 応答→destroy + kill 異常系）を `#[ignore]` ゲートの opt-in lane で実証する。bridge は**子プロセス内**で動作し、host 側（親）は L1 wire のみを見る — wire/export/host/core の公開契約は不変。

中核機構（design 決定サマリの pinned 形、全て後続 task が引き受ける）:

- **crate 配置（決定 c）**: `#[cfg(feature = "host")] pub mod host` を各 adapter crate に新設（`src/host.rs`）+ `[[bin]] required-features = ["host"]` の規格別 runner bin。新規 `*-host` crate なし・default feature 集合・plugin-side cdylib の依存木は不変
- **CLAP bridge（決定 d）**: `clack-host =0.2.0`（optional・default features = `libloading` で `PluginEntry::load` dlopen）— `PluginEntry::load(path)` → `get_plugin_factory()` → `plugin_descriptors()` → `PluginInstance::new`/`activate(PluginAudioConfiguration)`/`StoppedPluginAudioProcessor::start_processing`/`StartedPluginAudioProcessor::process`/`stop_processing`/`deactivate` を中立 lifecycle に写像。`HostHandlers::declare_extensions` は **`HostThreadCheck` を登録しない**（fixture の `get_extension` → `None` graceful fallback — plugin.rs:130・不実の `is_audio_thread` 応答をしない）
- **VST3 bridge（決定 e）**: `vst3 0.3.0` raw COM bindings 直駆動 + `libloading 0.8.9` — `Library::new` → `GetPluginFactory` → `countClasses`/`getClassInfo2`/`getClassInfoUnicode`（`"Audio Module Class"` filter）→ `createInstance` → QI `IComponent`/`IAudioProcessor` → `getControllerClassId` → `createInstance` → `IEditController` → 両 object `initialize(context)`（最小 `IHostApplication` を `ComWrapper` で供給）→ `setComponentHandler`（最小 `IComponentHandler`）。lifecycle = **`setActive(true)` → `setupProcessing(ProcessSetup{kRealtime, kSample32, …})` の順**（fixture は未活性なら `kResultFalse` — component.rs:597）→ `setProcessing(true)` → `process(ProcessData)`（`inputParameterChanges` = bridge 所有 `IParameterChanges`/`IParamValueQueue`）→ `setProcessing(false)` → `setActive(false)` → `terminate` 両 object。**規格 state 動詞は一切呼ばず**、`save_state`/`restore_state` は `Plugin` 既定の `Err(HostError::Unsupported)`
- **`'static` intern-once（設計 §'static メタデータ実体化）**: `PluginDescriptor`/`ParamInfo` の `&'static str` 境界を `Library::load` scan 時 + factory 構築時に限り `String::leak`/`Vec::leak` で満たす — bounded（descriptor あたり ~5+features・param あたり name のみ + table slice）・once-per-load・process-scoped（短命子プロセス）・feature 封じ。親側 `spec.descriptor` は fixture 既知値の **const literal pin**（handshake field-equality が子側合成写像を実検証 — test binary は dlopen しない）
- **runner（決定 a/g/h）**: `main() -> ExitCode` が fixture を `RUVIFORM_L1_REAL_PLUGIN_DIR`（任意 env・shell-only override）→ `current_exe` sibling の `{DLL_PREFIX}{crate_snake}{DLL_SUFFIX}` で解決 → `Library::load` → 失敗は `eprintln!` + `exit(1)`（`run` 到達前・Hello 未送出 → 親 spawn `Err(Ipc)`）→ `run(factories)`。`run` argv 規約（`--ruviform-shm-key` 必須・`--ruviform-plugin-id`・未知引数 reject）不変・`spec.args` は空
- **E2E（決定 g）**: adapter crate 内 `tests/l1_real_adapter_e2e.rs` — `#![cfg(feature = "host")]` + 各テスト `#[ignore = "…"]`・`env!("CARGO_BIN_EXE_<runner>")`（same-crate 規約）+ const `PluginDescriptor` pin + `PluginHost` 経由の device-independent E2E
- **lane（決定 h）**: `test-l1-real-plugin`（`cargo build --features host` → `cargo test --features host --lib`（in-process bridge unit test 層 — `--test`/`--ignored` 指定では実行されないため明示 step）→ `cargo test --features host --test l1_real_adapter_e2e -- --ignored`）+ `lint-l1-real-plugin` mise task + `ci.yml` 内 `l1-real-verify` job（workflow_dispatch・3-OS matrix・別 workflow ファイルなし）。欠落時は `assert!`/spawn `Err` で **fail**（env-gated silent skip なし）

## Non-Goals

- L1 wire/host/runner 基盤・`ruviform-plugin-export`/`plugin-host`/`plugin-core` の公開契約変更（#76/#113 確定面のまま消費）
- 実行時プラグインバイナリ指定（argv `--plugin-path`・`spec.args` 経由 path 受け渡し・loader 型 runner — ユーザー決定で rejected）
- third-party/外部取得プラグインバイナリでの E2E（fixture = workspace 生成 cdylib のみ）
- `.clap`/`.vst3` 規格準拠配布形（bundle・拡張子改名・ディレクトリ構造）の再現（raw cdylib dlopen で E2E 成立 — design deferred）
- OFX の L1 経路・OFX host-side `ErasedPlugin` bridge（video・NONRT — Issue #118 で追跡・AC5 は境界明文化まで）
- video wire・`VideoPluginInstance` の L1 載せ・L1 GUI（`OpenGui`/`CloseGui` プロセス越え — L1 design 継続 deferred）
- 実規格 state 面（CLAP `state` ext・VST3 `IBStream`/`setComponentState` — 現 fixture が state 面を持たず bridge は規格 state 動詞を呼ばない — design deferred）
- VST3 拡張面（`IConnectionPoint`・note/event bus・`IProgramListData`・`IEditController2` 系 — fixture が要求しない面は defined 境界）
- 自動再起動・supervisor ポリシー（#31 確定）・`ErasedPlugin`/`Plugin` への全域 `Send` bound（`create_sendable` default `None` 維持 — ADR rejected 分岐）
- `unsafe` 原則禁止の緩和（scoped `#[expect(unsafe_code)]` + SAFETY で封じ込め — OFX `ffi/loader.rs` 先例型を超えない）
- `runner_program`/`current_exe` 相対 `target/<profile>/` 解決の強化（Issue #123 — 現行規約を踏襲）

## Scope and Split Boundary

単一リリース意図の 1 計画（CLAP bridge・VST3 bridge は別 crate で**論理上並行可能**だが、runner/E2E/lane が両者の完了を前提とする shared critical path — 独立出荷は主張しない）。**5 buildable task**（task 文書は起票済み・各 task が ordered actions を持つ）。commit は論理ビルド可能単位で lead が管理 — 1 action/1 commit 強制・red 専用 commit はしない。

- Task 1（CLAP host bridge: feature/dep 配線 + `src/host.rs` + unit test）は CLAP 側の全後続の前提
- Task 2（VST3 host bridge: feature/dep 配線 + `src/host.rs` + COM host objects + unit test）は Task 1 と**構造的に独立**（別 crate・並行実装可能 — ただし両者が共有する lane/E2E 形は Task 4 で統一）
- Task 3（規格別 runner bin ×2 + fixture 解決 + `[[bin]] required-features`）は Task 1 + Task 2 に依存（各 runner が自身の bridge API を消費 — task frontmatter に `depends-on` edge 宣言済み）
- Task 4（E2E ×2 + mise task + ci job + OFX 境界明文化 + doc 同期）は Task 3 に依存
- Task 5（final verification + closeout）は Task 4 の全完了後に 1 回のみ — Full Gate の繰返し実行はしない
- テストは test-spec 3 件の契約を、各 adapter crate の `#[cfg(all(test, feature = "host"))]` unit test（in-process dlopen — bridge 契約）と `tests/l1_real_adapter_e2e.rs`（`#[ignore]` ゲート・`PluginHost` 経由）で実装する

## Files / Ownership

| Path | Responsibility | Notes |
| --- | --- | --- |
| `crates/ruviform-plugin-clap/Cargo.toml` | `[features] host = ["dep:clack-host", "dep:ruviform-plugin-export", "clack-extensions/clack-host"]` + `[dependencies] clack-host = { version = "=0.2.0", optional = true }` + `ruviform-plugin-export = { path = "../ruviform-plugin-export", optional = true }` + `[[bin]] name = "ruviform-plugin-clap-l1-runner" path = "src/bin/ruviform-plugin-clap-l1-runner.rs" required-features = ["host"]`（T3）+ `[dev-dependencies] ruviform-plugin-host = { path = "../ruviform-plugin-host" }`（T4） | Task 1/3/4。**`clack-extensions` は非 optional の既存 prod dep** — feature 内 `clack-extensions/clack-host` は plain dep/feature 構文（weak `dep?/feat` 構文は optional dep 専用で適用不可）。`clack-host` は `[dependencies]`(optional) と `[dev-dependencies]`（`features = ["clack-plugin"]` — `load_from_clack` typed-entry test 用・現行維持）の両 entry で feature union |
| `crates/ruviform-plugin-clap/src/lib.rs` | `#[cfg(feature = "host")] pub mod host;`（既存 `mod adapt/gain/plugin` と並置）+ crate doc に host bridge 節 | Task 1 |
| `crates/ruviform-plugin-clap/src/host.rs`（新規） | `ClapLibrary`（`PluginEntry` wrap — `load`/`descriptors`/`factories`）・`ClapPluginFactory`（`PluginFactory` impl — cached descriptor + `create()`）・`ClapPlugin`（`Plugin` impl — `PluginInstance<BridgeHandlers>` 保持・lifecycle/process/params 写像）・`ClapHostError`（`#[non_exhaustive]`）・intern-once helper・`BridgeHandlers`（`HostHandlers` impl — `declare_extensions` で `HostThreadCheck` 非登録） | Task 1。scoped `#[expect(unsafe_code)]` + SAFETY は `PluginEntry::load` のみ |
| `crates/ruviform-plugin-vst3/Cargo.toml` | `[features] host = ["dep:libloading", "dep:ruviform-plugin-export"]` + `libloading = { version = "0.8.9", optional = true }` + `ruviform-plugin-export = { path = "../ruviform-plugin-export", optional = true }` + `[[bin]] name = "ruviform-plugin-vst3-l1-runner" path = "src/bin/ruviform-plugin-vst3-l1-runner.rs" required-features = ["host"]`（T3）+ `[dev-dependencies] ruviform-plugin-host`（T4） | Task 2/3/4。`vst3 = "0.3.0"` は既存必須 dep のまま optional 化しない |
| `crates/ruviform-plugin-vst3/src/lib.rs` | `#[cfg(feature = "host")] pub mod host;`（既存 mod 群と並置）+ crate doc に host bridge 節 | Task 2 |
| `crates/ruviform-plugin-vst3/src/host.rs` または `src/host/` モジュール（新規） | `Vst3Library`（`libloading::Library` + `IPluginFactory` ComPtr — `load`/`descriptors`/`factories`）・`Vst3PluginFactory`（`PluginFactory` impl）・`Vst3Plugin`（`Plugin` impl — component+controller pair 保持）・`Vst3HostError`（`#[non_exhaustive]`）・COM host objects（最小 `IHostApplication`/`IComponentHandler`/`IParameterChanges`/`IParamValueQueue` — `Class`/`ComWrapper`）・intern-once helper | Task 2。全 vtable 呼出し + `libloading` は scoped `#[expect(unsafe_code)]` + SAFETY |
| `crates/ruviform-plugin-clap/src/bin/ruviform-plugin-clap-l1-runner.rs`・`crates/ruviform-plugin-vst3/src/bin/ruviform-plugin-vst3-l1-runner.rs`（新規） | `main() -> ExitCode` — `RUVIFORM_L1_REAL_PLUGIN_DIR` env → `current_exe` sibling の `{DLL_PREFIX}{name}{DLL_SUFFIX}` 解決 → `Library::load` → `eprintln!` + `exit(1)` or `run(lib.factories().into_iter().map(\|f\| Box::new(f) as Box<dyn PluginFactory>).collect())` | Task 3。`spec.args` 不使用（`run` が未知引数を usage reject） |
| `crates/ruviform-plugin-clap/tests/l1_real_adapter_e2e.rs`・`crates/ruviform-plugin-vst3/tests/l1_real_adapter_e2e.rs`（新規） | `#![cfg(feature = "host")]` + `#[ignore = "…"]` ゲート — `env!("CARGO_BIN_EXE_<runner>")` + fixture assert + const `PluginDescriptor` pin + `PluginHost`/`OutOfProcessSpec` E2E + kill/欠落異常系 | Task 4。dev-dep `ruviform-plugin-host` を消費 |
| `mise.toml` | `[tasks.test-l1-real-plugin]`（`cargo build -p ruviform-plugin-clap -p ruviform-plugin-vst3 --features host` → `cargo test -p … --features host --test l1_real_adapter_e2e -- --ignored`）+ `[tasks.lint-l1-real-plugin]`（clippy `--features host -- -D warnings`） | Task 4。icc/ofx lane と同型の並置 |
| `.github/workflows/ci.yml` | `l1-real-verify` job（既存 workflow_dispatch ファイル内・3-OS matrix `fail-fast: false`・`icc-verify` 同型 steps） | Task 4。別 workflow ファイルは作らない |
| `AGENTS.md`（検証入口 task 表）・`crates/README.md`・`crates/ruviform-plugin-clap/README.md`・`crates/ruviform-plugin-vst3/README.md` | 新規 mise task 行 + adapter 行・各 crate README に host bridge/feature `host` 節 + **OFX 境界明文化**（`VideoPluginInstance` が `run(factories)`/audio shm 面に載らない構造的境界 + Issue #118 追跡 + trigger — AC5） | Task 4 |
| `Cargo.lock` | `dep:` optional 昇格・新規 dep の差分 | Task 1/2。追跡継続 |

### 依存の確定（design「依存追加の判断」→ 本計画の配線）

- **`clack-host = "=0.2.0"`（`ruviform-plugin-clap`・optional prod dep・`dep:` under `host`）** — dev-dep → prod 昇格。MIT OR Apache-2.0・clack 族 exact-pin 整合。default features（`libloading`）で `PluginEntry::load` dlopen 経路。dev-dep 側の `features = ["clack-plugin"]` entry は現行維持（`load_from_clack` typed-entry test）— 両 entry で feature union。dep tree 追加: `libloading` + `objc2-foundation`（macOS のみ）
- **`clack-extensions/clack-host`（plain 構文・`host` feature 内）** — `clack-extensions` は非 optional 既存 prod dep のため `dep?/feat` weak 構文は適用不可。host 側 `PluginParams`/`HostThreadCheck` 型を `--features host` 時に有効化（dev-dep 側の `["clack-host", "thread-check"]` とは独立した prod 側配線）
- **`libloading = "0.8.9"`（`ruviform-plugin-vst3`・optional・`dep:` under `host`）** — ISC。`ruviform-plugin-ofx` と同一バージョン（`Cargo.lock` 1953 行 `vst3 0.3.0`・683 行 `libloading 0.8.9` 実在確認済み）— 新規バージョン導入なし
- **`vst3 0.3.0`** — 既存 prod dep のまま（optional 化しない）。host-side COM object は `Class`/`ComWrapper`/`Interface`（`vst3::` re-export 確認済み — lib.rs:65）
- **`ruviform-plugin-export`（両 adapter crate・optional・`dep:` under `host`）** — runner bin のみが使う内部 dep。adapter → export は baseline 許可方向
- **`ruviform-plugin-host`（両 adapter crate・dev-dep）** — E2E の `PluginHost`/`OutOfProcessSpec` のため。test 対象のみ
- **`vst3-host` 0.10.0 / `vst3-sys` / `vst3_com`** — 不採用（design「依存追加の判断」に評価・棄却理由記録済み — 再評価 trigger 付き deferred）
- `Cargo.lock` は追跡継続 — dep 昇格・新規 optional dep の差分を記録

## Implementation Sequence（16 action — 将来 task 名のみ確定）

| # | Action | Scope |
| --- | --- | --- |
| 1 | `ruviform-plugin-clap/Cargo.toml`: `[features] host` + `clack-host`(optional, `=0.2.0`) + `ruviform-plugin-export`(optional, path) + `clack-extensions/clack-host`（plain 構文）— dev-dep `clack-host`/`clack-extensions` entries は現行維持（feature union）+ `Cargo.lock` 更新 + dep 昇格理由の comment | clap deps |
| 2 | `src/host.rs` 新設 + `#[cfg(feature = "host")] pub mod host`: `ClapLibrary`（`PluginEntry` wrap・`load(path) -> Result<Self, ClapHostError>` — `PluginEntry::load` は unsafe・scoped `#[expect(unsafe_code)]` + SAFETY）+ `descriptors()`（load 時 scan: `entry.get_plugin_factory()` → `plugin_descriptors()` → `clap_plugin_descriptor` の id/name/vendor/version/features（順序保持）→ `PluginDescriptor` 全 field 写像 + intern-once `String::leak`/`Vec::leak` — `&'static` 実体化）+ `factories()` → `Vec<ClapPluginFactory>` + `ClapHostError`（`#[non_exhaustive]` — `PluginEntryError`/`PluginInstanceError` defined 写像）| clap bridge load/scan |
| 3 | `ClapPluginFactory`（`PluginFactory` impl — `descriptor()` = load 時 cached 値・`create()` → `ClapPlugin`・`create_sendable` は default `None` — `PluginInstance` は `!Send`/`!Sync` (`_no_send` PhantomData)）+ `ClapPlugin`（`Plugin` impl — `init` → `PluginInstance::new`（clap `plugin.init` 経路）・`prepare` → `activate(|_, _| BridgeAudioProcessor, PluginAudioConfiguration{sample_rate, min_frames_count, max_frames_count}) -> Result<StoppedPluginAudioProcessor, PluginInstanceError>`（戻り値 `StoppedPluginAudioProcessor` は `ClapPlugin` field に格納 — `start_processing`/`stop_processing`/`Drop` の `deactivate` が消費）+ `prepare` で bridge-owned buffer を一括確保（**input bounce pin**: `HostInput<'_>` は `&[f32]` のみ yield するが `InputChannel`/`AudioPorts::with_input_buffers` は `&'a mut [f32]` を要求するため input scratch `channels × max_frames` を確保し `process` は copy-in のみ・`AudioPorts`/`EventBuffer`/`OutputEvents` は Vec-backed（realloc risk）のため `ClapPlugin` の persistent field として preallocate — per-block alloc 禁止・`from_raw_buffers` 手組立で immutable `&[f32]` を `*mut` に見せる const-cast は **REJECTED**（UB-adjacent — SAFETY で正当化不可））・`start_processing`/`stop_processing` → 同名動詞（`Stopped`↔`Started` processor 保持）・`process_host` → `StartedPluginAudioProcessor::process(&InputAudioBuffers, &mut OutputAudioBuffers, &InputEvents, &mut OutputEvents, steady_time: Option<u64>, transport: Option<&TransportEvent>)`（arity 6 — `HostInput`/`HostOutput` → `InputAudioBuffers`/`OutputAudioBuffers` は bounce/persistent buffer 経由・`EventKind::ParamValue` → `EventBuffer` 内 `ParamValueEvent`・`ProcessResult` → `ProcessStatus` 読取り・`steady_time`/`transport` = `None`/`None` — **v1 defined boundary**: `TransportInfo`→`TransportEvent` 投影は `TransportInfo` snapshot から beat-domain loop field/`tempo_inc` を honest に導出不能（tempo map 非保持）のため deferred — fixture は transport 非参照・半投影の 0 埋めより明示 `None` を採る）・`params` → params extension `PluginParams::count`/`get_info` → 中立 `ParamInfo` 表（`unit: ""` — CLAP param_info に unit 面なし・intern-once）・`save_state`/`restore_state`/`open_gui`/`close_gui` = `Plugin` 既定 `Err(Unsupported)` 継承・**teardown は `Drop` 駆動**: 中立面に `deactivate`/`terminate` hook はなく `Deactivate`/`Destroy`/`Shutdown` 動詞は export Runner の lifecycle 遷移のみに留まる（export/lib.rs:424-429・646-666 — plugin hook 非呼出し）ため、`Drop` で best-effort `stop_processing`（Started 時）→ `instance.deactivate(stopped)`（Activated 時）→ instance drop（CLAP `plugin.destroy`））+ `BridgeHandlers`（`HostHandlers` impl — `Shared`/`MainThread`/`AudioProcessor` handler 型 + `declare_extensions` は `HostThreadCheck` を**登録しない** — fixture は `get_extension::<HostThreadCheck>()` が `None` を返す graceful fallback で挙動不変（plugin.rs:130・421））| clap bridge instance |
| 4 | `--features host` unit test（in-process dlopen — `current_exe` sibling `deps/{DLL_PREFIX}ruviform_plugin_clap{DLL_SUFFIX}` — 欠落は `assert!` fail）: `descriptors()` 実値 == fixture 既知値 const literal（id `org.ruviform.clap-gain-poc`・name `Ruviform Gain PoC`・vendor `Ruviform`・version `0.1.0`（env!("CARGO_PKG_VERSION")）・features `[audio-effect, stereo]` 順序保持）・params 表写像（1 param: `ParamId(0)`・`[0.0, 2.0]`・default `1.0`）・error 写像（不在 path・非規格バイナリ → `Err(ClapHostError)` — panic しない）+ narrow gate `cargo test -p ruviform-plugin-clap --features host`・`cargo clippy -p ruviform-plugin-clap --all-targets --features host -- -D warnings`・workspace `check`/`lint`/`test`（default 構成不変 — `host` 未指定で `host.rs`・新規 dep が compile 対象外）| clap gate |
| 5 | `ruviform-plugin-vst3/Cargo.toml`: `[features] host` + `libloading`(optional, `0.8.9`) + `ruviform-plugin-export`(optional, path) + `Cargo.lock` 更新 | vst3 deps |
| 6 | `src/host.rs`/`src/host/` 新設 + `#[cfg(feature = "host")] pub mod host`: `Vst3Library`（`libloading::Library::new` → `GetPluginFactory` symbol → `IPluginFactory` ComPtr — platform module hook `InitDll`/`ModuleEntry`/`bundleEntry` は export されていれば best-effort 呼出し）+ `descriptors()`（`countClasses` → `getClassInfoUnicode`（`IPluginFactory3`）/`getClassInfo2`（`IPluginFactory2`）列挙 → `category == "Audio Module Class"` のみ採用（`"Component Controller Class"` は pair 対象・独立プラグインではない）→ `PluginDescriptor { id: cid 16byte hex, name, vendor, version, features: subCategories }` + intern-once）+ `factories()` + `Vst3HostError`（`#[non_exhaustive]` — tresult 値を運ぶ `Com` variant 含む）| vst3 bridge load/scan |
| 7 | COM host objects（bridge 内 `Class`/`ComWrapper` 実装 — component.rs test の `TestQueue`/`TestChanges` 同型先例）: 最小 `IHostApplication`（`IPluginBase::initialize(context)` の context 供給）・最小 `IComponentHandler`（`setComponentHandler` 経由注入 — fixture は非保持 `kResultOk` acknowledge・controller.rs:226）・`IParameterChanges`/`IParamValueQueue`（`ProcessData.inputParameterChanges` 供給 — `EventKind::ParamValue` を normalized `ParamValue` のまま per-frame offset point として積む bridge 所有 object）| vst3 COM host |
| 8 | `Vst3PluginFactory`（`PluginFactory` impl — `descriptor()` cached・`create()` → `Vst3Plugin`・`create_sendable` default `None`）+ `Vst3Plugin`（`Plugin` impl — `init`/`create` 段: `createInstance(component CID)` → QI `IComponent`/`IAudioProcessor` → `getControllerClassId` → `createInstance(controller CID)` → `IEditController` → component・controller 双方の `initialize(context)`（`IHostApplication`）→ `setComponentHandler`／`prepare` → **`setActive(true)` → `setupProcessing(ProcessSetup{processMode: kRealtime, symbolicSampleSize: kSample32, maxSamplesPerBlock, sampleRate})` の順**（fixture は `Activated` 未満に `kResultFalse` — component.rs:597・`maxSamplesPerBlock ∈ (0, MAX_NEGOTIATED_FRAMES]`・`sampleRate ≥ 1.0`・kSample64 reject）→ `setProcessing(true)`（`start_processing`）→ `process(ProcessData)`（`HostInput`/`HostOutput` → `AudioBusBuffers{numChannels: 2, channelBuffers32}` — `channelBuffers32` は `*mut *mut Sample32` の writable 型: **input は bounce 固定** — `HostInput` の `&[f32]` を `*mut` 経由で alias させるのは禁止（component.rs:1195-1201 の bounce 先例 — plugin が input を書き換える in-place 形状を host 側が許さない）・bridge-owned input scratch + channel pointer 配列を `prepare` で `channels × max_frames` 一括確保（per-block alloc なし）・`process` は copy-in のみ・**output は `HostOutput.channel_mut`（`&mut [f32]`）に `channelBuffers32` を直接指させてよい**（mutable 既知 — alias 安全）・単一 in/out bus・kSample32・kRealtime defined 境界・`numSamples`・`inputParameterChanges` = bridge object・`processContext` = transport 写像 or null）→ `setProcessing(false)`（`stop_processing`）→ **teardown は `Drop` 駆動**（中立面に `deactivate`/`terminate` hook なし — export/lib.rs:424-429）: `Drop` で best-effort `setProcessing(false)`（Processing 残留時・fixture は Processing 中の `setActive(false)` を `kResultFalse` 拒否のため stop 先行対称）→ `setActive(false)`（Activated 時）→ `terminate`（両 object）→ release → module exit hooks → `Library` drop — `Arc<Vst3Library>` で instance → library drop 順序保証（OFX `OfxLibrary::share` 先例）・`params` → `IEditController::getParameterCount`/`getParameterInfo` → `ParamInfo` 表（intern-once — fixture: 1 param `ParamId(0)`・`[0.0, 1.0]`・default `1.0`・`unit: ""`）・**state 動詞（`IComponent`/`IEditController` の `setState`/`getState`/`setComponentState`）は一切呼ばない** — `save_state`/`restore_state`/`open_gui`/`close_gui` = `Plugin` 既定 `Err(Unsupported)`）| vst3 bridge instance |
| 9 | `--features host` unit test（in-process dlopen — `deps/` sibling fixture）: `descriptors()` == 既知値 const（component のみ採用: name `Gain`・vendor `Ruviform`・version `0.0.0`・features `["Fx"]`・id = CID hex — **`uid()` の TUID byte order は platform 依存**のため expected id は test が `GainComponent::CID`（`pub(crate)` — same-crate unit test から直接参照可）raw 16byte から実行時に derive・`#[cfg]`-conditional literal は regression pin のみ: Windows GUID-order `3d2c1b5a5f4e7b6a8c9d0e1f2a3b4c5d`・非 Windows big-endian `5a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d`）・`"Component Controller Class"` filter（controller が独立プラグインとして列挙されない）・params 表・error 写像（不在 path・`GetPluginFactory` 欠落 fixture・tresult 非 Ok → `Err(Vst3HostError)`）+ narrow gate（action 4 同型 — `-p ruviform-plugin-vst3`）| vst3 gate |
| 10 | runner bin ×2（`[[bin]] required-features = ["host"]` — `src/bin/ruviform-plugin-clap-l1-runner.rs`・`src/bin/ruviform-plugin-vst3-l1-runner.rs`）: `main() -> ExitCode` — fixture 解決 `RUVIFORM_L1_REAL_PLUGIN_DIR` env（任意 override・shell-only）→ unset 時 `current_exe` sibling → `dir.join(format!("{}{name}{}", DLL_PREFIX, DLL_SUFFIX))` → `Library::load` → `Err` は `eprintln!` + `exit(1)`（`run` 到達前 = Hello 未送出 → 親 spawn `Err(Ipc)`）→ `ruviform_plugin_export::run(factories().into_iter().map(\|f\| Box::new(f) as Box<dyn PluginFactory>).collect())`。`Scan` 応答は load 時 cached descriptor（`descriptor()` が plugin バイナリ依存の失敗を持たない構造 — `Scan`/argv 解決で dlopen 再実行しない）・**`spec.args` 不使用**（`RunnerArgs::parse` が未知引数を `ArgvReject::Usage` → exit 2 で拒否 — `run` argv 規約不変のため E2E は `spec.args` 空のまま）| runners |
| 11 | runner smoke gate: `cargo build -p ruviform-plugin-clap -p ruviform-plugin-vst3 --features host` で両 runner bin + fixture cdylib 生成を確認 + shell 直接起動の defined 挙動 pin — (i) 引数なし起動: fixture load 成功後 `run` argv parse が `missing --ruviform-shm-key` → exit 2（fixture load が通った証拠 + usage reject）・(ii) `RUVIFORM_L1_REAL_PLUGIN_DIR=<empty dir>` 起動 → stderr diagnostic + exit 1（binary 不在 defined rejection — `run` 未到達）| runners gate |
| 12 | `tests/l1_real_adapter_e2e.rs` ×2（`#![cfg(feature = "host")]` + 各テスト `#[ignore = "…"]` 理由付き）: **主 E2E** — `env!("CARGO_BIN_EXE_<runner>")`（same-crate）+ fixture path assert（`current_exe` → `deps/` → `target/<profile>/` の `{DLL_PREFIX}{name}{DLL_SUFFIX}` — `runner_program()` 同型・欠落は fail）+ `EXPECTED: PluginDescriptor`（CLAP: `org.ruviform.clap-gain-poc`/`Ruviform Gain PoC`/`Ruviform`/`0.1.0`/`&["audio-effect","stereo"]`・VST3: cid hex — **expected id は test が `vst3::uid(0x5A1B2C3D, 0x4E5F6A7B, 0x8C9D0E1F, 0x2A3B4C5D)` bytes から実行時 derive**（`vst3` は adapter crate の既存 prod dep で test target から参照可・`GainComponent::CID` は `pub(crate)` で tests/ から不可のため `uid(...)` 呼出しで等価 derive — `id` は `String::leak` で `&'static` 化）・`cfg`-conditional literal は regression pin のみ）/`Gain`/`Ruviform`/`0.0.0`/`&["Fx"]`）を `OutOfProcessSpec::new(EXPECTED, runner)`（`spec.args` 空・既定 `lookahead_blocks=2`/`command_timeout=10s`）→ `PluginHost` で `Scan`→`Instantiate`→`Initialize`→`Activate`→`StartProcessing`→N block `process`（`ParamValue` event 込み — CLAP normalized v → plain `2v`・VST3 → plain `v` — delivered block == gain × lookahead 前 input・underrun/pass-through 意味不変）→ `SaveState`/`RestoreState` → `Err(Unsupported)` defined 応答（bridge は規格 state 動詞を呼ばない — verb が wire に乗る経路の実証）→ `StopProcessing`→`Deactivate`→`Destroy` → 子 exit(0)。**異常系 1 本/規格**: `PluginHost::kill_child` → `Failed` + `HostNotification::InstanceFailed` once + 以後 defined 拒否（`Err(HostError::Failed)`）+ pass-through・自動再起動なし + **binary 不在**: 存在しない program path の spec → spawn `Err(HostError::Ipc)`（ハング・silent skip なし）| E2E |
| 13 | lane + doc 配線: `mise.toml` に `test-l1-real-plugin`（run 配列 = `cargo build --features host` → `cargo test -p ruviform-plugin-clap -p ruviform-plugin-vst3 --features host --lib`（in-process bridge unit test 層 — `--test`/`--ignored` 指定では実行されないため明示 step）→ `cargo test ... --features host --test l1_real_adapter_e2e -- --ignored`）/`lint-l1-real-plugin` task + `ci.yml` に `l1-real-verify` job（3-OS matrix・`fail-fast: false`・`icc-verify` 同型 steps — ALSA 不要: adapter dep 木に cpal 非出現）+ `AGENTS.md` 検証入口 task 表へ新規 2 行 + `crates/README.md` adapter 行更新 + 両 crate README に `host` feature/bridge/runner/lane 節 + **OFX 境界明文化**（`VideoPluginInstance` は全操作 NONRT・video wire 非確立で `run(factories)`/audio shm 面に載らない構造的境界 + Issue #118 で追跡 + trigger = NONRT video port の分離要求 — AC5・配置は crate README or plugin-ofx README の適切節）| lane+docs |
| 14 | lane gate: `mise run test-l1-real-plugin`（build → `--lib` unit test → `--ignored` 全 pass — 両規格 main + kill + 欠落系 + in-process bridge 契約層）+ `mise run lint-l1-real-plugin` exit 0 + default `cargo test --workspace` が本 lane に非依存で不変グリーン（`#[ignore]` は `cargo test` 既定経路で実行されない — `#![cfg]`/`required-features` で default build に成果物ゼロ）+ `cargo tree -p ruviform-plugin-clap -p ruviform-plugin-vst3` DAG 監査（adapter → export/host(dev) 方向・逆向き依存ゼロ・`host` off で新規 dep が default 木に非出現）| lane gate |
| 15 | **Full Gate（1 回のみ・本計画最後段）** — `mise run fmt-check`・`check`・`lint`・`test`・`test-rt`・`bench-smoke`・`mise run test-l1-real-plugin`・`mise run lint-l1-real-plugin`・`mise exec -- prek run --all-files`・doc audit・`cargo tree` DAG 監査・`mise exec -- cargo doc -p ruviform-plugin-clap -p ruviform-plugin-vst3 --features host --no-deps`（rustdoc warning 0）— 全 log `docs/impl/exp/data/issue-114-final-gate-*-*.log` 保存・`test-rt` は実一致件数で証明 | gate |
| 16 | canonical closeout — 実績 IR（`docs/impl/ir/issue-114-*.md`）・coverage 記述・task/plan status 同期・3 test-spec を実結果で `approved` 化・design/spec の実装相違は正本へ同期反映。GitHub merge は lead の authoritative `gh` のみ | closeout |

- test-spec（起票済み・proposed）: [adapter-host-bridge-contract-tests](../test-specs/adapter-host-bridge-contract-tests.md)・[real-adapter-l1-e2e-tests](../test-specs/real-adapter-l1-e2e-tests.md)・[l1-real-plugin-lane-tests](../test-specs/l1-real-plugin-lane-tests.md) — `verified-by` は実ファイルに更新済み
- **5 task 文書**（起票済み・todo・actions 1-4/5-9/10-11/12-14/15-16）: [clap-host-bridge](../tasks/clap-host-bridge.md)・[vst3-host-bridge](../tasks/vst3-host-bridge.md)・[real-adapter-runner-bins](../tasks/real-adapter-runner-bins.md)・[real-adapter-e2e-and-lane](../tasks/real-adapter-e2e-and-lane.md)・[real-adapter-final-verification](../tasks/real-adapter-final-verification.md)
- 各 task 間は狭い検証のみ（full suite 繰返しはしない — Full Gate は action 15 の一回）

## Test Oracles（design/test-spec の正本どおり・新規代替機構禁止）

- **handshake field-equality（AC2/AC3）**: `L1Instance::handshake` は `Hello`（version == `wire::WIRE_VERSION`）→ `Scan` → `Reply::Scanned(descriptors)` のいずれか 1 つが `Descriptor::matches(&spec.descriptor)` で field-equality（id/name/vendor/version/features 順序保持 — wire.rs:407）を要求。子は `Library::load` scan で実 metadata から合成、親は既知値を pin（VST3 `id` は `vst3::uid(...)` bytes から derive — literal は regression pin のみ）— **同一 scan API を両側で呼ばない**（同一関数の同値では写像の検証力を持たない — `uid()` は vendored 決定的 const fn で scan 経路ではない）。不一致・EOF・version mismatch は全て spawn `Err(HostError::Ipc)`
- **fixture 既知値（親側 pin）**: CLAP `PluginDescriptor { id: "org.ruviform.clap-gain-poc", name: "Ruviform Gain PoC", vendor: "Ruviform", version: "0.1.0" /* env!("CARGO_PKG_VERSION") */, features: &["audio-effect", "stereo"] }`（plugin.rs:197-203 — `features::AUDIO_EFFECT`/`STEREO` 順序）。VST3 `PluginDescriptor { id: <cid hex>, name: "Gain", vendor: "Ruviform", version: "0.0.0", features: &["Fx"] }`（factory.rs `getClassInfo2`/`getClassInfoUnicode` — cid = `GainComponent::CID` = `uid(0x5A1B2C3D, 0x4E5F6A7B, 0x8C9D0E1F, 0x2A3B4C5D)` — **`uid()` byte order は platform 依存**: Windows `[3D,2C,1B,5A,5F,4E,7B,6A,8C,9D,0E,1F,2A,3B,4C,5D]` → `"3d2c1b5a5f4e7b6a8c9d0e1f2a3b4c5d"`・非 Windows `[5A,1B,2C,3D,4E,5F,6A,7B,8C,9D,0E,1F,2A,3B,4C,5D]` → `"5a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d"` — **expected id は test が `vst3::uid(0x5A1B2C3D, 0x4E5F6A7B, 0x8C9D0E1F, 0x2A3B4C5D)`/`GainComponent::CID` の bytes から実行時に derive**（same-crate `vst3` dep — literal は regression pin のみ））
- **lifecycle 写像（CLAP）**: `init` → `PluginInstance::new`（create+init 一体 — 中立 `Created → Initialized` が CLAP instantiate+`plugin.init` を駆動）・`prepare` → `activate(|_, _| BridgeAudioProcessor, PluginAudioConfiguration{sample_rate: f64, min_frames_count, max_frames_count}) -> Result<StoppedPluginAudioProcessor, PluginInstanceError>`（戻り値は `ClapPlugin` field に格納 — `start_processing`/`Drop` の `deactivate` が消費）・`start_processing` → `StoppedPluginAudioProcessor::start_processing`・`process_host` → `StartedPluginAudioProcessor::process(&InputAudioBuffers, &mut OutputAudioBuffers, &InputEvents, &mut OutputEvents, steady_time: Option<u64>, transport: Option<&TransportEvent>)`（**input bounce**: `HostInput` の `&[f32]` を `prepare` 確保の bridge-owned scratch（`channels × max_frames`）へ copy-in — `InputChannel`/`with_input_buffers` は `&'a mut` 要求・`from_raw_buffers` const-cast 手組立は REJECTED（UB-adjacent）・`AudioPorts`/`EventBuffer`/`OutputEvents` は Vec-backed persistent field — `steady_time`/`transport` = `None`/`None` v1 defined boundary）・`stop_processing` → `stop_processing() -> StoppedPluginAudioProcessor`・**teardown = `Drop` 駆動** — 中立 `Deactivate`/`Destroy`/`Shutdown` 動詞は plugin hook を持たず export Runner の lifecycle 遷移のみ（export/lib.rs:424-429・646-666）のため、`Drop` で best-effort `stop_processing` → `instance.deactivate(stopped)` → instance drop（CLAP `plugin.destroy`）
- **lifecycle 写像（VST3 — 順序 pin）**: `createInstance(component CID)` → QI `IComponent`/`IAudioProcessor` → `getControllerClassId` → `createInstance(controller CID)` → `IEditController` → 両 `initialize(context)`（`IHostApplication` 供給）→ `setComponentHandler`／`prepare` → `setActive(true)` → `setupProcessing(ProcessSetup{processMode: kRealtime, symbolicSampleSize: kSample32, maxSamplesPerBlock, sampleRate})`（**この順序 — fixture は非 Activated で `kResultFalse`**）→ `setProcessing(true)` → `process(ProcessData)`（`numInputs: 1`/`numOutputs: 1` — 単一 in/out audio bus・`kSample32`・`kRealtime` defined 境界）→ `setProcessing(false)` → **instance `Drop` で** `setActive(false)` → `terminate`（両 object）→ release（中立 `Deactivate`/`Destroy` に plugin hook なし — export/lib.rs:424-429）。`setBusArrangements`/bus 照会は fixture では optional（stereo 既定受理）・`IPluginFactory3::setHostContext` は呼んでも省略してもよい（fixture は trivial `kResultOk`）
- **state 動詞 = defined 応答往復**: bridge は規格 state 動詞を一切呼ばない（CLAP `state` ext 未解決・VST3 `IComponent::getState`/`setState` は `kNotImplemented` stub・`IEditController::setState`/`getState` は `kResultOk` no-op stub — component.rs:509-516/controller.rs:87-93）。`save_state`/`restore_state` は `Plugin` 既定の `Err(HostError::Unsupported)` — E2E は `SaveState`/`RestoreState` command が wire を往復し `Err(Unsupported)` 応答で収束することを観測（verb の wire 経路実証）
- **process データ正当性**: delivered block == gain × `lookahead_blocks`（既定 2）前の入力 — L1 lookahead/pass-through 意味は不変（underrun/結果未着 → 直前受信 else 無音・常に `Ok(CONTINUE)`）。`ParamValue` event 込み: CLAP fixture の GAIN_PARAM は plain `[0.0, 2.0]`（normalized v → gain `2v` — e.g. `ParamValue::new(0.25)` → `×0.5`）・VST3 fixture は plain `[0.0, 1.0]`（normalized v → gain `v` — `0.25` → `×0.25`）— 規格別の期待係数を test が pin
- **異常系継承（AC4）**: `PluginHost::kill_child`（composite.rs:810）→ `Failed` + `HostNotification::InstanceFailed` once + 以後 `Err(HostError::Failed)` 系 defined 拒否 + pass-through + 自動再起動なし + 応答なしハングなし（`command_timeout` 10s bound）。規格バイナリ不在/dlopen/entry/scan 失敗 → 子 exit(1) pre-Hello → spawn `Err(Ipc)`・`create()` 内失敗 → panic catch → exit(1) post-Hello → `Err(Ipc)`。bridge foreign crash は子 crash として既存 `Failed` 契約に吸収
- **descriptor/params `'static` 実体化**: `Library::load` scan 時（descriptor id/name/vendor/version + features 各要素 + features slice）と factory 構築時（param `name`/`unit` + table slice — `unit: ""` literal は既に `'static`）に限り `String::leak`/`Vec::leak` — `descriptor()`/`params()` は intern 済み値を返すのみ・per-block/per-instance 増殖なし（design §'static メタデータ実体化の sanctioned exception 4 性質: bounded・once-per-load・process-scoped・feature 封じ）
- **argv 規約**: spawn argv = `spec.args`（空）+ `--ruviform-plugin-id <spec.descriptor.id>` + `--ruviform-shm-key <key>`（l1/mod.rs:1007-1011 — host は `--ruviform-plugin-id` を常時 append）。`RunnerArgs::parse` は `--ruviform-shm-key` 必須・`--ruviform-plugin-id` は `factories.len() != 1` のときのみ必須（欠落 → `ArgvReject::Usage` → exit 2・**非一致 id も `ArgvReject::Usage` → exit 2** — `no factory for plugin id`）・**未知引数 → `ArgvReject::Usage` → exit 2**・`descriptor()` panic → `ArgvReject::Panic` → exit 1。`run` は `factories[factory_index].create()` を 1 回呼ぶ（`create_sendable` 不使用 — `!Send` bridge instance は L1 経路で不要求）
- **Send**: `PluginInstance`（clack-host）は `!Send`/`!Sync`（`_no_send`/`_no_sync` PhantomData）・`Vst3Plugin`（raw COM pointer 保持）も `!Send` — 両 bridge factory の `create_sendable` は default `None`（L1 子は single-thread — discovery confirmed）

## Required mise tasks

- 既存タスク: `fmt-check`/`check`/`lint`/`test`/`test-rt`/`bench-smoke`・`mise exec -- prek run --all-files`・`mise exec -- node .agents/skills/doc-status/scripts/doc_status.js audit --json`・`mise exec -- cargo doc -p ruviform-plugin-clap -p ruviform-plugin-vst3 --features host --no-deps`（Full Gate の rustdoc warning 0 検証 — action 15）
- **新規タスク（Task 4 で追加 — 本 lane が唯一の実行者）**: `test-l1-real-plugin`（`cargo build -p ruviform-plugin-clap -p ruviform-plugin-vst3 --features host` → `cargo test -p … --features host --test l1_real_adapter_e2e -- --ignored`）・`lint-l1-real-plugin`（`cargo clippy -p ruviform-plugin-clap -p ruviform-plugin-vst3 --all-targets --features host -- -D warnings`）
- bridge unit test は `cargo test -p <adapter> --features host`（narrow gate — action 4/9）または `cargo test -p ruviform-plugin-clap -p ruviform-plugin-vst3 --features host`（lane の build 後・`--ignored` 指定なしの通常 test run で実行される層 — `#[ignore]` なしの unit test のみ実行）
- `rt_` 命名規約は維持 — bridge 追加テストに `rt_` 名は付けない（子内 bridge は `rt_` 対象外・host 側 `rt_` 既存テストが不変グリーン）
- CI: `.github/workflows/ci.yml` の `verify` ジョブ（3 OS matrix・workflow_dispatch）は本変更の default 経路をそのまま検証 — `l1-real-verify` job（3-OS matrix・`fail-fast: false`）が手動 dispatch 時に opt-in lane を実行。ローカル実証は Windows のみ・macOS/Linux は CI 実行時のみ記録（計画書は未実行の主張をしない）

## Risks / Readiness

| # | Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- |
| 1 | **VST3 COM host 手実装が本 Issue の最大新規コード**（component+controller pair・`IParameterChanges`/`IParamValueQueue`/`IHostApplication`/`IComponentHandler` 実装・`ProcessData` 組立・teardown 順） | med | high | PoC component.rs test の `TestQueue`/`TestChanges` が同型先例・fixture が要求する面に限定（single bus・kSample32・kRealtime の defined 境界）・`ComPtr`/`ComRef` が refcount を担う・Task 2 単独 task 化で review 集中 |
| 2 | **`uid()` の TUID byte order が platform 依存** — descriptor `id` の hex が Windows/非 Windows で異なり、単一 const literal では handshake field-equality が platform 毎に偽 fail | med | med | bridge は **raw 16byte の小文字 hex** のみを emit（規格準拠の正規化 GUID 形式は v1 不採 — fixture が報告する bytes をそのまま運ぶ）・E2E/unit test の expected id literal は `#[cfg(target_os = "windows")]`-conditional（両値を計算済み pin — Review Handoff 参照） |
| 3 | **feature union の複雑さ**（`clack-extensions/clack-host` plain 構文・`clack-host` の prod optional + dev `clack-plugin` feature の union・dev/prod 両 entry） | med | med | Task 1 で `cargo tree --features host`/default 両面の dep 木確認を gate 化（`host` off で `clack-host`/`libloading`/`ruviform-plugin-export` が default 木に非出現）— action 14 の `cargo tree` 監査 |
| 4 | `descriptors()`/`factories()`/`params()` の intern-once 実装誤り（`'static` 境界の leak 忘れ・per-call 増殖） | med | med | test-spec で「load/factory 構築時の 1 回限り」を固定 + unit test が `descriptors()` 複数回呼出で値同一性を pin（再 scan しない — cached 値） |
| 5 | **CLAP `init` の中性 lifecycle 写像誤り** — `PluginInstance::new` は CLAP `create_plugin`+`plugin.init` を一体で行うため、`create()` で instance を作ると `init` hook が `Initialize` より早く走り lifecycle 順序が崩れる | med | med | bridge は `ClapPlugin { instance: Option<PluginInstance> }` で `create()` は `None`・`init()` で `PluginInstance::new` を実行（中立 `Initialize` が規格 instantiate+init を駆動 — design の `init` → `PluginInstance::new` 写像を厳密化して test-spec に pin） |
| 6 | **VST3 fixture の state stub が `kResultOk` no-op を返す誤検証** — controller 経由 `setState`/`getState` を呼ぶと「成功した state round-trip」偽装 | low | med | bridge は規格 state 動詞を**一切呼ばない**を code + test-spec で pin — E2E は `Err(Unsupported)` defined 応答のみ観測（偽の round-trip 主張を禁止） |
| 7 | **drop 順序（instance → library unload）** — foreign object が unload 済み lib の vtable を触ると crash | low | high | `Arc<Library>` で factory/instance が library 参照を保持（OFX `OfxLibrary::share` 先例）・teardown 順（`setProcessing(false)`→`setActive(false)`→`terminate`→release→module exit hooks→`Library` drop）を test-spec に固定 |
| 8 | **`#[ignore]` lane は実行されないと腐る** — dispatch 以外では走らない | med | low | `l1-real-verify` job（workflow_dispatch）での手動 dispatch 時検出・runbook に `mise run test-l1-real-plugin` を明記（opt-in の性質上不可避 — spec AC6 整合） |
| 9 | runner/fixture の `target/<profile>/` 解決が test 構成で壊れる（`cargo test -p` 単体実行時に fixture cdylib 未生成） | med | med | lane task の先頭で `cargo build --features host`（runner+fixture 両生成を保証）・test 内 `assert!(path.is_file())` で skip せず fail・deps/ にも cdylib が出る実測確認済み（`target/debug/deps/ruviform_plugin_clap.dll`） |
| 10 | Windows/POSIX 差・CI のみ 3 OS（dlopen/spawn/COM ABI の実経路差・`libloading` symbol 型差・`TChar`/`String128` 写像） | med | med | ローカルは Windows 実証・3 OS matrix は `l1-real-verify` job に委譲（fail-fast: false で全 OS 結果収集 — issue-76 と同じ platform notes） |
| 11 | **bridge foreign crash は `catch_unwind` で catch 不能**（規格 plugin の SEGV/abort は unwind ではない） | low | low | 子プロセス crash → 既存 `Failed` 契約に吸収（これが L1 分離の検証対象そのもの — design Risks 記載どおり） |

## Rollback

- 新規ファイル（`host.rs`・runner bin・`l1_real_adapter_e2e.rs`）と additive 変更（feature/dep・`#[cfg]` mod 宣言・mise task・ci job・doc 節）が主体 — 履歴保持の corrective/revert commit で task 単位に戻す（reset/force なし）。`Cargo.lock` は `git checkout -- Cargo.lock` で戻し再生成
- feature `host` は default 集合に入らない — revert は feature + `#[cfg]` 節の除去で完結し、default 構成（cdylib plugin artifact・既定 dep 木）は全期間で不変
- `run`/`OutOfProcessSpec`/wire/`RtLeg`/plugin-core は無変更 — upstream 側の revert 対象なし
- `mise.toml`/`ci.yml`/`AGENTS.md`/README 系の変更は additive — task 除去で完結
- DB/persistent data migration なし

## Verification

- per-task narrow checks のみ — 各 task の `## Verification` が run 可能コマンドを持つ（`mise run check`/`lint`/`test`・adapter 集中は `cargo test/clippy -p <adapter> --features host`・lane は `mise run test-l1-real-plugin`/`lint-l1-real-plugin`）
- Full Gate は action 15 の一回のみ — 全 log 保存・`test-rt` 実一致件数で証明
- spec AC1–AC8 ↔ actions/test-specs の写像: AC1（runner オーサリング）→ actions 1・5・10–12（tspec e2e/lane）・AC2（実 CLAP L1 E2E）→ actions 1–4・10・12（tspec bridge/e2e）・AC3（実 VST3 L1 E2E）→ actions 5–9・10・12（tspec bridge/e2e）・AC4（異常系維持）→ action 12（tspec e2e）・AC5（OFX 境界明文化）→ action 13・16・AC6（CI lane）→ actions 13–14（tspec lane）・AC7（契約維持）→ actions 1–14 全般 + 14・15（`cargo tree`/default 構成不変の gate）・AC8（quality gate）→ actions 15–16
- doc 監査チェーン起因 findings 0（`mise exec -- node .agents/skills/doc-status/scripts/doc_status.js audit --json`）
- test-spec link は `verified-by` に更新済み — **実検証は未実施**（PLANNED・未実行）

## Open Questions / Gaps

- **`id` の CID hex 表現（platform-order）** — design「id = CID 16byte hex」を **raw TUID bytes の小文字 hex（正規化・ハイフン無し）** と pin した（bridge は fixture が `uid()` で報告した bytes をそのまま運ぶ — Windows `3d2c1b5a5f4e7b6a8c9d0e1f2a3b4c5d`・非 Windows `5a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d`）。規格準拠の canonical GUID 形式（`{XXXXXXXX-…}`）への正規化は fixture 単一規格形状で不要（YAGNI — 実 VST3 plugin 導入時の再評価論点として Risk 2 に記録）
- **`factories()` 戻り値形** — design 概念形は `Vec<ClapPluginFactory>` を示す一方で runner sketch は `lib.factories().map(...)` を書く（`Vec` に `map` は無い）— 実装は `Vec` を返し runner 側で `into_iter().map(|f| Box::new(f) as Box<dyn PluginFactory>)`（概念形の署名差分として plan で pin）
- **`descriptors()` 戻り値形** — load 時に intern 済み値を持つため `&[PluginDescriptor]`（cached slice 返却）を pin（`Vec` 再構築はしない — 概念形の微修正）
- **bus 数の defined 境界** — v1 は単一 in/out audio bus（`HostInput`/`HostOutput` slice 長 1・`AudioBusBuffers` bus index 0 のみ — fixture の `getBusCount` 形状と一致）を両規格で要求と pin。複数 bus 入力は v1 defined 境界（fixture が要求しない面 — deferred 記録と整合）
- それ以外の Open Questions なし — design が discovery の全論点（crate 配置・binding・fixture・E2E・lane・依存・`unsafe` 方針）を確定済み

## Self-Review

- [x] Every spec requirement maps to a task（8 AC → 5 task + 3 test-spec の写像を Verification に記録）
- [x] No placeholders remain（crate/feature/dep version・API 名・fixture descriptor 値・CID hex 両 platform 値・mise コマンド・CI job 名は確定値のみ — 未確定点は Open Questions で明示）
- [x] File paths, relations, status values, and terminology are consistent（`ClapLibrary`/`ClapPluginFactory`/`ClapPlugin`/`ClapHostError`・`Vst3Library`/`Vst3PluginFactory`/`Vst3Plugin`/`Vst3HostError`・`host` feature・`required-features`・`RUVIFORM_L1_REAL_PLUGIN_DIR`・`PluginEntry::load`/`PluginInstance::new`/`PluginAudioConfiguration`・`GetPluginFactory`/`IPluginFactory3`/`IComponent`/`IAudioProcessor`/`IEditController`/`IHostApplication`/`IComponentHandler`/`IParameterChanges`/`IParamValueQueue`/`setActive`/`setupProcessing`/`setProcessing`/`kRealtime`/`kSample32`・`OutOfProcessSpec`/`PluginHost`/`kill_child`/`InstanceFailed`/`HostError::Unsupported`/`Err(Ipc)`・`test-l1-real-plugin`/`lint-l1-real-plugin`/`l1-real-verify` — design・実コードの正本どおり）
- [x] pinned mechanism の owning task 重複なし — bridge 実装は T1（CLAP）/T2（VST3）で crate 別分割、runner/E2E/lane は T3/T4 が共有形を担当

## Implementation Handoff

- Recommended mode: delegated 実行 + lead の task 完了ごとの実レビュー（ユーザ承認済みを前提 — 環境依存の skill ID は本計画に埋め込まず、実環境で発見される capability に routing する）
- Independent work streams: **Task 1（CLAP bridge）と Task 2（VST3 bridge）は別 crate で論理上並行実装可能**（`Cargo.toml`/`src/host.rs`/`src/lib.rs` が別ファイル群 — `Cargo.lock` 更新のみ共有点・先に land した側を基準に後続が rebase で吸収）。Task 3 は両者の bridge API を消費するため両完了を待つ（`depends-on` edge 宣言済み）。Task 4/5 は直列
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- レビュー焦点: CLAP lifecycle 写像（`create()` → `None` instance・`init()` → `PluginInstance::new` — 中立 `Initialize` が規格 instantiate+init を駆動する順序）と `Stopped`/`Started` processor の状態保持（`process_host` は `Started` のみ・`stop_processing` → `Stopped` に戻す対称形）
- `HostHandlers::declare_extensions` が `HostThreadCheck` を登録しない（fixture の `None` fallback 経路 — 不実の `is_audio_thread` 応答をしない — plugin.rs:130・421 の graceful fallback 確認）
- VST3 bridge の呼出列（`createInstance` component → QI `IComponent`/`IAudioProcessor` → `getControllerClassId` → `createInstance` controller → 両 `initialize(context)` → `setComponentHandler` → `setActive(true)` → `setupProcessing` → `setProcessing(true)` → `process` → `setProcessing(false)` → `setActive(false)` → `terminate`）が fixture の必須順序と整合するか（`setupProcessing` は `Activated` 要求 — component.rs:597）・COM host objects（`IHostApplication`/`IComponentHandler`/`IParameterChanges`/`IParamValueQueue`）の `Class`/`ComWrapper` 実装が `TestQueue`/`TestChanges` 先例と同型か
- **`uid()` platform-order の CID hex** — bridge が raw 16byte を小文字 hex で emit し、expected id が `vst3::uid(0x5A1B2C3D, 0x4E5F6A7B, 0x8C9D0E1F, 0x2A3B4C5D)`/`GainComponent::CID` bytes からの **derive-in-test** で pin されているか（`#[cfg]`-conditional literal は regression pin のみ — Win `3d2c1b5a5f4e7b6a8c9d0e1f2a3b4c5d`・非 Win `5a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d`） — `id` が `--ruviform-plugin-id` の照合値として spec↔child で一致するか
- descriptor/param の intern-once（`String::leak`/`Vec::leak`）が load/factory 構築時の 1 回限り・bounded で、plugin-core の「no forged statics」方針（descriptor.rs `PluginDescriptorView` 節・wire.rs `Descriptor` コメント）に対する sanctioned exception 4 性質（bounded・once-per-load・process-scoped・feature 封じ）を保つか
- `ClapHostError`/`Vst3HostError` が `#[non_exhaustive]`・`unsafe` が host.rs 内 scoped `#[expect(unsafe_code)]` + SAFETY（OFX `ffi/loader.rs` 先例）の範囲に留まるか
- runner が `spec.args` を使わず fixture path を `RUVIFORM_L1_REAL_PLUGIN_DIR`/`current_exe` sibling でのみ解決するか・`run` argv 規約（`--ruviform-shm-key` 必須・`--ruviform-plugin-id`・未知引数 usage reject → exit 2）を拡張しないか・load 失敗が `exit(1)` pre-Hello → spawn `Err(Ipc)` で収束するか
- E2E が `env!("CARGO_BIN_EXE_<runner>")` same-crate 規約・`#![cfg(feature = "host")]`・`#[ignore]` ゲート・`spec.args` 空・const `PluginDescriptor` pin（`version` が CLAP `0.1.0`/VST3 `0.0.0` で混同なし・features 順序 `[audio-effect, stereo]`/`[Fx]`）・ParamValue 係数（CLAP `2v`・VST3 `v`）・`PluginHost::kill_child` 異常系・binary 不在 `Err(Ipc)` の fail 設計（silent skip なし）を守るか
- `mise.toml` の 2 task が design 記載の run 配列と一致し、ci.yml `l1-real-verify` job が `icc-verify` 同型 steps + 3-OS matrix で別 workflow ファイルを作らないか・`AGENTS.md` task 表との整合
