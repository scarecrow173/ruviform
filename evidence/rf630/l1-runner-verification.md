# RF630 L1-runner slice 実施 report — test plugin L1 out-of-process 経路

- Issue: https://github.com/scarecrow173/ruviform/issues/630 (L1 runner slice)
- Branch: `feat/630-l1-runner`(worktree `G:\orca\workspaces\ruviform\rf630-l1-runner`)
- Base: main(前提 slice: VST3 test_gain PR#641 / CLAP test_gain PR#657 `77070a0` merge 済)

## 実施内容(本 slice)

| 区分 | 内容 |
| --- | --- |
| `ruviform-plugin-test-vst3` | `l1-runner` feature(optional dep `ruviform-plugin-vst3`/`host` + `ruviform-plugin-export`、dev-dep `ruviform-plugin-host`)+ `[[bin]] ruviform-plugin-test-vst3-l1-runner`(`required-features`)+ `tests/l1_real_test_plugin_e2e.rs`(`#![cfg(feature="l1-runner")]` + `#[ignore]`×3) |
| `ruviform-plugin-test-clap` | 同型:`l1-runner` feature + `ruviform-plugin-test-clap-l1-runner` bin + `l1_real_test_plugin_e2e.rs` |
| Runner 動作 | adapter runner mirror — `RUVIFORM_L1_REAL_PLUGIN_DIR` → exe-dir で自 crate cdylib(`ruviform_plugin_test_{vst3,clap}.{dll,…}`)を解決 → `Vst3Library::load`/`ClapLibrary::load` → `factories()` → `ruviform_plugin_export::run`。解決/load 失敗は stderr + exit 1(Hello 前、parent は `Err(Ipc)`/`SpawnFailed` に収束、hang なし)。`OutOfProcessSpec::args` は空のまま |
| E2E oracle | VST3: `expected_id()` を literal `vst3::uid(0x6D4F_8A21,0x3B7C_9E05,0xF1A2_4C68,0xB9D3_7E51)` から test 時導出(`TestGainComponent::CID` は `pub(crate)`)+ platform byte-order literal pin。CLAP: literal descriptor `org.ruviform-test.clap-test-gain`/`test_gain`/`Ruviform Test`/`0.0.0`/`["audio-effect","stereo"]`。両者とも child の実 binary scan と独立 |
| Process law | 32 stereo shm block + `ParamValue(0.25)` → plain gain `0.5`(`[0.0,2.0]` 非恒等 domain)。delivered block は `0.5×` 先行 input の membership、先頭 `lookahead_blocks`(2)call と underrun は defined silence、全 call `Ok(CONTINUE)`、≥8/30 delivered |
| Defined rejections | `SaveState`/`RestoreState` → wire `Err(Unsupported)`(bridge の `Plugin` default)、`OpenGui`/`CloseGui` → host 側 `Err(Unsupported)` short-circuit。fake round-trip なし |
| Crash/Spawn | `kill_child` → `Failed` + `InstanceFailed` 通知 exactly once(exit status 付き)+ `Err(Failed)` 継続 + silence + restart なし + `Destroy` で slot 解放。missing `spec.program` → `SpawnFailed{cause: Spawn}` pre-Hello、scan 登録は残存 |
| `mise.toml` | `test-l1-real-plugin` に test crate build/`--lib --bins`/`--test l1_real_test_plugin_e2e -- --ignored` を追記。`check-l1-real-plugin`/`lint-l1-real-plugin` を 4 crate + package-qualified features(`*/host`,`*/l1-runner`)に拡張。`ci.yml` は変更なし(同 task が既存 lane で 3OS 実行) |
| 本番 adapter | `crates/ruviform-plugin-{vst3,clap}/src/**` 差分ゼロ — runner は test crate 内に配置する設計(adapter `fixture_lib_name()` compile-time 固定・`OutOfProcessSpec` に env 欄なしのため) |

## 実機実行証拠(Windows OS-L1)

実行 tuple:

| 項目 | 値 |
| --- | --- |
| TestID | rf630-l1-runner-win |
| OS | Windows(x64、Git Bash 経由の mise/cargo) |
| toolchain | mise pin の rust(edition 2024) |
| process boundary | **L1 out-of-process** — `PluginHost`/`OutOfProcessSpec` が runner binary を spawn、child が自 cdylib を dlopen、shm + wire v2 で駆動。Fake Plugin Port 非経由 |
| artifacts | `ruviform_plugin_test_vst3.dll` / `ruviform_plugin_test_clap.dll` cdylib + `ruviform-plugin-test-{vst3,clap}-l1-runner` exe(`target/debug/`、lane が build) |
| command | `mise run test-l1-real-plugin` |
| exit | 0 |

観測された実効果(`tests/l1_real_test_plugin_e2e.rs` 各 crate 3 test 全 pass):

- `real_test_plugin_l1_full_series` — spawn+handshake で child 合成 descriptor が parent pin と field-match、Initialize→Activate→StartProcessing→`Processing`、`latency_frames = 1024`(lookahead 2×512)、32 block process で gain 0.5 delivered、state/GUI verbs が defined `Err(Unsupported)`、clean teardown で child exit 0・通知なし
- `real_test_plugin_l1_kill_reports_failed_once` — kill → `Failed` 収束 + `InstanceFailed` exactly once + 以後 `Err(Failed)`/silence/no restart + `Destroy` ok
- `real_test_plugin_l1_missing_runner_is_spawn_failed` — `SpawnFailed{cause: Spawn}`、instance 未 mint・通知なし・hang なし、`Scan` 登録は残存

## 検証(実 command/exit)

| command | 結果 |
| --- | --- |
| `mise run fmt-check` | exit 0 |
| `mise run check ruviform-plugin-test-vst3` | exit 0 |
| `mise run check ruviform-plugin-test-clap` | exit 0 |
| `mise run lint ruviform-plugin-test-vst3` | exit 0(`clippy -- -D warnings`) |
| `mise run lint ruviform-plugin-test-clap` | exit 0(同上) |
| `mise run test ruviform-plugin-test-vst3` | exit 0 — 44 unit + host_path 3 + e2e target 0 tests(feature gate で非表示を確認) |
| `mise run test ruviform-plugin-test-clap` | exit 0 — 21 unit + host_path 3 + e2e target 0 tests(同上) |
| `mise run test-rt` | exit 0 — `cargo test --workspace rt_` 全 pass |
| `mise run check-l1-real-plugin` | exit 0(4 crate・両 feature 組合せ) |
| `mise run lint-l1-real-plugin` | exit 0(同上、`-D warnings`) |
| `mise run test-l1-real-plugin` | exit 0 — adapter e2e 3+3 + test-fixture e2e 3+3、runner `--lib --bins` 単体 test 含む全 pass |
| `mise exec -- prek run --all-files` | exit 0 |

補助証拠(canonical = mise): lockfile は `cargo check`(mise exec 経由)で再生成、`Cargo.lock` 差分は新規 path dep edge のみ。

## 未検証(NotRun / Unsupported)

| 項目 | 状態 | 理由・後続 |
| --- | --- | --- |
| State Save/Reload/Preset round-trip | Unsupported(定義済み) | 本番 bridge が `save_state`/`restore_state` 未実装 = adapter 側 feature gap。E2E は `Err(Unsupported)` を pin。adapter 補完は別 Issue |
| Native Editor / GUI-L2 | Unsupported(定義済み) | `OpenGui`/`CloseGui` は host 側 `Err(Unsupported)`。実 window dock は Issue #631 lane |
| MIDI/Tempo/Loop over L1 | NotRun | instrument class 未存在。wire codec の coverage は adapter fixture 側に留まる |
| 外部 SDK validator | NotRun/Deferred | license・version 記録と共に後続 slice。外部権威なしに ABI 適合を主張しない |
| macOS/Linux lane 実走 | NotRun | 本 slice の実測は Windows OS-L1 のみ。task/feature は platform-neutral で CI lane が 3OS で拾う |
| MSRV lane(`*-msrv`) | NotRun | ユーザー指定で対象外 |
| `*-full` lane | NotRun | 日常検証は変更範囲比例、Full CI に委譲 |
| consumer oracle(synvelis#101/#127/#128/#176、audio-lens#127)照合 | NotRun | 後続 slice。test fixture は AudioLens への新依存を強制しない(test-only 境界維持) |

## test-only / 配布境界

- runner bin・e2e は `l1-runner` feature + `required-features` + `#[ignore]` で既定 build/test から完全に隔離。default `cargo test` は e2e target を 0 tests で compile のみ。
- 本番 adapter crate は zero-diff。依存方向は test crate → adapter(`host` feature、optional regular + dev-dep union)と test crate → `plugin-host`(dev-dep)のみで DAG 非環。
- cdylib/runner 成果物は `target/` 配下の test fixture であり製品配布物ではない。
