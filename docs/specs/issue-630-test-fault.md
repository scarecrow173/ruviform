---
id: "SPEC-GhY5geFlPWhmRfRdTwHkzA"
type: "spec"
status: "approved"
title: "Issue #630 test_fault slice — in-band armed fault plugin + L1 Crash/Hang/Timeout/Isolation E2E"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/designs/issue-630-test-plugin-suite.md"
    - "docs/specs/issue-630-test-plugin-suite.md"
    - "docs/specs/issue-630-l1-runner.md"
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
        path: "docs/specs/issue-630-test-fault.md"
      - type: "file"
        path: "docs/designs/issue-630-test-fault.md"
      - type: "file"
        path: "docs/plans/issue-630-test-fault.md"
      - type: "file"
        path: "docs/tasks/issue-630-test-fault.md"
      - type: "file"
        path: "docs/test-specs/issue-630-test-fault.md"
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
    generated:
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/impl/exp/README.md"
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-630-test-plugin-suite.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
    - "docs/specs/issue-630-l1-runner.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-630-test-plugin-suite.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
    - "docs/specs/issue-630-l1-runner.md"
    - "docs/test-specs/issue-630-test-plugin-suite.md"
    - "POLICIES.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-630-test-plugin-suite.md"
    - "docs/specs/issue-630-l1-runner.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #630 test_fault slice — in-band armed fault plugin + L1 Crash/Hang/Timeout/Isolation E2E

## What Should Be Built

A `test_fault` plugin class inside each existing Issue #630 test
fixture cdylib (`ruviform-plugin-test-vst3`,
`ruviform-plugin-test-clap`), plus the L1 out-of-process E2E oracle
that exercises the host's Crash/Hang/Timeout/Isolation semantics
through the real plugin ABI:

- **`test_fault` class per format** — same stereo bus shape as
  `test_gain`, one `Fault` parameter (plain `[0.0, 4.0]`, default
  `0.0`, stepped, five modes `none`/`abort`/`exit`/`hang`/`error`),
  trivial pass-through DSP. VST3 gains a second component/controller
  CID pair (factory 2→4 classes); CLAP replaces
  `SinglePluginEntry` with a custom multi-plugin entry exposing both
  `test_gain` and `test_fault`.
- **In-band arming law (both formats identical)** — a `ParamValue`
  event inside a `process` block arms the mode; the armed block's
  `process()` dispatches it inline (`abort` →
  `std::process::abort()`, `exit(N)` → `std::process::exit(N)`,
  `hang` → non-returning wait, `error` → defined process failure).
  Unarmed/`none` is fully inert pass-through. `exit` carries its
  code as `plain = 2.0 + code/100.0` (e.g. `2.42` → `exit(42)`).
- **`tests/l1_fault_test_plugin_e2e.rs` per crate**
  (`#![cfg(feature = "l1-runner")]` + `#[ignore]`) covering:
  descriptor pin + clean pass-through; `abort` → `Failed` +
  exactly-once `InstanceFailed` + `Err(Failed)` + silence +
  bounded `Destroy`; `exit(42)` → `exit.code == Some(42)`;
  `exit(0)` still `Failed`; `hang` + short `command_timeout` →
  `Err(Ipc)` → `Failed{cause: Timeout}` + watchdog kill +
  substituted audio; bundled two-instance crash fan-out (both
  `Failed`, two notifications); dedicated isolation (`test_fault`
  dies, `test_gain` keeps `Processing` + delivering); `error` →
  `Ok(ProcessResult{status: Error})` defined rejection, stays
  `Processing`, `none` disarm restores `Continue`.
- **`mise.toml`** — one added run line in `test-l1-real-plugin`
  selecting `--test l1_fault_test_plugin_e2e -- --ignored`.
- **Doc/evidence chain** for this slice.

## Why Now

Issue #630's remaining acceptance for this slice is AC-2's
Crash/Hang/Timeout/Isolation part and AC-5's reproducible fault
injection. The L1 runner slice (`issue-630-l1-runner`) already
proved the out-of-process spawn/handshake/failure plumbing with
`test_gain`; this slice adds the fixture that can *cause* the
failures the host must survive — in-band, through the real ABI,
without env/argv knobs or production-adapter reuse.

## Acceptance Criteria (this slice)

- [ ] `test_fault` discoverable + instantiable through the real
  VST3 class scan and CLAP factory scan (host_path tests: two
  descriptors/factories, correct param metadata, unarmed
  pass-through, defined `error` rejection in-process).
- [ ] L1 E2E (ignored, `l1-runner` feature, real cdylib + runner):
  all eight cases per format pass on Windows OS-L1 via
  `mise run test-l1-real-plugin`.
- [ ] Crash/hang/timeout/isolation oracles match the documented
  contract: `Failed` convergence, exactly-once `InstanceFailed`,
  `Err(Ipc)` on timeout, child kill, bundled fan-out, dedicated
  isolation, defined rejection ≠ crash.
- [ ] `rt_`-safe: the inert arm-apply + pass-through path stays
  alloc/lock/wait-free; abort/exit/hang are never driven in-process.
- [ ] Honest boundaries recorded: pre-Hello/spawn-phase faults,
  external SDK validators, `test_stateful`/`test_instrument`/
  `test_editor`, non-Windows OS runs, `*-msrv`/`*-full` lanes are
  out of scope (deferred/NotRun).

## Non-Goals

- No production crate changes (`ruviform-plugin-{vst3,clap,host,
  export,core}` stay zero-diff); no env-var/argv arming; no new
  `unsafe` beyond the clack-macro-generated entry statics.
- `test_gain`'s class/descriptor/CID contracts unchanged.
- PR uses `Refs #630` — the Issue stays open for later slices.
