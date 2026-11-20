# Exercise driver-lab semantic journeys

The driver-lab fixtures are the toolkit-free semantic surface: nine
`UI-DVR-*` contract cases plus two consumer-shaped stories
(`LAB-SV-FADER-DOCK`, `LAB-AL-WAVEFORM-SEEK`) prove the
driver-neutral layer — ViewSpec mount/reconcile, gesture
Begin/Preview/Commit/Cancel, theme resolution parity, dock 2-phase
move/rollback, NativeSlot semantics, capability honesty, teardown,
and authoritative projection — through the fake layer and its GPUI
parity mirror. A developer runs these to verify semantics without
any OS window or toolkit.

## Sub-features

- `lab-contract` — `cases.json`/`stories.json` manifest ↔ mechanism
  consistency plus the executable UI-DVR fake-layer fixtures
  (`mise run test-ui-driver-contract`).
- `lab-component-view` — UI-DVR-02/03 and both LAB stories on the
  fake layer plus the two-theme equivalence proof
  (`mise run test-ui-component-view`).
- `lab-parity` — the shared `parity/new-kinds.json` fixture running
  identical semantics on Fake and GPUI drivers
  (`mise run test-ui-parity` / `test-ui-gpui`).
- `lab-native-slot` — `boundary.json`'s NativeSurfaceSlot contract
  fixtures: attach/hide/resize/clip/z/focus, attach-fault rollback,
  stale/forged leases, teardown (`mise run test-ui-native-slot`).

## How to get to it (user POV)

- `mise run test-ui-driver-contract` — `cargo test -p
  ruviform-ui-test --test driver_contract`.
- `mise run test-ui-component-view` — `cargo test -p
  ruviform-ui-test --test component_view`.
- `mise run test-ui-parity` — `cargo test -p ruviform-ui-test
  --test driver_parity` (GPUI side: `mise run test-ui-gpui`).
- `mise run test-ui-native-slot` — `cargo test -p
  ruviform-ui-test --test native_slot_contract`.
- The boundary guard `mise run check-ui-driver-boundary` proves no
  GPUI/Win32 type leaks into the driver-neutral surface.

## Driving it with the mise lanes

Preconditions:

- A built workspace (`mise install` + `mise run setup` once). All
  lanes write only `target/` — they are headless and CI-safe.
- The fixtures under `crates/ruviform-ui-test/fixtures/driver-lab/`
  (`cases.json`, `stories.json`, `boundary.json`, `parity/`) are
  read-only seeds — never edit them to make a journey pass.

- **Run the contract.** `mise run test-ui-driver-contract`. Exit
  `0`; `manifests_are_consistent` plus the executable UI-DVR-01/04/
  05/06/07/08 fixtures all pass — a manifest whose named mechanism
  is missing fails this lane.
- **Run the stories.** `mise run test-ui-component-view`. Exit
  `0`; `LAB-SV-FADER-DOCK` and `LAB-AL-WAVEFORM-SEEK` complete their
  expected lab-story traces, and the same semantic mount resolves
  identically under both product themes.
- **Run the parity mirror.** `mise run test-ui-parity`. Exit `0`;
  the shared `new-kinds.json` produces the same semantic outcomes
  on the fake driver (the GPUI mirror lives under `test-ui-gpui`).
- **Run the slot contract.** `mise run test-ui-native-slot`. Exit
  `0`; attach/rollback/lease/teardown semantics pass on the fake
  slot port — it never claims a real-dock pass.
- **Evidence.** These lanes are `cargo test` surfaces; record the
  command + exit code, and any `RUVIFORM_DRIVER_LAB_EVIDENCE` output
  the contract lane emits, under `evidence/<issue>/`.

## Gotchas

- `cases.json`/`stories.json` are manifests: a case without its
  named mechanism (cargo test or mise task) fails the contract —
  add the mechanism, never the manifest.
- Parity compares Fake and GPUI through the shared fixture — a
  divergence is a real semantic bug, not a fixture to relax.
- `test-ui-component-view` pins the two-theme equivalence; a third
  theme fixture is a new story, not a tweak of the existing two.
- NativeSlot fixtures prove slot-port semantics only — real-dock
  L2/L3 acceptance (`scarecrow173/ruviform#388`/`#389`) is a
  separate, still-open surface; fake-layer `pass` must not be
  reported as dock acceptance.
- `check-ui-driver-boundary` is part of the same surface's honesty:
  a leaked GPUI/Win32 type into the driver-neutral API fails it —
  run it after touching `ruviform-ui-test` public items.
