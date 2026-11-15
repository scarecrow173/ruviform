# RF-VTEST-03 test_stateful slice — 検証記録

- Issue: `scarecrow173/ruviform#630`（test plugin suite の state slice）
- Slice: state を持つ `test_stateful` plugin（VST3 + CLAP）+ in-process host-path + L1 out-of-process E2E
- OS: Windows (OS-L1) / toolchain: mise 管理 rust（workspace.lints 正本）
- 記録日: 2026-11-15
- Case Schema v1 語彙・`environment_kind=test-plugin`。deterministic evidence と real-run evidence を分離。

## 実装概要

- `crates/ruviform-plugin-test-vst3/src/stateful.rs`（新規）: `SharedGain`（`Arc<AtomicU64>` — plain gain f64 bits の lock-free cell）、16-byte blob codec（component=plain / controller=normalized）、`IBStream` read/write helper、`TestStatefulProcessor`、`TestStatefulComponent`、COM-level state-verb tests。
- `src/component.rs`: `ComponentStateSurface` trait（`set_state`/`get_state` の `IBStream` 面）導入、`TestComponent` に `Option<Arc<dyn ComponentStateSurface>>` + `with_state(...)` constructor。stateless class は `state_surface=None` → `kNotImplemented` 維持。
- `src/controller.rs`: `ParamController::new_stateful` + stateful state verbs（`setComponentState` で plain→normalized 変換、`setState`/`getState` は normalized blob 往復）。
- `src/factory.rs`: `countClasses` 6→8、index 6/7 = stateful component/controller（fresh `uid`、既存 CID と非衝突）。
- `src/gain.rs`: `apply_gain` を `pub(crate)` 化（DSP law 再利用）。
- `crates/ruviform-plugin-test-clap/src/stateful.rs`（新規）: `TestStatefulShared`（`AtomicLifecycle` + `ParamMirror` — mirror が persisted state cell を兼ねる）、`TestStatefulMainThread`（ports + params + `PluginStateImpl`）、`TestStatefulAudioProcessor`、`TestStatefulClapPlugin`（descriptor `org.ruviform-test.clap-test-stateful`、`declare_extensions` に `PluginState` 登録）。
- `src/lib.rs`: `TestSuiteFactory` に 3 つ目の descriptor 追加、`plugin_count()=3`、`create_plugin` dispatch。
- `Cargo.toml`: `clack-extensions` features に `"state"`。
- `tests/host_path.rs`（両 crate）: descriptor/factory count 更新、stateful round-trip/malformed/stateless-decline tests。
- `tests/l1_stateful_test_plugin_e2e.rs`（両 crate、新規）: delivered-audio ordering gate 付き round-trip + malformed。
- `mise.toml`: `test-l1-real-plugin` に stateful e2e 行追加。

## State blob law（確定）

- 16 byte: `[u32 LE magic "RFST"][u32 LE version=1][f64 LE gain]`。
- VST3: component payload = plain gain、controller payload = normalized gain（`plain = normalized × 2`）。
- CLAP: `clap.ext.state` は adapter framing を持たない単一 stream — plain gain の 16 byte blob をそのまま write/read。
- 検証: 長さ不一致・magic/version 不一致・非有限値は defined rejection。有限の range 外値は `[0.0, 2.0]` に clamp。CLAP は trailing bytes も reject。
- serialization は byte-exact deterministic（`f64::to_le_bits` → `u64::to_le_bytes`）。

## Persistent-state law（確定）

- `test_gain` は activation で mirror を default に reset する per-activation law を持つ。
- `test_stateful` はこれを意図的に外す: `activate_inner` は mirror を reset せず、init default（1.0）または `load`/restore で書き込まれた値を保持する。
- restore → activate → process の順で restored gain が DSP を駆動することが round-trip の意味を持つための必須差分。

## L1 ordering gate（確定）

- L1 data plane は lookahead（2 block）+ bounded drain を持ち、`SaveState`/`RestoreState` は FIFO stdin loop の同期 command だが `process` submit は fire-and-forget で state verb が mutation event block を追い越し得る。
- fixture の 3 gain（1.0/0.75/1.75）は constant input `0.5/-0.25` に対し pairwise-distinct な output（`0.5/-0.25`, `0.375/-0.1875`, `0.875/-0.4375`）を生む。
- 期待 gain の delivered output 観測 = それ以前の全 block（event block 含む）が child で処理済みの証明になる。各 state verb はこの gate の後にのみ送出する。

## 実行した検証（canonical = mise、実 exit code）

| command | result | exit |
| --- | --- | --- |
| `mise run fmt-check` | pass | 0 |
| `mise run check ruviform-plugin-test-vst3` | pass | 0 |
| `mise run lint ruviform-plugin-test-vst3` | pass（`clippy --all-targets -- -D warnings` 相当） | 0 |
| `mise run test ruviform-plugin-test-vst3` | 69 unit + host_path 15 pass | 0 |
| `mise run check ruviform-plugin-test-clap` | pass | 0 |
| `mise run lint ruviform-plugin-test-clap` | pass | 0 |
| `mise run test ruviform-plugin-test-clap` | 35 unit + host_path 10 pass | 0 |
| `mise run check-l1-real-plugin` | pass（`--features l1-runner` 全 target） | 0 |
| `mise run lint-l1-real-plugin` | pass | 0 |
| `mise run test-l1-real-plugin` | pass — adapter e2e、test-plugin e2e 3+3、fault e2e 8+8、**stateful e2e 2+2 = 4 case 全 pass**（実 cdylib + runner spawn、test 名: `real_stateful_test_plugin_l1_save_mutate_restore_save`/`real_stateful_test_plugin_l1_rejects_malformed_state` × VST3・CLAP） | 0 |
| `mise run test-rt` | pass — workspace `rt_` filter 全 pass（exit 0）。stateful の `rt_` allocation-free test を含む。state verb は non-RT 面のみ | 0 |
| `mise exec -- prek run --all-files` | pass | 0 |

cargo 直接実行は補助証拠（canonical = mise）。`*-msrv`/`*-full` lane はユーザー指定により未実行。

## State oracle の確定事項（実機観測で確定）

1. **VST3 blob framing**: production bridge は `[u32 comp_len][comp][u32 ctrl_len][ctrl]` で frame する — test は 40 byte（8 overhead + 16 + 16）を assert し、component payload の plain gain を offset 12 の f64 で decode する。
2. **CLAP blob**: framing なしの raw 16 byte stream — test は offset 8 の f64 を decode する。
3. **malformed case**: truncated / trailing-bytes / corrupted magic / non-finite payload は全て `HostError::PluginFailed`（defined rejection）で、instance は生存し `SaveState` が有効 blob を返し続ける。
4. **stateless decline**: `test_gain`/`test_fault`/`test_editor` は VST3 `kNotImplemented`、CLAP `HostError::Unsupported` を維持。
5. **descriptor independence**: VST3 stateful descriptor ID は component CID から独立導出（`vst3::uid(0x8D2F_6B94,0xA1C5_3E07,0xF4B8_9D26,0xC7E3_5A81)`）、CLAP は `org.ruviform-test.clap-test-stateful`。

## Pass/Fail/NotRun/Unsupported

- **Pass**: VST3 factory 8 class + stateful comp/ctrl state verbs、CLAP factory 3 plugin + `clap.ext.state`、in-process host-path round-trip byte-exact、malformed rejection + instance survival、stateless decline、L1 out-of-process round-trip + malformed、RT allocation-free — 両 format、Windows OS-L1。
- **NotRun**: 外部 SDK validator（VST3 SDK validator / CLAP validator — Deferred 継続）、`test_instrument`/CLAP `test_editor`（後続 slice）、非 Windows OS 実走（本 slice は Windows のみ。他 OS は既存 CI lane が同 task を実走）、`*-msrv`/`*-full`（daily CI 委譲）、GUI-L2/EXT-L3。
- **Unsupported**: なし（本 slice 範囲内に Unsupported 項目なし）。
- **Deferred**: preset/bank・GUI state・foreign format migration・commercial plugin integration — Non-Goal。

## 境界遵守

- production crate（`ruviform-plugin-{vst3,clap,host,export,core}`）zero-diff。
- `unsafe` 新規なし（clack macro 生成 static のみ）。
- `rt_` test は state verb を踏まない。state verb は non-RT 面（VST3 component/controller、CLAP main-thread `PluginStateImpl`）にのみ存在。
- `Cargo.lock` 差分なし（`clack-extensions` feature 追加のみ、新規依存なし）。
- `F:` ドライブに中間生成物なし。
