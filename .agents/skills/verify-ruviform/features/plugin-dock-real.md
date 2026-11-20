# Run the real plugin-dock scenario

The real-window half of the plugin-dock surface: 22 catalog cases
driven through `RealOsGuiTest` — real OS windows, a NativeSlot →
container-HWND lease → `ParentSlotLease` → `InProcessEditorOps`
redeem → `session.open(Docked)` path, a real `test_editor` child
HWND, multi-window focus, real OS input, live-geometry DPI, armed
close, honest `unsupported` for IME, session teardown/crash rows,
and the `real-dcomp-*` occlusion-bisect evidence. It requires an
interactive desktop session; `real-dcomp-pixel` currently reports a
honest `fail` (regression signal `scarecrow173/ruviform#532`), so
lane exit `1` is the designed signal, not a bug in the lane.

## Sub-features

- `dock-real-run` — `plugin-dock-real-scenario` runs the full
  real-window catalog and writes
  `evidence/rf631/plugin-dock-real-scenario.jsonl`.
- `dock-real-editor` — `real-editor-attach` redeems the
  `test_editor` descriptor: a real `IPlugView` creates a child HWND
  under the container lease (OS-observed attach → detach).
- `dock-real-capture` — `real-capture` reads window/slot-region
  pixels; `real-dcomp-*` cases observe the docked editor child's
  compositor pixels as occlusion-bisect evidence.
- `dock-real-remainder` — 2-window focus transfer, real OS input,
  command receipt, live-geometry DPI verdict, armed-lifecycle
  close, resize/re-dock oracles, and honest `unsupported` for IME.
- `dock-real-headless` — `--blocked-probe` regenerates the
  session-start-failure reports on any host, proving every catalog
  row maps to `not_run` + `environment_blocked` (fail-closed).

## How to get to it (user POV)

- Canonical lane (interactive desktop required):
  `mise run test-ui-plugin-dock-real` — builds
  `ruviform-plugin-test-vst3`, runs `plugin-dock-real-scenario
  --out evidence/rf631/plugin-dock-real-scenario.jsonl`.
- Fail-closed proof on any host:
  `mise run test-ui-plugin-dock-real-headless` — runs
  `--blocked-probe`, asserts all rows are `not_run` +
  `environment_blocked`, exit `1` on violation; output
  `evidence/rf631/plugin-dock-real-headless.jsonl`.
- Direct:
  `cargo run -p ruviform-plugin-dock-test --features real-dock --bin
  plugin-dock-real-scenario -- [--blocked-probe] --out <path>`.

## Driving it with the mise lane

Preconditions:

- An interactive desktop session — a session that cannot start
  fails closed: every case records `not_run` +
  `observed.environment_blocked`, exit `0`.
- `ruviform-plugin-test-vst3` builds (the lane builds it —
  `test_gain` for probe cases, `test_editor` for the real-editor
  rows).
- Run unattended: the lane owns real OS windows and real input for
  its duration.

- **Run the catalog.** `mise run test-ui-plugin-dock-real`. Expect
  exit `1` while `real-dcomp-pixel` reports its honest `fail` —
  check `evidence/rf631/plugin-dock-real-scenario.jsonl`: one `run`
  header plus 22 `case` rows, `environment_kind` claimed only by
  cases that really ran in a real window.
- **Read the verdicts.** `real-window-mount`, `real-native-slot`,
  `real-editor-attach`, focus/input/close/resize/re-dock/session
  rows carry their observed verdicts; `real-window-ime` and any
  missing-adapter rows stay `unsupported`; `real-dcomp-pixel` is the
  known `fail` (#532) — record it verbatim, never reword.
- **Confirm fail-closed.** `mise run
  test-ui-plugin-dock-real-headless` on the same host — exit `0`;
  the probe JSONL shows the exact `not_run` +
  `environment_blocked` mapping the lane would emit without a
  session.
- **Cleanup.** The lane's teardown closes its windows; confirm no
  `plugin-dock-real-scenario`/AUT processes remain and the JSONL
  output file survived.

## Gotchas

- Exit `1` is the designed honest signal while `real-dcomp-pixel`
  fails — never weaken it to `not_run`, never gate it out, and
  never report the lane green; `#532`'s compositor/presentation
  work owns that verdict.
- Headless hosts produce all `not_run` + `environment_blocked`,
  exit `0` — an environment verdict, never execution evidence.
- `environment_kind:"real-window"` may be claimed only by rows that
  actually ran — projection or skipped rows must not claim it.
- Editor-pixel dock acceptance stays deferred: `real-dcomp-*` rows
  are occlusion-bisect evidence, not a dock PASS.
- `#630` remainder (test_editor surface beyond `real-editor-attach`)
  and `#388`/`#389` L2/L3 native-dock acceptance are open — rows
  touching them keep `unsupported`/`not_run`; do not promote.
- `test-ui-real-os-scenario` is a different lane (real cursor/
  keyboard occupation) and is not part of this feature's drive.
