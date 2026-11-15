---
id: "IMPL-LmwhKtokutWwtYG7PzukdG"
type: "impl"
status: "completed"
title: "Issue #630 test_fault slice — 実装記録"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/tasks/issue-630-test-fault.md"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/fault.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/src/fault.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/tests/l1_fault_test_plugin_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/tests/l1_fault_test_plugin_e2e.rs"
      - type: "file"
        path: "docs/impl/ir/issue-630-test-fault.md"
      - type: "file"
        path: "docs/impl/exp/issue-630-test-fault.jsonl"
      - type: "file"
        path: "evidence/rf630/test-fault-verification.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/component.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/controller.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/factory.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/tests/host_path.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/tests/host_path.rs"
      - type: "file"
        path: "mise.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-630-test-fault.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-630-test-fault.md"
    - "docs/plans/issue-630-test-fault.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/exp/issue-630-test-fault.jsonl"
    - "docs/impl/ir/issue-630-l1-runner.md"
    - "docs/test-specs/issue-630-test-fault.md"
    - "evidence/rf630/test-fault-verification.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/tasks/issue-630-test-fault.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-630-test-fault.jsonl"
    rejected: []
---

# Issue #630 test_fault slice — 実装記録

## Summary

両 Issue #630 test fixture cdylib に `test_fault` class を追加し、
in-band armed fault（`ParamValue` event → process 内 dispatch）を
実 VST3/CLAP ABI + L1 out-of-process runner 経由で Crash/Hang/
Timeout/Isolation oracle として実走検証した。production crate は
zero-diff。

## Context

Follows `docs/tasks/issue-630-test-fault.md` →
`docs/plans/issue-630-test-fault.md` →
`docs/specs|designs/issue-630-test-fault.md`、verified against
`docs/test-specs/issue-630-test-fault.md`。Precedent:
`docs/impl/ir/issue-630-l1-runner.md`（runner + E2E 構造）、
`crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs` の
`voluntary_exit_zero_is_failure`。

## Implementation

### Changes

- `crates/ruviform-plugin-test-vst3/src/component.rs` —
  `ComponentState<P: PluginProcessor>`/`TestComponent<P>` に
  一般化（`TestGainComponent` alias 維持で既存 test 互換）。
  `collect_param_events` を free fn 化し `wanted` param id で
  queue をフィルタ（`GAIN_PARAM.id` 直書きを解消）。
- `crates/ruviform-plugin-test-vst3/src/controller.rs` —
  gain 専用 → `ParamController::new(&'static ParamInfo,
  step_count)` data-driven 化。
- `crates/ruviform-plugin-test-vst3/src/fault.rs`（新規）—
  `FAULT_PARAM`（plain `[0.0,4.0]`、default `0.0`、
  `AUTOMATABLE|STEPPED`）、`FAULT_STEP_COUNT=4`、fresh
  `uid(0x7C3A_91E4,0x5F2B_8D61,0xA4E7_0C93,0xD8B5_6F2A)` の
  component/controller CID、`decode_fault(plain)`（floor 帯域
  none/abort/exit/hang/error、exit code =
  `round((plain-2)*100)`）、`TestFaultProcessor`（trivial
  pass-through）。
- `crates/ruviform-plugin-test-vst3/src/factory.rs` —
  `countClasses` 4、index 2/3 = fault comp/ctrl。
- `crates/ruviform-plugin-test-clap/src/fault.rs`（新規）—
  `TestFaultClapPlugin`（`PluginParams`+`PluginAudioPorts`、
  descriptor `org.ruviform-test.clap-test-fault`、`IS_STEPPED`、
  stereo port、`fault_mirror: ParamMirror`）。
- `crates/ruviform-plugin-test-clap/src/lib.rs` —
  `SinglePluginEntry` → custom `Entry`+`PluginFactoryImpl`
  （`plugin_count()=2`、`plugin_id` dispatch）。unsafe は macro
  生成 static のみ。
- `tests/host_path.rs`（両 crate）— 2 descriptor/factory、
  fault metadata、unarmed pass-through、defined `error`
  rejection の in-process oracle 追加。
- `tests/l1_fault_test_plugin_e2e.rs`（両 crate・新規）—
  `#![cfg(feature = "l1-runner")]` + 8 `#[ignore]`d case。
- `mise.toml` — `test-l1-real-plugin` に fault e2e 行追加。

## Related Experiments

- `docs/impl/exp/issue-630-test-fault.jsonl`（adopted）—
  arm submit drop・error 観測面・delivery cadence の実機観測と
  oracle 修正の記録。

## Validation

`evidence/rf630/test-fault-verification.md` に全 exit code。
`mise run test-l1-real-plugin` で fault e2e 16 case（両 format
× 8）実走 pass。`fmt-check`/`check`/`lint`/`test` 両 crate、
`check-l1-real-plugin`/`lint-l1-real-plugin` pass。`test-rt`
は inert 経路のみ対象（abort/hang/exit は in-process で実行
しない）。

## Risks

- armed submit の fire-and-forget drop — `arm_fault` 繰り返し
  送出で対処済み。
- `hang` は child を watchdog kill まで待つ設計 — case 成功条件
  に kill 収束を含む。

## Follow-ups

- pre-Hello/spawn-phase fault（in-band arming では到達不能 —
  env-knob/専用 fault-runner は後続 slice へ Deferred）。
- `test_stateful`/`test_instrument`/`test_editor`、外部 SDK
  validator、非 Windows OS 実走 — 後続 slice/既存 CI lane。
