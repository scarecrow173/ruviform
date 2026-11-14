# RF630 first-slice 実施 report — VST3 `test_gain` test plugin (`ruviform-plugin-test-vst3`)

- Issue: https://github.com/scarecrow173/ruviform/issues/630 (first slice: VST3 `test_gain` のみ)
- Branch: `feat/630-vst3-test-plugin`(worktree 起点 `scarecrow173/rf630-vst3-test-plugin`)
- Base: `77ba02e9`(docs(issue-532) GPUI DComp 監査 merge 後の main)

## 実施内容(本 slice)

| 区分 | 内容 |
| --- | --- |
| 新規 crate | `crates/ruviform-plugin-test-vst3` — `[lib] crate-type = ["cdylib", "rlib"]`、test-only 宣言を crate README / lib docs に明記 |
| Entry points | `GetPluginFactory` + platform hooks(Win `InitDll`/`ExitDll`、Linux `ModuleEntry`/`ModuleExit`、macOS `bundleEntry`/`bundleExit` — trivially-true) |
| Factory | `IPluginFactory`/`2`/`3`、vendor `"Ruviform Test"`、2 class: `test_gain`(Audio Module Class)+ `test_gain Controller`(Component Controller Class)、adapter fixture と別 CID |
| Component | `IComponent` + `IAudioProcessor` + `IProcessContextRequirements`。stereo kMain in/out bus(mono も negotiate)、`kSample32` のみ、state stream は `kNotImplemented` |
| Controller | `IEditController`。param id 0 `Gain`、plain `[0.0, 2.0]` default `1.0`、normalized↔plain は非恒等(`plain = n * 2`) |
| DSP | `out = in * gain` per-channel、event frame offset で sample-accurate 適用、same-offset last-wins、zero-frame flush で次 block に persist |
| RT | scratch は `setupProcessing` で確保、`process` は alloc/block/IO なし。`IParameterChanges` scan は queue/point/総 budget で bounded、in-place alias は bounded bounce + writeback、partial overlap は reject |
| 独立実装 | `vst3` binding + neutral contracts 上に直接実装。`ruviform-plugin-vst3` 内部は不使用(pub(crate) で到達不能でもある)。dev-dep edge(`host` feature)のみで production host path 検証に使用 |

## 実機実行証拠(Windows)

実行 tuple:

| 項目 | 値 |
| --- | --- |
| TestID | rf630-vst3-test-gain-win |
| OS | Windows(x64、Git Bash 経由の mise/cargo) |
| toolchain | mise pin の rust(1.99.0 line)、edition 2024 |
| format | VST3 cdylib `ruviform_plugin_test_vst3.dll`(`target/debug/deps/` に生成、`tests/host_path.rs` が `current_exe` 相対で解決) |
| plugin binary/version | `test_gain` 0.0.0 / vendor `Ruviform Test` |
| process boundary | L0 in-process(`Vst3Library::load` → dlopen → COM)。L1 out-of-process は NotRun(後続 slice) |
| command | `mise run test ruviform-plugin-test-vst3` |
| exit | 0 |

観測された実効果(`tests/host_path.rs` 3 test 全 pass):

- `displays_test_gain_through_production_host` — `Vst3Library::load` が実 dll を scan し `descriptors()` が `test_gain`/`Ruviform Test`/`0.0.0`/`Fx` を 1 件報告(controller class は plugin descriptor から filter)
- `reports_gain_param_in_plain_domain` — distributed controller を `getControllerClassId` 経由で instantiate し、host が `normalizedParamToPlain` を通した plain domain `[0.0, 2.0]` default `1.0` を報告(非恒等 mapping が本番 host path で実効)
- `processes_audio_with_sample_accurate_gain_events` — golden PCM(`0.5`/`-0.25` stereo × 64 frame)に normalized `0.75` event @ offset 8 を投入 → frame 0–7 は unity、frame 8–63 は plain `1.5`(`0.75`/`-0.375`)。event なし 2 block 目は gain persist で全 frame ×1.5。drop で host の deactivate/terminate teardown が走る

## 検証(実 command/exit)

| command | 結果 |
| --- | --- |
| `mise run fmt-check` | exit 0 |
| `mise run check ruviform-plugin-test-vst3` | exit 0 |
| `mise run lint ruviform-plugin-test-vst3` | exit 0(`clippy -- -D warnings` clean) |
| `mise run test ruviform-plugin-test-vst3` | exit 0 — unit 44 + host_path 3 全 pass |
| `mise run build ruviform-plugin-test-vst3` | exit 0(cdylib + rlib) |
| `mise run test-rt`(`cargo test --workspace rt_`) | exit 0 — 本 crate の `rt_` test 3 件が収集・実行・pass: `block::tests::rt_prepare_finish_block_does_not_allocate`、`component::tests::rt_process_copies_gain_zero_copy`、`component::tests::rt_param_scan_stays_bounded_when_host_counts_lie` |

補助証拠(canonical = mise、cargo 直接実行は反復用): `cargo check`/`cargo test -p ruviform-plugin-test-vst3` 同等結果。

## 未検証(NotRun / Unsupported)

| 項目 | 状態 | 理由・後続 |
| --- | --- | --- |
| CLAP 形式 | NotRun | format 別 crate 分割は design で pin、実装は後続 slice |
| `test_instrument`/`test_stateful`/`test_fault`/`test_editor` | NotRun | first slice は `test_gain` のみ。MIDI/Tempo/Loop/State/Crash-Hang-Timeout-Isolation/Native Editor は後続 class |
| State Save/Reload/Preset | NotRun | 本 fixture は state surface を持たず `kNotImplemented` を返す。`test_stateful` で対応 |
| L1 out-of-process runner 経路 | NotRun | `ruviform-plugin-export`/`plugin-host` L1 経路との結合は後続 slice |
| 外部 SDK validator(VST3 SDK validator 等) | NotRun | license・version 記録と共に後続 slice で計画 |
| macOS/Linux build・module hook | NotRun | 本 slice は Windows OS-L1 のみ。cfg 経路は実装済みだが実機未検証 |
| GUI-L2/EXT-L3、consumer oracle(Synvelis/AudioLens)照合 | NotRun | Issue 受入の残項目、後続 slice |
| `ruviform-audio-test`(#628)runtime 統合 | NotRun | 未 merge・依存 edge ゼロで独立実装 |

## 独立実装 cross-check の記録

- plugin 側実装(component/controller/factory/DSP)は `vst3` binding + `ruviform-plugin-core`/`ruviform-audio-core` neutral contracts 上に直接実装。`ruviform-plugin-vst3` の `pub(crate)` internals は不使用・到達不能。
- 両者が共有するのは `vst3` crate(binding 層 = SDK 相当の共通層)のみ — `docs/designs/issue-630-test-plugin-suite.md` に記録。
- host 側検証は `ruviform-plugin-vst3` の production `Vst3Library` bridge(dev-dep `host` feature)経由で実施 — Fake Plugin Port 非経由。
