---
name: verify-ruviform
description: Verify ruviform UI surfaces (headless `dummy_app` AUT over `rvf.uitest.v1`, `plugin-dock-console` lanes, real-window GPUI/VST3 lanes) with the `rvf-ctl` control CLI and lane evidence. Use when asked to launch the app-under-test, run a health doctor, drive UI test cases, or record `evidence/<issue>/` verification for ruviform.
---

# verify-ruviform

Project-local verification skill for `scarecrow173/ruviform`. It covers
two surfaces:

- **Headless control surface** — the `dummy_app` app-under-test (AUT)
  speaking the `rvf.uitest.v1` protocol (`PROTOCOL_VERSION = 1`) over
  length-prefixed JSON frames, driven by `TestDriver` in
  `crates/ruviform-ui-test` or by the `rvf-ctl` CLI.
- **Lane surface** — `plugin-dock-console` (`crates/ruviform-plugin-dock-test`)
  and the mise `test-ui-*` lanes that produce `evidence/<issue>/*.jsonl`.

Verdict vocabulary everywhere: `pass` / `fail` / `not_run` /
`unsupported` / `blocked`. Never report a check that did not execute as
`pass`; a missing environment is `not_run` + `environment_blocked`,
never fake-`pass`ed (POLICIES.md §3: Required/Deferred/NotApplicable
requirement state × Pass/Fail/NotRun/Unsupported result).

## Launch

Headless AUT (`rvf.uitest.v1` server):

```sh
# build the AUT and the control CLI (same target dir — rvf-ctl finds
# dummy_app as its sibling binary by default)
cargo build -p ruviform-ui-test --bin dummy_app --bin rvf-ctl
```

The AUT only serves when a driver opts it in — `RVF_TEST_CONTROL`
(`stdio` | `tcp`) plus `RVF_TEST_ADDR` + `RVF_TEST_SECRET` for TCP —
and `TestDriver::spawn_stdio` / `spawn_tcp` sets all of that for you.
Running `target/debug/dummy_app` directly with no env exits
immediately (`AgentTransport::Off`); `RVF_TEST_DISABLE=1` refuses every
serve path (fail-closed kill switch). Useful AUT fixtures:
`--fail-init` (report a failed ready state), `--windows N`,
`--stall before-serve|after-serve` (never exits — for timeout paths).

**Ready check:** the driver sends `test.hello`; the AUT answers a
`Welcome` carrying `v`, `session_id`, `run_id`, `app`,
`ruviform_revision`, `backend` (`headless-fake` for `dummy_app`),
`capabilities`, `windows`, `os`, `arch`. A second `test.hello` is a
protocol error.

**Teardown:** send `test.shutdown` (graceful) → `CleanupReport`
(`windows_open`/`windows_closed`/`exited`/`app`), then confirm the
child process exits. `TestDriver`'s `Drop` kills and reaps a child that
is still alive, so a dropped driver never leaves an orphan.

Real-window entries (interactive desktop session required):

```sh
mise run test-ui-windowed-uitest     # windowed_test_app AUT, RVF_WINDOWED_UITEST=1, --test-threads=1
mise run test-ui-plugin-dock-real    # real plugin dock scenario
mise run test-ui-plugin-dock-lab     # interactive lab harness
```

On a headless/non-interactive host these lanes report every case
`not_run` + `environment_blocked` and still exit 0 — that fail-closed
honesty is by design, not a green run.

## Doctor

Read-only health probe — spawn, handshake, capabilities, graceful
shutdown, child exit. It never sends a fixture/clock/fault/window op.

```sh
# canonical (builds both binaries, writes evidence)
mise run test-ui-rvf-ctl

# direct (auxiliary evidence — canonical = mise)
target/debug/rvf-ctl doctor                                # stdout JSON
target/debug/rvf-ctl doctor --out evidence/rf805/rvf-ctl-doctor.json
target/debug/rvf-ctl doctor --transport tcp                # loopback + run-bound secret
target/debug/rvf-ctl doctor --app target/debug/dummy_app   # explicit AUT
target/debug/rvf-ctl doctor --dry-run                      # plan only — spawns nothing
```

Report: `type:"doctor"`, `lane:"rvf-ctl-doctor"`, `checks[]` for
`spawn`/`handshake`/`capabilities`/`shutdown`/`child-exit`, `welcome`,
`capabilities`, `cleanup`, `child_exit`, `environment`, `repo.sha`,
and `verdict`. Exit `0` = all checks pass; `1` = `fail`, or `not_run`
+ `environment_blocked` when the AUT binary is missing; `2` = CLI
misuse. Failures carry `error.stage` and `error.next_step`
(e.g. missing binary → `cargo build -p ruviform-ui-test --bin dummy_app`).

## Drive

The `rvf.uitest.v1` closed op set (`crates/ruviform-ui-test/src/proto.rs`):

| Op | Mutating? |
| --- | --- |
| `test.hello`, `test.capabilities`, `test.shutdown`, `test.notices` | no (hello/shutdown are session bounds) |
| `test.observe.windows`, `test.observe.receipts`, `test.observe.screenshot` | no — observation only |
| `test.fixture.prepare`, `test.clock.advance`, `test.fault.set`, `test.fault.clear`, `test.window.request_close` | yes — drive ops |

Transport: stdio is the default; loopback TCP is opt-in via
`RVF_TEST_CONTROL=tcp` + `RVF_TEST_ADDR` + `RVF_TEST_SECRET` (the
driver owns the listener and the run-bound secret — wrong secret ⇒
unauthenticated). Frames are length-prefixed JSON.

Headless drive lane (`plugin-dock-console`):

```sh
# capability projection — NOT execution evidence
target/debug/plugin-dock-console --list
target/debug/plugin-dock-console --list --include-real

# real execution — JSONL evidence
target/debug/plugin-dock-console --run --out evidence/<issue>/plugin-dock-console.jsonl
target/debug/plugin-dock-console --run --case view-mount --driver fake
target/debug/plugin-dock-console --run --scenario rf631.plugin-dock

# canonical lane (builds the test_gain VST3 cdylib first)
mise run test-ui-plugin-dock-console
```

`--run` exit code: `0` no `fail` verdicts · `1` a `fail` verdict or an
output write failure · `2` CLI misuse / a selector matching zero cases.
`real-*` selectors have no console driver path and are refused.

Real-window drive (interactive session required — same
environment_blocked honesty as above): `windowed_test_app` speaks the
same `rvf.uitest.v1` protocol over its windowed path
(`mise run test-ui-windowed-uitest`); `plugin-dock-lab` opens a GPUI
window; `mise run test-ui-plugin-dock-real` drives the real dock
scenario.

## Evidence

Convention: `evidence/<issue-or-rf>/` per issue (`evidence/rf631/` is
the precedent). Two artifact kinds:

- **JSONL** — one JSON object per line; line `type` is `run`, `case`,
  or `catalog`; every line carries `lane`, a verdict from the shared
  vocabulary, and `environment{os,arch,scale_factor,environment_kind}`.
  `environment_kind` is claimed only by cases that really ran in that
  environment — never by projection or skipped rows.
- **`verification.md`** — human-readable record: date, OS, toolchain,
  repo SHA, a lane-result table, and honest-verdict notes. See
  `evidence/rf631/verification.md`.

Proof standard: evidence must show a real user path driven end to end
— the actions taken *and* the resulting state (Welcome JSON, verdicts,
cleanup report, surviving files). A final output alone is not proof.
Side effects are confirmed by a second read-only observation after the
run (files still present, no leftover processes). Distinguish
change-scope local verification from integration verification on the
merged SHA.

## Cleanup

Remove only what you started:

- `test.shutdown` returns a `CleanupReport` — `exited: true` means the
  AUT tore itself down; the driver then observes the child's exit
  status.
- `TestDriver::Drop` kills and reaps a still-running child — a stalled
  AUT never becomes an orphan.
- `rvf-ctl doctor` kills the child itself when `child-exit` fails, and
  reports it in `checks[]`.
- Never `kill` by process name; kill only PIDs you spawned.
- Never delete evidence — `evidence/<issue>/` proof artifacts must
  survive teardown.
- `mbx`/`target` build artifacts are managed — do not redirect or
  hand-clean them.

## Helpers

`rvf-ctl` — the control CLI (`crates/ruviform-ui-test/src/bin/rvf_ctl.rs`,
test-only; not shipped with the product):

```sh
target/debug/rvf-ctl doctor [options]
  --app <path>        AUT binary (default: sibling dummy_app)
  --transport <t>     stdio (default) | tcp
  --dry-run           print the plan JSON, spawn nothing
  --out <path>        write the report to a file instead of stdout
  --help, -h          usage + exit codes + next steps

rvf-ctl --help        # same usage; exit 0
```

This slice ships `doctor` only; drive/evidence subcommands are later
slices. The `features/` map referenced by the Issue is likewise a later
slice — this skill is complete for the headless subset without it.
