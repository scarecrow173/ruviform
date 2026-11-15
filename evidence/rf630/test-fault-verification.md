# RF-VTEST-03 test_fault slice — 検証記録

- Issue: `scarecrow173/ruviform#630`（AC-2 Crash/Hang/Timeout/Isolation + AC-5 fault injection 部分のみ）
- Slice: in-band armed `test_fault` plugin（VST3 + CLAP）+ L1 out-of-process E2E
- OS: Windows (OS-L1) / toolchain: mise 管理 rust（workspace.lints 正本）
- 記録日: 2026-11-15
- Case Schema v1 語彙・`environment_kind=test-plugin`。deterministic evidence と real-run evidence を分離。

## 実装概要

- `crates/ruviform-plugin-test-vst3/src/fault.rs`: `FAULT_PARAM`（plain `[0.0,4.0]`、default `0.0`、`AUTOMATABLE|STEPPED`）、`decode_fault(plain)`（floor 帯域: none/abort/exit/hang/error、exit code = `round((plain-2)*100)`）、fresh `uid(0x7C3A_91E4,0x5F2B_8D61,0xA4E7_0C93,0xD8B5_6F2A)` の component/controller CID、`TestFaultProcessor`（trivial pass-through）。
- `component.rs`: `ComponentState<P: PluginProcessor>`/`TestComponent<P>` に一般化、`collect_param_events` free fn 化（`wanted` param id で queue フィルタ）。
- `controller.rs`: `ParamController::new(&'static ParamInfo, step_count)` data-driven 化。
- `factory.rs`: `countClasses` 4（gain comp/ctrl + fault comp/ctrl）。
- `crates/ruviform-plugin-test-clap/src/fault.rs`: `TestFaultClapPlugin`（`PluginParams`+`PluginAudioPorts`、descriptor `org.ruviform-test.clap-test-fault`、`IS_STEPPED`）。
- `lib.rs`: `SinglePluginEntry` → custom `Entry`+`PluginFactoryImpl`（`plugin_count()=2`）。
- arming は process block 内 `ParamValue` event のみ（argv/env 不使用、unsafe 新規なし）。

## 実行した検証（canonical = mise、実 exit code）

| command | result | exit |
| --- | --- | --- |
| `mise run fmt-check` | pass | 0 |
| `mise run check ruviform-plugin-test-vst3` | pass | 0 |
| `mise run lint ruviform-plugin-test-vst3` | pass（`clippy --all-targets -- -D warnings` 相当） | 0 |
| `mise run test ruviform-plugin-test-vst3` | 49 unit + host_path pass | 0 |
| `mise run check ruviform-plugin-test-clap` | pass | 0 |
| `mise run lint ruviform-plugin-test-clap` | pass | 0 |
| `mise run test ruviform-plugin-test-clap` | 25 unit + host_path pass | 0 |
| `mise run check-l1-real-plugin` | pass（`--features l1-runner` 全 target） | 0 |
| `mise run lint-l1-real-plugin` | pass | 0 |
| `mise run test-l1-real-plugin` | pass — adapter e2e 3+3、test-plugin e2e 3+3、**fault e2e 8+8 = 16 case 全 pass**（実 cdylib + runner spawn、test 名: `descriptor_and_clean_block`/`abort_fails_instance`/`voluntary_exit_reports_code`/`exit_zero_is_failure`/`hang_converges_on_timeout`/`bundled_crash_fans_out_to_co_resident`/`dedicated_isolation_keeps_other_instance`/`error_is_defined_rejection` × VST3・CLAP） | 0 |
| `mise run test-rt` | pass — workspace `rt_` filter 全 pass（exit 0）。fault の inert arm-apply/pass-through 経路を含む。abort/hang/exit は in-process で**実行しない**（process 死亡回避） | 0 |
| `mise exec -- prek run --all-files` | pass | 0 |

cargo 直接実行は補助証拠（canonical = mise）。`*-msrv`/`*-full` lane はユーザー指定により未実行。

## Fault oracle の確定事項（実機観測で確定）

1. **armed submit は繰り返し送出**: L1 submit は fire-and-forget で busy child が `dropped_submits` で落とし得る → `arm_fault` は armed block を 4 回送出。
2. **error 観測面**: plugin の process error は wire `RESULT_FAILED` flag ではなく `ProcessStatus::Error` として `host.process` 戻り値に現れる（`results_failed_observed` は synthesized failure 専用）。oracle は status 観測 + `Processing` 維持 + `none` disarm 後連続 `Continue`。
3. **delivery cadence**: lookahead（2 block）+ 非同期 result drain のため clean block は sustained submit で観測。
4. **bundled fan-out**: Bundled route は 1 child に最大 `WIRE_MAX_INSTANCES(64)` 同居 — 片方 abort で両方 `Failed` + 2 notification（co-resident 全員 fan-out 契約）。
5. **dedicated isolation**: Dedicated route で fault/gain を別 child に分離 — fault abort 後も gain は `Processing` + unity gain 配信継続。
6. **exit(0)**: voluntary exit(0) も `Failed` 収束（`voluntary_exit_zero_is_failure` 先例どおり）。

## Pass/Fail/NotRun/Unsupported

- **Pass**: descriptor pin + clean pass-through、abort crash、exit(42) code、exit(0) failure、hang→timeout kill、bundled fan-out、dedicated isolation、error defined rejection — 両 format × 8 case、Windows OS-L1。
- **NotRun**: pre-Hello/spawn-phase fault（in-band arming は Hello 前に到達不能 — env-knob/専用 fault-runner は後続 slice へ Deferred）、外部 SDK validator（Deferred 継続）、`test_stateful`/`test_instrument`/`test_editor`（後続 slice）、非 Windows OS 実走（本 slice は Windows のみ。他 OS は既存 CI lane が同 task を実走）、`*-msrv`/`*-full`（daily CI 委譲）、GUI-L2/EXT-L3。
- **Unsupported**: なし（本 slice 範囲内に Unsupported 項目なし）。
- **Deferred**: consumer oracle 照合（`scarecrow173/synvelis#101`/`#127`/`#128`/`#176`、`scarecrow173/audio-lens#127`）— 本 slice 範囲外。

## 境界遵守

- production crate（`ruviform-plugin-{vst3,clap,host,export,core}`）zero-diff。
- `unsafe` 新規なし（clack macro 生成 static のみ）。
- `rt_` test は abort/exit/hang path を踏まない。inert 経路は alloc/lock/wait-free。
- `Cargo.lock` 差分なし（新規依存なし）。
- `F:` ドライブに中間生成物なし。
