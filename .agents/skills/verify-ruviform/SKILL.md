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

`rvf-ctl drive` is the composable op surface: one invocation is one
session — spawn → `test.hello` → the ordered step list → graceful
`test.shutdown` → child-exit. Each positional argument is one op
written `<op>[=<json params>]` (the `test.` prefix is optional):

```sh
# canonical lane (builds both binaries, writes evidence)
mise run test-ui-rvf-ctl-drive

# direct (auxiliary evidence — canonical = mise)
target/debug/rvf-ctl drive \
  'capabilities' \
  'observe.windows' \
  'fixture.prepare={"name":"windows","seed":7,"params":{"count":1,"close_policy":"veto"}}' \
  'observe.windows' \
  'clock.advance={"domain":"host_tick","ticks":2}' \
  'observe.receipts={"since":0,"limit":64}' \
  'fault.set={"kind":"ipc_disconnect","window":1}' \
  'fault.clear={"kind":"ipc_disconnect","window":1}' \
  'notices' \
  'window.request_close={"window":"last"}' \
  'observe.windows' \
  'observe.screenshot={"window":"last"}' \
  --out evidence/rf805/rvf-ctl-drive.json

target/debug/rvf-ctl drive --dry-run '<step>...'   # plan only — spawns nothing
target/debug/rvf-ctl drive --app target/debug/windowed_test_app \
  --app-args "--windows 1" '<step>...'             # a different AUT
```

Step params are validated before anything spawns — a bad JSON
object, unknown param key, unknown clock domain (known:
`host_tick`), or unknown fault kind (known: `window_crash`,
`ipc_disconnect`) is exit 2 and never reaches the wire. Window
selectors accept a u64 id or `"last"` — the most recently observed
or created window (welcome → fixture receipt → `observe.windows`),
so one session can prepare a fixture and then drive the window it
created.

Per-step verdicts use the shared vocabulary: `pass` for an `ok`
answer, `unsupported` for an `unavailable` answer (capability
gating — e.g. `observe.screenshot` on `dummy_app`, whose declared
`screenshot` is `false`), `fail` for any other refusal, `not_run`
for a step that never reached the wire (unresolvable `"last"`, or a
channel already dead). Async `Notify` events buffered while an op
ran attach to that step's `notices`. The report shape mirrors
`doctor`: `type:"drive"`, `lane:"rvf-ctl-drive"`, `steps[]`,
`checks[]` for spawn/handshake/shutdown/child-exit, `welcome`,
`cleanup`, `child_exit`, `environment`, `repo.sha`, `verdict`, and
`error{stage,detail,next_step}` on failure. Exit `0` = every check
and step passed (`unsupported` is an honest answer, not a failure);
`1` = a `fail`, a step left `not_run`, or AUT-missing `not_run` +
`environment_blocked`; `2` = CLI misuse.

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

The `features/` map (`./features/README.md`) normalizes the
user-visible surfaces these ops drive — read it before composing a
drive sequence so the steps exercise a real user path rather than a
convenient subset.

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
target/debug/rvf-ctl drive [options] <step>...

  --app <path>        AUT binary (default: sibling dummy_app)
  --app-args "<s>"    extra AUT argv, whitespace-split
                      (e.g. "--windows 2", "--fail-init")
  --transport <t>     stdio (default) | tcp
  --dry-run           print the plan JSON, spawn nothing
  --out <path>        write the report to a file instead of stdout
  --help, -h          usage + exit codes + next steps

rvf-ctl --help        # same usage; exit 0
```

Canonical lanes: `mise run test-ui-rvf-ctl` (doctor report →
`evidence/rf805/rvf-ctl-doctor.json`), `mise run test-ui-rvf-ctl-drive`
(drive report → `evidence/rf805/rvf-ctl-drive.json`).

The feature map under `./features/` (`README.md` index + one file per
user-visible surface) records what each drive sequence is meant to
prove — drive against it, not against the op list alone.

## Bench

Frozen DSP SIMD benchmark + regression gate (Issue #807):

```sh
mise run bench-frozen                          # gated real run vs pinned baseline
mise run bench-frozen --test                   # 1-rep smoke (report_only; perf-gate never blocks — output_mismatch still exits 1)
mise run bench-frozen --selftest               # inject degraded dispatch -> gate must fail
mise run bench-frozen --write-baseline <path>  # emit an un-promoted baseline candidate
```

- Measures the production `ruviform-math::dsp` kernels (`dot`, `fir`) via
  an in-process A/B: `SimdToken::force_fallback()` vs `SimdToken::detect()`,
  alternating, 7 reps, median + min/max per-call ns. The gate metric is the
  A/B time ratio against `min_ratio` thresholds pinned in
  `crates/ruviform-math/benches/frozen/dsp_baseline.json` — fail-closed
  (unreadable/un-promoted baseline, below-threshold ratio, or output
  mismatches vs the scalar reference all fail the run mode).
- The pinned baseline is read-only during gating. Updating it is two-phase:
  `--write-baseline` emits a candidate with `"approval": null`; a human
  promotes it by copying it to the pin path with `approval` filled. The gate
  refuses un-promoted candidates.
- Evidence follows the `evidence/<issue>/` convention: JSONL `run`/`cell`/
  `gate` records (verdict vocabulary `pass`/`fail`/`not_run`/`unsupported`),
  `--selftest` gate-fail proof, the candidate and promoted pin, plus
  `decision.tsv` (hillclimb schema) and `verification.md`. Contract:
  `docs/test-specs/issue-807-frozen-dsp-bench.md`.
- This lane is manual/opt-in only — the CI `bench` job keeps calling
  `bench-smoke`; do not wire `bench-frozen` into CI.

Frozen E2E pipeline benchmark + regression gate (Issue #807, slice 2):

```sh
mise run bench-frozen-e2e                          # gated real run vs pinned baseline
mise run bench-frozen-e2e --test                   # 1-rep smoke (report_only; perf-gate never blocks — correctness failures still exit 1)
mise run bench-frozen-e2e --selftest               # inject wall-floor + Xrun degradations -> gate must fail
mise run bench-frozen-e2e --write-baseline <path>  # emit an un-promoted baseline candidate
```

- Measures the deterministic production audio-pipeline legs of
  `ruviform-audio-test` end to end (no mocks in the measured path):
  `output/session` (scripted output → `Session::open` → `pump_output_next`
  fires `plan.drive` → captured PCM) and `input/typed-rt` (scripted input →
  `InputCapture` → `RtRuntime` `input_writer`/`process`/`output_reader`),
  warmup + 7 reps of 64×256-frame callbacks each.
- Two gate layers, both fail-closed against
  `crates/ruviform-audio-test/benches/frozen/pipeline_baseline.json`:
  (a) normalized wall `norm = median(rep ns) / median(ref ns)` (an
  in-process mix64 yardstick, so the metric is machine-speed independent)
  vs per-cell `max_norm_ratio`; (b) correctness — oracle PCM mismatches,
  device errors, capture-pool loss, pipeline errors, rep-over-rep digest
  divergence, and the pinned `expected_digest` fingerprint of observed
  output. Any correctness count > 0 fails the cell before the threshold.
- Deterministic scripted backend = OS-L1 evidence only — not
  WASAPI/WinMM, real threads, GUI, or plugin-ABI evidence. The callback
  firing is manual; driver orchestration is non-RT (the RT alloc-free
  contract inside the callback is proven by `rt_pipeline_*` tests).
- Baseline is read-only during gating; `--write-baseline` emits an
  `"approval": null` candidate, promotion is manual (same two-phase rule
  as `bench-frozen`). Evidence under `evidence/<issue>/`; contract:
  `docs/test-specs/issue-807-e2e-pipeline-bench.md`.
- Manual/opt-in only — the CI `bench` job keeps calling `bench-smoke`;
  `ruviform-audio-test` benches are not part of `bench-smoke` either.

## Formal

D-layer property + bounded-model-check lanes (Issue #806 — slice 1
wire codecs `rvf.ipc.v1` / `rvf.uitest.v1`, slice 2 archive parsers
`ruviform-io-archive`):

```sh
mise run test-formal        # proptest: formal_ipc + formal_transport + formal_archive
mise run test-formal-kani   # Kani bounded proofs (native, else WSL2)
```

- Test files live next to the crates: `crates/ruviform-ui/tests/
  formal_ipc.rs` (`parse_intent`, `valid_op`, `Event::to_json`),
  `crates/ruviform-ui-test/tests/formal_transport.rs` (`read_frame` /
  `write_frame`), and `crates/ruviform-io-archive/tests/
  formal_archive.rs` (TAR/ZIP reader `ArchiveReadLimits` gates,
  `ArchiveErrorKind` stable classification, receipt invariants, and
  `Pending`/`discard_pending` recovery). Kani proofs sit in a
  `#[cfg(kani)] mod proofs` at the bottom of the same file — never in
  product sources. The archive proofs use a concrete skeleton plus a
  symbolic gate field (typeflag byte / limit budget); arbitrary byte
  streams stay concrete-only in proptest.
- `test-formal` is the always-runnable first layer (pure Rust, MSRV,
  same entry as CI). `test-formal-kani` resolves an environment in
  order: native `cargo kani` → a WSL2 distro with `cargo` +
  `cargo kani` (Windows path mapped to `/mnt/<drive>/`, builds in a
  WSL-native target dir, never through `/mnt`) → `not_run` +
  `environment_blocked` + exit 0. A real CBMC counterexample is `fail`
  + exit 1; environment absence is never reported as `pass`.
- Bounds and assumptions (what is proven vs unproven, float 1-ulp
  semantics, cap-path complementarity between proptest and Kani) are
  the contract of record: `docs/specs/formal-verification.md`. Keep it
  in sync when harness bounds change.
- Proptest regression files (`*.proptest-regressions` next to a test)
  are committed — they pin historically failing seeds so every run
  replays them first.
- Mutation-fail proof is required evidence: temporarily break one codec
  invariant, run the property suite, record the shrunk failure, revert
  completely, and re-run green. The mutant / observed failure / revert
  triple goes into `evidence/<issue>/` — a suite that cannot fail is
  not a gate.
- Verdict vocabulary and the local-vs-integration distinction follow
  the `## Evidence` rules above; `verification.md` records the Kani
  toolchain (verifier + CBMC versions) alongside OS/arch/repo SHA.
