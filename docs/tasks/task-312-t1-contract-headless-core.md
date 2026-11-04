---
id: "TASK-034S312gtUiTestT1GG77"
type: "task"
status: "done"
title: "Issue 312 T1: ruviform-ui-test skeleton — proto types + journal + FakeHost + fixture/TestApp + HeadlessIo"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source: []
  changes: []
  implements:
    - "docs/plans/issue-312-test-gui-host-lifecycle.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-312-test-gui-host-lifecycle.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-312-test-gui-host-lifecycle.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue 312 T1: ruviform-ui-test skeleton — proto types + journal + FakeHost + fixture/TestApp + HeadlessIo

## Work

Create `crates/ruviform-ui-test` (Cargo.toml: `ruviform-ui` + `serde` + `serde_json` only, workspace lints, publish=false) and register it in root `Cargo.toml` members. Implement the contract layer per design: `proto.rs` (op name consts, `test_codes`, all serde types — `Welcome`/`Capabilities`/`BackendMeta`/`AppMeta`/`WindowMeta`/`CleanupReport`/`ServeOutcome`/`ShutdownMode`/`ReadyState`/`FaultKind`/`ClockDomain`/`FixtureSpec`/`FixtureReceipt`/`Receipt`/`ReceiptPage`/`Screenshot`/`Notice`), `journal.rs` (`Journal` + `JournalHandle` WindowHandle decorator), `fake.rs` (`FakeHost`/`FakeWindow` — `run_multi` parity: bounded backlog, queue-delivered `Destroyed`, ipc_down suppression hooks), `fixture.rs` (`FixtureController`, `TestApp` traits), `headless.rs` (`HeadlessIo` `TestIo` impl — emit/journal/pump, windows metadata, faults, request_shutdown, notices, screenshot=unavailable). `lib.rs` crate docs + re-exports. Unit tests for journal bounds, FakeHost delivery contract, HeadlessIo emit/fault/shutdown.

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-312-test-gui-host-lifecycle.md`
- `docs/specs/issue-312-test-gui-host-lifecycle.md` (AC1, AC2)
- `docs/adr/issue-312-test-control-transport-security.md`
- `docs/designs/issue-312-test-gui-host-lifecycle.md`

## Dependencies

- None (first task).

## Affected Files

- `Cargo.toml` (members)
- `crates/ruviform-ui-test/Cargo.toml`, `src/lib.rs`, `src/proto.rs`, `src/journal.rs`, `src/fake.rs`, `src/fixture.rs`, `src/headless.rs`
- `crates/ruviform-ui-test/tests/headless.rs` (unit)

## Done When

- [x] `cargo check -p ruviform-ui-test` compiles with workspace lints.
- [x] `FakeHost` mirrors `run_multi` delivery invariants (drop/backlog/Destroyed-via-queue).
- [x] `HeadlessIo` implements `TestIo` end to end against a scripted `TestApp`.

## Verification

- [x] `mise run fmt-check`
- [x] `mise run check`
- [x] `mise run lint`
- [x] `mise run test` (crate unit tests)
