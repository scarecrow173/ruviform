# Drive the headless app-under-test

The headless `dummy_app` AUT serves `rvf.uitest.v1` over stdio or
loopback TCP. A driver can spawn it, probe its health with a
read-only doctor, then run a composable sequence of closed-set ops —
fixtures, clock advance, faults, window close — observing the
resulting windows, receipts, and notices before a clean teardown.
Every other verification surface builds on this journey.

## Sub-features

- `aut-doctor` — read-only health probe: spawn, `test.hello` +
  `test.capabilities` handshake, graceful `test.shutdown`, child
  exit — one JSON report, no mutating op.
- `aut-drive-session` — an ordered list of closed-set ops run in a
  single session (spawn → ops → shutdown → child-exit).
- `aut-drive-plan` — `--dry-run` prints the validated plan JSON
  without spawning anything.
- `aut-close-policy` — the `windows` fixture's `close_policy` knob
  exercises veto / defer / unanswered (default-approve) close paths.
- `aut-transport` — stdio is default; loopback TCP with a run-bound
  secret is opt-in via `--transport tcp`.

## How to get to it (user POV)

- Build both binaries:
  `cargo build -p ruviform-ui-test --bin dummy_app --bin rvf-ctl`.
- Canonical lanes: `mise run test-ui-rvf-ctl` (doctor report →
  `evidence/rf805/rvf-ctl-doctor.json`) and
  `mise run test-ui-rvf-ctl-drive` (drive report →
  `evidence/rf805/rvf-ctl-drive.json`).
- Direct: `target/debug/rvf-ctl doctor [--app <path>] [--transport
  stdio|tcp] [--dry-run] [--out <path>]` and
  `target/debug/rvf-ctl drive [same options] <step>...`.

## Driving it with rvf-ctl

Preconditions:

- `dummy_app` and `rvf-ctl` are built (the lane builds them; a
  missing AUT is reported `not_run` + `environment_blocked`).
- `rvf-ctl doctor` reports `verdict: "pass"`.

- **Probe.** Run `rvf-ctl doctor --out evidence/<issue>/rvf-ctl-doctor.json`.
  Exit `0`; the report shows all five checks (`spawn`, `handshake`,
  `capabilities`, `shutdown`, `child-exit`) `pass`, the `welcome`
  (backend `headless-fake`, `windows` alive at hello), and the
  honest capability set (`screenshot:false`, `input_injection:false`,
  `dom_access:false`).
- **Plan.** Run `rvf-ctl drive --dry-run '<step>...'`. Exit `0`; a
  `drive-plan` JSON lists each validated step and its wire params —
  nothing was spawned.
- **Drive a session.** Run
  `rvf-ctl drive 'observe.windows' 'fixture.prepare={"name":"windows","params":{"count":1,"close_policy":"veto"}}' 'observe.windows' --out <report>`.
  The fixture reply's `applied.windows` names the created id; the
  second `observe.windows` shows both windows.
- **Mutate and observe.** Add `'clock.advance={"domain":"host_tick","ticks":2}'`,
  `'fault.set={"kind":"ipc_disconnect","window":1}'`, `'observe.receipts={"since":0,"limit":64}'`,
  `'fault.clear={"kind":"ipc_disconnect","window":1}'`, `'notices'`.
  Tick and suppressed traffic appear in the receipt journal;
  `observe.windows` reports `ipc_down` while the fault is armed.
- **Exercise close policy.** `'window.request_close={"window":"last"}'`
  under `veto` leaves the window alive (`observe.windows` still
  lists it, `close_pending:false`); after
  `'fixture.prepare={"name":"windows","params":{"count":0,"close_policy":"unanswered"}}'`
  the same request destroys it (`window.destroyed` notice).
- **Confirm the unsupported answer.** `'observe.screenshot={"window":"last"}'`
  on `dummy_app` records `unsupported` — the host does not declare
  `screenshot`; that is the honest capability gate, never a failure.
- **Teardown.** The session always ends `test.shutdown` graceful →
  `cleanup{windows_open→windows_closed, exited:true}` →
  `child_exit.success`. Rerun the same drive and require identical
  verdicts — the second read-only observation that no state leaked.

## Gotchas

- `"window":"last"` resolves from session observations (welcome →
  fixture receipt → `observe.windows`). With `--app-args "--windows 0"`
  and no fixture, it is `not_run` — observe or create first.
- Step params validate before spawn: unknown op, bad JSON, unknown
  key, unknown domain/kind, or a non-object param is exit 2 and
  never reaches the wire.
- `dummy_app` ids are assigned by the host — read them from
  `welcome.windows`/`observe.windows`/the fixture `applied.windows`
  receipt rather than assuming a fixed value.
- `--fail-init` (via `--app-args "--fail-init"`) exercises the
  failed half of the handshake; `--stall before-serve|after-serve`
  parks forever — only for timeout-path runs, it blocks the caller.
- `test.shutdown` is the fixed session epilogue, not a step — every
  drive ends with a graceful teardown and a child-exit check.
- The TCP path is also a user path: `--transport tcp` repeats the
  session over loopback with a run-bound secret.
