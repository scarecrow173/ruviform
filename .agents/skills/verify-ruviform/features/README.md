# Ruviform verification map

This directory is the maintained source for verifying ruviform's
user-visible behavior. Read the index before driving, then use the
matching feature file as the recipe.

Ruviform is a foundation repository, so "user-facing feature" means
the surfaces a developer, agent, or consumer repo drives to verify
behavior: the `rvf.uitest.v1` control surface and its AUTs, the
plugin-dock scenario lanes, and the driver-lab semantic journeys.

Status: seeded in `scarecrow173/ruviform#805` slice 2 with five
features covering the headless control surface, the headless
plugin-dock console, the windowed uitest lane, the real plugin-dock
scenario, and the driver-lab journeys. Deferred to a later slice or
the maintenance loop: the interactive `plugin-dock-lab` console, the
webview showcase surface (deliberately left unmapped — it is slated
for removal under `scarecrow173/ruviform#465`, so a map entry would
drift on arrival), and surfaces whose acceptance is still open
(`#629` virtual audio endpoint, `#388`/`#389` NativeSurfaceSlot
L2/L3, `#630` remaining test_editor cases — the files that touch
them record the honest `unsupported`/`not_run` state).

## Baseline preconditions

- `mise install` and `mise run setup` have run (toolchain, hooks,
  APM deps). The mise task layer is the canonical entry — direct
  binary calls are auxiliary evidence, never a substitute.
- Build once per checkout when driving directly:
  `cargo build -p ruviform-ui-test --bin dummy_app --bin rvf-ctl`.
- Run `rvf-ctl doctor` and require `verdict: "pass"` before driving
  the `rvf.uitest.v1` surface.
- Real-window features require an interactive desktop session. On a
  headless host every real-window lane records `not_run` +
  `environment_blocked` and still exits 0 — that fail-closed honesty
  is the designed outcome, never a green run.
- Record results under `evidence/<issue-or-rf>/` with the verdict
  vocabulary of `POLICIES.md` §3.

## Driving conventions

- Start every recipe from the baseline state unless its
  preconditions say otherwise.
- Treat every command as literal — quoted step params, flags, and
  selectors are exact.
- `rvf-ctl` steps are `<op>[=<json params>]` from the closed op set;
  never reach around the set (no eval, shell, file access, or input
  injection exists on this channel).
- Fixtures and manifests (`fixtures/driver-lab/*`, scenario
  catalogs) are read-only seeds — never edit them to make a journey
  pass. A journey that fails reports the failure; the reason goes
  into `Gotchas`.
- Confirm side effects with a second read-only observation, not by
  trusting the mutating op's reply.
- Kill only what the run spawned. Proof artifacts survive cleanup —
  a teardown that eats evidence is a failed run.

## Proof and skip reporting

- Capture the action and the resulting state: the command line,
  exit code, and the structured reply (`welcome`, step `reply`,
  `cleanup`, `child_exit`, verdict rows) — a final output alone is
  not proof.
- A path that cannot run records `not_run` with the reason
  (`environment_blocked` for a missing desktop session or AUT) —
  never report it as `pass`.
- `environment_kind` is claimed only by cases that really ran in
  that environment — `deterministic-backend` for the headless fake,
  `real-window` for real OS windows.
- `unsupported` is an honest capability answer (the backend does
  not declare the capability), not a failure and not a skip.

## Feature entry contract

Each feature file starts with an H1 title and one paragraph
describing the user-visible behavior, then exactly four H2 sections
in this order:

1. `Sub-features` lists short IDs with one line for each behavior.
2. `How to get to it (user POV)` lists every user entry point.
3. `Driving it with <harness>` starts with `Preconditions:` and
   uses labeled bullets pairing each user action with an exact
   command and an observable result.
4. `Gotchas` lists traps that can waste or invalidate a run.

Keep implementation details out of the map: name only user paths,
stable handles, required state, commands, and observable proof.

## Features

- [Drive the headless app-under-test](./headless-aut-drive.md) —
  spawn `dummy_app`, doctor it, and run a composable `rvf.uitest.v1`
  op sequence in one session via `rvf-ctl drive`.
- [Run the headless plugin-dock console](./plugin-dock-console.md) —
  catalog projection plus selector-filtered execution of the
  headless dock cases into JSONL evidence.
- [Drive a real OS window under test](./windowed-uitest.md) — the
  windowed `rvf.uitest.v1` path: real GPUI windows, real capture,
  close-policy drives (interactive desktop required).
- [Run the real plugin-dock scenario](./plugin-dock-real.md) — 22
  real-window dock cases incl. the `test_editor` child HWND and the
  honest `real-dcomp-pixel` verdict (interactive desktop required).
- [Exercise driver-lab semantic journeys](./driver-lab.md) — the
  UI-DVR contract cases and consumer-shaped LAB stories through the
  mise test lanes.
