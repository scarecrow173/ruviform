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
