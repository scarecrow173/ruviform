---
id: "TSPEC-1jGomS13d3GgO1MNrOleAi"
type: "test-spec"
status: "approved"
title: "Issue #630 test_fault slice — test specification"
created: "2026-11-15"
updated: "2026-11-21"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-630-test-fault.md"
    - "docs/designs/issue-630-test-fault.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-630-test-fault.md"
  implements:
    - "docs/specs/issue-630-test-fault.md"
    - "docs/designs/issue-630-test-fault.md"
  implemented-by: []
  depends-on:
    - "docs/test-specs/issue-630-test-plugin-suite.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-630-test-fault.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-630-test-fault.md"
    - "docs/designs/issue-630-test-fault.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-630-test-fault.md"
    - "docs/designs/issue-630-test-fault.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #630 test_fault slice — test specification

## Feature: test_fault fixture (in-process, `tests/host_path.rs`)

- **Rule**: `test_fault` は `test_gain` と同一 cdylib/factory
  surface に第 2 plugin/class として発見される。
  **Example**: VST3 `countClasses()==4`・CLAP `plugin_count()==2`、
  discovery が `test_gain`+`test_fault` を返す。
- **Rule**: `Fault` param metadata は plain `[0,4]`・default `0`・
  stepped・`AUTOMATABLE`。
- **Rule**: 非武装/`none` は bitwise pass-through。
  **Example**: in `0.5/-0.25` → out 同一。
- **Rule**: `error` mode は defined rejection (process `Err`) で
  crash ではない。in-process で instance は継続利用可能
  (disarm 後 pass-through 回復)。abort/exit/hang は in-process
  では**絶対に**実行しない (process 死亡 = test バイナリ死亡)。

## Feature: L1 fault E2E (`tests/l1_fault_test_plugin_e2e.rs`, ignored)

全 case は実 cdylib + `l1-runner` runner を spawn。fixture/runner
欠落は assert failure (silent skip なし)。

- **descriptor_and_clean_block**: handshake descriptor pin、
  `Processing` 到達、unarmed block の bitwise pass-through、
  clean teardown・notification 無し。
- **abort_fails_instance**: `Failed` 収束、`InstanceFailed`
  exactly once (exit status あり)、以降 command `Err(Failed)`、
  process `Ok(CONTINUE)`+silence、`Destroy` ok。
- **voluntary_exit_reports_code**: `exit.code == Some(42)` —
  param encoding が exit code を end-to-end で伝搬する。
- **exit_zero_is_failure**: `exit(0)` も `Failed` + `Some(0)`。
- **hang_converges_on_timeout**: `command_timeout=500ms`、
  audio leg `Ok(CONTINUE)` 継続、`SaveState` → `Err(Ipc)`、
  `Failed{cause: Timeout}`、watchdog kill、`Destroy` bounded。
  ruviform#846 で pre-mint wait（spawn hand-off・`Hello`・`Scan`・
  `CreateInstance`）は独立した `spawn_timeout`（既定 10 s）へ
  分離済み — `command_timeout=500ms` は post-mint reply wait のみを
  縮め、`instantiate` が環境 stall で
  `SpawnFailed{cause: Timeout}` になる共有 deadline の構造的原因は
  解消された。残留する flake 観測の分類ルール・定量・対処方針は
  `docs/discovery/issue-841-spawn-deadline-flake.md`（ruviform#841）を
  参照。test 側 bounded re-instantiate は ruviform#849 で両 ABI の
  `instantiate` helper に実装済み — `SpawnFailed{cause: Timeout}` が
  retained diagnostics record で pre-mint phase（`PreSpawn`/`Spawn`/
  `DrainInit`/`Handshake`）かつ `cause == Some(Timeout)` と signature
  確認された場合のみ bounded（最大 3 retry）re-instantiate し、
  signature 不一致・bound 超過は正直に fail する。本 spec の
  assertion・gate・`#[ignore]` は不変。
- **bundled_crash_fans_out**: Bundled 2 instance、片方 abort →
  両方 `Failed` + 2 `InstanceFailed`。
- **dedicated_isolation**: Dedicated で fault/gain 分離、fault
  abort → gain は `Processing` 継続 + unity gain 配信。
- **error_is_defined_rejection**: `ProcessStatus::Error` を
  ≥2 回観測、`Processing` 維持、notification 無し、`none`
  disarm 後 6 連続 `Continue`。
