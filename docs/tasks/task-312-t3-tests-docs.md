---
id: "TASK-034S312gtUiTestT3II99"
type: "task"
status: "done"
title: "Issue 312 T3: agent gate unit tests + cross-process E2E + README/crates index"
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
    - "docs/tasks/task-312-t2-control-plane.md"
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

# Issue 312 T3: agent gate unit tests + cross-process E2E + README/crates index

## Work

`tests/agent.rs`: pre-auth `unauthenticated`, re-hello rejection, capability `unavailable`, malformed/`unknown_op`/`invalid_params` reuse, EOF→cleanup via in-memory `serve` with scripted `TestIo`, disable path. `tests/e2e_stdio.rs`: `CARGO_BIN_EXE_dummy_app` spawn → AC1 lifecycle (hello/caps/fixture/observe/shutdown/exit-0) → AC2 matrix (dual window, `keep-*` veto, restart new session_id, `ipc_disconnect` suppress/clear, driver-EOF cleanup, job fixture + `clock.advance`) → kill mid-run cleanup. `tests/e2e_tcp.rs`: loopback connect, wrong/absent secret close, session works after auth. `crates/ruviform-ui-test/README.md` (contract, wiring pattern, honest capability/NotRun table), `crates/README.md` row.

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-312-test-gui-host-lifecycle.md`
- `docs/specs/issue-312-test-gui-host-lifecycle.md` (AC1–AC3)
- `docs/test-specs/issue-312-test-gui-host-lifecycle.md`

## Dependencies

- T2 (control plane).

## Affected Files

- `crates/ruviform-ui-test/tests/agent.rs`, `tests/e2e_stdio.rs`, `tests/e2e_tcp.rs`
- `crates/ruviform-ui-test/README.md`, `crates/README.md`

## Done When

- [x] All test-spec rules have at least one covering test.
- [x] `cargo test -p ruviform-ui-test` green on this host.
- [x] README records capability matrix + #313/#243 handoff.

## Verification

- [x] `mise run fmt-check`
- [x] `mise run check`
- [x] `mise run lint`
- [x] `mise run test`
