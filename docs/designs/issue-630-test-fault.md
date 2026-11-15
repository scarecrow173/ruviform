---
id: "DESIGN-8g91MtjlYytEayPOtT2g3e"
type: "design"
status: "approved"
title: "Issue #630 test_fault slice — 設計"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-630-test-fault.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
    - "docs/designs/issue-630-l1-runner.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-630-test-fault.md"
  implements:
    - "docs/specs/issue-630-test-fault.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-630-test-fault.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
    - "docs/designs/issue-630-l1-runner.md"
  blocks:
    - "docs/plans/issue-630-test-fault.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-630-test-fault.md"
    - "docs/plans/issue-630-test-fault.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-630-test-fault.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #630 test_fault slice — 設計

## Fault law (両 format 共通)

`Fault` param: `ParamId(0)`、name `"Fault"`、plain `[0.0, 4.0]`、
default `0.0`、`AUTOMATABLE | STEPPED`。`denormalize` は linear
(quantize なし — `ParamInfo` は step 数を持たず、neutral/bridge
両層とも stepped 値を量子化しないことを確認済み)。

| plain | mode | 振る舞い |
| ----- | ---- | -------- |
| `0.0` | `none` | pass-through (default・完全 inert) |
| `[0.5,1.5)` 帯域外→ `floor()==1` | `abort` | `std::process::abort()` |
| `floor()==2` | `exit` | `std::process::exit(round((plain-2)*100))` |
| `floor()==3` | `hang` | 応答しない wait (RT 内・復帰しない前提) |
| `floor()==4` | `error` | `ProcessStatus::Error` を返す |

mode decode は `floor(plain)` の帯域 + exit 帯域の小数部×100 を
exit code とする純粋関数 `decode_fault(plain) -> (FaultMode, u32)`。

## Arming channel

`process` block 内の `ParamValue` event のみ (in-band)。
`spec.args` は dead path、env var は `unsafe set_var` が必要で
禁止 — いずれも不使用。arm された block の `process()` 末尾で
dispatch するため、crash/hang は child プロセス内で発生し、
parent (test バイナリ) は生き残る。`abort`/`exit`/`hang` は
in-process test からは絶対に踏まない (`rt_` test 境界)。

## VST3 側構造

- `component.rs`: `ComponentState<P: PluginProcessor>` +
  `TestComponent<P>` に一般化。`TestGainComponent =
  TestComponent<TestGainProcessor>` alias で既存 test 互換。
  `collect_param_events(changes, frames, wanted, events)` を
  free fn 化し、queue の param id を `wanted` (component の
  `param_id`) でフィルタ — `GAIN_PARAM.id` 直書きを解消。
  bounded scan (`MAX_BLOCK_EVENTS`/`MAX_PARAM_SCAN_OPERATIONS`)、
  non-finite/out-of-block drop、stable sort は不変。
- `controller.rs`: gain 専用 → `ParamController::new(&'static
  ParamInfo, step_count)` の data-driven 化。`GAIN_CONTROLLER_CID`
  定数 + `FAULT_CONTROLLER_CID` (fault.rs)。
- `fault.rs`: `FAULT_PARAM`、`FAULT_CONTROLLER_CID`、
  `FAULT_STEP_COUNT = 4`、`TestFaultComponent::CID =
  uid(0x7C3A_91E4, 0x5F2B_8D61, 0xA4E7_0C93, 0xD8B5_6F2A)`
  (test_gain とは別の fresh uid)、`TestFaultProcessor`。
- `factory.rs`: `countClasses` 4、index 2/3 = fault
  component/controller。`"Audio Module Class"` filter で
  descriptor は audio component のみ → test_gain handshake 契約不変。

## CLAP 側構造

- `fault.rs`: `TestFaultClapPlugin` — `PluginParams` +
  `PluginAudioPorts` extension、CLAP descriptor
  `id = "org.ruviform-test.clap-test-fault"`、
  `IS_STEPPED` flag、stereo port。`fault_mirror: ParamMirror` で
  plain 値を保持。
- `lib.rs`: `clack_export_entry!(SinglePluginEntry<...>)` を
  custom `Entry` + `PluginFactoryImpl` に置き換え
  (`plugin_count()=2`、`plugin_id` dispatch)。unsafe は macro
  生成 static のみ (crate 内 unsafe 新規なし)。

## E2E oracle 設計上の確定事項

- **armed submit は繰り返す**: L1 submit は fire-and-forget で
  busy child が drop し得る (`dropped_submits`)。単発 armed
  block は arm を落とし得るため `arm_fault` は 4 回送出
  (`full_series` が event を毎 block に載せる先例と同じ理屈)。
- **error mode の観測面**: plugin の process error は wire の
  `RESULT_FAILED` flag ではなく result payload の
  `ProcessStatus::Error` として `host.process` の戻り値に現れる
  (`results_failed_observed` は synthesized failure 専用)。
  oracle は `Ok(ProcessResult{status: Error})` の複数回観測 +
  `Processing` 維持 + `none` 後の連続 `Continue`。
- **hang**: `command_timeout = 500ms` の spec で `SaveState` →
  `Err(HostError::Ipc)` → `Failed{cause: Timeout}` → watchdog
  kill。audio leg は `Ok(CONTINUE)`+substituted output を継続、
  `passthrough_submits` 増加。
- **bundled fan-out**: `Bundled` route は 1 child に最大
  `WIRE_MAX_INSTANCES(64)` 同居 — 2 instance の `test_fault` で
  一方 abort → 両方 `Failed` + 2 notification。
- **dedicated isolation**: `OutOfProcessIsolation::Dedicated` で
  fault/gain を別 child に分離 — fault abort 後も gain は
  `Processing` + unity gain (`out == in` bitwise) を継続配信。

## RT boundary

arm 値適用 (`denormalize` + store) と pass-through DSP は
alloc/lock/wait-free。`hang` の wait は process 内だが RT thread
を permanent に塞ぐ意図的 fault — `rt_` test では踏まない。
