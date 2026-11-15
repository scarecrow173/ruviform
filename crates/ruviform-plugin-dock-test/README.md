# ruviform-plugin-dock-test

Test-only plugin-dock scenario harness (Issue #631 first slice,
RF-VTEST-04). **Not a shipping product** — no production crate may
depend on this crate; it is the consumer-layer composition root that
can see both the plugin side (`ruviform-plugin-*`) and the UI side
(`ruviform-ui-*`), which may not depend on each other.

## What it does

Runs the `rf631.plugin-dock` scenario cases through the
driver-neutral `ruviform-ui-test` contract (`ScenarioSpec` →
`run_scenario` → `ScenarioReport`) against the **real** `test_gain`
VST3 cdylib built by `ruviform-plugin-test-vst3`:

| case | driver | verdict today |
| --- | --- | --- |
| `view-mount` | `Fake` | `Pass` — semantic `NativeSlot` spec mounts (contract conformance only) |
| `artifact` | `PluginHostProbe` | `Pass` — resolve + SHA-256 + `Vst3Library::load` + descriptor |
| `host-lifecycle` | `PluginHostProbe` | `Pass` — Scan → Instantiate → Initialize → Activate(48 kHz, 1..=512) → Destroy |
| `editor-attach` | `PluginHostProbe` | `Unsupported` — `createView` null → `HostError::Unsupported` (fixture has no editor) |
| `session-lifecycle` | `PluginHostProbe` | `Pass` — begin → `SessionBusy` → refused `open(Docked)` → adapter-free `close` converge → `end_session` → slot/generation-bumped re-begin → Destroy (Issue #389) |
| `host-terminate` | `PluginHostProbe` | `Pass` — `Destroy` → `notify_terminated(HostLost)` → `Terminated{HostLost}` exactly once, second notify no-op (Issue #389) |
| `open-or-degrade` | `PluginHostProbe` | `Pass` — `open_or_degrade(Docked→Floating)` double refusal → `Failed{Unsupported}`, zero `Degraded`/attach-family events (Issue #389) |
| `dock-pixels` | `PluginHostProbe` | `NotRun` — no surface, no `VisualCapture` driver |

Verdict honesty is the point: only steps that really executed against
the binary may `Pass`; editor attach/dock/pixels stay
`Unsupported`/`NotRun` until a real editor fixture exists. A missing
artifact fails closed as `NotRun`.

## Scenario catalog

The eight cases are declared once in `HEADLESS_MATRIX`
(`scenario::HEADLESS_MATRIX`) — a driver-independent
`ScenarioMatrix`/`ScenarioEntry` table from `ruviform-ui-test`'s
`scenario::catalog` module. `run_all` is the select-all case of
`run_selected`, which sequences each selected entry's `run_entry`
call in catalog order; the lane binary, the integration tests, and
the `plugin-dock-console` binary read the same table. The
`real-dock` lane keeps its own session-scoped catalog
(`real_entries` in `real_dock`) consumed by both the live session
path and the environment-blocked fail-closed path; its metadata-only
projection lives in `real_catalog` (GPUI-free, available in every
build) so the console can name real selectors without compiling the
lane.

`ScenarioMatrix::project` answers "would `run_scenario` pre-flight
pass this entry on this driver" from declared capabilities only — it
executes nothing and upgrades no verdict.

## Artifact resolution

1. `RUVIFORM_PLUGIN_DOCK_TEST_VST3` — explicit file path (env only);
2. `current_exe()`'s `deps/` directory;
3. `current_exe()`'s own directory.

File name: `{DLL_PREFIX}ruviform_plugin_test_vst3{DLL_SUFFIX}`.

## Lane

```sh
mise run build ruviform-plugin-test-vst3   # build the cdylib
mise run test-ui-plugin-dock               # run the lane → evidence/rf631/
```

The lane is manual/opt-in — it is not a CI-required gate. JSONL
evidence lands in `evidence/rf631/plugin-dock-scenario.jsonl`.

## Real-window dock lane (`real-dock` feature, opt-in)

With `--features real-dock` the crate additionally links
`ruviform-ui-gpui` (`real-scenario`) and exposes the
`plugin-dock-real-scenario` binary, which runs the
`rf631.plugin-dock-real` cases through `RealOsGuiTest` on **real OS
windows**:

| case | verdict intent |
| --- | --- |
| `real-window-mount` | `Pass` only when the `plugin_dock_view_spec()` mount and target resolution really ran on a live window |
| `real-native-slot` | `Pass` only for steps that executed: declare → bind → renderer projection → `lease_parent` (real container HWND via `GpuiSurfaceAdapter`) → validate → `container_token` observation → stale/forged lease rejection → release |
| `real-editor-attach` | `Pass` only when `test_editor`'s real `IPlugView` attaches a real child HWND under the leased container (OS-observed: `EnumChildWindows`/`GetParent`/`IsWindowVisible`) and `session.close` destroys it (`IsWindow(child)==0`, events `[Attached{Docked}, Closed]`); observation mismatch → `Fail`, defined refusal → `Unsupported` |
| `real-capture` | window/slot-region pixels only — detail states the capture target is not plugin-editor pixels |

The chain recorded is the production one:
`NativeSlot → GpuiSurfaceAdapter (実 container HWND) → lease_parent →
ParentSlotLease → InProcessEditorOps redeem → session.open(Docked)`.
`ParentSlotLease` keeps its opaque `u64` payload; only the canonical
`SharedSurface` ledger redeems it. `ScenarioCapability::NativePluginDock`
is reported only when the surface backend can mint a real container
(`surface::BACKEND_AVAILABLE && !gpui-test`).

Headless/non-interactive hosts fail closed: every case records `NotRun`
with an environment-blocked detail and exit code stays 0 (only `Fail`
fails the process). GPUI types never cross this crate's API or evidence
schema.

```sh
mise run test-ui-plugin-dock-real   # builds test-plugin cdylib + runs the lane
cargo run -p ruviform-plugin-dock-test --features real-dock \
  --bin plugin-dock-real-scenario -- --out evidence/rf631/plugin-dock-real-scenario.jsonl
```

Manual/opt-in — not a CI-required gate; requires an interactive
desktop session.

## Headless scenario console (`plugin-dock-console`)

`plugin-dock-console` is the headless consumer of the merged catalog —
a CLI for enumerating and selectively running the headless cells. It
is not a GUI and never executes real-window cases.

```sh
cargo run -p ruviform-plugin-dock-test --bin plugin-dock-console -- --list
cargo run -p ruviform-plugin-dock-test --bin plugin-dock-console -- \
  --run --case view-mount --out evidence/rf631/plugin-dock-console.jsonl
```

- `--list` — one JSON row per `HEADLESS_MATRIX.project()` cell
  (`type:"catalog"`, `case`, `scenario`, `driver`, `required`,
  `runnable`, `missing`). This is a **capability projection, not
  evidence**: it carries no verdict vocabulary and claims no
  execution.
- `--list --include-real` — appends the four `rf631.plugin-dock-real`
  rows from `real_catalog` with `session_required:true` and
  `lane:"real-window-dock"` (`runnable`/`missing` are `null`).
  Requires the `real-dock` feature; without it the option is refused
  with exit 2 rather than silently dropping the rows.
- `--run [--case <id>]... [--scenario <id>]... [--driver <name>]...`
  — filters `HEADLESS_MATRIX.cells()` (repeatable flags: OR within a
  flag, AND across flags; `--driver` takes the kebab-case name
  `fake`/`plugin-host-probe`) and executes the selection through
  `scenario::run_selected` — the same `run_entry` catalog path as the
  lane. JSONL output uses run id `rf631-plugin-dock` with lane
  `plugin-dock-console`, so a filtered run is never mistaken for
  full-lane evidence. `--out <path>` writes to a file (stdout
  otherwise).

Fail-closed semantics: unknown selector values, selector combinations
matching zero headless cells, CLI misuse, and any attempt to run a
real-lane case/scenario/driver all exit 2 with nothing executed (real
execution stays with the opt-in `plugin-dock-real-scenario` lane — the
console builds no second executor). A `Fail` verdict exits 1;
`NotRun`/`Unsupported`/`Blocked` are recorded outcomes and exit 0. A
selected case whose `artifact` prerequisite was not part of the same
selection reports `NotRun` — prerequisites are never fabricated.
