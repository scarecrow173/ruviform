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
| `real-editor-attach` | `Unsupported` while `test_gain`'s `createView` returns null (`HostError::Unsupported`); `Pass` requires a real editor accepting `OpenGui` through the real container lease |
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
mise run test-ui-plugin-dock-real   # builds test_gain cdylib + runs the lane
cargo run -p ruviform-plugin-dock-test --features real-dock \
  --bin plugin-dock-real-scenario -- --out evidence/rf631/plugin-dock-real-scenario.jsonl
```

Manual/opt-in — not a CI-required gate; requires an interactive
desktop session.
