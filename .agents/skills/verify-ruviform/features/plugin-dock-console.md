# Run the headless plugin-dock console

`plugin-dock-console` is the headless half of the plugin-dock Test
Lab: it executes the `rf631.plugin-dock` headless catalog — view
mount, command receipt, real `test_gain` artifact identity and host
lifecycle, an honest `editor-attach` attempt, session lifecycle,
terminate, open-or-degrade, and dock pixels — with selector
filtering and JSONL evidence. A developer uses it to prove the
headless dock surface without opening a window.

## Sub-features

- `dock-list` — `--list` prints the capability projection of the
  catalog (one `catalog` row per cell) — a declaration, never
  execution evidence.
- `dock-run` — `--run` executes the selected headless cells and
  renders `run` + `case` JSONL rows with honest verdicts.
- `dock-select` — repeatable `--case`/`--scenario`/`--driver`
  selectors (OR within a flag, AND across flags) run a subset of
  the catalog.
- `dock-artifact` — the `artifact` case resolves the real
  `test_gain` VST3 cdylib and records its SHA-256 digest;
  artifact-dependent cases fail closed to `not_run` when it is
  absent.

## How to get to it (user POV)

- Canonical lane: `mise run test-ui-plugin-dock-console` — builds
  `ruviform-plugin-test-vst3`, prints `--list`, then `--run` writes
  `evidence/rf631/plugin-dock-console.jsonl`.
- Direct:
  `target/debug/plugin-dock-console --list [--include-real]` and
  `target/debug/plugin-dock-console --run [--case <id>]...
  [--scenario <id>]... [--driver <name>]... [--out <path>]`
  (build first: `cargo build -p ruviform-plugin-test-vst3 -p
  ruviform-plugin-dock-test`).

## Driving it with plugin-dock-console

Preconditions:

- The `test_gain` cdylib exists (`cargo build -p
  ruviform-plugin-test-vst3`) — the lane builds it; a direct run
  without it degrades the probe cases honestly.
- Baseline: `mise run test-ui-plugin-dock-console` exits `0` on a
  healthy checkout.

- **Project the catalog.** Run `plugin-dock-console --list`. Exit
  `0`; nine `type:"catalog"` rows describe the headless cells and
  their driver/capability columns — projection only, record it as
  context, not as execution evidence.
- **Run the full headless set.** Run `plugin-dock-console --run
  --out evidence/<issue>/plugin-dock-console.jsonl`. Exit `0` when
  no `fail` verdict; the JSONL holds one `run` header plus a `case`
  row per cell — `view-mount`, `command-receipt` (fake driver);
  `artifact`, `host-lifecycle`, `editor-attach`,
  `session-lifecycle`, `host-terminate`, `open-or-degrade`,
  `dock-pixels` (plugin-host-probe driver).
- **Filter.** Run `plugin-dock-console --run --case view-mount
  --driver fake`. Only matching cells execute; the `lane` field
  records `plugin-dock-console` so a filtered run is never mistaken
  for full-lane evidence.
- **Confirm side effects.** Re-run the same `--run` and diff the
  verdict rows — identical verdicts prove the lane leaves no
  residue. `evidence/` JSONL files must survive cleanup.
- **Read the honest rows.** `editor-attach` reports `unsupported`
  (`createView` is null — `test_gain` has no editor surface) and
  `dock-pixels` reports `not_run` (no docked editor surface, no
  capture) — both are recorded outcomes, not lane failures; exit
  stays `0` while no `fail` exists.

## Gotchas

- `--list` is a capability projection — never cite it as execution
  evidence; only `--run` rows carry verdicts.
- `--include-real` requires the `real-dock` feature build; without
  it the flag is refused with exit `2` rather than silently
  omitting rows.
- `real-*` selectors (and the `real-window` driver / `rf631.plugin-dock-real`
  scenario) have no console execution path — they are refused with
  exit `2` and a pointer to `mise run test-ui-plugin-dock-real`.
- A selector matching zero headless cells is exit `2`, not an empty
  success run.
- `1` is a `fail` verdict or an output-write failure — investigate
  the `case` row's `detail`, not just the exit code.
- `editor-attach` stays `unsupported` until a real editor fixture
  exists (`scarecrow173/ruviform#630` remainder) — do not pass it
  forward as verified.
