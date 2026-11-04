---
id: "TASK-034S312gtUiTestT2HH88"
type: "task"
status: "done"
title: "Issue 312 T2: control plane — TestAgent + transport frames/env + TestDriver + dummy_app bin"
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
    - "docs/tasks/task-312-t1-contract-headless-core.md"
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

# Issue 312 T2: control plane — TestAgent + transport frames/env + TestDriver + dummy_app bin

## Work

Implement `transport.rs` (u32le+JSON `read_frame`/`write_frame` with `MAX_INTENT_BYTES` cap; `RVF_TEST_CONTROL`/`RVF_TEST_ADDR`/`RVF_TEST_SECRET`/`RVF_TEST_DISABLE` env interpretation; loopback peer check; RandomState-based run-bound secret gen), `agent.rs` (`TestAgent`: session/auth state, capability gates, op dispatch per design table, `handle`/`serve`/`serve_stdio`/`serve_tcp`/`from_env`, EOF→graceful cleanup, `RVF_TEST_DISABLE` → `Disabled`), `driver.rs` (`TestDriver`: `spawn_stdio`/`spawn_tcp`, hello/welcome, `call` with Notify buffering, typed ops incl. `shutdown`/`wait_exit`/`restart`/kill-on-drop), and `src/bin/dummy_app.rs` (UiRuntime+DemoService+FakeHost+HeadlessIo, `keep-*` veto policy, fixtures `single`/`dual`/`keep`/`sleep-job`, transport via `from_env`).

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-312-test-gui-host-lifecycle.md`
- `docs/specs/issue-312-test-gui-host-lifecycle.md` (AC1–AC3)
- `docs/adr/issue-312-test-control-transport-security.md`
- `docs/designs/issue-312-test-gui-host-lifecycle.md`

## Dependencies

- T1 (types, `TestIo`, `FakeHost`, `HeadlessIo`).

## Affected Files

- `crates/ruviform-ui-test/src/agent.rs`, `src/transport.rs`, `src/driver.rs`, `src/bin/dummy_app.rs`, `src/lib.rs` (re-exports), `Cargo.toml` (bin)

## Done When

- [x] `dummy_app` compiles and answers `test.hello` over stdio frames.
- [x] Agent enforces auth/capability/disable gates per the fail-closed matrix.
- [x] `TestDriver` spawn/hello/call/shutdown/restart works against the bin (smoke).

## Verification

- [x] `mise run fmt-check`
- [x] `mise run check`
- [x] `mise run lint`
- [x] `mise run test` (crate unit tests)
