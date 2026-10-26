---
id: "IMPL-034MRyrqJRqI9iIjgk5hIX"
type: "impl"
status: "completed"
title: "Issue #114 CLAP host bridge — Task 1 implementation"
created: "2026-10-27"
updated: "2026-10-27"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-clap/src/host.rs"
      - type: "file"
        path: "docs/impl/ir/issue-114-clap-host-bridge-task-1-implementation.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-clap/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-clap/src/lib.rs"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "docs/tasks/clap-host-bridge.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/clap-host-bridge.md"
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

# Issue #114 CLAP host bridge — Task 1 implementation

## Summary

Issue #114 Task 1 完了 — plan actions 1–4（CLAP host bridge 全
範囲）を実施。`ruviform-plugin-clap` に feature `host` で
feature-sealed な host-side bridge `src/host.rs`（+1,377 行）を
新設し、`clack-host` で実 `.clap` cdylib を dlopen・scan・lifecycle
写像して plugin-core の中立契約 `PluginFactory`/`Plugin` へ載せた。

- `Cargo.toml`: `[features] host = ["dep:clack-host",
  "dep:ruviform-plugin-export", "clack-extensions/clack-host"]`
  （`clack-extensions` は非 optional 既存 prod dep — weak
  `dep?/feat` 構文は optional dep 専用のため plain `dep/feat`
  構文・comment 明記）+ `clack-host = { version = "=0.2.0",
  optional = true }`（dev-dep から optional prod 昇格 — 両 entry
  で feature union）+ `ruviform-plugin-export`（path・optional —
  Task 3 runner が `run()` を消費）。
- `src/lib.rs`: `#[cfg(feature = "host")] pub mod host;` + crate doc
  に host bridge 節 + `public_surface_is_binding_neutral` 内に
  `ClapHostError: Error + Send + Sync` の compile-time bound
  pin（`#[cfg(feature = "host")]` block）。
- `src/host.rs`: `ClapLibrary`（`PluginEntry` wrap — load 時に
  descriptor scan + param 表 eager 列挙を 1 回だけ実施・
  intern-once `String::leak`/`Vec::leak` で `'static` 実体化）・
  `descriptors()`/`factories()`・`ClapPluginFactory`（cached
  descriptor + `create()` — `create_sendable` は `PluginInstance`
  `!Send` のため default `None` 維持）・`ClapPlugin`（dormant →
  `init`=`PluginInstance::new`（CLAP instantiate+`plugin.init`
  一体）→ `prepare`=`activate` + RT 用 buffer 一括確保 →
  `start_processing`/`stop_processing`=`Stopped`↔`Started` 遷移 →
  `process`=`StartedPluginAudioProcessor::process` → `Drop`
  駆動対称 teardown）・`ClapHostError`（`#[non_exhaustive]` —
  `Entry`/`Instance` は `Box<dyn Error + Send + Sync>` の opaque
  payload）・`BridgeHandlers`（`declare_extensions` で
  `HostThreadCheck` 非登録 — fixture の `None` fallback 維持）・
  `fixture_lib_name`（T3 runner/T4 E2E 共有の fixture 解決
  helper）+ `#[cfg(all(test, feature = "host"))]` unit test 17 件。

敵対的レビュー 1 ラウンド適用 — 確定修正 7 件（OutputEvents
RT-alloc hole・opaque error payload・prepare ordering・min_frames
floor・load-hoisted param tables・count caps・rt_ test）を全て
実装に反映（「Review findings and resolutions」参照）。全 gate
green 実績。

## Context

approved plan `docs/plans/implement-issue-114-real-adapter-l1-e2e.md`
の Task 1（`docs/tasks/clap-host-bridge.md`・actions 1–4）。
upstream: approved design
`docs/designs/issue-114-real-adapter-l1-e2e.md`（決定サマリ c/d —
「CLAP bridge」「'static メタデータ実体化」「依存追加の判断」）・
spec `docs/specs/issue-114-real-adapter-l1-e2e.md`（proposed・
AC1・AC2・AC7）・ADR
`docs/adr/plugin-architecture-clap-vst3.md`（adapter 責務・bridge
配置）・`docs/adr/plugin-binding-method-clack-for-clap-coupler-rs-vst3-for-vst3.md`
（clack 採用根拠）・TSpec
`docs/test-specs/adapter-host-bridge-contract-tests.md`（`verified-by`
edge）。本 task は Task 3（規格別 runner bin — bridge が
`factories()`/`fixture_lib_name` を供給）の CLAP 側前提であり、
Task 2（VST3 bridge）とは別 crate で構造的に独立。

## Implementation

### Changes

- `crates/ruviform-plugin-clap/Cargo.toml`（+19 行）: `[features]
  host` 節新設 + `clack-host`/`ruviform-plugin-export` optional dep
  追加。`clack-extensions` は既存非 optional のまま plain
  `dep/feat` 構文で `clack-host` feature を有効化。`clack-host`
  昇格理由・union・Task 3 runner 前提を comment で記録。
- `crates/ruviform-plugin-clap/src/lib.rs`（+21/-? 行）: `pub mod
  host;` を `mod adapt/gain` と `mod plugin` の間に `#[cfg(feature
  = "host")]` 付きで配置 + crate doc に「Host bridge (`feature =
  "host"`)」節（中立契約写像・feature-sealed・unsafe 境界 1 箇所
  の明示）+ `ClapHostError` の `Error + Send + Sync` bound pin。
- `crates/ruviform-plugin-clap/src/host.rs`（新規・1,377 行）:
  - **`ClapLibrary`** — `PluginEntry::load`（crate 唯一の unsafe
    境界・scoped `#[expect(unsafe_code)]` + SAFETY）→
    `from_entry` で load 時 scan を 1 回実施:
    `get_plugin_factory()` → `plugin_descriptors()` を
    `MAX_SCAN_DESCRIPTORS` で clamp した iterator で列挙 → 各
    `clap_plugin_descriptor` の id/name/vendor/version/features
    （宣言順保持）を `intern_descriptor` で intern-once →
    `plugin_ids: Box<[CString]>`（instantiate 用 id を index
    aligned で保持）+ `descriptors: Vec<PluginDescriptor>` +
    `params_tables: Vec<&'static [ParamInfo]>`（後述の eager 列挙）。
    `descriptors()` は cached slice 返却（同一 pointer —
    `descriptors_are_cached_across_calls` が ptr-eq で pin）。
  - **`enumerate_params`** — params discovery は instance
    extension のため live instance が必須 → load 時に descriptor
    毎 1 回 throwaway `PluginInstance::new` で `PluginParams` を
    列挙し intern 済み表を返す（`params()` が pre-`init` を含む
    全 lifecycle で呼べる契約を成立させる load-hoisted 構造）。
    instantiate 不能・extension 不在は honest な空表 — 実失敗は
    `init` で表面化。`unit` は CLAP `param_info` に field が無い
    ため `""`（honest synthetic — filler ではない）。`count` は
    `MAX_SCAN_PARAMS` clamp 後に予約・scan。
  - **`ClapPluginFactory`** — `PluginFactory` impl。
    `descriptor()`=cached 値・`create()`=dormant `ClapPlugin` を
    `erase()`。`entry: PluginEntry`（clack 内部で Arc-backed —
    factory/instance が clone する限り module は mapped）。
  - **`ClapPlugin`** — `Plugin`/`PluginProcessor` impl:
    `init`=`PluginInstance::new`（fused
    instantiate+`plugin.init`・2 回目は `HostError::PluginFailed`）。
    `prepare`=`activate` — 引数検査（`min_frames==0`/
    `max_frames==0`/`max_frames>WIRE_MAX_FRAMES` →
    `PrepareError::UnsupportedConfig` — clack の
    `PluginAudioConfiguration::validate` panic を未然防止）→
    instance 不在 or activated/processing 中 →
    `PrepareError::ActivationFailed`（**port 照会より先** —
    `audio-ports` `get`/`count` は main-thread-only で processing
    中の呼出禁止）→ `port_channel_counts` で declared topology
    照会（`MAX_SCAN_PORTS`/`MAX_SCAN_CHANNELS` clamp）→
    `activate(|_,()| BridgeAudioProcessor,
    PluginAudioConfiguration{sample_rate, min_frames_count,
    max_frames_count})` → 戻り値 `StoppedPluginAudioProcessor` を
    `processor` field に格納 → input/output scratch
    （`declared channels × max_frames`）・`AudioPorts`・
    `EventBuffer`（`MAX_EVENTS_PER_BLOCK=256` capacity）を一括
    確保 — `process` が alloc しない構造。
    `start_processing`/`stop_processing` は `Stopped`↔`Started`
    遷移（非活性/未 start は `HostError::PluginFailed`）。
    `process` — `Started` 以外は `ProcessStatus::Error` defined
    拒否 → `frames ∉ [min_frames, max_frames]` 拒否（negotiated
    must-bound・`frames==0` は event offset 全 drop も未然防止）→
    `channels_match_declared` で equal-length 契約検査（wired
    `Some` channel の slice 長 ≠ declared は caller bug — silent
    clamp せず `Error`）→ input bounce（`HostInput` `&[f32]` →
    owned scratch — `*mut` 偽装の const-cast 手続きは UB-adjacent
    で rejected・unconnected channel は silence）→
    `with_input_buffers`/`with_output_buffers` を persistent
    `AudioPorts` 上に組立 → `push_events`（
    `EventKind::ParamValue` normalized → `ParamValueEvent`
    plain/denormalized — unknown id・非 param kind・out-of-block
    offset・cap 超過は defined drop・buffer は realloc しない）→
    plugin output events は **`OutputEvents::void()`** sink へ
    （v1 不読 — plugin が host `Vec` を RT 経路で肥大化させる
    hole を閉じる）→ `process(...)` arity 6・`steady_time=None`・
    `transport=None`（`TransportInfo` snapshot から beat-domain
    loop/tempo field を honest に導出不能 — tempo map 非保持の
    ため 0-stuff 半投影より明示 `None` の defined boundary）→
    `bounce_output` で host 側へ copy-back → `ProcessStatus`
    写像（`Continue`/`ContinueIfNotQuiet`→Continue・`Tail`→
    Continue+`TAIL_INFINITE`・`Sleep`→SLEEP・`Err`→Error）。
    `Drop` — best-effort 対称 teardown: `processor.take()` →
    `into_stopped()` → `instance.is_active()` なら
    `deactivate(stopped)` → instance drop で `plugin.destroy`
    （entry が module を mapped に保持）。
  - **`ClapHostError`** — `#[non_exhaustive]`。`Entry`/`Instance`
    は erased `Box<dyn Error + Send + Sync + 'static>`（
    `Display`/`source()` で診断は残すが公開 signature に binding
    crate 型を出さない crate policy — `From<PluginEntryError>`
    impl も公開せず crate 側 `fn entry` で変換）。
    `MissingPluginFactory`/`MissingPluginId` で scan 異常を defined
    化。
  - **`BridgeHandlers`** — `HostHandlers` impl（`BridgeShared`=
    `[thread-safe]` acknowledge-only・`()`=main・
    `BridgeAudioProcessor`=`[audio-thread]` Send marker）。
    `declare_extensions` は `HostThreadCheck` を**登録しない** —
    runner は単一 command thread で `process` を駆動し RT audio
    thread ではないため、`is_audio_thread=true` 応答は不実の
    capability claim。fixture は `plugin.init` で extension を照会
    し不在なら check 無しへ fallback（`plugin.rs`
    `GainShared::initialize_with_host`）— 挙動不変。
  - **intern-once helpers** — `intern_str`/`intern_cstr`/
    `intern_bytes`（空入力は `'static` literal 返却で leak すら
    回避）+ `map_param_flags`（CLAP-only flag — per-channel/
    key/note/port variants・`IS_PERIODIC`/`IS_ENUM`/
    `REQUIRES_PROCESS` — は中立 counterpart 無しで drop）+
    `port_channel_counts`（extension 不在は空 — stereo 以外も
    declared topology で scratch が追従）。
  - **`fixture_lib_name`** — `{DLL_PREFIX}ruviform_plugin_clap
    {DLL_SUFFIX}` の `current_exe`-relative 命名規約を公開（T3
    runner・T4 E2E 共有前提）。
  - **unit test 17 件**（`#[cfg(all(test, feature = "host"))]`・
    in-process dlopen — fixture は `current_exe` sibling
    `deps/{…}` 解決・欠落は `assert!` fail）: descriptor 既知値
    （id `org.ruviform.clap-gain-poc`・name `Ruviform Gain PoC`・
    vendor `Ruviform`・version `env!("CARGO_PKG_VERSION")`・
    features `[audio-effect, stereo]` 順序保持）・cached 同一性
    （ptr-eq）・missing path `Err(Entry)`・erased payload の
    `Display`+`source()` 生存・non-CLAP binary `Err`・factory
    cached descriptor + `create_sendable` `None`・params 表全
    lifecycle 同一（pre-`init` 含む）・lifecycle roundtrip
    passthrough block（gain 1.0 verbatim）・param event
    denormalized 到達（normalized 0.25→plain 0.5）・prepare-
    while-processing `ActivationFailed`・`max_frames >
    WIRE_MAX_FRAMES` `UnsupportedConfig`・`frames < min`/`> max`
    defined `Error`・channel length mismatch `Error`・
    **`rt_process_drives_fixture_block_allocation_free`**
    （`assert_no_alloc` 測定窓 — bounce/`with_*_buffers`/
    `push_events`/`OutputEvents::void` が窓内 non-alloc・
    warm-up 1 block 先行）・`process` before start `Error`・
    state/GUI verbs `Err(Unsupported)` default。

### Review findings and resolutions（敵対的レビュー 1 ラウンド — 確定修正 7 件全適用）

- **`OutputEvents::void()` の RT-alloc hole（fixed）**: 初版は
  plugin output events を Vec-backed `EventBuffer` で受けていた
  — plugin が `try_push` を連打すると capacity を超えて RT 経路で
  realloc し得る構造。`OutputEvents::void()` sink に変更し
  「v1 では output event を読まない」defined boundary と
  「plugin が host allocation を強制できない」RT 不変条件を同時に
  満たす形へ（`rt_` test の測定窓対象）。
- **opaque error payload（fixed）**: `ClapHostError` が
  `PluginEntryError`/`PluginInstanceError` の typed payload を持つ
  設計だった → crate policy（公開 API に binding crate 型を出さ
  ない）に反するため `Box<dyn Error + Send + Sync + 'static>`
  へ erase。`From` impl も公開せず private `fn entry` で変換 —
  `Display`/`source()` で診断は保全。lib.rs test に `Error +
  Send + Sync` の compile-time bound pin を追加。
- **prepare ordering（fixed）**: `prepare` の activated/
  processing 中拒否が `audio-ports` extension 照会の後にあった
  — `get`/`count` は main-thread-only で processing 中の呼出は
  CLAP 契約違反。拒否判定を port 照会より先に移動（test
  `prepare_while_processing_is_defined_error` が activated+started
  状態での `Err(ActivationFailed)` を pin）。
- **`min_frames` floor（fixed）**: `prepare` が `max_frames` 系の
  上界のみ検査していた → `min_frames==0`/`max_frames==0` は
  `PluginAudioConfiguration::validate` が panic する値域のため
  `PrepareError::UnsupportedConfig` で拒否 + `process` 側は
  `frames ∉ [min,max]` を拒否（`frames==0` は must-bound 違反に
  加え event offset 全 drop の副作用を持つ）。
- **load-hoisted param tables（fixed）**: 初版は `params()` を
  instance 経由の lazy 列挙としていたが、中立 `params()` は
  pre-`init` を含む全 lifecycle で呼べる必要がある → load 時に
  descriptor 毎 throwaway `PluginInstance` で eager 列挙し
  intern 済み `&'static [ParamInfo]` を `params_tables` cache に
  保持（`factories()`/`create()` は cache 読取りのみ — 再
  instantiate・追加 leak なし）。test が pre-`init`〜activated〜
  stopped の全地点で同一表を pin。
- **count caps（fixed）**: plugin-reported な count（factory の
  descriptor 数・`params.count`・`audio-ports` `count`/
  `channel_count`）が garbage 値を返すと `Vec::with_capacity`/
  `vec![…]` が abort し得る → `MAX_SCAN_DESCRIPTORS=1_024`・
  `MAX_SCAN_PARAMS=4_096`・`MAX_SCAN_PORTS=64`・
  `MAX_SCAN_CHANNELS=64` で scan loop・allocation 双方の前に
  clamp（magnitude は honest plugin より十分大きく、超過分は wire
  protocol の `WIRE_MAX_*` ceiling と同じ truncation 意味論）。
- **`rt_` test（fixed）**: Issue #22 RT 検証レーンの `rt_`
  prefix 契約に沿い `rt_process_drives_fixture_block_allocation_
  free` を追加 — `assert_no_alloc` 窓内で prepared plugin の
  `process_host` を駆動（warm-up block を窓外に置き loaded-plugin/
  clack の first-touch init を除外。`plugin.rs` test module の
  `AllocDisabler` global allocator が binary 全体を覆う）。

### Decisions During Implementation

- **output bounce を採用（deviation・deliberate）** — task doc は
  「`HostOutput.channel_mut` の `&mut [f32]` を `OutputAudioBuffers`
  に直接指させてよい」としたが、中立 `process_host` の outputs は
  `&mut dyn BufferViewMut` で 1 live borrow につき 1 `&mut [f32]`
  しか yield しない — 複数 channel を同時に指すには raw-pointer
  組立（追加 `unsafe`）が必要になる。bridge は unsafe 境界を
  `PluginEntry::load` 1 箇所に保つ方針のため、output も input と
  対称に owned scratch 経由で copy-back する bounce に変更
  （unconnected output channel は defined drop）。VST3 側は
  `channelBuffers32` への直接指しが task で明示許容されており
  非対称維持 — CLAP 側のこの差分は設計正本（design 決定サマリ）
  への同期対象として Follow-ups に記録。
- **`plugin_ids` を `ClapLibrary` に index-aligned 保持** —
  `factories()` 呼出し毎に id を再取得しない（factory 生成時は
  `CString::clone` のみ）。`entry`（clack 内部 Arc-backed）を
  factory/instance が clone するため module unmap は最後の
  instance drop まで発生しない — OFX `OfxLibrary::share` と同型
  の lifetime 保証。
- **`enumerate_params` の失敗は空表** — instantiate 不能・
  `clap_plugin_params` 不在は honest な `&[]` を返し、実失敗は
  `init`（`PluginInstance::new` の再試行）で表面化させる。load
  時 scan で instantiate 失敗を `Err` にすると「load できるが
  instantiate できない plugin」を `descriptors()` からも消して
  しまい観測性を損なう — scan は scan、instantiate は
  instantiate の責務分離。
- **`channels_match_declared` の厳格 equal-length** — `HostInput`/
  `HostOutput` の wired `Some` channel で slice 長 ≠ declared
  block は caller bug。silent clamp/copy-short は観測不能な
  corruption を生むため `ProcessStatus::Error` の defined
  rejection（`None` channel は unconnected-pin の合法境界で
  silence/drop）。
- **`min_frames`/`max_frames` 両端の `0` 拒否** — task doc は
  `max_frames ≤ WIRE_MAX_FRAMES` のみ指定だったが clack の
  `validate` は 0 値も panic する。`UnsupportedConfig` で事前
  拒否し panic 面を `Err` 境界へ変換（`prepare` doc `# Errors`
  に明記）。
- **`fixture_lib_name` を `pub` 公開** — T3 runner・T4 E2E が
  同一命名規約を共有するため host module の公開面に配置（feature
  `host` 内に限定 — default 木への公開 API 増加なし）。

## Related Experiments

### Adopted

なし — approved design/plan の確定実装のみ（実験ログなし）。

### Rejected

なし — 代替実装の評価は design の確定事項（clack 採用・
intern-once・bounce 方針）をそのまま実装した。

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
  - `cargo test -p ruviform-plugin-clap --features host` —
    **63 unit test 全 pass**（bridge test 17 件 + 既存
    adapter/`load_from_clap` typed-entry test 無退行）
  - `cargo clippy -p ruviform-plugin-clap --all-targets --features
    host -- -D warnings` — exit 0（scoped `#[expect(unsafe_code)]`
    + SAFETY は `PluginEntry::load` 1 箇所のみ・0 warning）
  - `cargo doc` — 0 warning
- 構造確認（実コマンド）:
  - `cargo tree -p ruviform-plugin-clap` — `host` off で
    `clack-host`/`libloading`/`ruviform-plugin-export` が
    non-optional dep 木に非出現・`clack-extensions` は既存 non-
    optional のまま
  - `cargo tree -p ruviform-plugin-clap --features host` —
    `clack-host`/`ruviform-plugin-export`/`clack-extensions
    clack-host` が `host` 指定時のみ出現
- 対象 test-spec:
  `docs/test-specs/adapter-host-bridge-contract-tests.md`
  （`verified-by` edge — descriptor 既知値・cached 同一性・
  params 全 lifecycle・error 写像・fixture 欠落 `assert!` 規則を
  担保）

## Non-Goals

- **規格別 runner bin（Task 3）** — `[[bin]]
  ruviform-plugin-clap-l1-runner`（`required-features=["host"]`・
  `RUVIFORM_L1_REAL_PLUGIN_DIR` 解決）の実装は T3 scope。本 task
  は `factories()`/`fixture_lib_name` の供給のみ。
- **L1 E2E・lane 配線（Task 4）** — `tests/l1_real_adapter_e2e.rs`
  ・`test-l1-real-plugin`/`lint-l1-real-plugin` mise task・
  `l1-real-verify` ci job・`[dev-dependencies]
  ruviform-plugin-host` は T4 scope。
- **output event mapping** — plugin → host の event 逆方向写像は
  v1 定義境界外（`OutputEvents::void()` で defined discard）。
- **state/GUI verbs** — `save_state`/`restore_state`/`open_gui`/
  `close_gui` は中立 `Plugin` default の `Err(Unsupported)` 継承
  — CLAP 規格の state/preset/extension 呼出しは一切しない。
- **`steady_time`/`transport` 投影** — `TransportInfo` →
  `clap_event_transport` 半投影は tempo map 非保持により deferred
  （`None`/`None` defined boundary — design 決定サマリの明示）。
- **macOS `.clap` bundle 解決** — `PluginEntry::load` は macOS で
  bundle 解決を行うが fixture/runner 規約は raw `.dylib` —
  Windows/Linux で検証済み・macOS は未検証として `load` doc に
  明記（3-OS CI lane での実測は T4 lane 側の範囲）。

## Risks

- **in-process dlopen は任意コード実行** — `PluginEntry::load` は
  外部 shared object を同一プロセスに map する unsafe 境界（呼出
  し側の契約として SAFETY comment・`# Errors` doc に明記。実害
  範囲は L1 runner child process に限定 — 短命・exit 回収）。
- **intern-once leak は bounded だが不可逆** — descriptor 毎
  ~5+N strings・param 毎 2 strings が process 寿命まで解放され
  ない。once-per-load・process-scoped・feature-sealed の sanctioned
  exception として module doc に方針明記（`descriptors()`/
  `params()` の per-call leak growth は cache 返却で構造的に
  排除）。
- **throwaway `PluginInstance` が load 時に plugin code を実行**
  — param 列挙のため descriptor 毎に instantiate+`plugin.init`+
  drop が走る。plugin 側 init が重い・副作用を持つ場合 load が
  遅延・副作用を受ける（scan 1 回 bounded・instantiate 不能は空
  表で退避 — 実害は `init` 時に再表面化）。
- **plugin-reported count の clamp は truncate** — `MAX_SCAN_*`
  を超える honest plugin（実質非存在）は先頭 N 件のみ採用され、
  残りは列挙されない（wire protocol の `WIRE_MAX_*` ceiling と
  同じ truncation 意味論 — reject ではなく defined cap）。
- **macOS bundle 解決パス未検証** — `PluginEntry::load` の macOS
  bundle 規約と fixture `.dylib` 直接解約の齟齬は `load` doc に
  明記済み（3-OS lane の l1-real-verify で検出される構造）。

## Follow-ups

- Task 3 `docs/tasks/real-adapter-runner-bins.md` —
  `factories()`/`fixture_lib_name`/`#[feature = "host"]` 公開面を
  消費する最初の下游（`depends-on` edge 宣言済み）。
- Task 4 `docs/tasks/real-adapter-e2e-and-lane.md` — bridge unit
  test 層（`--lib`）+ E2E（`--ignored` gate）+ lane mise task +
  `l1-real-verify` job。
- **output bounce 差分の正本同期** — CLAP 側は task doc の
  「output 直接 alias 許容」から「`&mut dyn BufferViewMut` の単一
  borrow 制約により bounce」へ確定実装が変わった。design 決定
  サミングへの同期反映は closeout（Task 5・plan action 16 の
  design/spec 同期フェーズ）で実施。
- **`l1_proxy_e2e` flake の継続観測** — Issue #113 で記録済みの
  `wedged_child_write_converges_and_teardown_is_bounded` /
  `rejected_block_replies_resolve_pass_through` の spawn/handshake
  競合 flake は本 task 非起因・既存リスクとして継続観測（再発時
  は handshake timeout または test 直列化で別途 triage）。
