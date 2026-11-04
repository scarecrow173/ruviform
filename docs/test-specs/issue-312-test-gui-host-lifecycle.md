---
id: "TSPEC-034S312gtUiTestTspFF66"
type: "test-spec"
status: "approved"
title: "Issue #312 ruviform-ui-test — lifecycle/control/fixture/clock/observation/fail-closed test specification"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "docs/specs/issue-312-test-gui-host-lifecycle.md"
  changes: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-312-test-gui-host-lifecycle.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-312-test-gui-host-lifecycle.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #312 ruviform-ui-test — lifecycle/control/fixture/clock/observation/fail-closed test specification

## Purpose

Pin the observable contract of `rvf.uitest.v1` + `TestIo`/`FakeHost`/`HeadlessIo`/`TestAgent`/`TestDriver` (spec AC1–AC5): handshake typing, capability-gated ops, honest observation/fault/clock semantics, shutdown/cleanup ownership, and fail-closed security. Retire only if the crate is removed or the protocol is replaced.

## Feature

Test-run-dedicated control channel: a driver spawns or attaches to an app-under-test embedding `TestAgent`; a typed, capability-gated op set provides fixture/clock/fault/observation/shutdown without user-op injection or second command-gateway semantics.

## Rules

- Rule: every pre-`test.hello` op returns `unauthenticated`; `test.hello` returns `Welcome` carrying session_id, echoed run_id, app/ruviform revisions, backend metadata, capabilities, and window list — or a `Failed` reason when `io.ready_state()` is not `Ready`.
- Rule: a second `test.hello` inside a live session is `unauthenticated`; TCP hellos with a missing or mismatched secret are `unauthenticated` and the connection closes; `serve_tcp` without an expected secret refuses to serve at all; `RVF_TEST_DISABLE=1` makes every serve path return `Disabled` immediately.
- Rule: ops requiring an absent capability (`fixture`, a `clock_domains` member, a `faults` member, `observation.*`, `screenshot`) return `unavailable` and mutate nothing; `screenshot` on `screenshot:false` backends returns `unavailable` (never a fabricated image).
- Rule: `test.clock.advance {domain:"host_tick", n}` emits exactly `n` `WindowEvent::Tick` through `io.emit` and reports `{advanced:n}`; any other domain is `unavailable`. OS time/GPU/webview timers are never claimed as controllable (`capabilities.clock_domains` lists only `host_tick`, `input_injection` and `dom_access` are always `false`).
- Rule: `test.fault.set {kind:"window_crash", window}` destroys the window and emits `Windowed{Destroyed}` (terminal, id retired); `{kind:"ipc_disconnect", window}` suppresses subsequent inbound `Message` events for that window — suppressed traffic is recorded as a `suppressed` receipt and `test.fault.clear` restores delivery. Unknown kinds/windows are `unavailable`/`invalid_params`, never silent.
- Rule: `test.window.request_close` produces `Windowed{CloseRequested}`; the app's close policy (`Veto`) leaves the window alive and observable — never auto-destroyed by the control path.
- Rule: `observe.receipts {since,limit}` returns a bounded monotonic-seq page plus `truncated`; entries carry kind/window/direction/bytes — never bulk payload bodies.
- Rule: `test.shutdown` replies `{bye:true, cleanup}` then closes the stream; `graceful` destroys open windows (each emits `Destroyed`) and reports job/window counts; a driver EOF triggers the same graceful cleanup and ends serve with `DriverGone` — no orphaned windows, handles, jobs, or ports.
- Rule: malformed frames/op names reuse `rvf.ipc.v1` codes (`malformed_message`, `message_too_large`, `unknown_op`, `invalid_params`); the connection survives a single malformed frame.
- Rule: `FakeHost` delivery matches the `run_multi` contract — eval to unknown/destroyed windows is a silent drop, not-ready windows buffer a bounded drop-oldest backlog flushed on `Ready`, `Destroyed` is delivered through the event queue as the window-terminal event.

## Examples

- Given `dummy_app` spawned via `spawn_stdio`, when the driver performs hello→capabilities→`fixture.prepare{dual}`→`observe.windows`, then two windows exist, receipts page contains the inbound `Ready`/`Message` kinds, `shutdown{graceful}` yields `bye` + cleanup, the child exits 0, and a subsequent `windows()`/port probe shows nothing left.
- Given a `keep-*` window, when `test.window.request_close` is issued and the app vetoes, then `observe.windows` still lists the window with `close_pending` cleared and receipts show the `CloseRequested`+`Veto` sequence.
- Given `ipc_disconnect` set on w1, when the driver injects a `HostEvent::Message` for w1 via fixture then reads receipts, then a `suppressed` receipt exists and the app received nothing; after `fault.clear`, a new message produces a `result` eval receipt.
- Given `driver.restart`, then the new session reports a different `session_id` and identical capabilities.
- Given the child killed mid-run (`kill`), then `wait_exit` reports a non-clean status, driver teardown reaps the child, and no port/handle remains open.
- Given a TCP run, when hello carries a wrong/absent secret or the peer is non-loopback, then the connection closes without a Welcome.
- Given `RVF_TEST_DISABLE=1`, when the app starts, then serve returns `Disabled` and no control channel is reachable (stdio/TCP alike).
- Given a `demo.sleep` job fixture, when `clock.advance{host_tick, n}` runs, then job progress/drained receipts appear in the journal — proving tick-domain advance only.

## Guarantees

- All pre-existing workspace tests pass unmodified; `ruviform-ui`/`ruviform-ui-webview` have zero diff.
- `cargo test -p ruviform-ui-test` covers every rule above on a headless CI host (Windows/Linux/macOS alike).
- Real-windowed `TestIo` wiring, real screenshot capture, and non-stdio attach paths are NotRun in this issue (handoff: #313, #243).
