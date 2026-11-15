# RF630 CLAP-slice 実施 report — CLAP `test_gain` test plugin (`ruviform-plugin-test-clap`)

- Issue: https://github.com/scarecrow173/ruviform/issues/630 (CLAP slice: `test_gain` のみ)
- Branch: `feat/630-clap-test-plugin`(worktree `G:\orca\workspaces\ruviform\rf630-clap-test-plugin`)
- Base: main(直近 merge は #656 系)

## 実施内容(本 slice)

| 区分 | 内容 |
| --- | --- |
| 新規 crate | `crates/ruviform-plugin-test-clap` — `[lib] crate-type = ["cdylib", "rlib"]`、test-only 宣言を crate README / lib docs に明記 |
| Entry point | `clack_export_entry!(SinglePluginEntry<TestGainClapPlugin>)`(`src/lib.rs`)— macro が生成コードに scoped `#[allow(unsafe_code)]` を出す先例踏襲、crate 内に `unsafe` なし |
| Descriptor | CLAP id `org.ruviform-test.clap-test-gain`、name `test_gain`、vendor `"Ruviform Test"`、version `0.0.0`、features `audio-effect` + `stereo` — adapter fixture(`org.ruviform.clap-gain-poc`/`"Ruviform"`)と別 identity で同一 scan に同居可能 |
| Ports | `clap.ext.audio-ports`: stereo main in/out、`in_place_pair` 宣言 |
| Params | `clap.ext.params`: id 0 `Gain`、plain `[0.0, 2.0]` default `1.0`、automatable、dimensionless。normalized↔plain は非恒等(`plain = n * 2`) |
| DSP | `out[c][n] = in[c][n] * gain(n)` — `src/gain.rs` の独立 `TestGainProcessor`(neutral `PluginProcessor`)。in-place block は共有領域を in-place scale(aliased `&`/`&mut` を構築しない) |
| Events | `src/block.rs` が `InputEvents` の `ParamValueEvent`(plain)→ normalized `ParamValue` へ frame-offset 昇順・same-offset last-wins・zero-frame flush で変換。host 側 `denormalize`(`host.rs` `push_events`)との往復で両側 mapping を検証 |
| Lifecycle | `src/plugin.rs` が clack `Plugin`/`PluginShared`/`PluginMainThread`/`PluginAudioProcessor` + `PluginAudioPorts` + `PluginParams`/`PluginAudioProcessorParams` を実装し neutral `LifecycleState` に写像。protocol violation(process before start 等)は reject、silent repair なし |
| RT | `process` は alloc/block/wait/IO なし。scratch(`BlockScratch`/`EventBuffer`)は `activate` で確保、音声は host buffer への zero-copy `PlanarBufferView`、同期は `AtomicU64` mirror のみ |
| 独立実装 | plugin 側実装は `clack-* =0.2.0` binding + neutral contracts 上に直接実装。`ruviform-plugin-clap` internals は不使用。共有は binding 層のみ(Cargo.toml コメント・design doc に記録)。`ruviform-plugin-clap` は dev-dep(`features = ["host"]`)edge のみで adapter 本体は pristine |

## 実機実行証拠(Windows)

実行 tuple:

| 項目 | 値 |
| --- | --- |
| TestID | rf630-clap-test-gain-win |
| OS | Windows(x64、Git Bash 経由の mise/cargo) |
| toolchain | mise pin の rust(1.99.0 line)、edition 2024 |
| format | CLAP cdylib `ruviform_plugin_test_clap.dll`(`target/debug/deps/` に生成、`tests/host_path.rs` が `current_exe`→`deps/` 相対で解決、missing は assert fail) |
| plugin binary/version | `test_gain` 0.0.0 / vendor `Ruviform Test` / id `org.ruviform-test.clap-test-gain` |
| process boundary | L0 in-process(`ClapLibrary::load` → dlopen → `clap_entry` → factories → `ErasedPlugin`)。L1 out-of-process は NotRun(後続 slice) |
| command | `mise run test ruviform-plugin-test-clap` |
| exit | 0 |

観測された実効果(`tests/host_path.rs` 3 test 全 pass):

- `discovers_test_gain_through_production_host` — `ClapLibrary::load` が実 dll を scan し `descriptors()` が `test_gain`/`Ruviform Test`/`0.0.0`/`audio-effect` を 1 件報告
- `reports_gain_param_in_plain_domain` — production `params()` 経路が plain `[0.0, 2.0]` default `1.0` を報告(非恒等 mapping が本番 host path で実効)
- `processes_audio_with_sample_accurate_gain_events` — golden PCM(`0.5`/`-0.25` stereo × 64 frame)に normalized `0.75` event @ offset 8 を投入 → frame 0–7 unity、frame 8–63 plain `1.5`(`0.75`/`-0.375`)。event なし 2 block 目は gain persist で全 frame ×1.5。drop で deactivate/terminate teardown が走る

unit 側(`src/plugin.rs` `#[cfg(test)]`、clack-host fixture 経由・plugin 側 mock なし): lifecycle 写像 happy path / activate failure→Initialized / process-before-start reject、process law(offset 適用・in-place・zero-frame flush・Sleep)、`rt_process_drives_paired_stereo_block_allocation_free`(`assert_no_alloc`)、`host_driven_entry_runs_full_lifecycle_through_abi`(`clap_entry`→`PluginInstance` 実 ABI 経路)、`params_extension_reports_and_flushes_gain`。

## 検証(実 command/exit)

| command | 結果 |
| --- | --- |
| `mise run fmt-check` | exit 0 |
| `mise run check ruviform-plugin-test-clap` | exit 0 |
| `mise run lint ruviform-plugin-test-clap` | exit 0(`clippy -- -D warnings` clean) |
| `mise run test ruviform-plugin-test-clap` | exit 0 — unit 21 + host_path 3 全 pass |
| `mise run build ruviform-plugin-test-clap` | exit 0(cdylib + rlib) |
| `mise run test-rt`(`cargo test --workspace rt_`) | exit 0 — 本 crate の `rt_process_drives_paired_stereo_block_allocation_free` が name filter で収集・実行・pass を確認(テスト名で実収集を証跡化) |
| `mise exec -- prek run --all-files` | exit 0(下記「prek」参照) |

補助証拠(canonical = mise、cargo 直接実行は反復用): `cargo check -p ruviform-plugin-test-clap --all-targets` 同等結果。

## 未検証(NotRun / Unsupported)

| 項目 | 状態 | 理由・後続 |
| --- | --- | --- |
| `test_stateful`(State Save/Reload/Preset) | NotRun | **本番 VST3/CLAP 両 host bridge が `save_state`/`restore_state` 未実装 = adapter 側の機能 gap**。別 Issue 事項として記録、adapter 側補完は本 slice 範囲外(必要なら Issue 起票を検討) |
| `test_instrument`(Tempo/Loop/MIDI) | NotRun | 後続 plugin class slice |
| `test_fault`(Crash/Hang/Timeout/Isolation) | NotRun | L1 runner slice 依存 |
| `test_editor`(Native Editor) | NotRun | GUI-L2、`scarecrow173/ruviform#631` 連携 |
| L1 out-of-process runner 経路 | NotRun | `ruviform-plugin-export`/`plugin-host` L1 経路との結合は後続 slice |
| 外部 SDK validator(clap-validator 等) | NotRun/Deferred | license・version 記録と共に後続 slice で計画。外部権威なしに ABI 適合を主張しない |
| macOS/Linux build・実走 | NotRun | 本 slice は Windows OS-L1 のみ |
| GUI-L2/EXT-L3、consumer oracle(`scarecrow173/synvelis#101`/`#127`/`#128`/`#176`、`scarecrow173/audio-lens#127`)照合 | NotRun | Issue 受入の残項目、後続 slice |
| `ruviform-audio-test`(#628)runtime 統合 | NotRun | 依存 edge ゼロで独立実装 |
| MSRV lane(`*-msrv`) | NotRun | ユーザー指定で本 slice 対象外 |
| `*-full` lane | NotRun | 日常検証は変更範囲比例、Full CI に委譲 |

## 独立実装 cross-check の記録

- plugin 側実装(descriptor/lifecycle/params/ports/DSP)は `clack-* =0.2.0` binding + `ruviform-plugin-core`/`ruviform-audio-core` neutral contracts 上に直接実装。`ruviform-plugin-clap` の `pub(crate)` internals は不使用。
- 両者が共有するのは `clack-*` crate 群(binding 層 = SDK 相当の共通層)と neutral contracts のみ — `docs/designs/issue-630-test-plugin-suite.md`・`docs/designs/issue-630-clap-test-plugin.md` に記録。
- host 側検証は `ruviform-plugin-clap` の production `ClapLibrary` bridge(dev-dep `host` feature)経由で実施 — Fake Plugin Port 非経由。adapter 本体ソースは変更なし。
- `clack-host`(dev-dep)の safe fixture は plugin 側 ABI を in-process で駆動する unit/lifecycle/RT テスト専用。本番 host 検証の主証拠は `tests/host_path.rs`。

## test-only / 配布境界

- 本 crate は test fixture であり出荷物ではない。production crate は本クレートに依存しない(依存方向は本クレート → neutral/audio-core + dev-dep adapter のみ)。
- cdylib 成果物は `target/` 配下の test fixture であり、製品配布物・AudioLens への新依存を強制しない。
